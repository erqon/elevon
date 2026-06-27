use aileron_config::{Config, DatabaseConfig};
use anyhow::Result;

pub mod models;

pub async fn get_db(config: Config) -> Result<toasty::Db> {
    let driver = match &config.database {
        #[cfg(feature = "turso")]
        DatabaseConfig::Turso(cfg) => turso::turso_driver(cfg),

        #[cfg(feature = "postgres")]
        DatabaseConfig::Postgres(cfg) => postgres::postgres_driver(cfg)?,

        #[cfg(all(not(feature = "turso"), feature = "postgres"))]
        DatabaseConfig::Turso(_) => panic!("Turso support not compiled in"),

        #[cfg(all(not(feature = "postgres"), feature = "turso"))]
        DatabaseConfig::Postgres(_) => panic!("Postgres support not compiled in"),
    };

    let db = toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .build(driver)
        .await?;
    Ok(db)
}

#[cfg(feature = "turso")]
mod turso {
    use aileron_config::TursoConfig;
    use toasty_driver_turso::Turso;

    pub fn turso_driver(config: &TursoConfig) -> Turso {
        toasty_driver_turso::Turso::file(config.path.to_string()).concurrent_writes()
    }
}

#[cfg(feature = "postgres")]
mod postgres {
    use aileron_config::PostgresConfig;
    use anyhow::{Context, Result};
    use toasty_driver_postgresql::PostgreSQL;

    pub fn postgres_driver(config: &PostgresConfig) -> Result<PostgreSQL> {
        let driver = toasty_driver_postgresql::PostgreSQL::new(config.url.to_string())
            .context("Failed to connect to the PostgreSQL database")?;
        Ok(driver)
    }
}
