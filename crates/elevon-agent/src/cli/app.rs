use anyhow::{Result, bail};
use elevon_contracts::{
    cli::{
        ListQueryParams,
        app::{AppCommands, DeploymentCommands},
    },
    deploy::log_stream_events,
};

use crate::{
    api::db::models::{App, AppTabled, Deployment, TabledView, app::DeploymentTabled},
    cli::CliCommand,
    socket::UnixClient,
};

impl CliCommand for AppCommands {
    async fn run(command: Self) -> Result<()> {
        let unix_client = UnixClient::new()?;
        const BASE_PATH: &str = "/cli/apps/local";

        match command {
            AppCommands::List(params) => {
                let path = format!("{BASE_PATH}/list");
                let stream = unix_client
                    .client
                    .get(unix_client.resolve_url(&path))
                    .query(&params)
                    .send()
                    .await?
                    .bytes_stream();

                log_stream_events(stream).await?;
            }

            AppCommands::Deployment { subcommand } => {
                DeploymentCommands::run(subcommand).await?;
            }

            AppCommands::Remove(_args) => {
                // api_socket
                //     .send(ApiSocketEvent::AppCommands(AppCommands::Remove(args)))
                //     .await
                //     .context("failed to remove app")?;
            }
        }

        Ok(())
    }
}

pub async fn handle_app_list(
    db: &mut toasty::Db,
    params: ListQueryParams,
) -> Result<Vec<AppTabled>> {
    let result = App::all()
        .limit(params.limit)
        .offset(params.offset)
        .exec(db)
        .await?;
    let rows: Vec<AppTabled> = result.into_iter().map(|key| key.to_tabled()).collect();

    Ok(rows)
}

impl CliCommand for DeploymentCommands {
    async fn run(command: Self) -> Result<()> {
        let unix_client = UnixClient::new()?;
        const BASE_PATH: &str = "/cli/deployments/local";

        match command {
            DeploymentCommands::List(params) => {
                let path = format!("{BASE_PATH}/list");
                let stream = unix_client
                    .client
                    .get(unix_client.resolve_url(&path))
                    .query(&params)
                    .send()
                    .await?
                    .bytes_stream();

                log_stream_events(stream).await?;
            }
        }

        Ok(())
    }
}

pub async fn handle_deployment_list(
    db: &mut toasty::Db,
    params: ListQueryParams,
) -> Result<Vec<DeploymentTabled>> {
    let result = Deployment::all()
        .include(Deployment::fields().app())
        .limit(params.limit)
        .offset(params.offset)
        .exec(db)
        .await?;
    let rows: Vec<DeploymentTabled> = result.into_iter().map(|r| r.to_tabled()).collect();
    Ok(rows)
}

pub async fn handle_app_removal(db: &mut toasty::Db, app_name: String) -> Result<()> {
    let app = App::get_by_name(db, app_name).await.ok();

    let Some(_app) = app else {
        bail!("app not found");
    };

    Ok(())
}
