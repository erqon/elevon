use anyhow::Result;

pub async fn get_db(remote_url: Option<String>) -> Result<toasty::Db> {
    let db_path = elevon_fs::agent::get_database_path()?;

    let mut driver = toasty_driver_turso::Turso::file(db_path)
        .concurrent_writes();

    if let Some(remote_url) = remote_url {
        driver = driver.with_remote_url(remote_url);
    }

    let db = toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .build(driver)
        .await?;

    Ok(db)
}

pub async fn init_db(remote_url: Option<String>) -> Result<()> {
    let db = get_db(remote_url).await?;
    db.push_schema().await?;
    Ok(())
}
