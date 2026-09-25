use std::{collections::HashSet, sync::LazyLock};

use anyhow::Result;
use bollard::{
    auth::DockerCredentials,
    plugin::{ContainerCreateBody, HostConfig, PortBinding, PortMap, RestartPolicy},
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, RemoveImageOptions,
    },
};
use elevon_contracts::deploy::{
    AppDeployPayload, AppPayload, AppRole, AppRollbackPayload, AppRuntimeOptions, StreamEvent,
    StreamLogLevel, TlsType, WebApp,
};
use elevon_fs::agent::{
    AppEnvOptions, TlsOptions, add_app_env, load_app_env, load_app_string_env, write_tls_file,
};
use futures_util::TryStreamExt;
use tokio::sync::RwLock;

use crate::{
    api::{
        db::models::{
            App, Deployment, DeploymentRuntimeOption, DeploymentRuntimeOptions, DeploymentStatus,
        },
        state::SharedApiState,
        stream::{StreamSender, emit},
    },
    proxy::types::{AgentEvent, DeployAppData, DeployAppState},
};

static ALLOCATED_PORTS: LazyLock<RwLock<HashSet<u16>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

fn find_free_port() -> Option<u16> {
    for port in 3334..=9998 {
        match elevon_http::check_port(port) {
            Some(p) => return Some(p),
            None => continue,
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

fn prepare_env_variables(app_config: &AppPayload, deployment_id: &str) -> Result<()> {
    let env_options = AppEnvOptions::app(&app_config.project, &app_config.name, deployment_id);

    if let Some(vars) = &app_config.vars {
        for (key, value) in vars {
            add_app_env(key.clone(), value.clone(), env_options.clone())?;
        }
    }

    Ok(())
}

struct PullImageOptions<'cfg, 'dep> {
    pub project: &'cfg str,
    pub name: &'cfg str,
    pub image_ref: &'cfg str,
    pub deployment_id: &'dep str,
}

async fn pull_image(
    tx: &StreamSender,
    docker: &bollard::Docker,
    options: PullImageOptions<'_, '_>,
) -> Result<()> {
    emit(
        tx,
        StreamEvent::log(format!(
            "[{}] Pulling the image from registry",
            options.name
        )),
    )
    .await;

    let env_options = AppEnvOptions::app(options.project, options.name, options.deployment_id);
    let project_env = load_app_env(env_options)?;

    let (registry_server, registry_username, registry_password) = match (
        project_env.get("REGISTRY_SERVER"),
        project_env.get("REGISTRY_USERNAME"),
        project_env.get("REGISTRY_PASSWORD"),
    ) {
        (Some(s), Some(u), Some(p)) => (Some(s.clone()), Some(u.clone()), Some(p.clone())),
        _ => (None, None, None),
    };

    let image_options = CreateImageOptionsBuilder::new()
        .from_image(options.image_ref)
        .build();

    let credentials = Some(DockerCredentials {
        username: registry_username,
        password: registry_password,
        serveraddress: registry_server,
        ..Default::default()
    });

    let mut pull_stream = docker.create_image(Some(image_options), None, credentials);

    while let Some(event) = pull_stream.try_next().await? {
        tracing::debug!(?event, "docker pull event");
    }

    emit(
        tx,
        StreamEvent::log(format!("[{}] Image was pulled", options.name)),
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

    let app_env_options = AppEnvOptions::app(
        &app_config.project,
        &app_config.name,
        &deployment.id.to_string(),
    );

    // TODO: Currently it laods everything from the file but it should be fixed in a way like,
    // saving env keys in db and loading them up.
    let app_env = load_app_string_env(app_env_options)?;

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
            name: app_config.runtime_options.restart,
            ..Default::default()
        }),
        nano_cpus: app_config.runtime_options.cpu_limit,
        memory: app_config.runtime_options.memory_limit,
        network_mode: app_config.runtime_options.network.clone(),
        port_bindings: Some(port_bindings),
        ..Default::default()
    });

    let config = ContainerCreateBody {
        image: Some(app_config.image_ref.clone()),
        cmd: app_config.runtime_options.cmd.clone(),
        env: Some(app_env),
        host_config,
        ..Default::default()
    };

    let container = docker.create_container(Some(options), config).await?;

    docker.start_container(&container.id, None).await?;

    Ok(container.id)
}

struct _DeployAppOptions<'cfg, 'tx> {
    db_tx: &'tx mut toasty::Transaction<'cfg>,
    port: u16,
    app_config: &'cfg AppPayload,
}

/// Creates Deployment, sets TLS files, sets Environment variables,
/// and then runs the container. Afterwards updates Deployment's status.
async fn _deploy_app(
    tx: &StreamSender,
    docker: &bollard::Docker,
    mut deployment: Deployment,
    options: _DeployAppOptions<'_, '_>,
) -> Result<(Deployment, String)> {
    prepare_env_variables(options.app_config, &deployment.id.to_string())?;

    let pull_image_options = PullImageOptions {
        project: &options.app_config.project,
        name: &options.app_config.name,
        image_ref: &options.app_config.image_ref,
        deployment_id: &deployment.id.to_string(),
    };

    pull_image(tx, docker, pull_image_options).await.inspect_err(
        |err| tracing::error!(app = %options.app_config.name, error = %err, "failed to get or create app"),
    )?;

    // TODO: During rollbacks previous deployment might have been running on a different
    // domain, with different certificates, so the requests will fail due to the certs missmatch.
    if let Some(tls) = options.app_config.tls.as_ref() {
        let tls_options = TlsOptions {
            project: options.app_config.project.clone(),
            app: Some(options.app_config.name.clone()),
        };

        write_tls_file(tls.cert.as_bytes(), TlsType::Cert, tls_options.clone())?;
        write_tls_file(tls.key.as_bytes(), TlsType::Key, tls_options)?;
    }

    let container_id = run_container(docker, &deployment, options.app_config, options.port).await;

    let res = match container_id {
        Err(err) => {
            clear_port(&options.port).await;

            emit(
                tx,
                StreamEvent::log(format!(
                    "[{}] Container failed to start",
                    options.app_config.name
                )),
            )
            .await;

            deployment
                .update()
                .image_ref(Some(options.app_config.image_ref.clone()))
                .status(DeploymentStatus::Failed)
                .exec(options.db_tx)
                .await?;

            Err(err)
        }
        Ok(container_id) => {
            emit(
                tx,
                StreamEvent::log(format!(
                    "[{}] Container started running",
                    options.app_config.name
                )),
            )
            .await;

            deployment
                .update()
                .image_ref(Some(options.app_config.image_ref.clone()))
                .container_id(container_id.clone())
                .status(DeploymentStatus::Active)
                .exec(options.db_tx)
                .await?;

            Ok((deployment, container_id))
        }
    }?;
    Ok(res)
}

async fn drain_app(
    state: &SharedApiState,
    db: &mut toasty::Db,
    mut deployment: Deployment,
    app_data: DeployAppData,
) -> Result<()> {
    toasty::update!(deployment {
        status: DeploymentStatus::Drained
    })
    .exec(db)
    .await?;

    state
        .proxy_socket
        .send(AgentEvent::DrainApp(app_data))
        .await?;

    Ok(())
}

async fn prune_old_releases(
    docker: &bollard::Docker,
    db: &mut toasty::Db,
    app: &App,
    just_drained_id: Option<uuid::Uuid>,
) -> Result<()> {
    let deployments = Deployment::list_by_app_id(db, &app.id, app.keep_releases as usize).await?;

    for old in deployments {
        if old.status == DeploymentStatus::Active || Some(old.id) == just_drained_id {
            continue; // never prune the one currently serving
        }

        if let Some(container_id) = &old.container_id {
            // best-effort, DrainJanitor may have already reaped it
            let _ = docker.remove_container(container_id, None).await;
        }

        let env_options = AppEnvOptions::app(&app.project, &app.name, &old.id.to_string());
        let _ = std::fs::remove_file(elevon_fs::agent::get_app_env(env_options)?);

        if let Some(image_ref) = &old.image_ref {
            let still_referenced = Deployment::filter(
                Deployment::fields()
                    .image_ref()
                    .eq(image_ref)
                    .and(Deployment::fields().id().ne(old.id)),
            )
            .first()
            .exec(db)
            .await?
            .is_some();

            if !still_referenced {
                let remove_options = RemoveImageOptions::default();
                if let Err(err) = docker
                    .remove_image(image_ref, Some(remove_options), None)
                    .await
                {
                    tracing::warn!(%image_ref, %err, "failed to remove unreferenced image");
                }
            }
        }

        Deployment::delete_by_id(db, old.id).await?;
    }

    Ok(())
}

async fn deploy_app(
    tx: &StreamSender,
    state: SharedApiState,
    app_config: AppPayload,
    // this present = rollback
    deployment_to_run: Option<Deployment>,
    deployment_to_drain: Option<Deployment>,
) -> Result<()> {
    let mut db = state.db.get();
    let mut cloned_db = db.clone();

    let is_rollback = deployment_to_run.is_some();

    emit(
        tx,
        StreamEvent::log(format!("[{}] Deploying...", app_config.name)),
    )
    .await;

    let port = get_free_port().await?;

    emit(
        tx,
        StreamEvent::log(format!(
            "[{}] Starting running a container on port {}",
            app_config.name, port
        )),
    )
    .await;

    let db_app = App::get_or_create(&mut db, &app_config).await?;

    let mut db_tx = cloned_db.transaction().await?;

    let deployment = match deployment_to_run {
        Some(v) => v,
        None => {
            let deployment = toasty::create!(Deployment {
                app_id: db_app.id,
                port: port
            })
            .exec(&mut db_tx)
            .await?;

            let deployment_runtime_options =
                DeploymentRuntimeOptions::from(&app_config.runtime_options);

            let options = DeploymentRuntimeOptions {
                port: app_config.web_app.as_ref().map(|w| w.port),
                ..deployment_runtime_options
            };

            toasty::create!(DeploymentRuntimeOption {
                deployment_id: deployment.id,
                options
            })
            .exec(&mut db_tx)
            .await?;

            deployment
        }
    };

    let deploy_app_options = _DeployAppOptions {
        db_tx: &mut db_tx,
        port,
        app_config: &app_config,
    };

    let (new_deployment, new_container_id) =
        _deploy_app(tx, &state.docker, deployment, deploy_app_options).await?;

    let app_data = DeployAppData {
        id: new_deployment.id.to_string(),
        project: app_config.project.to_string(),
        name: app_config.name.to_string(),
        container_id: new_container_id,
        web_app: match (&app_config.runtime_options.role, &app_config.web_app) {
            (AppRole::Web, Some(web_app)) => Some(WebApp {
                port,
                domain: web_app.domain.clone(),
            }),
            _ => None,
        },
        ..Default::default()
    };

    if AppRole::Web == app_config.runtime_options.role {
        emit(
            tx,
            StreamEvent::log(format!(
                "[{}] Updating proxy routing for traffic",
                app_config.name
            )),
        )
        .await;

        // TODO: In case of any failure in here the container is kept running,
        // need to be deleted.

        state
            .proxy_socket
            .send(AgentEvent::UpsertRoute(app_data.clone()))
            .await?;
    }

    let deployment_to_drain = match deployment_to_drain {
        Some(v) => Some(v),
        None => Deployment::get_latest_deployment(&mut db, &db_app.id).await?,
    };
    let deployment_to_drain_id = deployment_to_drain.as_ref().map(|d| d.id);

    // Marks the current deployment as draining, so no new requests will be handled by it,
    // and later the DrainJanitor service will terminate the container.
    if let Some(deployment_to_drain) = deployment_to_drain
        && let Some(container_id) = deployment_to_drain.container_id.clone()
    {
        emit(
            tx,
            StreamEvent::log(format!(
                "[{}] Found previously released container, draining it...",
                app_config.name,
            )),
        )
        .await;

        let current_app_data = DeployAppData {
            id: deployment_to_drain.id.to_string(),
            container_id,
            state: DeployAppState::Draining,
            ..app_data
        };

        drain_app(&state, &mut db, deployment_to_drain, current_app_data).await?;
    }

    if !is_rollback
        && let Err(err) =
            prune_old_releases(&state.docker, &mut db, &db_app, deployment_to_drain_id).await
    {
        emit(
            tx,
            StreamEvent::Log {
                level: StreamLogLevel::Warn,
                message: format!(
                    "[{}] Failed to prune old releases: {}",
                    app_config.name, err
                ),
            },
        )
        .await;
    }

    db_tx.commit().await?;

    Ok(())
}

pub async fn deploy_apps(
    tx: &StreamSender,
    state: SharedApiState,
    payload: AppDeployPayload,
) -> Result<()> {
    for app in payload.apps {
        deploy_app(tx, state.clone(), app, None, None).await?;
    }

    Ok(())
}

pub async fn rollback_apps(
    tx: &StreamSender,
    state: SharedApiState,
    payload: AppRollbackPayload,
) -> Result<()> {
    let mut db = state.db.get();

    for app in payload.apps {
        let db_app = App::get_by_project_and_name(&mut db, &app.project, &app.name).await?;

        let Some(db_app) = db_app else {
            emit(
                tx,
                StreamEvent::log(format!("[{}] App not found, skipping...", app.name)),
            )
            .await;

            continue;
        };

        let previous_deployment = Deployment::get_previous_deployment(&mut db, &db_app.id).await?;

        // Prevous deployment must exist since what are you trying to rollback to, right?
        // Also current deployment might not be active because of a failure or something,
        // so rollback can still happen.
        let Some(previous_deployment) = previous_deployment else {
            emit(
                tx,
                StreamEvent::log(format!(
                    "[{}] Previous deployment doesn't exist, skipping...",
                    app.name
                )),
            )
            .await;

            continue;
        };

        let (Some(image_ref), Some(deployment_runtime_options)) = (
            &previous_deployment.image_ref,
            &previous_deployment.runtime_options.get(),
        ) else {
            emit(
                tx,
                StreamEvent::log(format!(
                    "[{}] Previous deployment doesn't have image properties, skipping...",
                    app.name
                )),
            )
            .await;

            continue;
        };

        let runtime_options = AppRuntimeOptions::from(&deployment_runtime_options.options);
        let deployment_to_drain = Deployment::get_latest_deployment(&mut db, &db_app.id).await?;

        let app_payload = AppPayload {
            project: app.project.clone(),
            name: app.name.clone(),
            image_ref: image_ref.to_string(),
            keep_releases: previous_deployment.app.get().keep_releases,
            runtime_options: runtime_options.clone(),
            web_app: match (
                &runtime_options.role,
                &deployment_runtime_options.options.port,
                &previous_deployment.app.get().domain,
            ) {
                (AppRole::Web, Some(port), Some(domain)) => Some(WebApp {
                    port: *port,
                    domain: domain.clone(),
                }),
                _ => None,
            },
            ..Default::default()
        };

        deploy_app(
            tx,
            state.clone(),
            app_payload,
            Some(previous_deployment),
            deployment_to_drain,
        )
        .await?;
    }

    Ok(())
}
