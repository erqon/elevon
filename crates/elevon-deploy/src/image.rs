mod progress;
pub mod util;

use std::path::Path;

use bollard::{
    Docker,
    auth::DockerCredentials,
    body_full,
    query_parameters::{BuildImageOptionsBuilder, PushImageOptionsBuilder},
};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::{
    config::{BuildConfig, registry::RegistryConfig},
    util::COMMIT_SHA,
};

use self::progress::{ProgressMode, drain_progress_stream, print_success};

#[tracing::instrument(
    name = "build",
    skip_all,
    fields(image = %image_name, registry = %registry_server)
)]
pub async fn build_image(
    image_name: &str,
    registry_server: &str,
    build_config: &BuildConfig,
    config_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let (context, dockerfile) = util::get_build_context(config_path, build_config);
    let docker = Docker::connect_with_local_defaults()?;

    let full_image_name = util::full_image_name(registry_server, image_name, COMMIT_SHA);

    let status_msg = format!("building {full_image_name}");
    tracing::Span::current().pb_set_message(&status_msg);

    let options = BuildImageOptionsBuilder::default()
        .dockerfile(&dockerfile)
        .t(&full_image_name)
        .rm(true)
        .build();

    let tar = util::tar_context(&context)?;
    let stream = docker.build_image(options, None, Some(body_full(tar.into())));

    drain_progress_stream(stream, ProgressMode::Build, "build").await?;

    print_success(&format!("Built {full_image_name}"));
    Ok(())
}

#[tracing::instrument(
    name = "push",
    skip_all,
    fields(image = %image_name, registry = %creds.server)
)]
pub async fn push_image(image_name: &str, creds: RegistryConfig) -> anyhow::Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    let full_image_name = util::full_image_name(&creds.server, image_name, COMMIT_SHA);

    let options = PushImageOptionsBuilder::default().tag(COMMIT_SHA).build();
    let credentials = DockerCredentials {
        username: Some(creds.username),
        password: Some(creds.password),
        serveraddress: Some(creds.server.clone()),
        ..Default::default()
    };

    let status_msg = format!("pushing {full_image_name}");
    tracing::Span::current().pb_set_message(&status_msg);

    let stream = docker.push_image(&full_image_name, Some(options), Some(credentials));
    drain_progress_stream(stream, ProgressMode::Push, "push").await?;

    print_success(&format!("Pushed {full_image_name}"));
    Ok(())
}
