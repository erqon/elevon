use std::path::PathBuf;

use anyhow::Result;
use bollard::query_parameters::InspectContainerOptionsBuilder;
use elevon_fs::agent::get_socket_path;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::api::db::AgentDb;
use crate::api::db::models::{Deployment, DeploymentStatus};
use crate::env::ElevonEnv;
use crate::proxy::types::{AgentEvent, RouteConfig, RouteState};

pub struct AppState {
    pub agent_db: AgentDb,
    pub docker: bollard::Docker,
    pub env: RwLock<ElevonEnv>,
    pub socket_client: SocketClient,
}

impl AppState {
    pub async fn new(env: &ElevonEnv) -> Result<Self> {
        let agent_db = AgentDb::new(env.turso_remote_url.as_deref()).await?;
        let docker = bollard::Docker::connect_with_defaults()?;
        let env = RwLock::new(env.clone());

        let state = Self {
            agent_db,
            docker,
            env,
            socket_client: SocketClient::new(),
        };

        state.check_running_containers().await?;

        Ok(state)
    }

    async fn check_running_containers(&self) -> Result<()> {
        let db_active_containers = self.get_running_route_containers().await?;
        let mut db = self.agent_db.db.clone();

        for active_container in db_active_containers {
            let docker_container = self
                .docker
                .inspect_container(&active_container.container_id, None)
                .await?;

            if let Some(state) = docker_container.state
                && let Some(running) = state.running
                && !running
            {
                let id = Uuid::parse_str(&active_container.id)?;

                Deployment::update_by_id(id)
                    .status(DeploymentStatus::Drained)
                    .exec(&mut db)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn get_running_route_containers(&self) -> Result<Vec<RouteConfig>> {
        let mut db = self.agent_db.db.clone();

        let deployments =
            Deployment::filter(Deployment::fields().status().eq(DeploymentStatus::Active))
                .include(Deployment::fields().app())
                .exec(&mut db)
                .await?;

        let mut routes: Vec<RouteConfig> = Vec::new();

        for mut deployment in deployments {
            let app = deployment.app.get();

            let (Some(container_id), Some(domain)) =
                (deployment.container_id.clone(), app.domain.clone())
            else {
                continue;
            };

            let app_name = app.name.clone();
            let project_name = app.project.clone();

            let options = InspectContainerOptionsBuilder::default()
                .size(false)
                .build();

            let container = match self
                .docker
                .inspect_container(&container_id, Some(options))
                .await
            {
                Ok(container) => Some(container),
                Err(_) => {
                    toasty::update!(deployment {
                        status: DeploymentStatus::Drained
                    })
                    .exec(&mut db)
                    .await?;

                    None
                }
            };

            if let Some(container) = container {
                let Some(state) = container.state.and_then(|s| s.running).map(|running| {
                    if running {
                        RouteState::Active
                    } else {
                        RouteState::Draining
                    }
                }) else {
                    continue;
                };

                let route_config = RouteConfig {
                    id: deployment.id.to_string(),
                    project: project_name,
                    name: app_name,
                    domain,
                    port: deployment.port,
                    state,
                    container_id,
                };
                routes.push(route_config);
            } else {
                continue;
            }
        }

        Ok(routes)
    }
}

#[derive(Clone)]
pub struct SocketClient {
    pub socket_path: PathBuf,
}

impl SocketClient {
    pub fn new() -> Self {
        Self {
            socket_path: get_socket_path(false),
        }
    }

    pub async fn connect(&self) -> Result<UnixStream> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        Ok(stream)
    }

    pub async fn send(&self, mut stream: UnixStream, event: AgentEvent) -> Result<()> {
        let payload = serde_json::to_vec(&serde_json::json!(event))?;
        stream.write_all(&payload).await?;
        Ok(())
    }
}

impl Default for SocketClient {
    fn default() -> Self {
        Self::new()
    }
}
