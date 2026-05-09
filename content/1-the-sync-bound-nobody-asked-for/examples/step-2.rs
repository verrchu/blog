pub struct Foreign(std::cell::Cell<()>);

struct MyTask {
    foreign: Foreign,
}

static_assertions::assert_impl_all!(MyTask: Send);
static_assertions::assert_not_impl_any!(MyTask: Sync);

pub trait Task {
    fn run(&self) -> impl Future<Output = ()> + Send;
}

impl Task for MyTask {
    async fn run(&self) {}
}

pub fn spawn<T: Task + Send + 'static>(t: T) {
    tokio::spawn(async move { t.run().await });
}
