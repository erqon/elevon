use anyhow::Result;
use clap::{Args, Subcommand};
use jiff::Timestamp;

use crate::{api::db::models::AuthKey, env::ElevonEnv};

#[derive(Subcommand)]
pub enum KeyCommands {
    #[command(about = "Command to create new auth keys")]
    Create(KeyCreateArgs),

    List,

    Revoke,
}

impl KeyCommands {
    pub async fn run(command: &KeyCommands, env: &ElevonEnv) -> Result<()> {
        match command {
            KeyCommands::Create(args) => {
                create(args.name.clone(), env.turso_remote_url.clone()).await?;
            }
            KeyCommands::List => list().await?,
            KeyCommands::Revoke => {}
        }

        Ok(())
    }
}

#[derive(Args)]
pub struct KeyCreateArgs {
    #[arg(long, short, help = "A name for the key", default_value = "Default")]
    pub name: String,
}

pub async fn create(name: impl Into<String>, remote_url: Option<String>) -> Result<()> {
    tracing::info!("Creating an auth key...");
    let mut db = crate::api::db::init_db(remote_url.as_deref()).await?;

    let api_key = elevon_http::token::opaque();
    let hashed_api_key = elevon_http::token::hash(&api_key);

    let now = Timestamp::now();
    let expires_at = now.checked_add(jiff::Span::new().hours(30 * 24))?;

    toasty::create!(AuthKey {
        name: name.into(),
        key_hash: hashed_api_key,
        enabled: true,
        expires_at
    })
    .exec(&mut db)
    .await?;

    tracing::info!("Your API Key: {}", &api_key);

    Ok(())
}

async fn list() -> Result<()> {
    let mut db = crate::api::db::init_db(None).await?;

    let auth_keys = AuthKey::all().exec(&mut db).await?;

    for key in auth_keys {
        tracing::info!("{}", key);
    }

    Ok(())
}
