# Migrating to `qubit-cas` 0.11

> Historical note: this page describes version 0.11 only. See the [0.14 migration note](migration-0.14.md) for the current API.

The Chinese version is [`migration-0.11.zh_CN.md`](migration-0.11.zh_CN.md).
This note covers the behavior and dependency changes that matter when upgrading
an existing application.

## 1. Make the hard async boundary explicit

`max_total_elapsed` is a soft continuation budget. It can reject a future retry,
but an already admitted operation may finish. Configure `flow_timeout` when an
independent hard wall-clock limit is required:

```rust
let executor = qubit_cas::CasExecutor::<usize, ()>::builder()
    .max_total_elapsed(Some(std::time::Duration::from_secs(10)))
    .flow_timeout(Some(std::time::Duration::from_secs(10)))
    .build()
    .expect("valid CAS configuration");
```

`flow_timeout` defaults to `None` and is ignored by synchronous execution.
`attempt_timeout` remains the per-attempt async limit; combine it with
`retry_on_timeout()` when timed-out attempts should be retried.

## 2. Use the retry re-export

Import advanced policy types from `qubit_cas::retry`:

```rust
use qubit_cas::retry::{BackoffPolicy, RetryPolicy};
```

The adapter remains pinned to `qubit-retry = 0.22.0` because
`CasRetryFailure` preserves that terminal diagnostic contract. If an application
also names `qubit-retry`, align its direct dependency and lockfile with 0.22.

## 3. Preserve retry diagnostics

`CasError::into_parts()` returns the classification, retry failure, CAS context,
last application failure, and diagnostics. `completion_callback_failures()`
borrows retained completion-observer failures. The older
`into_parts_with_diagnostics()` spelling is no longer the API to use.

`CasRetryFailure` retains timeout scope, cancellation, callback failures, and
infrastructure details. `CasError::current()` can retain the snapshot observed
before a timeout.

## 4. Event accounting

`CasEvent::RetryRequested` records a retry rule's intent. It is not proof that a
new attempt was admitted. Count executed operations with
`CasExecutionReport::attempts_total()` or the success attempt count. With
`max_attempts = 1`, a conflict can still produce `AttemptFailed`,
`RetryRequested`, and `ExecutionFinished`, while the executed attempt count is
one.

## 5. Choose the matching execution path

Use `execute_async_result` when an async caller needs only the terminal result;
use `execute_async` when it needs the report. Both paths honor `flow_timeout`.
For numeric, allocation-free state transitions without reports or business
retry, migrate instead to the separate `qubit-fast-cas` crate.

After migration, run `cargo test --all-features` and inspect timeout and event
mapping tests before deploying.
