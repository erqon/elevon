#[derive(Clone)]
pub struct AppState {
    pub db: toasty::Db,
    pub config: aileron_config::Config,
}
