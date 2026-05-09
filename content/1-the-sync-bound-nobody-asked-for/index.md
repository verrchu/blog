---
title: "The `Sync` bound nobody asked for"
date: 2026-05-04
description: "The hidden Sync bound &self imposes on async trait impls, and a way to lift it."
---

[src]: https://github.com/verrchu/blog/tree/main/content/1-the-sync-bound-nobody-asked-for
[tokio-spawn]: https://docs.rs/tokio/1.52.2/tokio/task/fn.spawn.html
[focusing-on-ownership]: https://smallcultfollowing.com/babysteps/blog/2014/05/13/focusing-on-ownership/
[nsfw]: https://matklad.github.io/2023/12/10/nsfw.html
[lobsters-thread]: https://lobste.rs/s/c8cv7a/sync_bound_nobody_asked_for
[mutex-sync-impl]: https://doc.rust-lang.org/1.95.0/std/sync/struct.Mutex.html#impl-Sync-for-Mutex%3CT%3E

`&self` as a receiver in an async method of a trait whose returned future must be `Send`
implicitly forces `Sync` on the trait implementor type — even if neither the trait nor its
callers ever _explicitly_ ask for `Sync`.

Here's a quick demonstration.

**1.1.** Future without `Send` bound, `Send + Sync` impl type — compiles:

{{< code src="examples/demo-1.1.rs" lang="rust" >}}

**1.2.** Future without `Send` bound, `Send`-only impl type — also compiles:

{{< code src="examples/demo-1.2.rs" lang="rust" >}}

**2.1.** Future with `Send` bound, `Send + Sync` impl type — compiles:

{{< code src="examples/demo-2.1.rs" lang="rust" >}}

**2.2.** Future with `Send` bound, `Send`-only impl type — does **not** compile:

{{< code src="examples/demo-2.2.rs" lang="rust" >}}

<details>
<summary>Full compile error (we'll see it again later)</summary>

```text
error: future cannot be sent between threads safely
  --> examples/demo-2.2.rs:11:22
   |
11 |     async fn f(&self) {}
   |                      ^ future returned by `f` is not `Send`
   |
   = help: within `SendOnly`, the trait `Sync` is not implemented for `Cell<()>`
   = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock`
note: captured value is not `Send` because `&` references cannot be sent unless their referent is `Sync`
  --> examples/demo-2.2.rs:11:16
   |
11 |     async fn f(&self) {}
   |                ^^^^^ has type `&SendOnly` which is not `Send`, because `SendOnly` is not `Sync`
note: required by a bound in `T::f::{anon_assoc#0}`
  --> examples/demo-2.2.rs:7:47
   |
 7 |     fn f(&self) -> impl Future<Output = ()> + Send;
   |                                               ^^^^ required by this bound in `T::f::{anon_assoc#0}`
```

</details>

So clearly: if the returned future doesn't need to be `Send`, the impl type doesn't need to be
`Sync`. If the returned future needs to be `Send`, the impl type needs to be `Sync`.

|                                       | impl type: `Send + Sync` | impl type: `Send + !Sync` |
|---------------------------------------|--------------------------|---------------------------|
| returned future: without `Send` bound | **1.1** — compiles       | **1.2** — compiles        |
| returned future: with `Send` bound    | **2.1** — compiles       | **2.2** — fails           |

So why do we even need an async method of a trait to return a `Send` future?

Most async runtimes spawn futures onto a thread pool, which means a spawned future
has to be safe to move between threads.
[`tokio::spawn`][tokio-spawn] makes the requirement explicit:

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

In the example below, everything compiles because `MyTask` is trivially `Send + Sync`.

{{< code src="examples/step-1.rs" lang="rust" >}}

Now imagine we really need to use an external type as part of `Self` — a type we don't
control. The type is `Send` but `!Sync` (due to interior mutability).
Since the task is only ever executed by a single thread at any given moment, this should
be fine in theory:

{{< code src="examples/step-2.rs" lang="rust" >}}

```text
error: future cannot be sent between threads safely
  --> examples/step-2.rs:15:24
   |
15 |     async fn run(&self) {}
   |                        ^ future returned by `run` is not `Send`
   |
   = help: within `MyTask`, the trait `Sync` is not implemented for `Cell<()>`
   = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock`
note: captured value is not `Send` because `&` references cannot be sent unless their referent is `Sync`
  --> examples/step-2.rs:15:18
   |
15 |     async fn run(&self) {}
   |                  ^^^^^ has type `&MyTask` which is not `Send`, because `MyTask` is not `Sync`
note: required by a bound in `Task::run::{anon_assoc#0}`
  --> examples/step-2.rs:11:49
   |
11 |     fn run(&self) -> impl Future<Output = ()> + Send;
   |                                                 ^^^^ required by this bound in `Task::run::{anon_assoc#0}`
```

The error walks the chain: `fn run` captures `&self` as `&MyTask`; for
the returned future to satisfy `+ Send`, `&MyTask` has to be `Send`; and
`&T: Send` only holds when `T: Sync`. The `&self` parameter has been demanding
`Sync` on `Self` all along — `Foreign` just made the demand visible.

There are (at least) two ways to address this:

**a) Wrap `Foreign` in a `Mutex` (or `RwLock`).** [`Mutex<T>: Sync` only requires
`T: Send`][mutex-sync-impl], not `T: Sync`, so wrapping the `!Sync` `Foreign` is enough to make
`Self: Sync`. This is in line with what the compiler suggests[^compiler-wiggle]:

```text
= note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock`
```

It compiles. The cost is synchronization overhead on every state access even though the
access pattern is single-threaded — `Self` is never actually shared across threads.
Suboptimal™.

[^compiler-wiggle]: Strictly speaking, the compiler is suggesting swapping the `Cell` inside
    `Foreign` for an `RwLock` — i.e., changing `Foreign`'s internals. By stipulation we can't
    do that, since `Foreign` is _foreign_. Wrapping `Foreign` in a `Mutex` from the outside is
    the same underlying move though: reach for a `Sync` synchronization primitive somewhere
    in the chain.

**b) Switch `&self` to `&mut self`.** `&mut T: Send` only requires `T: Send` — no `Sync`
involved. The trait stops demanding `Sync` on the impl type, and `Self` stays untouched:

{{< code src="examples/step-3.rs" lang="rust" >}}

This compiles. The trait carries no `Sync` requirement, and `Foreign` is still in `Self`
unchanged.

Underneath all of this is `&mut T` being the unique reference, not the mutable one.
Often the instinct in Rust is to reach for `&` over `&mut` to tighten the contract: no
mutation allowed. Here it goes the other way. `&mut self` guarantees exclusive access to
`Self` for the duration of the call, which rules out cross-thread sharing and drops
the `Sync` bound with it.

## Links

- Niko Matsakis, [*Focusing on ownership*][focusing-on-ownership] — on `&mut` as uniqueness vs
  mutation.
- Alexey Kladov, [*Non-Send Futures When?*][nsfw] — on `Send` bound for spawned futures.
- [Discussion thread on Lobste.rs][lobsters-thread].

---

*Full accompanying source code can be found [here][src]. Built with `rustc 1.95.0`.
Library versions used: `tokio` 1.52.2.*
