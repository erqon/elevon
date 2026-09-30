use anyhow::{Context, Result};
use elevon_contracts::cli::{
    ListQueryParams,
    app::{AppCommands, DeploymentCommands},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        db::models::{App, AppTabled, Deployment, TabledView, app::DeploymentTabled},
        event::{ApiSocketEvent, ApiSocketEventResponse},
    },
    cli::CliComponent,
    socket::SocketEventHandler,
};

impl CliComponent for AppCommands {
    async fn run(command: Self) -> Result<()> {
        let api_socket = Self::get_socket()?;

        match command {
            AppCommands::List(params) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::AppCommands(AppCommands::List(params)))
                    .await
                    .context("failed to list auth keys")?;

                if let Some(ApiSocketEventResponse::AppResponse(AppSocketResponse::List(rows))) =
                    response
                {
                    tracing::info!(
                        "{}",
                        tabled::Table::new(rows).with(tabled::settings::Style::modern())
                    );
                }
            }

            AppCommands::Deployment { subcommand } => {
                DeploymentCommands::run(subcommand).await?;
            }

            AppCommands::Remove(_args) => {}
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "key", content = "data")]
pub enum AppSocketResponse {
    List(Vec<AppTabled>),
    Deployment(DeploymentSocketResponse),
}

impl SocketEventHandler for AppCommands {
    type Command = AppCommands;
    type Response = AppSocketResponse;
    type EventRespose = ApiSocketEventResponse;

    async fn handle_event(
        command: Self::Command,
        state: crate::api::state::SharedApiState,
        emitter: crate::socket::Emitter<Self::EventRespose>,
    ) -> Result<Option<Self::Response>> {
        match command {
            AppCommands::List(params) => {
                let data = handle_app_list(&mut state.db.get(), params.get_params()).await?;
                Ok(Some(AppSocketResponse::List(data)))
            }

            AppCommands::Deployment { subcommand } => {
                let result = DeploymentCommands::handle_event(subcommand, state, emitter).await?;
                if let Some(res) = result {
                    return Ok(Some(AppSocketResponse::Deployment(res)));
                }
                Ok(None)
            }

            AppCommands::Remove(_args) => Ok(None),
        }
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

impl CliComponent for DeploymentCommands {
    async fn run(command: Self) -> Result<()> {
        let api_socket = Self::get_socket()?;

        match command {
            DeploymentCommands::List(params) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::DeploymentCommands(
                        DeploymentCommands::List(params),
                    ))
                    .await
                    .context("failed to list auth keys")?;

                if let Some(ApiSocketEventResponse::DeploymentResponse(
                    DeploymentSocketResponse::List(rows),
                )) = response
                {
                    tracing::info!(
                        "{}",
                        tabled::Table::new(rows).with(tabled::settings::Style::modern())
                    );
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "key", content = "data")]
pub enum DeploymentSocketResponse {
    List(Vec<DeploymentTabled>),
}

impl SocketEventHandler for DeploymentCommands {
    type Command = DeploymentCommands;
    type Response = DeploymentSocketResponse;
    type EventRespose = ApiSocketEventResponse;

    async fn handle_event(
        command: Self::Command,
        state: crate::api::state::SharedApiState,
        _emitter: crate::socket::Emitter<Self::EventRespose>,
    ) -> Result<Option<Self::Response>> {
        match command {
            DeploymentCommands::List(params) => {
                let data = handle_deployment_list(&mut state.db.get(), params.get_params()).await?;
                Ok(Some(DeploymentSocketResponse::List(data)))
            }
        }
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
