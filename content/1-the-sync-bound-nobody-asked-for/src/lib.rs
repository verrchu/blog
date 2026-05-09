// The post's content lives in the sibling `examples/` directory. Each step is
// a self-contained `Task` trait + `MyTask` impl + `spawn` consumer.
//
//   examples/step-1.rs — baseline: `&self` method, unit-struct impl. Compiles.
//   examples/step-2.rs — `MyTask` embeds a `Foreign` field (Send, !Sync).
//                        Fails: future returned by `run` is not Send because
//                        `&MyTask` requires `MyTask: Sync`.
//   examples/step-3.rs — fix: switch `&self` to `&mut self`. Compiles —
//                        `&mut T: Send` only needs `T: Send`, no Sync.
//
// Run `cargo check --example step-N` to inspect each one in isolation.
