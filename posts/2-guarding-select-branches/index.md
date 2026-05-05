---
title: "`select!` evaluates before it guards"
date: 2026-05-05
---

A common async pattern: a loop that accepts incoming items on a channel while
concurrently awaiting an optional in-flight operation. Items accumulate in a
buffer; when it fills up, a batch flush is spawned. While the flush runs, the
loop keeps draining the channel into the buffer.

[`tokio::select!`](https://docs.rs/tokio/1.52.2/tokio/macro.select.html) handles
the concurrency — it polls multiple futures and runs the handler for whichever
completes first. But one of the futures, the flush, only exists when a batch is
in flight. For the rest of the time it's `None`.

`select!` supports this with preconditions. A branch can carry an `if` guard;
when the guard is `false`, the branch is disabled for that iteration.

The intuitive thing to write:

{{< code src="examples/step-1.rs" lang="rust" >}}

This compiles. It also panics at runtime.

On the first loop iteration `flush` is `None`. The guard `if flush.is_some()`
evaluates to `false`, so the branch won't be polled — but `select!` evaluates
every branch's future expression **before** checking guards.
`flush.as_mut().unwrap()` runs unconditionally and panics on `None`.

The [docs](https://docs.rs/tokio/1.52.2/tokio/macro.select.html) are explicit
about this:

> If the branch is disabled, `<async expression>` is still evaluated, but the
> resulting future is not polled.

The distinction matters:

|                       | Guard `true` | Guard `false` |
|-----------------------|:------------:|:-------------:|
| Expression evaluated? | ✓            | ✓             |
| Future polled?        | ✓            | ✗             |

The fix is to wrap the expression in an `async {}` block. Creating the block is
side-effect-free regardless of `flush`'s state. The `unwrap()` moves inside the
block and only executes when the block is polled — which the guard prevents.

{{< code src="examples/step-2.rs" lang="rust" >}}

The one-line diff:

```rust
// before — expression evaluated eagerly, panics when flush is None
result = flush.as_mut().unwrap(), if flush.is_some() => { ... }

// after — unwrap deferred to poll time, guard prevents the poll
result = async { flush.as_mut().unwrap().await }, if flush.is_some() => { ... }
```

Two things to note about `biased`:

- Without `biased`, `select!` randomises the polling order for fairness. With
  `biased`, branches are polled top to bottom. For this pattern that's what you
  want: drain the channel first, then check the flush.
- `biased` also suppresses the "unfair polling" lint, since the intentional
  ordering is the whole point.

## Links

- tokio docs,
  [`select!`](https://docs.rs/tokio/1.52.2/tokio/macro.select.html)
  — macro reference, including precondition semantics.
