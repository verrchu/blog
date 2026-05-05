use crossbeam_channel::{Receiver, Select};
use std::collections::HashMap;

pub fn multiplex(data_rxs: &[Receiver<u32>]) {
    let mut sel = Select::new();
    let mut receivers: HashMap<usize, &Receiver<u32>> = HashMap::new();

    for rx in data_rxs {
        let idx = sel.recv(rx);
        receivers.insert(idx, rx);
    }

    loop {
        let idx = sel.ready();
        let rx = receivers[&idx];

        match rx.try_recv() {
            Ok(_val) => {}
            Err(e) if e.is_disconnected() => {
                sel.remove(idx);
                receivers.remove(&idx);
                if receivers.is_empty() {
                    break;
                }
            }
            Err(_) => {}
        }
    }
}
