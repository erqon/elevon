use anyhow::{Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        db::models::{App, AppTabled, TabledView},
        event::{ApiSocketEvent, ApiSocketEventResponse},
    },
    cli::ListQueryParams,
    socket::{Socket, SocketEventHandler, SocketType},
};

#[derive(Subcommand, Serialize, Deserialize)]
pub enum AppCommands {
    #[command(about = "List all apps")]
    List(ListQueryParams),
}

impl AppCommands {
    pub async fn run(command: Self) -> Result<()> {
        let api_socket = Socket::new(SocketType::Api)?;

        match command {
            AppCommands::List(params) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::AppCommands(AppCommands::List(params)))
                    .await
                    .context("failed to list auth keys")?;

                if let Some(ApiSocketEventResponse::AppResponse(AppSocketResponse::AppList(rows))) =
                    response
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
pub enum AppSocketResponse {
    AppList(Vec<AppTabled>),
}

impl SocketEventHandler for AppCommands {
    type Command = AppCommands;
    type Response = AppSocketResponse;
    type EventRespose = ApiSocketEventResponse;

    async fn handle_event(
        command: Self::Command,
        state: crate::api::state::SharedApiState,
        _emitter: crate::socket::Emitter<Self::EventRespose>,
    ) -> Result<Self::Response> {
        match command {
            AppCommands::List(params) => {
                let data =
                    handle_app_list(&mut state.db.get(), params.limit, params.offset).await?;
                Ok(AppSocketResponse::AppList(data))
            }
        }
    }
}

pub async fn handle_app_list(
    db: &mut toasty::Db,
    limit: usize,
    offset: usize,
) -> Result<Vec<AppTabled>> {
    let limit = if limit == 0 { i64::MAX as usize } else { limit };

    let result = App::all().limit(limit).offset(offset).exec(db).await?;
    let rows: Vec<AppTabled> = result.into_iter().map(|key| key.to_tabled()).collect();

    Ok(rows)
}
