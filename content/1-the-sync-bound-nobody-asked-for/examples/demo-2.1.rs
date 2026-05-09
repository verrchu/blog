struct SendSync;

static_assertions::assert_impl_all!(SendSync: Send, Sync);

pub trait T {
    fn f(&self) -> impl Future<Output = ()> + Send;
}

impl T for SendSync {
    async fn f(&self) {}
}
