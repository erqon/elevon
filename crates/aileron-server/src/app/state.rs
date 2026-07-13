use std::sync::Arc;

use crate::services::auth::TokenService;

#[derive(Clone)]
pub struct AppState {
    pub db: toasty::Db,
    pub tokens: Arc<TokenService>,
}
