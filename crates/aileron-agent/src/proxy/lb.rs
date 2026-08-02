use std::sync::Arc;

use async_trait::async_trait;
use pingora::{
    Error, ErrorType, Result,
    proxy::{ProxyHttp, Session},
    upstreams::peer::HttpPeer,
};

use crate::proxy::state::ProxyState;

pub struct LB {
    pub state: Arc<ProxyState>,
}

#[async_trait]
impl ProxyHttp for LB {
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

        let routes = self.state.routes.load();
        let Some(cfg) = routes.get(host) else {
            return Error::e_explain(ErrorType::HTTPStatus(404), "no route for host");
        };

        let addr = format!("{}:{}", cfg.host, cfg.port);
        tracing::info!("request host={host} -> {addr}");

        Ok(Box::new(HttpPeer::new(addr, false, String::new())))
    }
}
