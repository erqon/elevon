pub mod error;
mod router;
pub mod state;

pub async fn serve(config: aileron_config::Config) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let db = crate::db::get_db(&config.database).await?;
    let state = state::AppState { db };

    let app = router::router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
