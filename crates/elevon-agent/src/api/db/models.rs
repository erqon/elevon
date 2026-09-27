pub mod app;
pub mod auth;

pub use app::*;
pub use auth::*;

pub trait TabledView {
    type TabledType;
    
    fn to_tabled(&self) -> Self::TabledType;
}
