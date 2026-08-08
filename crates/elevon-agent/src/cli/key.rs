use anyhow::Result;
use clap::{Args, Subcommand};
use jiff::{Timestamp, ToSpan};

use crate::{db::models::AuthKey, env::ElevonEnv};

#[derive(Subcommand)]
pub enum KeyCommands {
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
            KeyCommands::List => {}
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
    let mut db = crate::db::init_db(remote_url.as_deref()).await?;

    let api_key = elevon_http::token::opaque();
    let hashed_api_key = elevon_http::token::hash(&api_key);

    let now = Timestamp::now();
    let expires_at = now.checked_add(30.days())?;

    toasty::create!(AuthKey {
        name: name.into(),
        key_hash: hashed_api_key,
        enabled: true,
        expires_at
    })
    .exec(&mut db)
    .await?;

    tracing::info!("Your API Key: {}", &api_key);

    tracing::info!("Elevon Agent has been setup. Now you can run ---");

    Ok(())
}
