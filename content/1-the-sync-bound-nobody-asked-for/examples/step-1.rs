struct MyTask;

static_assertions::assert_impl_all!(MyTask: Send, Sync);

pub trait Task {
    fn run(&self) -> impl Future<Output = ()> + Send;
}

impl Task for MyTask {
    async fn run(&self) {}
}

pub fn spawn<T: Task + Send + 'static>(t: T) {
    tokio::spawn(async move { t.run().await });
}
