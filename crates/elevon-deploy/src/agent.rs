use std::collections::HashMap;

use anyhow::Result;
use elevon_contracts::deploy::{AppPayload, AppReleasePayload, AppRole, WebApp};
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap},
};

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
            .join(&format!("/api/v1{}", endpoint))
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

    pub async fn push_env(&self, app_name: &str, vars: &HashMap<String, String>) -> Result<()> {
        let url = self.absolute_url("/env");
        let headers = self.headers();

        let payload = serde_json::json!({
            "updates": [{
                "app_name": app_name,
                "vars": vars
            }]
        });

        self.client
            .put(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    pub async fn push_release(
        &self,
        config: &Config,
        apps: Vec<(String, AppConfig)>,
    ) -> Result<()> {
        let url = self.absolute_url("/deploy");
        let headers = self.headers();

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

                AppPayload {
                    name: name.to_string(),
                    image: crate::image::util::full_image_name(
                        &config.registry.server,
                        &config.image,
                        COMMIT_SHA,
                    ),
                    role: cfg.role.clone(),
                    web_app,
                }
            })
            .collect();

        let payload = serde_json::json!(AppReleasePayload { apps: apps_payload });

        self.client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }
}
