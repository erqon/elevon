use std::{collections::HashSet, net::TcpListener, sync::LazyLock};

use anyhow::Result;
use bollard::{
    Docker,
    auth::DockerCredentials,
    plugin::{ContainerCreateBody, HostConfig, PortBinding, PortMap},
    query_parameters::{CreateContainerOptionsBuilder, CreateImageOptionsBuilder},
};
use elevon_fs::agent::load_app_env;
use futures::StreamExt;
use tokio::sync::RwLock;

use crate::{
    api::{
        db::models::{App, Deployment, DeploymentStatus},
        dto::AppDeployData,
    },
    env::ElevonEnv,
};

static ALLOCATED_PORTS: LazyLock<RwLock<HashSet<u16>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

fn get_image_url(elevon_env: &ElevonEnv, app_config: &AppDeployData) -> String {
    format!(
        "{}/{}:{}",
        elevon_env.registry_server, app_config.image_url, app_config.commit_sha
    )
}

pub async fn pull_image(elevon_env: &ElevonEnv, app_config: &AppDeployData) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    let full_image_url = get_image_url(elevon_env, app_config);

    let options = CreateImageOptionsBuilder::new()
        .from_image(&full_image_url)
        .build();

    let credentials = if !elevon_env.registry_server.is_empty() {
        Some(DockerCredentials {
            username: Some(elevon_env.registry_username.clone()),
            password: Some(elevon_env.registry_password.clone()),
            serveraddress: Some(elevon_env.registry_server.clone()),
            ..Default::default()
        })
    } else {
        None
    };

    let mut stream = docker.create_image(Some(options), None, credentials);
    while let Some(result) = stream.next().await {
        // TODO: Add a way of streaming the progress back to deploy
        result?;
    }

    Ok(())
}

pub async fn run_image(
    elevon_env: &ElevonEnv,
    app_config: &AppDeployData,
    db: &mut toasty::Db,
) -> Result<(String, u16)> {
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

    let start_result = async {
        let full_image_url = get_image_url(elevon_env, app_config);

        let container_name = format!("{}-{}", &app_config.name, &deployment.id);
        let options = CreateContainerOptionsBuilder::new()
            .name(&container_name)
            .build();

        let app_env: Vec<String> = load_app_env(&app_config.name, None)?
            .iter()
            .map(|(key, val)| format!("{}={}", key, val))
            .collect();

        let mut port_bindings = PortMap::new();
        port_bindings.insert(
            format!("{}/tcp", &app_config.port),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(port.to_string()),
            }]),
        );

        let host_config = Some(HostConfig {
            port_bindings: Some(port_bindings),
            ..Default::default()
        });

        let config = ContainerCreateBody {
            image: Some(full_image_url),
            env: Some(app_env),
            host_config,
            ..Default::default()
        };

        let container = docker.create_container(Some(options), config).await?;

        docker.start_container(&container.id, None).await?;

        Ok::<(), anyhow::Error>(())
    }
    .await;

    if let Err(err) = start_result {
        let mut ports = ALLOCATED_PORTS.write().await;
        ports.remove(&port);

        toasty::update!(deployment {
            status: DeploymentStatus::Failed
        })
        .exec(db)
        .await?;

        return Err(err);
    } else {
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
    }

    Ok((deployment.id.to_string(), port))
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
