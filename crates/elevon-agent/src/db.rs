pub mod models;

use anyhow::Result;

pub async fn get_db(turso_remote_url: Option<&str>) -> Result<toasty::Db> {
    let db_path = elevon_fs::agent::get_database_path()?;

    let mut driver = toasty_driver_turso::Turso::file(db_path).concurrent_writes();

    if let Some(remote_url) = turso_remote_url {
        driver = driver.with_remote_url(remote_url);
    }

    let db = toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .build(driver)
        .await?;

    Ok(db)
}

pub async fn init_db(turso_remote_url: Option<&str>) -> Result<toasty::Db> {
    let db = get_db(turso_remote_url).await?;

    match db.push_schema().await {
        Ok(()) => Ok(db),
        Err(err) => {
            let msg = err.to_string().to_lowercase();
            if msg.contains("already exists") || msg.contains("exist") {
                tracing::info!("Schema already initialized");
                Ok(db)
            } else {
                Err(err.into())
            }
        }
    }
}
