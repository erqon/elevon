use git_version::git_version;

pub const COMMIT_SHA: &str = git_version!();
