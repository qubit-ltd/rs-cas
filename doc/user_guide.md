# Qubit CAS User Guide

This guide explains how to choose and operate `qubit-cas` in a production
application. The Chinese version is [`user_guide.zh_CN.md`](user_guide.zh_CN.md).

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

`latency_first`, `contention_adaptive`, and `reliability_first` are starting
points, not universal defaults. Measure the workload before changing a preset.
The builder configures attempts, retry delay, jitter, elapsed budgets,
observability, and asynchronous timeout behavior. Advanced retry types are
available from `qubit_cas::retry`.

## 5. Soft budgets and hard timeouts

`max_operation_elapsed` limits accumulated user-operation time.\
`max_total_elapsed` is a soft continuation budget covering retries, delays, and
hooks: an already admitted operation may finish.\
`flow_timeout` is an independent hard wall-clock timeout for asynchronous
execution and has no effect on synchronous execution.\
`attempt_timeout` bounds one asynchronous operation attempt; configure
`retry_on_timeout()` when a timed-out attempt should be retried.

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

`CasExecutionReport` records attempts, conflicts, ratios, elapsed time, and the
terminal outcome. `CasHooks` observes `CasEvent` values for one execution.
`ReportOnly` is the default; `EventStream` adds events; and
`EventStreamWithAlert` adds contention thresholds. Hooks run in the execution
path, so keep them cheap and non-blocking. Configure the panic policy explicitly
when a listener can fail; never use hooks for non-idempotent state mutation.

## 8. Errors and diagnostic ownership

Inspect `CasError::kind()` for control flow and `CasError::error()` for the
preserved business error. `CasError::current()` may contain the last snapshot,
including the snapshot retained before an async timeout. `CasRetryFailure`
preserves retry limits, timeout scope, cancellation, callback failures, and
infrastructure diagnostics for the pinned `qubit-retry` 0.22 contract.

## 9. Performance and `qubit-fast-cas`

Prefer result-only execution on hot paths that do not need reports. Prefer
`ReportOnly` over event streaming, and send events to a non-blocking channel if
they must be exported. For a compact `u64` state machine that needs no reports,
hooks, async support, or business retry, use
[`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas) instead.

## 10. Troubleshooting and limits

- Unexpected `RetryExhausted`: check whether the closure returns `retry` for a
  permanent business error.
- Unexpected `MaxTotalElapsedExceeded`: remember that it is a soft continuation
  budget; inspect the retained retry failure for the exact cause.
- Unexpected `AttemptTimeout`: distinguish per-attempt timeout from `flow_timeout`.
- High conflict ratio: use `ContentionAdaptive`, reduce shared hot-key scope,
  or split the state into smaller immutable values.
- Side effects repeated: move them after successful execution or make them
  idempotent with an external operation identifier.

The executor does not provide transactions across multiple atomic values, and
it cannot make non-replayable operations safe. Use a lock or database
transaction when the update requires a long critical section or external
coordination.
