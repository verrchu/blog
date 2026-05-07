use tokio::sync::{mpsc, oneshot};

const CAP: usize = 32; // arbitrary

pub enum Msg {
    Inc,
    Get { reply_to: oneshot::Sender<u64> },
}

pub struct Counter;

impl Counter {
    pub fn spawn() -> mpsc::Sender<Msg> {
        let (tx, rx) = mpsc::channel(CAP);
        tokio::spawn(Self::run(rx));

        tx
    }

    async fn run(mut rx: mpsc::Receiver<Msg>) {
        let mut counter: u64 = 0;
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Inc => counter += 1,
                Msg::Get { reply_to } => {
                    let _ = reply_to.send(counter);
                }
            }
        }
    }
}
