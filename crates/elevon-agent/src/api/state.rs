use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use bollard::plugin::{ContainerStateStatusEnum, HealthStatusEnum};
use elevon_contracts::deploy::{StreamLogLevel, WebApp};
use tokio::sync::RwLock;

use crate::api::db::AgentDb;
use crate::api::db::models::deployment::DeploymentHealthCheck;
use crate::api::db::models::{Deployment, DeploymentStatus};
use crate::api::stream::{ActionLogger, OptionLoggerExt};
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

        let deployments = Deployment::filter(
            Deployment::fields()
                .status()
                .eq(DeploymentStatus::Active)
                .or(Deployment::fields().status().eq(DeploymentStatus::Pending)),
        )
        .include(Deployment::fields().app())
        .include(Deployment::fields().options())
        .exec(&mut db)
        .await?;

        let mut routes: Vec<DeployAppData> = Vec::new();

        for mut deployment in deployments {
            let app = deployment.app.get();
            let deployment_options = deployment.options.get();

            let (Some(container_id), Some(web_deployment)) =
                (deployment.container_id.clone(), deployment.web.clone())
            else {
                continue;
            };

            let app_name = app.name.clone();
            let project_name = app.project.clone();

            let deployment_status = self.check_container_state(&container_id).await;
            if deployment_status != DeploymentStatus::Active {
                if let Some(deployment_options) = deployment_options
                    && self
                        .check_container_health(
                            &container_id,
                            None,
                            &deployment_options.healthcheck,
                            None,
                        )
                        .await
                        .is_err()
                {
                    continue;
                }

                deployment
                    .update()
                    .status(deployment_status)
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
                status: deployment_status,
                container_id,
                web_app,
            };

            routes.push(route_config);
        }

        Ok(routes)
    }

    pub async fn check_container_state(&self, container_id: &str) -> DeploymentStatus {
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
            ContainerStateStatusEnum::RESTARTING
            | ContainerStateStatusEnum::STOPPING
            | ContainerStateStatusEnum::EXITED => DeploymentStatus::Failed,
            _ => DeploymentStatus::Drained,
        }
    }

    async fn is_container_healthy(&self, container_id: &str) -> bool {
        let Ok(container) = self.docker.inspect_container(container_id, None).await else {
            return false;
        };

        let Some(state) = container.state else {
            return false;
        };

        if let Some(health) = &state.health
            && let Some(status) = health.status
        {
            return matches!(
                status,
                HealthStatusEnum::HEALTHY | HealthStatusEnum::STARTING
            );
        }

        let is_running_active = matches!(
            (
                state.running,
                state.restarting,
                state.dead,
                state.oom_killed
            ),
            (
                Some(true),
                Some(false) | None,
                Some(false) | None,
                Some(false) | None
            )
        );

        let is_running_status = matches!(state.status, Some(ContainerStateStatusEnum::RUNNING));

        is_running_active && is_running_status
    }

    pub async fn check_container_health(
        &self,
        container_id: &str,
        app_name: Option<&str>,
        healthcheck: &DeploymentHealthCheck,
        logger: Option<&(dyn ActionLogger + Send + Sync)>,
    ) -> Result<()> {
        let log_id = app_name.unwrap_or(container_id);

        let timeout_duration = Duration::from_secs(healthcheck.timeout);
        let interval_duration = Duration::from_secs(healthcheck.interval);
        let stabilization_duration = Duration::from_secs(healthcheck.stabilization);

        let start_time = Instant::now();
        let mut consecutive_successes = 0;

        for attempt in 0..=healthcheck.retries {
            let check_result =
                tokio::time::timeout(timeout_duration, self.is_container_healthy(container_id))
                    .await;

            match check_result {
                Ok(true) => {
                    consecutive_successes += 1;

                    if start_time.elapsed() >= stabilization_duration
                        && consecutive_successes >= healthcheck.required_successes
                    {
                        logger
                            .log_event(
                                format!("[{log_id}] Container stabilized successfully",),
                                StreamLogLevel::Info,
                            )
                            .await?;
                        return Ok(());
                    }
                }
                _ => {
                    consecutive_successes = 0;
                    logger.log_event(
                        format!("[{log_id}] Health check failed or timed out; resetting stability counter (attempt {attempt}/{})", healthcheck.retries),
                        StreamLogLevel::Warn,
                    ).await?;
                }
            }

            if attempt < healthcheck.retries {
                tokio::time::sleep(interval_duration).await;
            }
        }

        logger
            .log_event(
                format!(
                    "[{log_id}] Container achieved stability threshold ({consecutive_successes}/{})",
                    healthcheck.required_successes
                ),
                StreamLogLevel::Info,
            )
            .await?;

        // TODO: This might hang or leave resources if Docker is unresponsive.
        if let Err(err) = self.docker.stop_container(container_id, None).await {
            logger
                .log_event(
                    format!("[{log_id}] Failed to stop container {container_id}: {err}",),
                    StreamLogLevel::Warn,
                )
                .await?;
            anyhow::bail!("failed to stop container {container_id}: {err}");
        }

        anyhow::bail!("Container failed to pass health check")
    }
}
