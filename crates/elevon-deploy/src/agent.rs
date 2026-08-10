use std::collections::HashMap;

use anyhow::Result;
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap},
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

    pub async fn push_env<'a>(
        &self,
        app_name: &'a str,
        vars: &'a HashMap<String, String>,
    ) -> Result<()> {
        let url = self.absolute_url("/env");
        let headers = self.headers();

        let payload = serde_json::json!({
            "updates": [{
                "app_name": app_name,
                "vars": vars
            }]
        });

        let response = self
            .client
            .put(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        println!("response: {:?}", response);

        Ok(())
    }
}
