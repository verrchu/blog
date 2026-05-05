use crossbeam_channel::{Receiver, Select};

pub fn multiplex(
    control_rx: &Receiver<String>,
    data_rxs: &[Receiver<u32>],
) {
    let mut sel = Select::new();

    sel.recv(control_rx);
    for rx in data_rxs {
        sel.recv(rx);
    }

    loop {
        match sel.ready() {
            0 => match control_rx.try_recv() {
                Ok(_cmd) => {}
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
