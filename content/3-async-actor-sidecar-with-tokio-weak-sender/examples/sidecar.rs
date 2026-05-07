use std::time::Duration;

use tokio::sync::mpsc;

const CAP: usize = 32;
const PERIOD: Duration = Duration::from_secs(5);

// ---- the supplementary actor ----

// Like the standalone ticker: no external messages, the handle is purely a
// shutdown handle.
pub enum TickerMsg {}

pub struct TickerHandle {
    _tx: mpsc::Sender<TickerMsg>,
}

pub struct Ticker;

impl Ticker {
    // The ticker is handed a sender into the worker's tick channel. Ticks are
    // simple pings, so the payload is just `()`.
    fn spawn(worker_tx: mpsc::Sender<()>) -> TickerHandle {
        let (tx, rx) = mpsc::channel(CAP);
        tokio::spawn(Self::run(rx, worker_tx));

        TickerHandle { _tx: tx }
    }

    async fn run(mut rx: mpsc::Receiver<TickerMsg>, worker_tx: mpsc::Sender<()>) {
        let mut interval = tokio::time::interval(PERIOD);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if worker_tx.send(()).await.is_err() {
                        break; // worker gone
                    }
                }
                msg = rx.recv() => match msg {
                    Some(msg) => match msg {}, // unreachable: TickerMsg is uninhabited
                    None => break,             // TickerHandle dropped -> shut down
                },
            }
        }
    }
}

// ---- the main actor ----

// The worker also takes no external messages here: its handle is just a
// shutdown handle. The work is driven by ticks from the Ticker sidecar.
pub enum WorkerMsg {}

pub struct WorkerHandle {
    _tx: mpsc::Sender<WorkerMsg>,
}

pub struct Worker;

impl Worker {
    pub fn spawn() -> WorkerHandle {
        let (tx, rx) = mpsc::channel(CAP);
        // A dedicated second channel for ticks. The hand-rolled select! can
        // afford this; a generic scaffolding cannot (see the next section).
        let (tick_tx, tick_rx) = mpsc::channel::<()>(CAP);

        // Spawn the sidecar alongside the worker; the worker owns its handle,
        // so the worker dropping it shuts the ticker down.
        let ticker = Ticker::spawn(tick_tx);
        tokio::spawn(Self::run(rx, tick_rx, ticker));

        WorkerHandle { _tx: tx }
    }

    async fn run(
        mut rx: mpsc::Receiver<WorkerMsg>,
        mut tick_rx: mpsc::Receiver<()>,
        _ticker: TickerHandle,
    ) {
        loop {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some(msg) => match msg {}, // unreachable: WorkerMsg is uninhabited
                    None => break,             // WorkerHandle dropped -> shut down
                },
                tick = tick_rx.recv() => match tick {
                    Some(()) => Self::on_tick(),
                    None => break, // ticker gone
                },
            }
        }
        // Returning here drops `_ticker`, which shuts the Ticker down too.
    }

    fn on_tick() {
        // periodic work goes here
    }
}
