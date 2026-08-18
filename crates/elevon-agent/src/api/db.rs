pub mod models;

use anyhow::Result;

use crate::api::db::models::{Deployment, DeploymentStatus};

pub struct AgentDb {
    pub db: toasty::Db,
}

impl AgentDb {
    pub async fn new(turso_remote_url: Option<&str>) -> Result<Self> {
        let db = get_db(turso_remote_url).await?;

        match db.push_schema().await {
            Ok(()) => Ok(Self { db }),
            Err(err) => {
                let msg = err.to_string().to_lowercase();
                if msg.contains("already exists") || msg.contains("exist") {
                    Ok(Self { db })
                } else {
                    Err(err.into())
                }
            }
        }
    }

    pub async fn get_active_deployments(&mut self) -> Result<Vec<Deployment>> {
        let deployments =
            Deployment::filter(Deployment::fields().status().eq(DeploymentStatus::Active))
                .exec(&mut self.db)
                .await?;

        Ok(deployments)
    }
}

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
