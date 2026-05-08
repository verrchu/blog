---
title: "The `Sync` bound nobody asked for"
date: 2026-05-04
---

[src]: https://github.com/verrchu/blog/tree/main/content/1-the-sync-bound-nobody-asked-for

`&self` on an async trait method whose returned future must be `Send` implicitly forces `Sync`
on the impl type — even if neither the trait nor its callers ever ask for `Sync`.

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

`F: Send` cascades through everything the future captures.

Whenever a future captures a reference and itself has to be `Send`, two facts about
Rust's reference types matter:

- `&T: Send` requires `T: Sync`.
- `&mut T: Send` only requires `T: Send`.

So a `Send` future that captures `&mut T` only needs `T: Send`, but a `Send` future
that captures `&T` needs `T: Sync`.

In the example below, everything compiles because `MyWorker` is trivially `Send + Sync`.

{{< code src="examples/step-1.rs" lang="rust" >}}

The `Worker` trait only visibly asks for `Send`, so giving the impl type interior mutability
seems reasonable. But `Cell` is `Send` and `!Sync`, so it makes `MyWorker` `!Sync` too, which
breaks the `Sync` requirement coming from `&self`.

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

The error walks the chain: `fn work` captures `&self` as `&MyWorker`; for
the returned future to satisfy `+ Send`, `&MyWorker` has to be `Send`; and
`&T: Send` only holds when `T: Sync`. The `&self` parameter has been demanding
`Sync` on `Self` all along — `Cell` just made the demand visible.

The cheap fix is to make `Self: Sync`: swap the non-`Sync` interior-mutability primitive
for a `Sync` one (a `Mutex`, an `RwLock`, an atomic). The impl type becomes `Sync` and the
trait compiles unchanged. But we've added synchronisation overhead on every state access for
a worker whose state is only ever touched from inside a single spawned task. Suboptimal™.

The better move is to switch `&self` to `&mut self`. `&mut T: Send` requires only `T: Send`,
no `Sync` involved:

{{< code src="examples/step-3.rs" lang="rust" >}}

This compiles. Now the trait carries no `Sync` requirement anywhere.

Underneath all of this is `&mut T` being the unique reference, not the mutable one.
The instinct in Rust is to reach for `&` over `&mut` to tighten the contract: no
mutation allowed. Here it goes the other way. `&mut self` guarantees unique access to
`Self` for the duration of the call, which rules out cross-thread sharing and drops
the `Sync` bound with it.

## Links

- Niko Matsakis,
  [*Focusing on ownership*](https://smallcultfollowing.com/babysteps/blog/2014/05/13/focusing-on-ownership/)
  — the canonical writeup on `&mut` as uniqueness rather than mutation.

---

*Full accompanying source code can be found [here][src]. Built with `rustc 1.95.0`.
Library versions used: `tokio` 1.52.2.*
