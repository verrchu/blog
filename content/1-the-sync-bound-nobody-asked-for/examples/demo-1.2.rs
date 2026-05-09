struct SendOnly(std::cell::Cell<()>);

static_assertions::assert_impl_all!(SendOnly: Send);
static_assertions::assert_not_impl_any!(SendOnly: Sync);

pub trait T {
    fn f(&self) -> impl Future<Output = ()>;
}

impl T for SendOnly {
    async fn f(&self) {}
}
