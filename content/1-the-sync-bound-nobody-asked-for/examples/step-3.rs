use std::cell::Cell;

pub trait Worker {
    fn work(&mut self) -> impl Future<Output = ()> + Send;
}

#[allow(dead_code)]
struct MyWorker(Cell<()>);

static_assertions::assert_impl_all!(MyWorker: Send);
static_assertions::assert_not_impl_any!(MyWorker: Sync);

impl Worker for MyWorker {
    async fn work(&mut self) {}
}

pub fn spawn<W: Worker + Send + 'static>(mut w: W) {
    tokio::spawn(async move {
        loop {
            w.work().await;
        }
    });
}
