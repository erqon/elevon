use std::sync::Arc;

use anyhow::Result;
use bollard::plugin::ContainerStateStatusEnum;
use elevon_contracts::deploy::WebApp;
use tokio::sync::RwLock;

use crate::api::db::AgentDb;
use crate::api::db::models::{Deployment, DeploymentStatus};
use crate::env::ElevonEnv;
use crate::proxy::types::DeployAppData;
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
    pub async fn new(env: &ElevonEnv) -> Result<Self> {
        let db = AgentDb::new(env.turso_remote_url.as_deref()).await?;
        let docker = bollard::Docker::connect_with_defaults()?;
        let env = RwLock::new(env.clone());

        let proxy_socket = Arc::new(Socket::new(SocketType::Proxy, true)?);
        let api_socket = Arc::new(Socket::new(SocketType::Api, true)?);

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
        // This is going to update the values anyway so we can just use it
        self.get_running_route_containers().await?;
        Ok(())
    }

    // TODO: Fix it so it also returns non web app containers and puts them into `runtime`
    pub async fn get_running_route_containers(&self) -> Result<Vec<DeployAppData>> {
        let mut db = self.db.get();

        // TODO: Add some kind of background checker for Restaring and Pending deployments to check their health and remove on failure.
        let deployments = Deployment::filter(
            Deployment::fields()
                .status()
                .eq(DeploymentStatus::Active)
                .or(Deployment::fields().status().eq(DeploymentStatus::Pending))
                .or(Deployment::fields()
                    .status()
                    .eq(DeploymentStatus::Restarting)),
        )
        .include(Deployment::fields().app())
        .exec(&mut db)
        .await?;

        let mut routes: Vec<DeployAppData> = Vec::new();

        for mut deployment in deployments {
            let app = deployment.app.get();

            let (Some(container_id), Some(web_deployment)) =
                (deployment.container_id.clone(), deployment.web.clone())
            else {
                continue;
            };

            let app_name = app.name.clone();
            let project_name = app.project.clone();

            let container_status = self.check_container_state(&container_id).await;

            if container_status != DeploymentStatus::Active {
                deployment
                    .update()
                    .status(container_status)
                    .exec(&mut db)
                    .await?;
                continue;
            }

            let web_app = Some(WebApp {
                domain: web_deployment.domain,
                port: web_deployment.port,
            });

            let route_config = DeployAppData {
                id: deployment.id.to_string(),
                project: project_name,
                name: app_name,
                status: container_status,
                container_id,
                web_app,
            };

            routes.push(route_config);
        }

        Ok(routes)
    }

    async fn check_container_state(&self, container_id: &str) -> DeploymentStatus {
        let Ok(container) = self.docker.inspect_container(container_id, None).await else {
            return DeploymentStatus::Failed;
        };

        let Some(state) = container.state else {
            return DeploymentStatus::Failed;
        };
        let Some(status) = state.status else {
            return DeploymentStatus::Failed;
        };

        match status {
            ContainerStateStatusEnum::RUNNING => DeploymentStatus::Active,
            ContainerStateStatusEnum::RESTARTING => DeploymentStatus::Restarting,
            ContainerStateStatusEnum::STOPPING | ContainerStateStatusEnum::EXITED => {
                DeploymentStatus::Failed
            }
            _ => DeploymentStatus::Drained,
        }
    }
}
