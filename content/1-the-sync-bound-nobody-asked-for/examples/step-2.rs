use std::cell::Cell;

pub trait Worker {
    fn work(&self) -> impl Future<Output = ()> + Send;
}

#[allow(dead_code)]
struct MyWorker(Cell<()>);

static_assertions::assert_impl_all!(MyWorker: Send);
static_assertions::assert_not_impl_any!(MyWorker: Sync);

impl Worker for MyWorker {
    async fn work(&self) {}
}

pub fn spawn<W: Worker + Send + 'static>(w: W) {
    tokio::spawn(async move {
        loop {
            w.work().await;
        }
    });
}
