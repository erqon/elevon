use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const REPO_URL: &str = "https://github.com/erqon/elevon";
const API_REPO_URL: &str = "https://api.github.com/repos/erqon/elevon";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
}

struct HttpClient {
    client: reqwest::Client,
    crate_name: String,
    version: String,
}

impl HttpClient {
    pub fn new(crate_name: String, version: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .user_agent("elevon-updater")
            .build()?;

        Ok(Self {
            client,
            crate_name,
            version,
        })
    }

    async fn resolve_version(&self) -> Result<String> {
        if self.version != "latest" {
            return Ok(self.version.trim_start_matches('v').to_owned());
        }

        let url = format!("{API_REPO_URL}/releases?per_page=100");
        tracing::debug!(%url, "fetching releases");

        let releases: Vec<Release> = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("request failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("GitHub returned an error for {url}"))?
            .json()
            .await
            .context("failed to decode GitHub releases response")?;

        let tag = releases
            .into_iter()
            .map(|release| release.tag_name)
            .find(|tag| tag.starts_with(&self.crate_name))
            .context("no CLI release found")?;

        Ok(tag
            .strip_prefix(&self.crate_name)
            .expect("tag prefix was checked")
            .to_owned())
    }

    async fn download_binary(&self) {
        // let url = format!("{REPO_URL}/releases/download/")
    }
}

fn resolve_asset(crate_name: &str, version: &str) -> Result<Option<String>> {
    let output = Command::new("uname")
        .arg("-s")
        .output()
        .context("failed to get os")?;

    if output.status.success() {
        let os_raw = String::from_utf8_lossy(&output.stdout);
        let os = os_raw.trim().to_lowercase();

        if os != "linux" || os != "darwin" {
            bail!("unsupported operating system");
        }

        let arch_output = Command::new("uname")
            .arg("-m")
            .output()
            .context("failed to get arch")?;

        if arch_output.status.success() {
            let arch_raw = String::from_utf8_lossy(&arch_output.stdout);
            let mut arch = arch_raw.trim().to_lowercase();

            match arch.as_str() {
                "x86_64" | "amd64" => arch = "x86_64".to_string(),
                "arm64" | "aarch64" => arch = "aarch64".to_string(),
                _ => {}
            }

            return Ok(Some(format!("{crate_name}-v{version}-{os}-{arch}.tar.gz")));
        }
    }

    Ok(None)
}
