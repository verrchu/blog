---
title: "Graceful shutdown with `WeakSender`"
date: 2026-05-16
---

Actors built on `tokio` mpsc channels shut down for free: drop the last sender and the
actor stops. A sidecar that sends back into the actor quietly breaks that — its sender
pins the actor open forever. The fix is `WeakSender`: a sender that can reach the actor
while it's alive without keeping it alive.

## What is an actor

An actor is typically understood as some state plus message passing. There's state, and
messages can be sent to whoever owns that state; for each message, some action is
performed depending on the current state and the message that came in. That's it.

### Counter actor

A simple example is a counter actor. The "state" is just a number, and the whole thing
is essentially a `recv` loop with a match on the message. It doesn't even need an
explicit state struct — it's a future that captures a local `counter` starting at zero:

{{< code src="examples/counter.rs" lang="rust" >}}

`Counter::spawn` returns the `Sender` — that's the only way to talk to it. `Inc` bumps
the state; `Get` carries a `oneshot::Sender` the actor uses to ship the current value
back to whoever asked.

The thing to notice: when no senders are left, the channel is considered *disconnected*
and `rx.recv()` returns `None`. That's a dead-simple graceful shutdown mechanic — to stop
the actor, all that's needed is to drop every sender. Nothing else to wire up.

### Ticker actor

So even an actor that takes *no input at all* — one that just drives itself off a timer —
still benefits from exposing a sender. Not to send anything: the entire point of the
sender here is to be a shutdown handle. Holding it keeps the ticker alive; dropping it
forces the ticker to stop. That's the only thing it does.

Here's that shape. The message type is an uninhabited `enum Msg {}` (nobody can ever
construct one to send), and `spawn` returns a `Handle` that's a thin wrapper around the
`Sender`. The actor does its work on a `tokio::time::interval` tick, with a
`tokio::select!` over the timer and the receiver so dropping the `Handle` still breaks
the loop:

{{< code src="examples/ticker.rs" lang="rust" >}}

No `Inc`/`Get` here — the only "message" is the absence of senders. Drop the `Handle`,
the `select!`'s `rx.recv()` arm yields `None`, the loop breaks, the task ends. Same
shutdown invariant as the counter, just with a self-driving body instead of a
request/response one.

## A main actor with a sidecar

Now the interesting one. Take a `Worker` whose periodic work isn't self-driven — the
stimulus comes from a separate `Ticker` actor that pings it on a schedule. This is the
shape that matters for the rest of the post: a *main* actor (the worker) and a
*supplementary* one (the ticker) that has to send into it.

Both keep the shutdown-handle shape from before. `WorkerMsg` and `TickerMsg` are both
uninhabited `enum`s — neither takes any external messages, so each `Handle` is purely a
kill switch. The ticker is spawned alongside the worker; the worker owns the
`TickerHandle`, so the worker going away takes the ticker with it.

{{< code src="examples/sidecar.rs" lang="rust" >}}

The key detail is the wiring between them. The worker's `select!` has two arms: its own
input channel (uninhabited — only used to detect the handle drop) and a *dedicated second
receiver* for ticks. The ticker holds the sending half of that tick channel and pings
`()` every period.

For now this still shuts down cleanly: drop the `WorkerHandle`, the worker's `rx.recv()`
returns `None`, it breaks, the function returns, `_ticker` (the `TickerHandle`) drops, the
ticker's own `rx.recv()` returns `None`, it exits. What kept this clean is the dedicated
tick channel — the ticker's sender goes into a *separate* channel, not the worker's input
channel, so it never had a chance to pin the worker. That luxury disappears the moment
the worker is generic.

## When the worker is generic

In a real codebase the actor loop isn't hand-written per actor — it's a reusable
scaffolding. A `Handle<Req>` generic over the message type, a single input receiver, and
a trait the concrete actor implements: how to `init` state from a context, how to react
to a request. No timer, no second receiver — just one channel pumped into `on_req`.

{{< code src="examples/generic.rs" lang="rust" >}}

The scaffolding has exactly one receiver. There is no seam to inject a dedicated tick
channel without rewriting `run` for every behavior — which defeats the point of making it
generic. So the ticker has nowhere to send *except the worker's own input channel*: the
behavior's `init` is handed a clone of the worker's own `Sender`, hands that clone to the
ticker, and the `Req` enum grows a `Tick` variant. `on_req` matches it like any other
request. The dedicated tick channel from the hand-rolled version is gone.

This works, and it's the natural thing to reach for. But look at what the cloned sender
did to shutdown: the ticker now holds a *strong* `Sender` into the worker's own input
channel. Drop every external `Handle` and the worker still doesn't stop — the ticker's
clone keeps the channel alive, the worker keeps serving it, the ticker keeps pinging it,
and nothing ever drops. The clean "drop the handle ⇒ everything shuts down" story is
broken, structurally, by the one wiring choice the generic scaffolding left available.

## The fix: hand the sidecar a weak sender

There's an easy way out, and it's a one-word change to the type. The ticker doesn't need
to *keep the worker alive* — it only needs to *talk to it while it's alive*. That is
exactly the `Arc`/`Weak` distinction, and `mpsc::Sender` has the same split:
`tx.downgrade()` produces a [`WeakSender`][weak-sender] that can be `upgrade()`d back to a
real `Sender` only while at least one strong sender still exists.

So downgrade once, in `spawn`, and float the *weak* sender through the scaffolding
instead of a strong clone. The strong `Sender` lives in exactly one place — the
`Handle` — so dropping every `Handle` deterministically disconnects the channel no matter
how many sidecars are still running. The weak sender is created at spawn time and passed
down through `init`, independent of the worker's state.

{{< code src="examples/generic-weak.rs" lang="rust" >}}

The scaffolding is unchanged in shape; only the self-sender's type flipped from `Sender`
to `WeakSender`. The behavior hands that weak sender to the ticker exactly as before. The
only place that has to do anything new is the ticker's send path: `upgrade()` first, and
treat `None` as "the worker is gone, follow it out".

Now retrace the shutdown. Drop every `Handle`; the only strong `Sender` is gone; the
worker's `rx.recv()` returns `None` and it breaks; the ticker's next `upgrade()` returns
`None` and it exits. The invariant from the very first counter actor — *drop the handle
⇒ the actor and everything it spawned shut down* — holds again, all the way up through a
generic, sidecar-augmented worker.

## It's just `Arc`/`Weak`, one layer down

None of this is new machinery. It's the [`Rc`][rc] / [`Weak`][weak] split (or
[`Arc`][arc] / `Weak` across threads), applied to a channel instead of a value.

`Rc<T>` / `Arc<T>` *own* the value: it's dropped the instant the *strong* count hits
zero. `Weak<T>` is a non-owning observer — any number of weak handles can exist and the
value is still freed as soon as the last strong handle goes. A weak handle `upgrade()`s
to a strong one only while the value is still alive, and yields `None` afterwards. The
textbook use is breaking a reference cycle: parent owns child strongly, child points back
at parent weakly.

[arc]: https://doc.rust-lang.org/std/sync/struct.Arc.html
[rc]: https://doc.rust-lang.org/std/rc/struct.Rc.html
[weak]: https://doc.rust-lang.org/std/rc/struct.Weak.html
[weak-sender]: https://docs.rs/tokio/1/tokio/sync/mpsc/struct.WeakSender.html

{{< code src="examples/rc-weak.rs" lang="rust" >}}

`mpsc::Sender` / [`WeakSender`][weak-sender] is the exact same split, one layer down. The
"value" being kept alive is the channel — really the receiver's willingness to keep
waiting. Strong senders keep it open; a `WeakSender` doesn't, and `upgrade()` returns
`None` once every strong sender is gone. `Rc::downgrade` ⇄ `Sender::downgrade`,
`Weak::upgrade` ⇄ `WeakSender::upgrade` — same mechanics, different thing being owned.

One nuance. The parent/child case breaks a *cycle*: two things pointing straight at each
other. The generic worker isn't quite that — there's no real cycle, there's an *inverted
dependency* to sever. The sidecar depends on the worker (it sends results back to it);
the worker must *not*, in turn, depend on the sidecar being alive to decide it's done. A
worker should shut down on its own terms, regardless of whether its sidecars are still
running. Making the sidecar's sender weak encodes exactly that asymmetry: the dependency
points one way, and shutdown follows the strong edges only.

## Why this lives in tokio specifically

`std::sync::mpsc` has no notion of a weak sender. `crossbeam-channel` has none either —
its `Sender::clone()` is the only way to multiply senders, and any clone unconditionally
keeps the channel open. Code written against those libraries that wants this pattern
ends up tracking sidecar shutdown out-of-band: a `CancellationToken`, a shared
`AtomicBool`, an extra oneshot per sidecar, all wired into every new side task by hand.

`tokio::sync::mpsc::WeakSender` removes the bookkeeping. The strong handle stays in the
worker's owner; sidecars carry weak handles; cycles between collaborating tasks resolve
themselves on `Handle` drop.

## Links

- Alice Ryhl, [*Actors with Tokio*][ryhl-actors] — the canonical writeup of the
  spawned-task + handle structure this post builds on. Worth reading first; it
  establishes the shutdown-by-handle-drop invariant and discusses the cycle problem from a
  different angle.
- [`tokio::sync::mpsc::WeakSender`][weak-sender] — API reference.
- [`Sender::downgrade`][downgrade] and [`WeakSender::upgrade`][upgrade] — the two halves
  of the conversion.
- [`std::sync::Arc`][arc] / [`std::sync::Weak`][weak] — the same ownership pattern, one
  layer down.
- [tokio-rs/tokio#4023](https://github.com/tokio-rs/tokio/issues/4023) — the original
  proposal and motivation thread for `WeakSender`.

[downgrade]: https://docs.rs/tokio/1/tokio/sync/mpsc/struct.Sender.html#method.downgrade
[upgrade]: https://docs.rs/tokio/1/tokio/sync/mpsc/struct.WeakSender.html#method.upgrade
[ryhl-actors]: https://ryhl.io/blog/actors-with-tokio/
