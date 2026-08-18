use tokio::runtime::{Builder, Runtime};

pub fn create_tokio_runtime() -> Runtime {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build")
}

pub fn run_async<T>(future: impl Future<Output = T>) -> T {
    let rt = create_tokio_runtime();
    rt.block_on(future)
}
