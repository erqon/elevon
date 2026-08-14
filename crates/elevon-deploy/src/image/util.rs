use std::path::{Path, PathBuf};

use crate::config::{BuildConfig, BuildOptions};

pub fn full_image_name(registry_server: &str, image_name: &str, tag: &str) -> String {
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
    use std::fs::File;
    use tar::{Builder, EntryType, Header, HeaderMode};
    use walkdir::WalkDir;

    let root = dir.canonicalize()?;
    anyhow::ensure!(
        root.is_dir(),
        "build context not a directory: {}",
        root.display()
    );

    let mut ar = Builder::new(Vec::new());
    ar.mode(HeaderMode::Deterministic);

    for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }

        let rel = path.strip_prefix(&root)?;
        if rel.starts_with(".git") || rel == Path::new("elevon-deploy") {
            continue;
        }

        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Regular);
        header.set_mode(0o644);
        header.set_size(meta.len());
        header.set_path(rel)?;
        header.set_cksum();
        ar.append(&header, File::open(path)?)?;
    }

    Ok(ar.into_inner()?)
}
