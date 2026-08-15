use std::collections::HashMap;

use clap::Subcommand;
use elevon_config::ResolveEnvCredentials;

use crate::{
    agent::AgentClient,
    config::{AppConfig, registry::RegistryConfig},
};

#[derive(Subcommand)]
pub enum EnvCommands {
    Push,
}

impl EnvCommands {
    pub async fn run(
        &self,
        image_name: &str,
        agent_client: &AgentClient,
        registry_credentials: &RegistryConfig,
        root_vars: Option<HashMap<String, String>>,
        selected: Vec<(String, AppConfig)>,
    ) -> anyhow::Result<()> {
        match self {
            EnvCommands::Push => {
                agent_client
                    .push_env("default", &registry_credentials.vars())
                    .await?;

                if let Some(root_vars) = root_vars {
                    agent_client.push_env("default", &root_vars).await?;
                }

                for (name, app) in selected {
                    let env_name = format!("{}.{}", &name, &image_name);

                    let Some(env_cfg) = app.env.as_ref() else {
                        continue;
                    };

                    let vars = env_cfg.resolved_credentials()?;
                    agent_client.push_env(&env_name, &vars).await?;

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
