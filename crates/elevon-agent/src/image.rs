use std::{
    collections::HashSet,
    net::TcpListener,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use bollard::{
    auth::DockerCredentials,
    plugin::{ContainerCreateBody, HostConfig, PortBinding, PortMap},
    query_parameters::{CreateContainerOptionsBuilder, CreateImageOptionsBuilder},
};
use elevon_contracts::deploy::{
    AppPayload, AppRole, StreamEvent, StreamLogLevel, resolve_app_env_name,
};
use elevon_fs::agent::{load_app_env, load_app_string_env};
use futures::StreamExt;
use tokio::sync::RwLock;

use crate::{
    api::{
        db::models::{App, Deployment, DeploymentStatus},
        state::AppState,
        stream::{StreamSender, emit},
    },
    proxy::types::{AgentEvent, RouteConfig, RouteState},
};

static ALLOCATED_PORTS: LazyLock<RwLock<HashSet<u16>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

pub async fn pull_image(
    tx: &StreamSender,
    docker: &bollard::Docker,
    app_config: &AppPayload,
) -> Result<()> {
    emit(
        tx,
        StreamEvent::Log {
            level: StreamLogLevel::Info,
            message: format!("[{}] Pulling the image from registry", app_config.name),
        },
    )
    .await;

    let resolved_path = resolve_app_env_name(&app_config.project, "default");
    let project_env = load_app_env(&resolved_path, None)?;

    let (registry_server, registry_username, registry_password) = match (
        project_env.get("REGISTRY_SERVER"),
        project_env.get("REGISTRY_USERNAME"),
        project_env.get("REGISTRY_PASSWORD"),
    ) {
        (Some(s), Some(u), Some(p)) => (Some(s.clone()), Some(u.clone()), Some(p.clone())),
        _ => (None, None, None),
    };

    let options = CreateImageOptionsBuilder::new()
        .from_image(&app_config.image)
        .build();

    let credentials = Some(DockerCredentials {
        username: registry_username,
        password: registry_password,
        serveraddress: registry_server,
        ..Default::default()
    });

    let mut stream = docker.create_image(Some(options), None, credentials);
    while let Some(result) = stream.next().await {
        // TODO: Stream progress back to deploy
        result?;
    }

    emit(
        tx,
        StreamEvent::Log {
            level: StreamLogLevel::Info,
            message: format!("[{}] Image was pulled", app_config.name),
        },
    )
    .await;

    Ok(())
}

pub async fn run_image(
    tx: &StreamSender,
    docker: &bollard::Docker,
    app: &App,
    app_config: &AppPayload,
    db: &mut toasty::Db,
) -> Result<(String, String, u16)> {
    let port = {
        let mut ports = ALLOCATED_PORTS.write().await;
        let port = find_free_port().ok_or_else(|| anyhow::anyhow!("No free port found"))?;
        ports.insert(port);
        port
    };

    emit(
        tx,
        StreamEvent::Log {
            level: StreamLogLevel::Info,
            message: format!("[{}] Running a container on port {}", app_config.name, port),
        },
    )
    .await;

    let mut deployment = toasty::create!(Deployment {
        app_id: app.id,
        container_id: None,
        status: DeploymentStatus::Pending,
        port,
    })
    .exec(db)
    .await?;

    let container_id = async {
        let container_name = format!("{}-{}", app_config.name, deployment.id);
        let options = CreateContainerOptionsBuilder::new()
            .name(&container_name)
            .build();

        let resolved_project_env = resolve_app_env_name(&app_config.project, "default");
        let project_env = load_app_string_env(&resolved_project_env)?;
        let mut app_env =
            load_app_string_env(&resolve_app_env_name(&app_config.project, &app_config.name))?;

        app_env.extend(project_env);

        let mut port_bindings = PortMap::new();

        if let Some(web_app) = &app_config.web_app {
            port_bindings.insert(
                format!("{}/tcp", web_app.port),
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".to_string()),
                    host_port: Some(port.to_string()),
                }]),
            );
        }

        let host_config = Some(HostConfig {
            port_bindings: Some(port_bindings),
            ..Default::default()
        });

        let config = ContainerCreateBody {
            image: Some(app_config.image.clone()),
            env: Some(app_env),
            host_config,
            ..Default::default()
        };

        let container = docker.create_container(Some(options), config).await?;

        docker.start_container(&container.id, None).await?;

        Ok::<String, anyhow::Error>(container.id)
    }
    .await;

    match container_id {
        Err(err) => {
            let mut ports = ALLOCATED_PORTS.write().await;
            ports.remove(&port);

            toasty::update!(deployment {
                status: DeploymentStatus::Failed
            })
            .exec(db)
            .await?;

            Err(err)
        }
        Ok(container_id) => {
            toasty::update!(deployment {
                container_id: container_id.clone(),
                status: DeploymentStatus::Active
            })
            .exec(db)
            .await?;

            emit(
                tx,
                StreamEvent::Log {
                    level: StreamLogLevel::Info,
                    message: format!("[{}] Container started running", app_config.name),
                },
            )
            .await;

            Ok((deployment.id.to_string(), container_id, port))
        }
    }
}

fn find_free_port() -> Option<u16> {
    for port in 3334..=9998 {
        match TcpListener::bind(("0.0.0.0", port)) {
            Ok(_) => return Some(port),
            Err(_) => continue,
        }
    }
    None
}

pub async fn deploy_apps(
    tx: &StreamSender,
    state: Arc<AppState>,
    apps: Vec<AppPayload>,
) -> Result<()> {
    let mut db = state.agent_db.db.clone();

    for app in apps {
        emit(
            tx,
            StreamEvent::Log {
                level: StreamLogLevel::Info,
                message: format!("[{}] Deploying...", app.name),
            },
        )
        .await;

        let db_app = App::get_or_create(&mut db, &app).await?;

        pull_image(tx, &state.docker, &app).await.inspect_err(
            |err| tracing::error!(app = %app.name, error = %err, "failed to get or create app"),
        )?;
        let (deployment_id, container_id, port) =
            run_image(tx, &state.docker, &db_app, &app, &mut db)
                .await
                .inspect_err(
                    |err| tracing::error!(app = %app.name, error = %err, "failed to run container"),
                )?;

        if let (AppRole::Web, Some(web_app)) = (&app.role, &app.web_app) {
            if let Some(mut previous_deployment) =
                Deployment::get_previous_deployment(&mut db, &db_app.id, &container_id).await?
            {
                emit(
                    tx,
                    StreamEvent::Log {
                        level: StreamLogLevel::Info,
                        message: format!(
                            "[{}] Found previously released container, draining it...",
                            app.name,
                        ),
                    },
                )
                .await;

                if let Some(container_id) = previous_deployment.container_id.clone() {
                    toasty::update!(previous_deployment {
                        status: DeploymentStatus::Drained
                    })
                    .exec(&mut db)
                    .await?;

                    let route_config = RouteConfig {
                        id: deployment_id.clone(),
                        name: app.name.clone(),
                        domain: web_app.domain.clone(),
                        port,
                        state: RouteState::Draining,
                        container_id,
                    };

                    let drain_stream = state.socket_client.connect().await?;
                    state
                        .socket_client
                        .send(drain_stream, AgentEvent::DrainRoute(route_config))
                        .await?;
                }
            }

            emit(
                tx,
                StreamEvent::Log {
                    level: StreamLogLevel::Info,
                    message: format!("[{}] Updating proxy routing for traffic", app.name),
                },
            )
            .await;

            let route_config = RouteConfig {
                id: deployment_id,
                name: app.name,
                domain: web_app.domain.clone(),
                port,
                state: RouteState::Active,
                container_id,
            };

            let upsert_stream = state.socket_client.connect().await?;
            state
                .socket_client
                .send(upsert_stream, AgentEvent::UpsertRoute(route_config))
                .await?;
        }
    }

    Ok(())
}
