use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub async fn run(mut rx: mpsc::Receiver<u32>, batch_size: usize) {
    let mut queue: Vec<u32> = Vec::new();
    let mut flush: Option<JoinHandle<usize>> = None;

    loop {
        tokio::select! {
            biased;

            msg = rx.recv() => {
                match msg {
                    Some(item) => queue.push(item),
                    None => break,
                }
            }

            // BUG: panics at runtime when `flush` is `None`.
            // `select!` evaluates this expression even when the guard is false.
            result = flush.as_mut().unwrap(), if flush.is_some() => {
                let _count = result.expect("flush task panicked");
                flush = None;
            }
        }

        if flush.is_none() && queue.len() >= batch_size {
            let batch: Vec<u32> = queue.drain(..batch_size).collect();
            flush = Some(tokio::spawn(async move { batch.len() }));
        }
    }
}
