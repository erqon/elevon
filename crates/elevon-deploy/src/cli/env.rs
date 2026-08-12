use clap::Subcommand;
use elevon_config::ResolveEnvCredentials;

use crate::{
    agent::AgentClient,
    config::{app::AppConfig, registry::RegistryConfig},
};

#[derive(Subcommand)]
pub enum EnvCommands {
    Push,
}

impl EnvCommands {
    pub async fn run(
        &self,
        agent_client: &AgentClient,
        registry_credentials: &RegistryConfig,
        selected: &Vec<(&String, &AppConfig)>,
    ) -> anyhow::Result<()> {
        match self {
            EnvCommands::Push => {
                agent_client
                    .push_env("default", &registry_credentials.vars())
                    .await?;

                for (name, app) in selected {
                    let vars = app
                        .env
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("app `{name}` is missing `env`"))?
                        .resolved_credentials()?;

                    agent_client.push_env(name, &vars).await?;
                    tracing::debug!(
                        "pushing env for app `{name}`: {:?}",
                        vars.keys().collect::<Vec<_>>()
                    );
                }

                tracing::info!("Environment variables were pushed successfully!");
            }
        }

        Ok(())
    }
}
