mod socket;
mod state;
mod types;

use std::{collections::HashMap, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use pingora::{
    Error, ErrorType, Result,
    proxy::{ProxyHttp, Session, http_proxy_service},
    server::{Server, ShutdownWatch},
    services::background::{BackgroundService, background_service},
    upstreams::peer::HttpPeer,
};

use crate::proxy::{socket::SocketControl, state::ProxyState};

pub struct Proxy {
    pub state: Arc<ProxyState>,
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(&self, session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
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

        Ok(Box::new(HttpPeer::new(upstream, false, String::new())))
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

pub fn run_proxy() {
    let proxy_state = Arc::new(ProxyState {
        routes: ArcSwap::from_pointee(HashMap::new()),
        lbs: ArcSwap::from_pointee(HashMap::new()),
    });

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

    server.add_service(lb);
    server.add_service(control);
    server.add_service(lb_health_check);
    server.run_forever();
}
