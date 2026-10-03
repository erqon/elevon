pub mod app;
pub mod auth;
pub mod deployment;

pub use app::{App, AppTabled};
pub use auth::{AuthKey, AuthKeyTabled};
pub use deployment::{
    Deployment, DeploymentRuntimeOption, DeploymentRuntimeOptions, DeploymentStatus,
};

pub trait TabledView {
    type TabledType;

    fn to_tabled(&self) -> Self::TabledType;

    fn default_value() -> String {
        "-".to_string()
    }
}
