pub mod app;
pub mod auth;

pub use app::{
    App, AppTabled, Deployment, DeploymentRuntimeOption, DeploymentRuntimeOptions, DeploymentStatus,
};
pub use auth::{AuthKey, AuthKeyTabled};

pub trait TabledView {
    type TabledType;

    fn to_tabled(&self) -> Self::TabledType;

    fn default_value() -> String {
        "-".to_string()
    }
}
