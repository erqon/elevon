use std::sync::Arc;

use anyhow::Result;
use bollard::plugin::ContainerStateStatusEnum;
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

            let (Some(container_id), Some(web_deployment)) =
                (deployment.container_id.clone(), deployment.web.clone())
            else {
                continue;
            };

            let app_name = app.name.clone();
            let project_name = app.project.clone();

            let options = InspectContainerOptionsBuilder::default()
                .size(false)
                .build();

            let Ok(container) = self
                .docker
                .inspect_container(&container_id, Some(options))
                .await
            else {
                deployment
                    .update()
                    .status(DeploymentStatus::Failed)
                    .exec(&mut db)
                    .await?;

                continue;
            };

            println!("state: {:?}", container.state);

            let Some(state) = container.state else {
                continue;
            };
            let Some(status) = state.status else { continue };

            // TODO: Somehow find a way to replace DeployAppState with DeploymentStatus throughout the code

            let (app_state, app_status) = match status {
                ContainerStateStatusEnum::RUNNING => (DeployAppState::Active, DeploymentStatus::Active),
                ContainerStateStatusEnum::PAUSED => (DeployAppState::Paused, DeploymentStatus::Pending),
                ContainerStateStatusEnum::EXITED => (DeployAppState::Draining, DeploymentStatus::Failed),
                _ => (DeployAppState::Draining, DeploymentStatus::Drained),
            };

            if matches!(app_state, DeployAppState::Draining | DeployAppState::Paused) {
                deployment
                    .update()
                    .status(app_status)
                    .exec(&mut db)
                    .await?;

                continue;
            }

            //             let Some(state) = container.state.and_then(|s| s.running).map(|running| {
            //                 if running {
            //                     DeployAppState::Active
            //                 } else {
            //                     DeployAppState::Draining
            //                 }
            //             }) else {
            //                 deployment
            //                     .update()
            //                     .status(DeploymentStatus::Failed)
            //                     .exec(&mut db)
            //                     .await?;
            //
            //                 continue;
            //             };

            //             if state == DeployAppState::Draining {
            //                 deployment
            //                     .update()
            //                     .status(DeploymentStatus::Drained)
            //                     .exec(&mut db)
            //                     .await?;
            //
            //                 continue;
            //             }

            let web_app = Some(WebApp {
                domain: web_deployment.domain,
                port: web_deployment.port,
            });

            let route_config = DeployAppData {
                id: deployment.id.to_string(),
                project: project_name,
                name: app_name,
                state: app_state,
                container_id,
                web_app,
            };

            routes.push(route_config);
        }

        Ok(routes)
    }
}
