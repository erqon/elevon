use sha2::{Digest, Sha256};
use std::process::Command;

/// Returns an image tag with the following style:
/// {image_repository}:{git_commit_sha}-{Option<{git diff sha}>}
pub fn get_image_tag() -> std::io::Result<String> {
    // TODO: Handle non git and no git commit repos

    let commit_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_else(|| "0000000".to_string());

    let diff_bytes = Command::new("git")
        .args(["diff", "HEAD"])
        .output()
        .map(|o| o.stdout)
        .unwrap_or_default();

    let untracked_bytes = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
        .map(|o| o.stdout)
        .unwrap_or_default();

    let is_dirty = !diff_bytes.is_empty() || !untracked_bytes.is_empty();

    if !is_dirty {
        return Ok(commit_sha);
    }

    let mut hasher = Sha256::new();
    hasher.update(&diff_bytes);
    hasher.update(&untracked_bytes);
    let hash_result = hasher.finalize();

    let diff_hash: String = hash_result
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    let short_diff_hash = &diff_hash[..7];

    if commit_sha == "0000000" {
        Ok(format!("0000000-{short_diff_hash}"))
    } else {
        Ok(format!("{commit_sha}-{short_diff_hash}"))
    }
}
