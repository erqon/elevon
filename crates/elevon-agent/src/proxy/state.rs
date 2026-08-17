use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
    time::{Duration, Instant},
};

use arc_swap::ArcSwap;
use dashmap::DashMap;
use pingora::lb::{LoadBalancer, health_check::TcpHealthCheck, selection::RoundRobin};

use crate::proxy::types::{BackendRuntime, RouteConfig, RouteState};

pub struct ProxyState {
    pub docker: bollard::Docker,
    pub routes: ArcSwap<HashMap<String, Vec<RouteConfig>>>,
    pub lbs: ArcSwap<HashMap<String, Arc<LoadBalancer<RoundRobin>>>>,
    pub runtime: DashMap<String, Arc<BackendRuntime>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            docker: bollard::Docker::connect_with_defaults().unwrap(),
            routes: ArcSwap::from_pointee(HashMap::new()),
            lbs: ArcSwap::from_pointee(HashMap::new()),
            runtime: DashMap::new(),
        }
    }

    pub fn upsert_route(&self, config: RouteConfig) {
        let name = config.domain.clone();

        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();
            let backends = next.entry(name.clone()).or_default();

            for backend in backends.iter_mut() {
                if backend.name == config.name && backend.id != config.id {
                    backend.state = RouteState::Draining;
                }
            }

            backends.push(config.clone());
            next
        });

        let routes = self.routes.load();
        let backends = routes.get(&name).cloned().unwrap_or_default();

        for backend in &backends {
            match backend.state {
                RouteState::Active => {
                    self.runtime
                        .entry(backend.container_id.clone())
                        .or_insert_with(|| {
                            BackendRuntime::new(backend.container_id.clone(), backend.port)
                        });
                }
                RouteState::Draining => {
                    let inflight = self
                        .runtime
                        .get(&backend.container_id)
                        .map(|r| r.inflight.load(std::sync::atomic::Ordering::Relaxed))
                        .unwrap_or(0);

                    let drain_started_at = self
                        .runtime
                        .get(&backend.container_id)
                        .and_then(|r| r.drain_started_at)
                        .or(Some(Instant::now()));

                    self.runtime.insert(
                        backend.container_id.clone(),
                        Arc::new(BackendRuntime {
                            container_id: backend.container_id.clone(),
                            state: RouteState::Draining,
                            port: backend.port,
                            inflight: AtomicUsize::new(inflight),
                            drain_started_at,
                        }),
                    );
                }
            }
        }

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
}
