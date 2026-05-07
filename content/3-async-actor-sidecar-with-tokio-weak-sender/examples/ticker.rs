use std::time::Duration;

use tokio::sync::mpsc;

const CAP: usize = 1; // nothing is ever sent; the channel exists only as a kill switch
const PERIOD: Duration = Duration::from_secs(5);

// Uninhabited: there is no message anyone could construct and send.
pub enum Msg {}

pub struct Handle {
    _tx: mpsc::Sender<Msg>,
}

pub struct Ticker;

impl Ticker {
    pub fn spawn() -> Handle {
        let (tx, rx) = mpsc::channel(CAP);
        tokio::spawn(Self::run(rx));

        Handle { _tx: tx }
    }

    async fn run(mut rx: mpsc::Receiver<Msg>) {
        let mut interval = tokio::time::interval(PERIOD);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    Self::on_tick();
                }
                msg = rx.recv() => match msg {
                    Some(msg) => match msg {}, // unreachable: Msg is uninhabited
                    None => break,             // all senders dropped -> shut down
                },
            }
        }
    }

    fn on_tick() {
        // periodic work goes here: flush a buffer, refresh a cache, emit a heartbeat...
    }
}
