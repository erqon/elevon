use std::{collections::HashSet, net::TcpListener, sync::LazyLock};

use anyhow::Result;
use bollard::{
    Docker,
    auth::DockerCredentials,
    plugin::{ContainerCreateBody, HostConfig, PortBinding, PortMap},
    query_parameters::{CreateContainerOptionsBuilder, CreateImageOptionsBuilder},
};
use elevon_contracts::deploy::{AppPayload, format_app_env_name};
use elevon_fs::agent::{load_app_env, load_app_string_env};
use futures::StreamExt;
use tokio::sync::RwLock;

use crate::api::db::models::{App, Deployment, DeploymentStatus};

static ALLOCATED_PORTS: LazyLock<RwLock<HashSet<u16>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

pub async fn pull_image(app_config: &AppPayload) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    // FIX: This still creates empty env file in case the project has a single app with no apps:
    let project_env = load_app_env(&app_config.project, None)?;

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

    Ok(())
}

pub async fn run_image(
    app_config: &AppPayload,
    db: &mut toasty::Db,
) -> Result<(String, String, u16)> {
    let docker = Docker::connect_with_local_defaults()?;
    let port = {
        let mut ports = ALLOCATED_PORTS.write().await;
        let port = find_free_port().ok_or_else(|| anyhow::anyhow!("No free port found"))?;
        ports.insert(port);
        port
    };

    let mut app = App::get_or_create(db, &app_config).await?;
    let mut deployment = toasty::create!(Deployment {
        app_id: app.id,
        status: DeploymentStatus::Pending,
        port,
    })
    .exec(db)
    .await?;

    let container_id = async {
        let container_name = format!("{}-{}", &app_config.name, &deployment.id);
        let options = CreateContainerOptionsBuilder::new()
            .name(&container_name)
            .build();

        let project_env = load_app_string_env(&app_config.project)?;
        let mut app_env =
            load_app_string_env(&format_app_env_name(&app_config.project, &app_config.name))?;
        app_env.extend(project_env);

        let mut port_bindings = PortMap::new();

        if let Some(web_app) = &app_config.web_app {
            port_bindings.insert(
                format!("{}/tcp", &web_app.port),
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
            toasty::update!(app {
                current_port: Some(port)
            })
            .exec(db)
            .await?;

            toasty::update!(deployment {
                status: DeploymentStatus::Active
            })
            .exec(db)
            .await?;

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
