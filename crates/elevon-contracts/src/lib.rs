pub mod deploy;

pub fn handle_cli_yes(yes: bool, initial_log_message: impl Into<String>) -> anyhow::Result<()> {
    if !yes {
        tracing::info!("{}", initial_log_message.into());

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            tracing::info!("Cancelled");
            std::process::exit(0);
        }
    }

    Ok(())
}
