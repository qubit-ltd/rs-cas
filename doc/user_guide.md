# Qubit CAS User Guide

This guide covers `qubit-cas` 0.13 for Rust applications that update shared
immutable snapshots. The Chinese version is [`user_guide.zh_CN.md`](user_guide.zh_CN.md).

## Inventory reservation and setup

Concurrent orders share remaining stock. Install these dependencies and run the
README inventory example: stock moves from 3 to 2, and zero stock produces
OutOfStock. Only the committed attempt's output reaches the caller. Send
notifications or charge payments after successful execution. Even idempotent
external I/O inside a retryable operation requires care: cancellation does not
roll back its effects.

```toml
[dependencies]
qubit-cas = { version = "0.13", features = ["tokio"] }
qubit-atomic = "0.13"
tokio = { version = "1.52", features = ["macros", "rt-multi-thread", "time"] }
```

## 1. CAS model, snapshots, and linearization

`CasExecutor<T, E>` reads an immutable `Arc<T>` snapshot, calls the operation,
and conditionally installs the replacement returned by `CasDecision::update`.
The compare-and-swap succeeds only if the observed snapshot is still current.
The successful compare-and-swap is the update's linearization point. A
`finish` decision linearizes at the observation of the current snapshot and
does not write it.

## 2. Decisions

Use `update(next, output)` for a state replacement, `finish(output)` when no
write is needed, `retry(error)` for a retryable business condition, and
`abort(error)` for an immediate terminal business error. The operation may run
again after a conflict or retryable failure, so it must be deterministic and
safe to replay.

## 3. Rich and result-only execution

Use `execute_result` or `execute_async_result` when the caller needs only
`Result<CasSuccess<...>, CasError<...>>`. These paths avoid report and hook
construction. Use `execute`/`execute_async` when attempts, conflicts, elapsed
time, or terminal outcomes are needed. Use the `*_with_hooks` variants for
per-execution events or alerts.

## 4. Strategies and builders

`latency_first`, `contention_backoff`, and `reliability_first` provide presets.
ContentionBackoff uses fixed exponential-backoff parameters and jitter; it does
not learn from observed contention. A plain builder defaults to 5 attempts, no
delay, and no soft budgets. LatencyFirst uses 100 attempts, a 5ms operation budget,
and a 20ms total budget.

Settings follow call order. `strategy` replaces attempts, soft budgets, and
backoff, while preserving async timeouts and their action. Later setters such as
`max_attempts` and `no_delay` replace the corresponding setting. Attempts include
the initial operation.

## 5. Soft budgets and hard timeouts

`max_operation_elapsed` measures accumulated attempt time, including CAS adapter
work. `max_total_elapsed` covers operations, backoff, and control callbacks in
the retry flow. Both are soft admission budgets: an admitted operation may still
commit successfully after the budget expires.

`flow_timeout` covers async attempts and backoff, excluding the full cost of
start/finish hooks. `attempt_timeout` bounds an async attempt. Timeout aborts by
default; `retry_on_timeout()` permits retry. Synchronous entry points ignore
both hard timeouts. Deadlines are cooperative: blocking calls or futures that
never yield cannot be preempted reliably.

## 6. Synchronous and asynchronous execution

Synchronous delays block the calling thread. Async methods require the `tokio`
feature and yield while waiting. A result-only async operation has the same
decision semantics as its synchronous counterpart:

```rust
use std::time::Duration;
use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

#[tokio::main]
async fn main() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(3)
        .max_total_elapsed(Some(Duration::from_secs(5)))
        .flow_timeout(Some(Duration::from_secs(30)))
        .attempt_timeout(Some(Duration::from_millis(100)))
        .retry_on_timeout()
        .build()
        .expect("valid CAS settings");

    let success = executor
        .execute_async_result(&state, |current| async move {
            CasDecision::update(*current + 1, *current + 1)
        })
        .await
        .expect("async CAS should succeed");

    assert_eq!(*success.current().as_ref(), 1);
}
```

## 7. Reports, events, alerts, and listener panics

`execute` returns a report. `execute_with_hooks` additionally supports events via
`CasHooks::on_event` and threshold-based alerts via `on_contention_alert`. There
is no separate observability-mode enum.

Reports retain attempts, conflicts, business retries, timeouts, elapsed time,
and the terminal outcome. Listeners run inline and must remain inexpensive and
non-blocking. With unwinding enabled, listener panics are isolated and retained
in the final report's `listener_failures()` without changing the business result.
The finished event contains the report snapshot before that listener runs; it
cannot include its own later panic. Operation panics propagate. Panic isolation
does not apply to panic=abort builds.

## 8. Errors and diagnostic ownership

Use `kind()` for broad classification, `termination()` for attempts/budget/timeout
termination, and `error()` for the original business error. The last failure is
independent of termination: a flow timeout during backoff can still retain the
preceding business retry error.

`current()` is the snapshot associated with the last failure, not a freshly
loaded value. An attempt timeout retains its starting snapshot; a backoff timeout
keeps the preceding failure. Before the first attempt it may be None.

`diagnostic()` retains CAS-owned Cancellation, Callback, Clock, Timer, or other
infrastructure classifications with their diagnostic text. `completion_diagnostics()`
retains callback failures after the terminal result was frozen, in order.
Use the category in program logic; message wording is for troubleshooting only.
Ordinary abort, conflict, budget, and timeout termination have no infrastructure
diagnostic. Dropping a future does not return a cancellation CasError.

## 9. Performance and `qubit-fast-cas`

Prefer result-only execution on hot paths that do not need reports. Prefer
`execute` over event streaming, and send events to a non-blocking channel if
they must be exported. For a compact `u64` state machine that needs no reports,
hooks, async support, or business retry, use
[`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas) instead.

## 10. Troubleshooting and limits

- Unexpected `RetryExhausted`: check whether the closure returns `retry` for a
  permanent business error.
- Unexpected `TotalBudgetExceeded`: remember that it is a soft continuation
  budget; inspect termination and the retained last failure for the exact cause.
- Unexpected `AttemptTimeout`: distinguish per-attempt timeout from `flow_timeout`.
- High conflict ratio: use `ContentionBackoff`, reduce shared hot-key scope,
  or split the state into smaller immutable values.
- Side effects repeated: move them after successful execution or make them
  idempotent with an external operation identifier.

The executor does not provide transactions across multiple atomic values, and
it cannot make non-replayable operations safe. Use a lock or database
transaction when the update requires a long critical section or external
coordination.

`update` allocates an Arc for each new snapshot; `update_arc` accepts a preallocated
snapshot. Warm finish bookkeeping does not allocate, but this is not a claim that
every CAS operation avoids allocation. Run `cargo bench --bench contention` to
measure contention and tail latency.

Return to the [README](../README.md), [API](https://docs.rs/qubit-cas), or
[0.13 migration note](migration-0.13.md).
