use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::config::{BuildConfig, BuildOptions};

pub fn image_reference(registry_server: &str, image_name: &str, tag: &str) -> String {
    format!(
        "{}/{}:{}",
        registry_server.trim_end_matches('/'),
        image_name.trim_start_matches('/'),
        tag,
    )
}

pub fn get_build_context(
    config_path: impl AsRef<Path>,
    build_config: &BuildConfig,
) -> (PathBuf, String) {
    let config_dir = config_path
        .as_ref()
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let (context, dockerfile): (PathBuf, String) = match build_config {
        BuildConfig::Path(path) => (config_dir.join(path), "Dockerfile".to_string()),
        BuildConfig::Options(BuildOptions { path, dockerfile }) => (
            config_dir.join(path),
            dockerfile
                .clone()
                .unwrap_or_else(|| "Dockerfile".to_string()),
        ),
    };

    (context, dockerfile)
}

pub fn tar_context(dir: &Path) -> anyhow::Result<Vec<u8>> {
    use ignore::WalkBuilder;
    use std::fs::File;
    use tar::{Builder, EntryType, Header, HeaderMode};

    let root = dir.canonicalize()?;

    anyhow::ensure!(
        root.is_dir(),
        "build context not a directory: {}",
        root.display()
    );

    let walker = WalkBuilder::new(&root)
        .add_custom_ignore_filename(".dockerignore")
        .build();

    let mut ar = Builder::new(Vec::new());
    ar.mode(HeaderMode::Deterministic);

    for result in walker {
        let entry = result.context("failed to get entry")?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;

        if !metadata.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(&root)
            .context("failed to make archive path relative")?;

        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Regular);
        header.set_mode(0o644);
        header.set_size(metadata.len());
        header.set_path(relative_path)?;
        header.set_cksum();
        ar.append(&header, File::open(path)?)?;
    }

    Ok(ar.into_inner()?)
}
