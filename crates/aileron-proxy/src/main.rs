use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;

pub struct LB(Arc<LoadBalancer<RoundRobin>>);

#[async_trait]
impl ProxyHttp for LB {
    /// For this small example, we don't need context storage
    type CTX = ();
    fn new_ctx(&self) -> () {
        ()
    }

    async fn upstream_peer(&self, session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        let upstream = self
            .0
            .select(b"", 256) // hash doesn't matter for round robin
            .unwrap();

        let req = session.req_header();
        let host = session
            .get_header("host")
            .map(|v| v.to_str().unwrap_or_default())
            .unwrap_or("-");

        println!("upstream peer is: {upstream:?}");
        println!("request {} {} (Host: {host})", req.method, req.uri);

        let peer = Box::new(HttpPeer::new(upstream, false, String::new()));
        Ok(peer)
    }
}

fn main() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let mut upstreams =
        LoadBalancer::try_from_iter(["127.0.0.1:3000", "127.0.0.1:3001", "127.0.0.1:3002"])
            .unwrap();

    let hc = TcpHealthCheck::new();
    upstreams.set_health_check(hc);
    upstreams.health_check_frequency = Some(std::time::Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let upstreams = background.task();

    let mut lb = http_proxy_service(&my_server.configuration, LB(upstreams));
    lb.add_tcp("0.0.0.0:6188");

    my_server.add_service(background);
    my_server.add_service(lb);

    my_server.run_forever();
}
