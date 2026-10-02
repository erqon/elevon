use anyhow::Result;

pub mod cli;
pub mod deploy;

pub fn handle_cli_yes(yes: bool, initial_log_message: impl Into<String>) -> Result<()> {
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

pub fn cli_verify_action_with_items(yes: bool, items: &[String], message: &str) -> Result<()> {
    let formatted_keys: String = items
        .iter()
        .map(|id| format!("- {}", id))
        .collect::<Vec<_>>()
        .join("\n");

    let message = [
        &format!("{message}:"),
        "",
        &formatted_keys,
        "",
        "Continue? [y/N]",
    ]
    .join("\n");

    handle_cli_yes(yes, message)
}
