---
title: "The `Sync` bound nobody asked for"
date: 2026-05-04
---

Most async runtimes spawn futures onto a thread pool, which means a spawned future
has to be safe to move between threads.
[`tokio::spawn`](https://docs.rs/tokio/1.52.2/tokio/task/fn.spawn.html) makes the
requirement explicit:

```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
```

`F: Send` cascades through everything the future captures. There are exceptions[^1],
but on a typical async codebase `F: Send` is the default — and it's the assumption
the rest of this post builds on.

[^1]: Notably [`tokio::task::spawn_local`](https://docs.rs/tokio/1.52.2/tokio/task/fn.spawn_local.html)
    inside a [`LocalSet`](https://docs.rs/tokio/1.52.2/tokio/task/struct.LocalSet.html),
    or runtime [`block_on`](https://docs.rs/tokio/1.52.2/tokio/runtime/struct.Runtime.html#method.block_on)
    with no spawning at all.

Whenever a future captures a reference and itself has to be `Send`, two facts about
Rust's reference types matter:

- `&T: Send` requires `T: Sync`.
- `&mut T: Send` only requires `T: Send`.

So a `Send` future that captures `&mut T` only needs `T: Send`, but a `Send` future
that captures `&T` needs `T: Sync`. This asymmetry affects async trait design: taking
`&self` in a method whose returned future must be `Send` forces a `Sync` bound on the
impl type — even when nothing in the trait or its consumers explicitly asks for `Sync`.

In the example below, `&self` implicitly demands `MyWorker: Sync`. Everything compiles
because `MyWorker` is trivially `Send + Sync`.

{{< code src="examples/step-1.rs" lang="rust" >}}

The `Worker` trait only visibly asks for `Send`, so giving the impl type interior mutability
seems perfectly reasonable. But `Cell` is `Send` and `!Sync`, so it makes `MyWorker` `!Sync`
too — and that breaks the implicit `Sync` requirement from `&self`.

{{< code src="examples/step-2.rs" lang="rust" >}}

```text
error: future cannot be sent between threads safely
  --> examples/step-2.rs:14:25
   |
14 |     async fn work(&self) {}
   |                         ^ future returned by `work` is not `Send`
   |
   = help: within `MyWorker`, the trait `Sync` is not implemented for `Cell<()>`
   = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock`
note: captured value is not `Send` because `&` references cannot be sent unless their referent is `Sync`
  --> examples/step-2.rs:14:19
   |
14 |     async fn work(&self) {}
   |                   ^^^^^ has type `&MyWorker` which is not `Send`, because `MyWorker` is not `Sync`
note: required by a bound in `Worker::work::{anon_assoc#0}`
  --> examples/step-2.rs:4:50
   |
 4 |     fn work(&self) -> impl Future<Output = ()> + Send;
   |                                                  ^^^^ required by this bound in `Worker::work::{anon_assoc#0}`
```

The error walks the chain: the async `work` captures `&self` as `&MyWorker`; for
the returned future to satisfy `+ Send`, `&MyWorker` has to be `Send`; and
`&T: Send` only holds when `T: Sync`. The `&self` parameter has been implicitly
demanding `Sync` on `Self` all along — `Cell` just made the demand visible.

There are two ways out:

1. **Make `Self: Sync`.** Replace the non-`Sync` interior-mutability primitive with a
   `Sync` one — `Mutex`, `RwLock`, an atomic. The impl type becomes `Sync`, and the
   trait compiles unchanged. But you've added synchronisation overhead to every state
   access, for a worker whose state is only ever touched from inside a single spawned
   task. It's suboptimal™.
2. **Switch `&self` to `&mut self`.** `&mut T: Send` requires only `T: Send`,
   with no involvement of `Sync` at all:

{{< code src="examples/step-3.rs" lang="rust" >}}

This compiles. Now the trait carries no `Sync` requirement anywhere.

Underneath all of this is `&mut T` being the *unique* reference, not the *mutable*
one. In Rust the instinct is often to reach for `&` over `&mut` — in trait method
signatures, in function parameters, etc. — to *tighten* the contract: no
mutation allowed. Here it goes the other way. Using `&mut self` instead of
`&self` guarantees unique access to `Self` for the duration of the call, which
rules out cross-thread sharingSo the `Sync` bound disappears, and the "looser"
receiver actually buys us a *smaller* bound surface, not a bigger one.

More on mutability vs ownership can be found
[here](https://smallcultfollowing.com/babysteps/blog/2014/05/13/focusing-on-ownership/).
