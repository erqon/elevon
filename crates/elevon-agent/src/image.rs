use anyhow::Result;
use bollard::{
    Docker,
    auth::DockerCredentials,
    plugin::{ContainerCreateBody, HostConfig, PortBinding, PortMap},
    query_parameters::{CreateContainerOptionsBuilder, CreateImageOptionsBuilder},
};
use elevon_fs::agent::load_app_env;
use futures::StreamExt;

use crate::{api::dto::AppDeployData, env::ElevonEnv};

pub async fn pull_image(elevon_env: &ElevonEnv, image_url: &str) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    let full_image_url = format!("{}/{}", &elevon_env.registry_server, image_url);

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
        result?;
    }

    Ok(())
}

pub async fn run_image(elevon_env: &ElevonEnv, app_config: &AppDeployData) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    let full_image_url = format!("{}/{}", &elevon_env.registry_server, &app_config.image_url);

    let options = CreateContainerOptionsBuilder::new()
        .name(&app_config.name)
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
            host_port: Some(app_config.port.to_string()),
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

    Ok(())
}
