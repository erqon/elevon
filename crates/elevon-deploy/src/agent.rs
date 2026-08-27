use std::collections::HashMap;

use anyhow::Result;
use elevon_config::ResolveEnvCredentials;
use elevon_contracts::deploy::{
    AppDeployPayload, AppEnvPayload, AppEnvSetPayload, AppOptions, AppPayload, AppRole, WebApp,
    log_stream_events,
};
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap},
};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::image::progress::{print_success, print_success_compact};
use crate::{
    config::{AppConfig, Config},
    util::COMMIT_SHA,
};

pub struct AgentClient {
    client: Client,
    base_url: Url,
    api_key: String,
}

impl AgentClient {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, reqwest::Error> {
        let base_url = Url::parse(base_url).expect("Invalid base URL configuration");

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
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

    pub async fn push_env(
        &self,
        project_name: String,
        app_name: String,
        vars: &HashMap<String, String>,
    ) -> Result<()> {
        let url = self.absolute_url("/env");
        let headers = self.headers();

        let apps_payload = AppEnvPayload {
            project: project_name,
            name: app_name,
            vars: vars.clone(),
            tls: None,
        };

        let payload = serde_json::json!(AppEnvSetPayload {
            apps: vec![apps_payload]
        });

        self.client
            .put(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    #[tracing::instrument(name = "push-envs", skip_all)]
    pub async fn push_envs(
        &self,
        project_name: &str,
        apps: Vec<(String, AppConfig)>,
    ) -> Result<()> {
        let url = self.absolute_url("/env");
        let headers = self.headers();

        tracing::Span::current().pb_set_message(&format!("pushing envs for {project_name}"));

        let apps_payload: Vec<AppEnvPayload> = apps
            .iter()
            .map(|(name, config)| -> Result<Option<AppEnvPayload>> {
                let vars = match &config.env {
                    Some(vars) => vars.resolved_credentials()?,
                    None => HashMap::default(),
                };
                let tls_with_credentials = match &config.tls {
                    Some(tls) => Some(tls.resolved_credentials()?),
                    None => None,
                };

                Ok(Some(AppEnvPayload {
                    project: project_name.to_string(),
                    name: name.clone(),
                    vars: vars.clone(),
                    tls: tls_with_credentials,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();

        let payload = serde_json::json!(AppEnvSetPayload { apps: apps_payload });

        self.client
            .put(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        print_success_compact(&format!("Pushed app envs for {project_name}"));

        Ok(())
    }

    #[tracing::instrument(name = "deploy", skip_all)]
    pub async fn push_deploy(&self, config: &Config, apps: Vec<(String, AppConfig)>) -> Result<()> {
        let url = self.absolute_url("/deploy");
        let headers = self.headers();

        let app_names: Vec<_> = apps.iter().map(|(key, _)| key).collect();
        tracing::Span::current().pb_set_message(&format!("deploying {}", config.name));
        tracing::info!("Deploying apps: {:?}", app_names);

        let apps_payload: Vec<AppPayload> = apps
            .iter()
            .map(|(name, cfg)| {
                let web_app: Option<WebApp> = match &cfg.role {
                    AppRole::Web => Some(WebApp {
                        domain: config.routing.domain.clone(),
                        port: config.routing.port,
                    }),
                    AppRole::Worker => None,
                };

                let cmd = cfg.cmd.as_ref().and_then(|c| shlex::split(c));
                let options = AppOptions {
                    role: cfg.role.clone(),
                    cmd,
                };

                AppPayload {
                    project: config.name.clone(),
                    image: crate::image::util::full_image_name(
                        &config.registry.server,
                        &config.image,
                        COMMIT_SHA,
                    ),
                    name: name.to_string(),
                    options,
                    web_app,
                }
            })
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
}
