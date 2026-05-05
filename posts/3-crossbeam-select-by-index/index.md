---
title: "Dynamic channel dispatch with crossbeam `Select`"
date: 2026-05-05
---

crossbeam's [`Select`](https://docs.rs/crossbeam-channel/0.5/crossbeam_channel/struct.Select.html)
multiplexes over a set of channels. You register receivers, block until one is
ready, and dispatch based on which one fired. It's the synchronous counterpart to
`tokio::select!`, built for cases where the set of channels isn't known at compile
time.

The core loop is `ready()` + `try_recv()`. `ready()` blocks until at least one
registered receiver has a pending message (or is disconnected) and returns its
**index** — the same `usize` that `recv()` returned when the receiver was
registered. The caller then calls `try_recv()` on the original receiver to
actually consume the message. (`ready()` only signals readiness; it can return
spuriously, so `try_recv()` may still come back empty.)

## The index

Each call to `Select::recv()` returns a sequential index starting from 0. The
[doc examples](https://docs.rs/crossbeam-channel/0.5/crossbeam_channel/struct.Select.html#method.ready)
rely on this directly — `rs[index].try_recv()` — but the docs never spell out
"the first `recv` returns 0, the second returns 1, ..." in prose. The guarantee
comes from the return type (`usize`) and the examples that use it positionally.

Two related properties:

- After `remove(idx)`, the index is gone forever.
  [The docs](https://docs.rs/crossbeam-channel/0.5/crossbeam_channel/struct.Select.html#method.remove):
  "If new operations are added after removing some, the indices of removed
  operations will not be reused."
- Adding new receivers after a removal continues the counter from where it left
  off, leaving a gap.

## Static dispatch

When all receivers are known upfront, fixed receivers occupy known index
positions and a slice of data receivers fills the rest. The index returned by
`ready()` maps back to the slice via offset arithmetic:

{{< code src="examples/step-1.rs" lang="rust" >}}

Index 0 is always the control channel. Data receivers start at 1, so
`data_rxs[idx - 1]` gets the right one. This only works as long as every
receiver is registered in a known, fixed order.

## Dynamic subscriptions

When new receivers appear at runtime — discovered via a separate channel — the
`Select` must be rebuilt. `Select` borrows every registered receiver for its
entire lifetime. Pushing a new receiver into the backing `Vec` can reallocate it,
invalidating all existing borrows. The compiler won't let you.

The solution: extract the `Select` construction into a helper and call it again
whenever the set changes:

{{< code src="examples/step-2.rs" lang="rust" >}}

Each rebuild creates a fresh `Select` with fresh borrows. The old indices are
meaningless, but since receivers are always registered in the same order, the
mapping (0 = discovery, 1.. = data) stays consistent.

The cost is O(n) per rebuild. In practice this is cheap — `Select::recv` just
pushes a handle onto a `Vec`, so rebuilding N receivers is N pointer copies.

## Removing disconnected channels

`Select::remove(idx)` disables a registered operation without rebuilding the
whole `Select`. Other indices remain valid because removed indices are never
reused.

When you use `remove`, the contiguous `idx - N` offset into a slice breaks
down — there are now gaps. A `HashMap<usize, _>` mapping select-indices to
receivers handles this:

{{< code src="examples/step-3.rs" lang="rust" >}}

## Combining add and remove

The natural next step is combining `remove()` for disconnects with `recv()` for
new subscriptions — dynamic add/remove without full rebuild. The obstacle is
ownership: `Select` borrows receivers for a lifetime tied to their storage. If
that storage is a `Vec`, you can't `push` while the `Select` holds borrows into
it.

An arena allocator solves this. `typed_arena::Arena::alloc` takes `&self` (not
`&mut self`), so it doesn't conflict with outstanding borrows. Allocated values
never move. The pattern becomes:

```rust
let storage = Arena::new();

// ... later, when a new receiver arrives:
let rx = storage.alloc(new_rx);     // stable &Receiver, no reallocation
let idx = sel.recv(rx);             // register with Select
index_map.insert(idx, rx);

// ... when a receiver disconnects:
sel.remove(idx);                    // O(1), no rebuild
index_map.remove(&idx);
```

This gives O(1) additions and removals, at the cost of leaked memory in the
arena (removed receivers are never freed while the arena lives). For long-running
loops with bounded churn, the trade-off is usually fine.

## Links

- crossbeam docs,
  [`Select`](https://docs.rs/crossbeam-channel/0.5/crossbeam_channel/struct.Select.html)
  — struct reference with the `recv`, `ready`, and `remove` examples that
  demonstrate index-based dispatch.
