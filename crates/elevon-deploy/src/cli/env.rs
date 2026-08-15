use std::collections::HashMap;

use clap::Subcommand;

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
        project_name: &str,
        agent_client: &AgentClient,
        registry_credentials: &RegistryConfig,
        root_vars: Option<HashMap<String, String>>,
        selected: Vec<(String, AppConfig)>,
    ) -> anyhow::Result<()> {
        match self {
            EnvCommands::Push => {
                agent_client
                    .push_env(project_name.to_string(), &registry_credentials.vars())
                    .await?;

                if let Some(root_vars) = root_vars {
                    agent_client
                        .push_env(project_name.to_string(), &root_vars)
                        .await?;
                }

                agent_client.push_envs(project_name, selected).await?;

                tracing::info!("Environment variables were pushed successfully!");
            }
        }

        Ok(())
    }
}
