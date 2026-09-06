use sha2::{Digest, Sha256};
use std::process::Command;

/// Returns an image tag with the following style:
/// {image_repository}:{git_commit_sha}-{Option<{git diff sha}>}
pub fn get_image_tag() -> std::io::Result<String> {
    // TODO: Handle non git directories

    let sha_output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;

    let mut commit_sha = String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string();

    let diff_output = Command::new("git").args(["diff", "HEAD"]).output()?;
    let untracked_output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()?;

    let is_dirty = !diff_output.stdout.is_empty() || !untracked_output.stdout.is_empty();

    if !is_dirty {
        return Ok(commit_sha);
    }

    let mut hasher = Sha256::new();
    hasher.update(&diff_output.stdout);
    hasher.update(&untracked_output.stdout);
    let hash_result = hasher.finalize();

    let diff_hash: String = hash_result
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    let short_diff_hash = &diff_hash[..7];

    commit_sha.push('-');
    commit_sha.push_str(short_diff_hash);

    Ok(commit_sha)
}
