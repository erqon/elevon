use std::collections::HashMap;

use anyhow::{Context, Result};
use elevon_config::ResolveEnvCredentials;
use elevon_contracts::deploy::{
    AppDeployPayload, AppPayload, AppRole, AppRollback, AppRollbackPayload, AppRuntimeOptions,
    WebApp, log_stream_events,
};
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap},
};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::{config::registry::RegistryConfig, image::progress::print_success};
use crate::{
    config::{Config, app::AppConfig},
    util::get_image_tag,
};

pub struct AgentClient {
    client: Client,
    base_url: Url,
    api_key: String,
}

impl AgentClient {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self> {
        let base_url = Url::parse(base_url).context("[elevon.agent.url] invalid base URL")?;

        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            base_url,
            api_key: api_key.to_string(),
        })
    }

    fn absolute_url(&self, endpoint: &str) -> Url {
        self.base_url
            .join(endpoint)
            .expect("Failed to append endpoint")
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.api_key).parse().unwrap(),
        );

        headers
    }

    #[tracing::instrument(name = "deploy", skip_all)]
    pub async fn push_deploy(
        &self,
        config: &Config,
        apps: Vec<(String, AppConfig)>,
        registry_config: RegistryConfig,
    ) -> Result<()> {
        let url = self.absolute_url("/deploy");
        let headers = self.headers();

        let app_names: Vec<_> = apps.iter().map(|(key, _)| key).collect();
        tracing::Span::current().pb_set_message(&format!("deploying {}", config.name));
        tracing::info!("Deploying apps: {:?}", app_names);

        let project_vars = config.prepare_project_env_vars(registry_config)?;

        let apps_payload: Vec<AppPayload> = apps
            .iter()
            .map(|(name, cfg)| -> Result<Option<AppPayload>> {
                let mut vars = cfg
                    .env
                    .as_ref()
                    .map_or_else(|| Ok(HashMap::default()), |env| env.resolved_credentials())?;

                vars.extend(project_vars.clone());

                let tls_with_credentials = cfg
                    .tls
                    .as_ref()
                    .map(|tls| tls.resolved_credentials())
                    .transpose()?;

                let web_app: Option<WebApp> = match &cfg.role {
                    AppRole::Web => {
                        let resolved_routing = config.routing.resolved_credentials()?;

                        Some(WebApp {
                            domain: resolved_routing.domain.clone(),
                            port: resolved_routing.port,
                        })
                    }
                    AppRole::Worker => None,
                };

                let cmd = cfg.cmd.as_ref().and_then(|c| shlex::split(c));
                let runtime_config = cfg.runtime.clone().unwrap_or_default();

                let runtime_options = AppRuntimeOptions {
                    role: cfg.role.clone(),
                    cmd,
                    restart: Some(runtime_config.restart),
                    cpu_limit: runtime_config.cpu.as_ref().map(|c| c.nano_cpus()),
                    memory_limit: runtime_config.memory.as_ref().map(|m| m.bytes()),
                    network: runtime_config.network,
                };

                Ok(Some(AppPayload {
                    image_ref: crate::image::util::image_reference(
                        &config.registry.server,
                        &config.image,
                        &get_image_tag()?,
                    ),
                    project: config.name.clone(),
                    name: name.to_string(),
                    keep_releases: config
                        .keep_releases
                        .unwrap_or_else(|| Config::default_keep_releases().unwrap()),
                    runtime_options,
                    vars: Some(vars),
                    tls: tls_with_credentials,
                    web_app,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();

        let payload = serde_json::json!(AppDeployPayload { apps: apps_payload });

        let event_stream = self
            .client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?
            .bytes_stream();

        log_stream_events(event_stream).await?;

        print_success(&format!("Deployed {}", config.name));

        Ok(())
    }

    pub async fn push_rollback(
        &self,
        config: &Config,
        apps: Vec<(String, AppConfig)>,
    ) -> Result<()> {
        let url = self.absolute_url("/deploy/rollback");
        let headers = self.headers();

        let app_names: Vec<_> = apps.iter().map(|(key, _)| key).collect();
        tracing::Span::current().pb_set_message(&format!("rolling back {}", config.name));
        tracing::info!("Rolling back apps: {:?}", app_names);

        let apps_payload: Vec<AppRollback> = apps
            .iter()
            .map(|(name, _)| AppRollback {
                project: config.name.clone(),
                name: name.clone(),
            })
            .collect::<Vec<_>>();

        let payload = serde_json::json!(AppRollbackPayload { apps: apps_payload });

        let event_stream = self
            .client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?
            .bytes_stream();

        log_stream_events(event_stream).await?;

        print_success(&format!("Rolled back {}", config.name));

        Ok(())
    }
}
