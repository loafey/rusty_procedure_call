use std::future::Future;

mod example;
mod sum_ports;
mod unused_test;

#[allow(unused)]
fn block_async<T>(v: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(v)
}
