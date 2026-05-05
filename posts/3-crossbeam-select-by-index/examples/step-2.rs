use crossbeam_channel::{Receiver, Select};

fn mk_select<'a>(
    discovery_rx: &'a Receiver<Receiver<u32>>,
    data_rxs: &'a [Receiver<u32>],
) -> Select<'a> {
    let mut sel = Select::new();

    sel.recv(discovery_rx);
    for rx in data_rxs {
        sel.recv(rx);
    }

    sel
}

pub fn multiplex(discovery_rx: &Receiver<Receiver<u32>>) {
    let mut data_rxs: Vec<Receiver<u32>> = Vec::new();
    let mut sel = mk_select(discovery_rx, &data_rxs);

    loop {
        match sel.ready() {
            0 => match discovery_rx.try_recv() {
                Ok(new_rx) => {
                    data_rxs.push(new_rx);
                    sel = mk_select(discovery_rx, &data_rxs);
                }
                Err(e) if e.is_disconnected() => break,
                Err(_) => {}
            },
            idx => {
                let rx = &data_rxs[idx - 1];
                let _ = rx.try_recv();
            }
        }
    }
}
