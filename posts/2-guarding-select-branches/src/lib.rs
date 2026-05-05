// The post's content lives in the sibling `examples/` directory.
//
//   examples/step-1.rs — a `select!` loop with a conditional branch that uses
//                        a bare `unwrap()`. Compiles, but panics at runtime:
//                        `select!` evaluates every branch expression before
//                        checking the `if` guard.
//   examples/step-2.rs — wraps the expression in `async { ... }`. The unwrap
//                        is deferred to poll time, which the guard prevents.
//
// Run `cargo check --example step-N` to inspect each one.
