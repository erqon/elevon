use std::sync::Arc;

use anyhow::Result;
use bollard::query_parameters::InspectContainerOptionsBuilder;
use elevon_contracts::deploy::WebApp;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::api::db::AgentDb;
use crate::api::db::models::{Deployment, DeploymentStatus};
use crate::env::ElevonEnv;
use crate::proxy::types::{DeployAppData, DeployAppState};
use crate::socket::{Socket, SocketType};

pub type SharedApiState = Arc<ApiState>;

pub struct ApiState {
    pub db: AgentDb,
    pub docker: bollard::Docker,
    pub env: RwLock<ElevonEnv>,
    pub proxy_socket: Arc<Socket>,
    pub api_socket: Arc<Socket>,
}

impl ApiState {
    pub async fn new(agent_name: &str, env: &ElevonEnv) -> Result<Self> {
        let db = AgentDb::new(agent_name, env.turso_remote_url.as_deref()).await?;
        let docker = bollard::Docker::connect_with_defaults()?;
        let env = RwLock::new(env.clone());

        let proxy_socket = Arc::new(Socket::new(agent_name, SocketType::Proxy)?);
        let api_socket = Arc::new(Socket::new(agent_name, SocketType::Api)?);

        let state = Self {
            db,
            docker,
            env,
            proxy_socket,
            api_socket,
        };

        state.check_running_containers().await?;

        Ok(state)
    }

    async fn check_running_containers(&self) -> Result<()> {
        let db_active_containers = self.get_running_route_containers().await?;
        let mut db = self.db.get();

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

    // TODO: Fix it so it also returns non web app containers and puts them into `runtime`
    pub async fn get_running_route_containers(&self) -> Result<Vec<DeployAppData>> {
        let mut db = self.db.get();

        let deployments =
            Deployment::filter(Deployment::fields().status().eq(DeploymentStatus::Active))
                .include(Deployment::fields().app())
                .exec(&mut db)
                .await?;

        let mut routes: Vec<DeployAppData> = Vec::new();

        for mut deployment in deployments {
            let app = deployment.app.get();

            let (Some(container_id), Some(domain)) =
                (deployment.container_id.clone(), app.domain.clone())
            else {
                continue;
            };

            let app_name = app.name.clone();
            let agent_name = app.agent.clone();
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
                        DeployAppState::Active
                    } else {
                        DeployAppState::Draining
                    }
                }) else {
                    continue;
                };

                if state == DeployAppState::Draining {
                    continue;
                }

                let web_app = Some(WebApp {
                    domain,
                    port: deployment.port,
                });

                let route_config = DeployAppData {
                    id: deployment.id.to_string(),
                    agent: agent_name,
                    project: project_name,
                    name: app_name,
                    state,
                    container_id,
                    web_app,
                };
                routes.push(route_config);
            } else {
                continue;
            }
        }

        Ok(routes)
    }
}
