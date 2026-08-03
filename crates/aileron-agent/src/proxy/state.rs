use std::{collections::HashMap, sync::Arc};

use arc_swap::ArcSwap;
use pingora::lb::{LoadBalancer, health_check::TcpHealthCheck, selection::RoundRobin};

#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub id: String,
    pub host: String,
    pub port: u16,
}

pub struct ProxyState {
    pub routes: ArcSwap<HashMap<String, Vec<RouteConfig>>>,
    pub lbs: ArcSwap<HashMap<String, Arc<LoadBalancer<RoundRobin>>>>,
}

impl ProxyState {
    pub fn upsert_route(&self, name: impl Into<String>, config: RouteConfig) {
        let name = name.into();
        self.routes.rcu(|current| {
            let mut next = (**current).clone();
            let backends = next.entry(name.clone()).or_default();

            if let Some(existing) = backends.iter_mut().find(|e| e.id == config.id) {
                *existing = config.clone();
            } else {
                backends.push(config.clone());
            }

            next
        });

        let routes = self.routes.load();
        let backends = routes.get(&name).cloned().unwrap_or_default();
        let addrs = backends.iter().map(|b| format!("{}:{}", b.host, b.port));

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
