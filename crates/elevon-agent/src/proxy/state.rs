use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
    time::{Duration, Instant},
};

use anyhow::Result;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use pingora::lb::{LoadBalancer, health_check::TcpHealthCheck, selection::RoundRobin};

use crate::proxy::types::{BackendRuntime, RouteConfig, RouteState};

const API_BASE_URL: &'static str = "http://localhost:3000";

pub struct ProxyState {
    pub docker: Arc<bollard::Docker>,
    pub routes: ArcSwap<HashMap<String, Vec<RouteConfig>>>,
    pub lbs: ArcSwap<HashMap<String, Arc<LoadBalancer<RoundRobin>>>>,
    pub runtime: DashMap<String, Arc<BackendRuntime>>,
    api_client: reqwest::Client,
}

impl ProxyState {
    pub fn new() -> Arc<Self> {
        Arc::new(ProxyState {
            docker: Arc::new(bollard::Docker::connect_with_defaults().unwrap()),
            routes: ArcSwap::from_pointee(HashMap::new()),
            lbs: ArcSwap::from_pointee(HashMap::new()),
            runtime: DashMap::new(),
            api_client: reqwest::Client::new(),
        })
    }

    pub fn upsert_route(&self, config: RouteConfig) {
        let name = config.domain.clone();

        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();
            let backends = next.entry(name.clone()).or_default();
            backends.push(config.clone());
            next
        });

        let routes = self.routes.load();
        let backends = routes.get(&name).cloned().unwrap_or_default();

        let addrs: Vec<String> = backends
            .iter()
            .filter(|b| b.state == RouteState::Active)
            .map(|b| format!("127.0.0.1:{}", b.port))
            .collect();

        let mut lb = LoadBalancer::<RoundRobin>::try_from_iter(addrs).expect("valid backends");
        lb.set_health_check(TcpHealthCheck::new());
        lb.parallel_health_check = true;
        let lb = Arc::new(lb);

        self.lbs.rcu(|current| {
            let mut next = (**current).clone();
            next.insert(name.clone(), lb.clone());
            next
        });

        tracing::info!("Routes changed, current state: {:?}", self.routes.load());
    }

    pub fn drain_route(&self, route: RouteConfig) {
        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();
            let backends = next.entry(route.domain.clone()).or_default();

            for backend in backends.iter_mut() {
                if backend.container_id == route.container_id {
                    backend.state = RouteState::Draining;
                }
            }

            next
        });

        let inflight = self
            .runtime
            .get(&route.container_id)
            .map(|r| r.inflight.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0);

        let drain_started_at = self
            .runtime
            .get(&route.container_id)
            .and_then(|r| r.drain_started_at)
            .or(Some(Instant::now()));

        self.runtime.insert(
            route.container_id.clone(),
            Arc::new(BackendRuntime {
                container_id: route.container_id.clone(),
                state: RouteState::Draining,
                port: route.port,
                inflight: AtomicUsize::new(inflight),
                drain_started_at,
            }),
        );

        tracing::info!(%route.container_id, "route marked as draining");
    }

    pub fn get_backend_by_port(&self, port: u16) -> Option<Arc<BackendRuntime>> {
        self.runtime
            .iter()
            .find(|r| r.value().port == port)
            .map(|r| r.value().clone())
    }

    pub fn increase_inflight_count(&self, container_id: &str) {
        if let Some(runtime) = self.runtime.get(container_id) {
            runtime
                .inflight
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn decrease_inflight_count(&self, container_id: &str) {
        if let Some(runtime) = self.runtime.get(container_id) {
            runtime
                .inflight
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn ready_to_terminate(&self, grace_period: Duration) -> Vec<(String, u16)> {
        self.runtime
            .iter()
            .filter(|r| {
                let runtime = r.value();
                if runtime.state != RouteState::Draining {
                    return false;
                }
                let inflight = runtime.inflight.load(std::sync::atomic::Ordering::Relaxed);
                let past_grace = runtime
                    .drain_started_at
                    .is_some_and(|t| t.elapsed() >= grace_period);

                inflight == 0 || past_grace
            })
            .map(|r| (r.key().clone(), r.value().port))
            .collect()
    }

    pub fn remove_backend(&self, container_id: &str) {
        self.runtime.remove(container_id);

        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();
            for backends in next.values_mut() {
                backends.retain(|b| b.container_id != container_id);
            }
            next
        });
    }

    pub async fn load_conainters(&self) -> Result<()> {
        for _ in 0..20 {
            let response = self
                .api_client
                .get(format!("{API_BASE_URL}/api/proxy/containers"))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let containers: Vec<RouteConfig> = resp.json().await?;
                    if containers.is_empty() {
                        return Ok(());
                    }

                    tracing::info!(
                        "Found {} running containers, upserting them...",
                        containers.len()
                    );

                    for container in containers {
                        self.upsert_route(container);
                    }

                    return Ok(());
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::warn!(status = %status, body, "proxy containers request failed, retrying");
                }
                Err(err) => {
                    tracing::warn!(%err, "proxy containers request error, retrying");
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }

        anyhow::bail!("failed to load initial containers from API");
    }
}
