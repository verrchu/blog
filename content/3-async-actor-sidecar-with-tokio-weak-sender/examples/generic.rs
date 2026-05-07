use std::future::Future;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

const CAP: usize = 32;
const PERIOD: Duration = Duration::from_secs(5);

// Generic handle: a thin, cloneable wrapper around the worker's input sender.
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

// The pluggable behavior. The scaffolding below knows nothing about the
// concrete message type or the state. Note there is no `on_tick`: this
// scaffolding has no timer of its own.
pub trait WorkerBehavior {
    type Req: Send + 'static;
    type State: Send;
    type Ctx: Send;

    // `self_tx` is a clone of the worker's OWN input sender. It's the only
    // injection point: the loop below has exactly one receiver, so a sidecar
    // that needs to reach the worker has nowhere else to send.
    fn init(
        ctx: &mut Self::Ctx,
        self_tx: mpsc::Sender<Self::Req>,
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
    tokio::spawn(run::<B>(ctx, tx.clone(), rx));

    Handle { tx }
}

// One receiver, no timer, no select!. The scaffolding has exactly one job:
// pump the single input channel into `on_req`.
async fn run<B>(mut ctx: B::Ctx, self_tx: mpsc::Sender<B::Req>, mut rx: mpsc::Receiver<B::Req>)
where
    B: WorkerBehavior,
{
    let mut state = B::init(&mut ctx, self_tx).await;
    while let Some(req) = rx.recv().await {
        B::on_req(req, &mut state, &mut ctx).await;
    }
    // every Handle dropped -> recv() yields None -> shut down
}

// ---- a concrete behavior whose periodic work comes from a Ticker sidecar ----

pub enum Msg {
    Sample(u64),
    Get(oneshot::Sender<u64>),
    // The Ticker's ping arrives as a normal request, on the SAME channel.
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

    async fn init(_ctx: &mut AggCtx, self_tx: mpsc::Sender<Msg>) -> AggState {
        // The Ticker is handed a strong CLONE of our own input sender —
        // there is no other channel into this worker.
        tokio::spawn(ticker(self_tx));

        AggState { samples: Vec::new(), ticks: 0 }
    }

    async fn on_req(req: Msg, state: &mut AggState, _ctx: &mut AggCtx) {
        match req {
            Msg::Sample(v) => state.samples.push(v),
            Msg::Get(reply) => {
                let _ = reply.send(state.ticks);
            }
            Msg::Tick => state.ticks += 1, // periodic work
        }
    }
}

// The Ticker sidecar: periodically pings the worker through the worker's OWN
// input channel.
async fn ticker(worker_tx: mpsc::Sender<Msg>) {
    let mut interval = tokio::time::interval(PERIOD);
    loop {
        interval.tick().await;
        if worker_tx.send(Msg::Tick).await.is_err() {
            break; // worker gone
        }
    }
}
