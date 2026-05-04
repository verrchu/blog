// The post's content lives in the sibling `examples/` directory. Each step is
// a self-contained `Worker` trait + `MyWorker` impl + `spawn` consumer.
//
//   examples/step-1.rs — baseline: `&self` method, unit-struct impl. Compiles.
//   examples/step-2.rs — same trait; impl wraps `Cell<()>` (Send, !Sync).
//                        Fails: future returned by `work` is not Send because
//                        `&MyWorker` requires `MyWorker: Sync`.
//   examples/step-3.rs — fix: switch `&self` to `&mut self`. Compiles —
//                        `&mut T: Send` only needs `T: Send`.
//
// Run `cargo check --example step-N` to inspect each one in isolation.
