use clap::Subcommand;

#[derive(Subcommand)]
pub enum DashboardCommands {
    /// Install Aileron dashboard binary to be able to start it later.
    Install,
}
