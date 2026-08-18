mod socket;
mod state;
pub mod types;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use elevon_http::runtime::run_async;
use futures::stream::{self, StreamExt};
use pingora::{
    Error, ErrorType, Result,
    http::ResponseHeader,
    protocols::l4::socket::SocketAddr,
    proxy::{ProxyHttp, Session, http_proxy_service},
    server::{Server, ShutdownWatch},
    services::background::{BackgroundService, background_service},
    upstreams::peer::HttpPeer,
};

use crate::proxy::{socket::SocketControl, state::ProxyState};

pub fn run_proxy() {
    let proxy_state = ProxyState::new();

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let mut lb = http_proxy_service(
        &server.configuration,
        Proxy {
            state: proxy_state.clone(),
        },
    );
    lb.add_tcp("0.0.0.0:6188");

    let control = background_service(
        "socket control",
        SocketControl {
            state: proxy_state.clone(),
        },
    );

    let lb_health_check = background_service(
        "lb health check",
        LbHealthCheck {
            state: proxy_state.clone(),
        },
    );

    let drain_janitor = background_service(
        "drain janitor",
        DrainJanitor {
            state: proxy_state.clone(),
        },
    );

    let cloned_state = proxy_state.clone();
    std::thread::spawn(move || {
        run_async(async move {
            if let Err(err) = cloned_state.load_conainters().await {
                tracing::warn!("initial container load failed: {err}");
            }
        })
    });

    server.add_service(lb);
    server.add_service(control);
    server.add_service(lb_health_check);
    server.add_service(drain_janitor);
    server.run_forever();
}

pub struct RequestCtx {
    pub container_id: Option<String>,
}

pub struct Proxy {
    pub state: Arc<ProxyState>,
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = RequestCtx;
    fn new_ctx(&self) -> Self::CTX {
        RequestCtx { container_id: None }
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let host = session
            .get_header("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");

        let lbs = self.state.lbs.load();
        let Some(lb) = lbs.get(host) else {
            return Error::e_explain(ErrorType::HTTPStatus(404), "no route for host");
        };

        let upstream = lb
            .select(b"", 256)
            .ok_or_else(|| Error::explain(ErrorType::HTTPStatus(502), "no healthy upstream"))?;

        let selected_port = match upstream.addr {
            SocketAddr::Inet(addr) => Some(addr.port()),
            _ => None,
        };
        if let Some(port) = selected_port {
            let selected = self.state.get_backend_by_port(port);
            if let Some(backend) = selected {
                ctx.container_id = Some((*backend.container_id).to_string());
            }
        }

        Ok(Box::new(HttpPeer::new(upstream, false, String::new())))
    }

    async fn request_filter(&self, _session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        if let Some(container_id) = ctx.container_id.clone() {
            self.state.increase_inflight_count(&container_id);
        }

        Ok(false)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        _upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if let Some(container_id) = ctx.container_id.clone() {
            self.state.decrease_inflight_count(&container_id);
        }

        Ok(())
    }
}

pub struct LbHealthCheck {
    pub state: Arc<ProxyState>,
}

#[async_trait]
impl BackgroundService for LbHealthCheck {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = shutdown.changed() => break,
                    _ = tick.tick() => {
                        let lbs: Vec<_> = self.state.lbs.load_full().values().cloned().collect();
                        stream::iter(lbs)
                            .for_each_concurrent(32, |lb| async move {
                                lb.backends().run_health_check(true).await;
                            })
                            .await;
                    }
            }
        }
    }
}

pub struct DrainJanitor {
    pub state: Arc<ProxyState>,
}

#[async_trait]
impl BackgroundService for DrainJanitor {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                _ = shutdown.changed() => break,
                _ = tick.tick() => {
                    let grace = Duration::from_secs(30);
                    for (container_id, _port) in self.state.ready_to_terminate(grace) {
                        if let Err(err) = self.state.docker
                            .stop_container(&container_id, None)
                            .await
                        {
                            tracing::warn!(%container_id, %err, "failed to stop container");
                            continue;
                        }

                        let _ = self.state.docker
                            .remove_container(&container_id, None)
                            .await;

                        self.state.remove_backend(&container_id);
                        tracing::info!(%container_id, "drained container terminated");
                        let routes = self.state.routes.load();
                        tracing::info!("current routes: {:?}", routes);
                    }
                }
            }
        }
    }
}
