use std::future::Future;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

const CAP: usize = 32;
const PERIOD: Duration = Duration::from_secs(5);

pub struct Handle<Req> {
    tx: mpsc::Sender<Req>,
}

impl<Req> Clone for Handle<Req> {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

impl<Req> Handle<Req> {
    pub async fn send(&self, req: Req) {
        let _ = self.tx.send(req).await;
    }
}

pub trait WorkerBehavior {
    type Req: Send + 'static;
    type State: Send;
    type Ctx: Send;

    // The only change from the strong version: the self-sender handed to the
    // behavior is now a WeakSender. A sidecar built from it cannot, on its
    // own, keep the worker's input channel alive.
    fn init(
        ctx: &mut Self::Ctx,
        self_tx: mpsc::WeakSender<Self::Req>,
    ) -> impl Future<Output = Self::State> + Send;

    fn on_req(
        req: Self::Req,
        state: &mut Self::State,
        ctx: &mut Self::Ctx,
    ) -> impl Future<Output = ()> + Send;
}

pub fn spawn<B>(ctx: B::Ctx) -> Handle<B::Req>
where
    B: WorkerBehavior + 'static,
{
    let (tx, rx) = mpsc::channel(CAP);

    // Downgrade once, here. The strong `tx` lives only in `Handle`; the weak
    // one floats through the scaffolding to the behavior, independent of state.
    let self_tx = tx.downgrade();
    tokio::spawn(run::<B>(ctx, self_tx, rx));

    Handle { tx }
}

async fn run<B>(
    mut ctx: B::Ctx,
    self_tx: mpsc::WeakSender<B::Req>,
    mut rx: mpsc::Receiver<B::Req>,
) where
    B: WorkerBehavior,
{
    let mut state = B::init(&mut ctx, self_tx).await;
    while let Some(req) = rx.recv().await {
        B::on_req(req, &mut state, &mut ctx).await;
    }
    // only strong Handle(s) dropped -> recv() yields None -> shut down
}

// ---- the same concrete behavior, now sidecar-safe ----

pub enum Msg {
    Sample(u64),
    Get(oneshot::Sender<u64>),
    Tick,
}

pub struct AggCtx;

pub struct AggState {
    samples: Vec<u64>,
    ticks: u64,
}

pub struct Aggregator;

impl WorkerBehavior for Aggregator {
    type Req = Msg;
    type State = AggState;
    type Ctx = AggCtx;

    async fn init(_ctx: &mut AggCtx, self_tx: mpsc::WeakSender<Msg>) -> AggState {
        // The Ticker gets the WEAK sender. It can ping the worker while the
        // worker is alive, but it cannot keep it alive.
        tokio::spawn(ticker(self_tx));

        AggState { samples: Vec::new(), ticks: 0 }
    }

    async fn on_req(req: Msg, state: &mut AggState, _ctx: &mut AggCtx) {
        match req {
            Msg::Sample(v) => state.samples.push(v),
            Msg::Get(reply) => {
                let _ = reply.send(state.ticks);
            }
            Msg::Tick => state.ticks += 1,
        }
    }
}

async fn ticker(worker_tx: mpsc::WeakSender<Msg>) {
    let mut interval = tokio::time::interval(PERIOD);
    loop {
        interval.tick().await;

        // Upgrade just to send. `None` means every strong Handle is gone and
        // the worker is shutting down -> the Ticker follows it out.
        let Some(worker_tx) = worker_tx.upgrade() else {
            break;
        };
        if worker_tx.send(Msg::Tick).await.is_err() {
            break;
        }
    }
}
