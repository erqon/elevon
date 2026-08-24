use crate::cli::CliArgs;

pub fn run(_args: CliArgs) -> anyhow::Result<()> {
    // The agent domain has to be saved in project's environment variables,
    // then be loaded during runtime.
    //
    // This has to do the `setup` and `install-systemd` jobs, which would
    // make them useless commands to keep.
    //
    // Basically setup db -> setup systemd files -> and start `api` and `proxy`.
    //
    // Also the main thing has to be that this just saved config data
    // either in env (agent.domain) or as files (tls.cert, tls.key).
    // Then they are being loaded during runtime dynamically,
    // not trought command arguments.

    Ok(())
}
