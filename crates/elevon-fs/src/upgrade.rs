use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use serde::Deserialize;
use tar::Archive;
use tempfile::NamedTempFile;
use tokio::io::{AsyncWriteExt, BufWriter};

const RELEASE_URL: &str = "https://github.com/elevon-sh/elevon/releases";
const API_REPO_URL: &str = "https://api.github.com/repos/elevon-sh/elevon";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
}

pub struct ReleaseClient {
    client: reqwest::Client,
    crate_name: String,
    version: String,
}

impl ReleaseClient {
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

    async fn resolve_version(&self, current_version: &str) -> Result<Option<String>> {
        let prefix = format!("{}-v", self.crate_name);

        if self.version != "latest" {
            let requested_version = self.version.trim_start_matches('v');

            if requested_version == current_version {
                println!("Already running version {current_version}");
                return Ok(None);
            }

            return Ok(Some(format!(
                "{prefix}{}",
                self.version.trim_start_matches('v')
            )));
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
            .find(|tag| tag.starts_with(&prefix))
            .with_context(|| format!("no release found with tag prefix {prefix}"))?;

        let version = tag
            .strip_prefix(&prefix)
            .context("failed to strip prefix from tag")?;

        if version == current_version {
            println!("Already running version {current_version}");
            return Ok(None);
        } else {
            println!("Upgrading from {current_version} to {version}");
        }

        Ok(Some(tag))
    }

    async fn download_binary(&self, current_version: &str) -> Result<Option<NamedTempFile>> {
        let tag = match self.resolve_version(current_version).await? {
            Some(t) => t,
            None => return Ok(None),
        };

        let prefix = format!("{}-v", self.crate_name);
        let version = tag
            .strip_prefix(&prefix)
            .with_context(|| format!("invalid release tag: {tag}"))?;
        let asset = self.resolve_asset(version)?;
        let url = format!("{RELEASE_URL}/download/{tag}/{asset}");

        tracing::debug!(%url, "downloading release asset");

        let mut response = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("download request failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("GitHub returned an error for {url}"))?;

        let temporary_file = NamedTempFile::new().context("failed to create temporary file")?;
        let file = tokio::fs::File::from_std(temporary_file.reopen()?);
        let mut writer = BufWriter::new(file);

        while let Some(chunk) = response.chunk().await? {
            writer
                .write_all(&chunk)
                .await
                .context("failed to write downloaded asset")?;
        }

        writer
            .flush()
            .await
            .context("failed to flush downloaded asset")?;

        Ok(Some(temporary_file))
    }

    pub async fn install_binary(&self, current_version: &str, destination: &Path) -> Result<bool> {
        let archive_file = match self.download_binary(current_version).await? {
            Some(v) => v,
            None => return Ok(false),
        };

        let archive = fs::File::open(archive_file.path()).context("failed to open archive")?;
        let mut archive = Archive::new(GzDecoder::new(archive));
        let temporary_dir = tempfile::tempdir().context("failed to create extraction directory")?;
        let mut extracted_binary = None;

        for entry in archive
            .entries()
            .context("failed to read archive entries")?
        {
            let mut entry = entry.context("failed to read archive entry")?;
            if !entry.header().entry_type().is_file() {
                continue;
            }

            let path = entry.path().context("failed to read archive entry path")?;
            let Some(name) = path.file_name() else {
                continue;
            };

            if name == self.crate_name.as_str()
                || name.to_string_lossy().starts_with(&self.crate_name)
            {
                let extracted = temporary_dir.path().join(&self.crate_name);
                entry
                    .unpack(&extracted)
                    .context("failed to extract release binary")?;
                extracted_binary = Some(extracted);
                break;
            }
        }

        let extracted_binary =
            extracted_binary.context("release archive does not contain the agent binary")?;
        let parent = destination
            .parent()
            .context("installed binary path has no parent directory")?;
        fs::create_dir_all(parent).context("failed to create binary directory")?;

        let temporary_binary =
            NamedTempFile::new_in(parent).context("failed to create replacement binary")?;
        fs::copy(&extracted_binary, temporary_binary.path())
            .context("failed to stage replacement binary")?;
        fs::set_permissions(temporary_binary.path(), fs::Permissions::from_mode(0o755))
            .context("failed to set replacement binary permissions")?;
        temporary_binary
            .persist(destination)
            .map_err(|error| error.error)
            .context("failed to replace installed binary")?;

        Ok(true)
    }

    fn resolve_asset(&self, version: &str) -> Result<String> {
        let os = match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            _ => bail!("unsupported operating system"),
        };

        let arch = match std::env::consts::ARCH {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            _ => bail!("unsupported architecture"),
        };

        Ok(format!("{}-v{version}-{os}-{arch}.tar.gz", self.crate_name))
    }
}
