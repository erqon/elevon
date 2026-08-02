use std::collections::HashMap;

use arc_swap::ArcSwap;

#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub host: String,
    pub port: u16,
}

pub type RoutingTable = HashMap<String, RouteConfig>;

pub struct ProxyState {
    pub routes: ArcSwap<RoutingTable>,
}

impl ProxyState {
    // pub fn set_routes(&self, table: RoutingTable) {
    //     self.routes.store(Arc::new(table));
    // }

    pub fn upsert_route(&self, name: impl Into<String>, config: RouteConfig) {
        let name = name.into();
        self.routes.rcu(|current| {
            let mut next = (**current).clone();
            next.insert(name.clone(), config.clone());
            next
        });
    }
}
