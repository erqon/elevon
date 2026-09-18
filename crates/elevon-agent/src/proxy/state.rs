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
use pingora::lb::{LoadBalancer, health_check::TcpHealthCheck, selection::RoundRobin};

use crate::{
    api::event::{ApiSocketEvent, ApiSocketEventResponse},
    env::ElevonEnv,
    proxy::{
        tls::DynamicCert,
        types::{BackendRuntime, DeployAppData, DeployAppState, RouteBackendRuntime},
    },
    socket::{Socket, SocketType},
};

pub struct AgentState {
    pub domain: String,
    pub port: u16,
}

pub struct ProxyState {
    pub agent: AgentState,
    pub docker: Arc<bollard::Docker>,
    pub routes: ArcSwap<HashMap<String, Vec<DeployAppData>>>,
    pub lbs: ArcSwap<HashMap<String, Arc<LoadBalancer<RoundRobin>>>>,
    pub runtime: DashMap<String, Arc<BackendRuntime>>,
    pub dynamic_cert: DynamicCert,
    pub proxy_socket: Arc<Socket>,
    pub api_socket: Arc<Socket>,
}

impl ProxyState {
    pub fn new(env: &ElevonEnv) -> anyhow::Result<Arc<Self>> {
        let dynamic_cert = DynamicCert::new();

        dynamic_cert.setup_agent_certs(&env.agent_domain)?;

        let agent_state = AgentState {
            domain: env.agent_domain.clone(),
            port: env.agent_port,
        };

        let proxy_socket = Socket::new(SocketType::Proxy)?;
        let api_socket = Socket::new(SocketType::Api)?;

        Ok(Arc::new(ProxyState {
            agent: agent_state,
            docker: Arc::new(bollard::Docker::connect_with_defaults().unwrap()),
            routes: ArcSwap::from_pointee(HashMap::new()),
            lbs: ArcSwap::from_pointee(HashMap::new()),
            runtime: DashMap::new(),
            dynamic_cert,
            proxy_socket: Arc::new(proxy_socket),
            api_socket: Arc::new(api_socket),
        }))
    }

    pub async fn load_conainters(&self) -> Result<()> {
        for _ in 0..20 {
            let response: Result<Option<ApiSocketEventResponse>> = self
                .api_socket
                .send_and_receive(ApiSocketEvent::RunningContainers)
                .await;

            match response {
                Ok(Some(res)) => {
                    if let ApiSocketEventResponse::RunningContainers(containers) = res {
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
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!(%err, "proxy containers request error, retrying");
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }

        anyhow::bail!("failed to load initial containers from API");
    }

    pub fn upsert_route(&self, route: DeployAppData) {
        let Some(web_app) = route.web_app.clone() else {
            tracing::warn!(app = %&route.name, "refusing application route on non web app");
            return;
        };

        if web_app.domain == self.agent.domain {
            tracing::warn!(
                domain = %web_app.domain,
                "refusing application route on reserved agent domain"
            );
            return;
        }

        if route.state != DeployAppState::Active {
            tracing::debug!(
                domain = %web_app.domain,
                state = ?route.state,
                "skipping inactive application route"
            );
            return;
        }

        self.routes.rcu(|current| {
            let mut next = current.as_ref().clone();
            let backends = next.entry(web_app.domain.clone()).or_default();

            self.dynamic_cert
                .add_cert(&route.project, &route.name, web_app.domain.clone())
                .unwrap_or_else(|err| {
                    tracing::error!(
                        domain = %web_app.domain,
                        state = ?route.state,
                        error = %err,
                        "failed to save/update certificates"
                    )
                });

            if let Some(existing) = backends.iter_mut().find(|a| a.id == route.id) {
                *existing = route.clone();
            } else {
                backends.push(route.clone());
            }

            next
        });

        let routes = self.routes.load();
        let backends = routes.get(&web_app.domain).cloned().unwrap_or_default();

        let addrs: Vec<String> = backends
            .iter()
            .filter(|b| b.state == DeployAppState::Active)
            .map(|_| format!("127.0.0.1:{}", web_app.port))
            .collect();

        let mut lb = LoadBalancer::<RoundRobin>::try_from_iter(addrs).expect("valid backends");
        lb.set_health_check(TcpHealthCheck::new());
        lb.parallel_health_check = true;
        let lb = Arc::new(lb);

        self.lbs.rcu(|current| {
            let mut next = (**current).clone();
            next.insert(web_app.domain.clone(), lb.clone());
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
            .filter_map(|route| {
                let web_app = route.web_app.clone()?;

                if route.state != DeployAppState::Active {
                    return None;
                }

                Some(format!("127.0.0.1:{}", web_app.port))
            })
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

    pub fn drain_app(&self, app: DeployAppData) {
        let mut backend_runtime = BackendRuntime {
            container_id: app.container_id.clone(),
            state: DeployAppState::Draining,
            ..Default::default()
        };

        if let Some(web_app) = app.web_app {
            self.routes.rcu(|current| {
                let mut next = current.as_ref().clone();
                let backends = next.entry(web_app.domain.clone()).or_default();

                for backend in backends.iter_mut() {
                    if backend.container_id == app.container_id.clone() {
                        backend.state = DeployAppState::Draining;
                    }
                }

                next
            });
            self.rebuild_load_balancer(web_app.domain);

            let inflight = self
                .runtime
                .get(&app.container_id)
                .map(|r| {
                    r.route
                        .as_ref()
                        .map(|ro| ro.inflight.load(std::sync::atomic::Ordering::Relaxed))
                        .unwrap_or(0)
                })
                .unwrap_or(0);

            let route_backend_runtime = RouteBackendRuntime {
                port: web_app.port,
                inflight: AtomicUsize::new(inflight),
                drain_started_at: Some(Instant::now()),
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

    pub fn ready_to_terminate(&self, grace_period: Duration) -> Vec<String> {
        self.runtime
            .iter()
            .filter_map(|r| {
                let runtime = r.value();
                if runtime.state != DeployAppState::Draining {
                    return None;
                }

                let Some(route) = runtime.route.as_ref() else {
                    return Some(r.key().clone());
                };

                let inflight = route.inflight.load(std::sync::atomic::Ordering::Relaxed);
                let past_grace = route
                    .drain_started_at
                    .is_some_and(|t| t.elapsed() >= grace_period);

                if inflight == 0 || past_grace {
                    Some(r.key().clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn remove_backend(&self, container_id: &str) {
        let backend_runtime = self.runtime.remove(container_id);

        if let Some((_, backend_runtime)) = backend_runtime {
            if backend_runtime.route.is_none() {
                return;
            }

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
    }
}
