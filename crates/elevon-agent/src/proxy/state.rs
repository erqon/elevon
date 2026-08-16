use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
    time::Instant,
};

use arc_swap::ArcSwap;
use dashmap::DashMap;
use pingora::lb::{LoadBalancer, health_check::TcpHealthCheck, selection::RoundRobin};

use crate::proxy::types::{BackendRuntime, RouteConfig, RouteState};

pub struct ProxyState {
    pub routes: ArcSwap<HashMap<String, Vec<RouteConfig>>>,
    pub lbs: ArcSwap<HashMap<String, Arc<LoadBalancer<RoundRobin>>>>,
    pub runtime: DashMap<String, BackendRuntime>,
}

impl ProxyState {
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
                        .or_insert_with(BackendRuntime::default);
                }
                RouteState::Draining => {
                    if let Some(mut runtime) = self.runtime.get_mut(&backend.container_id) {
                        runtime.state = RouteState::Draining;
                        if runtime.drain_started_at.is_none() {
                            runtime.drain_started_at = Some(Instant::now());
                        }
                    } else {
                        self.runtime.insert(
                            backend.container_id.clone(),
                            BackendRuntime {
                                state: RouteState::Draining,
                                inflight: AtomicUsize::new(0),
                                drain_started_at: Some(Instant::now()),
                            },
                        );
                    }
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
}
