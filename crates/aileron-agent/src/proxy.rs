mod lb;
mod socket;
mod state;

use std::sync::Arc;

use arc_swap::ArcSwap;
use pingora::{
    proxy::http_proxy_service, server::Server, services::background::background_service,
};

use crate::proxy::{
    lb::LB,
    socket::SocketControl,
    state::{ProxyState, RoutingTable},
};

pub fn run_proxy() {
    let proxy_state = Arc::new(ProxyState {
        routes: ArcSwap::from_pointee(RoutingTable::new()),
    });

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let mut lb = http_proxy_service(
        &server.configuration,
        LB {
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

    server.add_service(lb);
    server.add_service(control);
    server.run_forever();
}
