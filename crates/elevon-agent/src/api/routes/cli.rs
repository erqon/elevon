use axum::{Router, extract::State, routing::get};
use elevon_contracts::deploy::{StreamEvent, StreamLogLevel};

use crate::api::{
    db::models::AuthKey,
    event::handle_key_list,
    state::SharedApiState,
    stream::{StreamResponse, emit, spawn_streaming_task},
};

pub fn router() -> Router<SharedApiState> {
    Router::new().nest("/key", KeyCommands::register_routes())
}

trait CommandsTrait {
    fn register_routes() -> Router<SharedApiState>;
}

struct KeyCommands;

impl KeyCommands {
    async fn list(_: AuthKey, State(state): State<SharedApiState>) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            let keys = handle_key_list(&mut state.db.get()).await?;
            let mut table = tabled::Table::new(keys);
            table.with(tabled::settings::Style::modern());

            emit(
                &tx,
                StreamEvent::Log {
                    level: StreamLogLevel::Info,
                    message: format!("\n{}", table),
                },
            )
            .await;

            Ok(())
        })
    }
}

impl CommandsTrait for KeyCommands {
    fn register_routes() -> Router<SharedApiState> {
        Router::new().route("/list", get(KeyCommands::list))
    }
}
