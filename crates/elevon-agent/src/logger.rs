use anyhow::Result;

pub trait ActionLogger {
    fn log(&self, message: String) -> impl Future<Output = Result<()>> + Send;
}
