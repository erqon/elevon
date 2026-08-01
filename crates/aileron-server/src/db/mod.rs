pub mod model;

use anyhow::Result;

use crate::DatabaseConfig;

pub async fn get_db(config: &DatabaseConfig) -> Result<toasty::Db> {
    let driver = toasty_driver_turso::Turso::file(config.path.clone()).concurrent_writes();

    let db = toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .build(driver)
        .await?;

    Ok(db)
}
