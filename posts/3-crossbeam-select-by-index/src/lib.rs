// The post's content lives in the sibling `examples/` directory.
//
//   examples/step-1.rs — static dispatch: fixed receivers at known indices,
//                        data receivers accessed via `idx - 1` offset.
//   examples/step-2.rs — dynamic subscriptions: new receivers arrive on a
//                        discovery channel; Select is rebuilt on each change.
//   examples/step-3.rs — removing disconnected receivers via `select.remove()`
//                        with a HashMap for index-to-receiver lookup.
//
// Run `cargo check --example step-N` to inspect each one.
