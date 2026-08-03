use clap::Subcommand;

#[derive(Subcommand)]
pub enum DashboardCommands {
    /// Install elevon dashboard binary to be able to start it later.
    Install,
}
