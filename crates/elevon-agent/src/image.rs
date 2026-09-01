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
    resolve_app_env_name,
};
use elevon_fs::agent::{add_app_env, load_app_env, load_app_string_env, write_tls_file};
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
async fn run_image(
    docker: &bollard::Docker,
    deployment: &Deployment,
    app_config: &AppPayload,
    port: u16,
) -> Result<String> {
    let container_name = format!("{}-{}", app_config.name, deployment.id);
    let options = CreateContainerOptionsBuilder::new()
        .name(&container_name)
        .build();

    let project_env = load_app_string_env(&resolve_app_env_name(&app_config.project, "default"))?;
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
    app_config: &'a AppPayload,
    new_deployment: &'a Deployment,
}

async fn deploy_app<'a>(
    tx: &StreamSender,
    docker: &bollard::Docker,
    options: DeployAppOptions<'a>,
) -> Result<String> {
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

    let container_id = run_image(
        docker,
        &options.new_deployment,
        options.app_config,
        options.port,
    )
    .await;

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

            Ok(container_id)
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

        let new_port = get_free_port().await?;
        let mut new_deployment = toasty::create!(Deployment {
            app_id: db_app.id,
            status: DeploymentStatus::Pending,
            port: new_port,
            prev_deployment_id: latest_deployment.as_ref().map(|d| d.id),
        })
        .exec(&mut db)
        .await?;

        pull_image(tx, &state.docker, &app).await.inspect_err(
            |err| tracing::error!(app = %app.name, error = %err, "failed to get or create app"),
        )?;

        let resolved_path = resolve_app_env_name(&app.project, &app.name);

        if let Some(tls) = app.tls.as_ref() {
            write_tls_file(
                &resolved_path,
                tls.cert.as_bytes(),
                TlsType::Cert,
                Some(true),
            )?;
            write_tls_file(&resolved_path, tls.key.as_bytes(), TlsType::Key, Some(true))?;
        }

        for (key, value) in app.vars.clone() {
            add_app_env(&resolved_path, None, key.clone(), value.clone())?;
        }

        let deploy_app_options = DeployAppOptions {
            port: new_port,
            app_config: &app,
            new_deployment: &new_deployment,
        };

        let new_container_id = match deploy_app(tx, &state.docker, deploy_app_options).await {
            Err(err) => {
                toasty::update!(new_deployment {
                    status: DeploymentStatus::Failed
                })
                .exec(&mut db)
                .await?;

                Err(err)
            }
            Ok(container_id) => {
                toasty::update!(new_deployment {
                    container_id: container_id.clone(),
                    status: DeploymentStatus::Active
                })
                .exec(&mut db)
                .await?;

                Ok(container_id)
            }
        }?;

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

        // Currently this doesn't deploy Worker apps, doing so requires updating UpsertRoute in a way
        // that it would be DeployApp or something, that would start the container and upsert as a route
        // in case of it being a web app.
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
