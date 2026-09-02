use std::{
    collections::HashSet,
    net::TcpListener,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use bollard::{
    auth::DockerCredentials,
    plugin::{ContainerCreateBody, HostConfig, PortBinding, PortMap, RestartPolicy},
    query_parameters::{CreateContainerOptionsBuilder, CreateImageOptionsBuilder},
};
use elevon_contracts::deploy::{
    AppPayload, AppRole, AppRollbackPayloadData, StreamEvent, StreamLogLevel, TlsType, WebApp,
};
use elevon_fs::agent::{
    AppEnvOptions, TlsOptions, add_app_env, load_app_env, load_app_string_env, write_tls_file,
};
use futures::StreamExt;
use tokio::sync::RwLock;

use crate::{
    api::{
        db::models::{App, Deployment, DeploymentStatus},
        state::AppState,
        stream::{StreamSender, emit},
    },
    proxy::types::{AgentEvent, DeployAppData, DeployAppState},
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

    let env_options = AppEnvOptions::app_base(&app_config.project, "default");
    let project_env = load_app_env(env_options)?;

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

async fn run_container(
    docker: &bollard::Docker,
    deployment: &Deployment,
    app_config: &AppPayload,
    port: u16,
) -> Result<String> {
    let container_name = format!("{}-{}", app_config.name, deployment.id);
    let options = CreateContainerOptionsBuilder::new()
        .name(&container_name)
        .build();

    let project_env_options = AppEnvOptions::app_base(&app_config.project, "default");
    let app_env_options = AppEnvOptions::app(
        &app_config.project,
        &app_config.name,
        &deployment.id.to_string(),
    );

    let project_env = load_app_string_env(project_env_options)?;
    let mut app_env = load_app_string_env(app_env_options)?;

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
        restart_policy: Some(RestartPolicy {
            name: app_config.options.restart,
            ..Default::default()
        }),
        nano_cpus: app_config.options.cpu_limit,
        memory: app_config.options.memory_limit,
        network_mode: app_config.options.network.clone(),
        port_bindings: Some(port_bindings),
        ..Default::default()
    });

    let config = ContainerCreateBody {
        image: Some(app_config.image.clone()),
        cmd: app_config.options.cmd.clone(),
        env: Some(app_env),
        host_config,
        ..Default::default()
    };

    let container = docker.create_container(Some(options), config).await?;

    docker.start_container(&container.id, None).await?;

    Ok(container.id)
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

async fn get_free_port() -> Result<u16> {
    let mut ports = ALLOCATED_PORTS.write().await;
    let port = find_free_port().ok_or_else(|| anyhow::anyhow!("No free port found"))?;
    ports.insert(port);
    Ok(port)
}

async fn clear_port(port: &u16) {
    let mut ports = ALLOCATED_PORTS.write().await;
    ports.remove(port);
}

struct DeployAppOptions<'a> {
    port: u16,
    db: &'a mut toasty::Db,
    db_app: &'a App,
    app_config: &'a AppPayload,
}

/// Creates Deployment, sets TLS files, sets Environment variables,
/// and then runs the container. Afterwards updates Deployment's status.
async fn deploy_app<'a>(
    tx: &StreamSender,
    docker: &bollard::Docker,
    options: DeployAppOptions<'a>,
) -> Result<(Deployment, String)> {
    emit(
        tx,
        StreamEvent::Log {
            level: StreamLogLevel::Info,
            message: format!(
                "[{}] Starting running a container on port {}",
                options.app_config.name, options.port
            ),
        },
    )
    .await;

    let inspect = docker.inspect_image(&options.app_config.image).await?;
    let image_ref = inspect
        .repo_digests
        .and_then(|digests| digests.into_iter().next())
        .or(inspect.id)
        .ok_or_else(|| anyhow::anyhow!("no image reference found after pull"))?;

    let mut deployment = toasty::create!(Deployment {
        app_id: options.db_app.id,
        image_digest: image_ref,
        status: DeploymentStatus::Pending,
        port: options.port
    })
    .exec(options.db)
    .await?;

    if let Some(tls) = options.app_config.tls.as_ref() {
        let tls_options = TlsOptions {
            project: options.app_config.project.clone(),
            app: Some(options.app_config.name.clone()),
        };

        write_tls_file(tls.cert.as_bytes(), TlsType::Cert, tls_options.clone())?;
        write_tls_file(tls.key.as_bytes(), TlsType::Key, tls_options)?;
    }

    let env_options = AppEnvOptions::app(
        &options.app_config.project,
        &options.app_config.name,
        &deployment.id.to_string(),
    );

    for (key, value) in options.app_config.vars.clone() {
        add_app_env(key.clone(), value.clone(), env_options.clone())?;
    }

    let container_id = run_container(docker, &deployment, options.app_config, options.port).await;

    match container_id {
        Err(err) => {
            clear_port(&options.port).await;

            emit(
                tx,
                StreamEvent::Log {
                    level: StreamLogLevel::Error,
                    message: format!("[{}] Container failed to start", options.app_config.name),
                },
            )
            .await;

            toasty::update!(deployment {
                status: DeploymentStatus::Failed
            })
            .exec(options.db)
            .await?;

            Err(err)
        }
        Ok(container_id) => {
            emit(
                tx,
                StreamEvent::Log {
                    level: StreamLogLevel::Info,
                    message: format!("[{}] Container started running", options.app_config.name),
                },
            )
            .await;

            toasty::update!(deployment {
                container_id: container_id.clone(),
                status: DeploymentStatus::Active
            })
            .exec(options.db)
            .await?;

            Ok((deployment, container_id))
        }
    }
}

async fn drain_app(
    state: &AppState,
    db: &mut toasty::Db,
    mut deployment: Deployment,
    app_data: DeployAppData,
) -> Result<()> {
    toasty::update!(deployment {
        status: DeploymentStatus::Drained
    })
    .exec(db)
    .await?;

    let drain_stream = state.socket_client.connect().await?;
    state
        .socket_client
        .send(drain_stream, AgentEvent::DrainApp(app_data))
        .await?;

    Ok(())
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
        let latest_deployment = Deployment::get_latest_deployment(&mut db, &db_app.id).await?;

        pull_image(tx, &state.docker, &app).await.inspect_err(
            |err| tracing::error!(app = %app.name, error = %err, "failed to get or create app"),
        )?;

        let new_port = get_free_port().await?;

        let deploy_app_options = DeployAppOptions {
            port: new_port,
            db: &mut db,
            db_app: &db_app,
            app_config: &app,
        };

        let (new_deployment, new_container_id) =
            deploy_app(tx, &state.docker, deploy_app_options).await?;

        let app_data = DeployAppData {
            id: new_deployment.id.to_string(),
            project: app.project,
            name: app.name.clone(),
            container_id: new_container_id,
            web_app: match (&app.options.role, &app.web_app) {
                (AppRole::Web, Some(web_app)) => Some(WebApp {
                    port: new_port,
                    domain: web_app.domain.clone(),
                }),
                _ => None,
            },
            ..Default::default()
        };

        // Worker apps don't need proxy routing, but draining the previous deployment currently
        // only happens in this Web-only branch too — worker containers from prior deployments
        // are never drained/stopped.
        if AppRole::Web == app.options.role {
            // Marks the current deployment as draining, so no new requests will be handled by it,
            // and later the DrainJanitor service will terminate the container.
            if let Some(latest_deployment) = latest_deployment {
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

                if let Some(container_id) = latest_deployment.container_id.clone() {
                    let current_app_data = DeployAppData {
                        id: latest_deployment.id.to_string(),
                        container_id,
                        state: DeployAppState::Draining,
                        ..app_data.clone()
                    };

                    drain_app(&state, &mut db, latest_deployment, current_app_data).await?;
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

            let upsert_stream = state.socket_client.connect().await?;
            state
                .socket_client
                .send(upsert_stream, AgentEvent::UpsertRoute(app_data))
                .await?;
        }
    }

    Ok(())
}

pub async fn rollback_apps(
    tx: &StreamSender,
    state: Arc<AppState>,
    apps: Vec<AppRollbackPayloadData>,
) -> Result<()> {
    let mut db = state.agent_db.db.clone();

    // TODO: During rollback previous app's image might be usinig different env variables
    // since it might have been updated afterwards.
    // So some kind of env backups would be a nice feature for future, but for now this should be ok.

    for app in apps {
        let db_app = App::get_by_project_and_name(&mut db, &app.project, &app.name).await?;

        let Some(db_app) = db_app else {
            emit(
                tx,
                StreamEvent::Log {
                    level: StreamLogLevel::Info,
                    message: format!("[{}] App not found, skipping...", app.name,),
                },
            )
            .await;

            continue;
        };

        let previous_deployment = Deployment::get_previous_deployment(&mut db, &db_app.id).await?;
        let _latest_deployment = Deployment::get_latest_deployment(&mut db, &db_app.id).await?;

        // Prevous deployment must exist since what are you trying to rollback to, right?
        // Also current deployment might not be active because of a failure or something,
        // so rollback can still happen.
        let Some(_previous_deployment) = previous_deployment else {
            continue;
        };

        let _prev_app_data = DeployAppData {
            ..Default::default()
        };

        // if let (Some(domain), Some(container_id)) =
        //     (db_app.domain, previous_deployment.container_id)
        // {
        //     // let route_config = RouteConfig {
        //     //     id: previous_deployment.id.to_string(),
        //     //     project: app.project,
        //     //     name: app.name,
        //     //     domain,
        //     //     port: previous_deployment.port,
        //     //     state: RouteState::Active,
        //     //     container_id,
        //     // };
        // }
    }

    Ok(())
}
