use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use elevon_contracts::deploy::AppRole;
use pingora::lb::{LoadBalancer, health_check::TcpHealthCheck, selection::RoundRobin};

use crate::{
    env::ElevonEnv,
    proxy::{
        tls::DynamicCert,
        types::{
            AppData, AppState as AppStateType, BackendRuntime, RouteBackendRuntime, RouteData,
        },
    },
};

const API_BASE_URL: &str = "http://localhost:3000";

pub struct AgentState {
    pub domain: String,
    pub port: u16,
}

pub struct ProxyState {
    pub agent: AgentState,
    pub docker: Arc<bollard::Docker>,
    pub routes: ArcSwap<HashMap<String, Vec<RouteConfig>>>,
    pub lbs: ArcSwap<HashMap<String, Arc<LoadBalancer<RoundRobin>>>>,
    pub runtime: DashMap<String, Arc<BackendRuntime>>,
    pub dynamic_cert: DynamicCert,
    api_client: reqwest::Client,
}

impl ProxyState {
    pub fn new(env: &ElevonEnv) -> anyhow::Result<Arc<Self>> {
        let dynamic_cert = DynamicCert::new();

        dynamic_cert.setup_agent_certs(&env.agent_domain)?;

        let agent_state = AgentState {
            domain: env.agent_domain.clone(),
            port: 3000,
        };

        Ok(Arc::new(ProxyState {
            agent: agent_state,
            docker: Arc::new(bollard::Docker::connect_with_defaults().unwrap()),
            routes: ArcSwap::from_pointee(HashMap::new()),
            lbs: ArcSwap::from_pointee(HashMap::new()),
            runtime: DashMap::new(),
            dynamic_cert,
            api_client: reqwest::Client::new(),
        }))
    }

    pub fn upsert_route(&self, route: RouteData) {
        if route.domain == self.agent.domain {
            tracing::warn!(
                domain = %route.domain,
                "refusing application route on reserved agent domain"
            );
            return;
        }

        // if route.state != RouteState::Active {
        //     tracing::debug!(
        //         domain = %route.domain,
        //         state = ?config.state,
        //         "skipping inactive application route"
        //     );
        //     return;
        // }

        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();
            let backends = next.entry(route.domain.clone()).or_default();

            self.dynamic_cert
                .add_cert(
                    &config.project,
                    &config.name,
                    config.domain.clone(),
                    Some(true),
                )
                .unwrap_or_else(|err| {
                    tracing::error!(
                        domain = %config.domain,
                        state = ?config.state,
                        error = %err,
                        "failed to save/update certificates"
                    )
                });

            if let Some(existing) = backends.iter_mut().find(|route| route.id == config.id) {
                *existing = config.clone();
            } else {
                backends.push(config.clone());
            }

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

    fn rebuild_load_balancer(&self, domain: String) {
        let routes = self.routes.load();

        let addrs: Vec<String> = routes
            .get(&domain)
            .into_iter()
            .flatten()
            .filter(|route| route.state == AppStateType::Active)
            .map(|route| format!("127.0.0.1:{}", route.port))
            .collect();

        if addrs.is_empty() {
            self.lbs.rcu(|current| {
                let mut next = (**current).clone();
                next.remove(&domain);
                next
            });
            return;
        }

        let mut lb = LoadBalancer::<RoundRobin>::try_from_iter(addrs)
            .expect("active application routes must have valid addresses");

        lb.set_health_check(TcpHealthCheck::new());
        lb.parallel_health_check = true;

        let lb = Arc::new(lb);

        self.lbs.rcu(|current| {
            let mut next = (**current).clone();
            next.insert(domain.to_string(), lb.clone());
            next
        });
    }

    pub fn drain_app(&self, app: AppData) {
        let mut backend_runtime = BackendRuntime {
            container_id: app.container_id,
            state: AppStateType::Draining,
            ..Default::default()
        };

        if let Some(route) = app.route {
            self.routes.rcu(|current| {
                let mut next = current.as_ref().clone();
                let backends = next.entry(route.domain).or_default();

                for backend in backends.iter_mut() {
                    if backend.container_id == app.container_id {
                        backend.state = AppStateType::Draining;
                    }
                }

                next
            });
            self.rebuild_load_balancer(route.domain);

            let inflight = self
                .runtime
                .get(&app.container_id)
                .map(|r| {
                    r.route
                        .map(|ro| ro.inflight.load(std::sync::atomic::Ordering::Relaxed))
                        .unwrap_or(0)
                })
                .unwrap_or(0);

            // TODO: Suspicious 
            let drain_started_at = self
                .runtime
                .get(&app.container_id)
                .and_then(|r| r.route.and_then(|ro| ro.drain_started_at))
                .or(Some(Instant::now()));

            let route_backend_runtime = RouteBackendRuntime {
                port: route.port,
                inflight: AtomicUsize::new(inflight),
                drain_started_at,
            };

            backend_runtime.route = Some(route_backend_runtime);
        }

        self.runtime
            .insert(app.container_id.clone(), Arc::new(backend_runtime));

        tracing::info!(%app.container_id, "route marked as draining");
    }

    pub fn get_backend_by_port(&self, port: u16) -> Option<Arc<BackendRuntime>> {
        self.runtime
            .iter()
            .find(|r| r.value().route.as_ref().is_some_and(|ro| ro.port == port))
            .map(|r| r.value().clone())
    }

    pub fn increase_inflight_count(&self, container_id: &str) {
        if let Some(route) = self
            .runtime
            .get(container_id)
            .as_ref()
            .and_then(|r| r.route.as_ref())
        {
            route.inflight.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn decrease_inflight_count(&self, container_id: &str) {
        if let Some(route) = self
            .runtime
            .get(container_id)
            .as_ref()
            .and_then(|r| r.route.as_ref())
        {
            route.inflight.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn ready_to_terminate(&self, grace_period: Duration) -> Vec<(String, u16)> {
        self.runtime
            .iter()
            .filter_map(|r| {
                let runtime = r.value();
                if runtime.state != AppStateType::Draining {
                    return None;
                }

                let route = runtime.route.as_ref()?;

                let inflight = route.inflight.load(std::sync::atomic::Ordering::Relaxed);
                let past_grace = route
                    .drain_started_at
                    .is_some_and(|t| t.elapsed() >= grace_period);

                if inflight == 0 || past_grace {
                    Some((r.key().clone(), route.port))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn remove_backend(&self, container_id: &str) {
        self.runtime.remove(container_id);

        let domains: Vec<String> = self
            .routes
            .load()
            .iter()
            .filter(|(_, routes)| {
                routes
                    .iter()
                    .any(|route| route.container_id == container_id)
            })
            .map(|(domain, _)| domain.clone())
            .collect();

        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();

            for backends in next.values_mut() {
                backends.retain(|backend| backend.container_id != container_id);
            }

            next.retain(|_, backends| !backends.is_empty());
            next
        });

        for domain in domains {
            self.rebuild_load_balancer(domain);
        }
    }

    pub async fn load_conainters(&self) -> Result<()> {
        for _ in 0..20 {
            let response = self
                .api_client
                .get(format!("{API_BASE_URL}/proxy/containers"))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let containers: Vec<AppData> = resp.json().await?;
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
