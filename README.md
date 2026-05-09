# Qubit CAS

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-cas.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-cas)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-cas/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-cas?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-cas.svg?color=blue)](https://crates.io/crates/qubit-cas)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

## Overview

A typed compare-and-swap executor for Rust. `qubit-cas` packages the usual
"load a shared snapshot, derive a new value, apply it by compare-and-swap,
retry on contention" loop into a reusable `CasExecutor`.

CAS can be read as "compare, then swap": a new value is applied atomically
only when the shared state still matches the snapshot you read. If another
writer changes the state first, the attempt fails and can be retried by policy.
Its strengths are low-latency lock-free paths and no lost updates under
concurrency; the trade-off is extra retries under high contention, which can
increase CPU cost and tail latency.

The crate builds on [`qubit-atomic`](https://crates.io/crates/qubit-atomic),
[`qubit-function`](https://crates.io/crates/qubit-function), and
[`qubit-retry`](https://crates.io/crates/qubit-retry). It is useful when shared
state is stored as an immutable `Arc<T>` snapshot and every update should be
expressed as an explicit, typed decision.

## Features

- **Typed decisions**: after user operations return `CasDecision::update`,
  `finish`, `retry`, or `abort`, `CasExecutor` automatically runs the matching
  flow: write a new state, complete without writing, retry, or terminate.
- **Retry-aware CAS loop**: compare-and-swap conflicts and business-level
  retry decisions are retried through `qubit-retry` with configurable attempts,
  elapsed-time budgets, delays, and jitter.
- **Synchronous and asynchronous APIs**: `execute` works without an async
  runtime; `execute_async` is available with the `tokio` feature.
- **Async timeout control**: per-attempt timeouts can be retried or converted
  into immediate aborts with `CasTimeoutPolicy`.
- **Observable execution reports**: every execution returns a `CasOutcome`
  containing a `CasExecutionReport` with attempts, conflicts, conflict ratio,
  elapsed time, and terminal outcome.
- **Lifecycle event stream**: per-execution `CasHooks` can observe unified
  `CasEvent` values without changing the business operation.
- **Strategy-based executors**: built-in `LatencyFirst`,
  `ContentionAdaptive`, and `ReliabilityFirst` profiles cover common retry
  behavior.
- **Structured results**: `CasSuccess`, `CasError`, and `CasAttemptFailure`
  expose the final state, previous state, output, error kind, and last failure.

## Installation

```toml
[dependencies]
qubit-cas = "0.4"
qubit-atomic = "0.10"
```

`qubit-cas` expects the shared state to be held in `qubit_atomic::AtomicRef<T>`.
Add `qubit-atomic` as a direct dependency when your application constructs or
stores that state.

Enable asynchronous execution with:

```toml
[dependencies]
qubit-cas = { version = "0.4", features = ["tokio"] }
qubit-atomic = "0.10"
```

Optional features:

- `tokio`: enables `CasExecutor::execute_async` and per-attempt async timeout
  handling through Tokio.

The default feature set is empty. Synchronous CAS execution does not pull in an
async runtime.

## When to Use It

Use `qubit-cas` when an update can be described as a pure transformation from
the current immutable snapshot to a decision:

- A small shared state object is held in `AtomicRef<T>` and replaced as a whole.
- Concurrent writers are expected, but lost updates are not acceptable.
- Retrying from the latest snapshot is cheaper than holding a lock across the
  operation.
- Callers need structured observability for attempts, conflicts, retryable
  business failures, aborts, timeouts, and elapsed budgets.

Prefer a mutex, database transaction, or domain-specific lock when the critical
section is long-running, update logic has side effects that cannot be safely
replayed, or the state cannot be represented as an immutable replacement value.

## Quick Start

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

#[derive(Debug, PartialEq, Eq)]
struct Inventory {
    stock: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OrderError {
    OutOfStock,
}

fn main() {
    let state = AtomicRef::from_value(Inventory { stock: 3 });
    let executor = CasExecutor::<Inventory, OrderError>::latency_first();

    let outcome = executor.execute(&state, |current: &Inventory| {
        if current.stock == 0 {
            return CasDecision::abort(OrderError::OutOfStock);
        }

        CasDecision::update(
            Inventory {
                stock: current.stock - 1,
            },
            current.stock - 1,
        )
    });

    println!(
        "CAS attempts={}, conflicts={}, conflict_ratio={:.2}",
        outcome.report().attempts_total(),
        outcome.report().conflicts(),
        outcome.report().conflict_ratio(),
    );

    match outcome.into_result() {
        Ok(success) => {
            println!("stock updated successfully, remaining: {}", success.output());
            assert!(success.is_updated());
            assert_eq!(*success.output(), 2);
            assert_eq!(state.load().stock, 2);
        }
        Err(error) => {
            // Out-of-stock is a business outcome, not a panic condition.
            eprintln!("order rejected: {error:?}");
        }
    }
}
```

This example demonstrates a CAS-based "place order and decrement stock"
flow:

- `AtomicRef::from_value(Inventory { stock: 3 })` creates the shared
  inventory snapshot with initial stock `3`.
- `execute` reads the current snapshot on each attempt:
  - If stock is `0`, it returns
    `CasDecision::abort(OrderError::OutOfStock)` and stops immediately.
  - Otherwise, it returns `CasDecision::update(...)`, decrementing stock by
    `1` and returning the new stock as business output.
- The write is applied via CAS (compare-and-swap): if contention makes an
  attempt lose the race, the executor retries from the latest snapshot to
  avoid lost updates under concurrent writes.
- The example uses `match` to handle outcomes explicitly: validate
  write/output on success, and handle business failures (for example,
  out-of-stock).

## Decision Model

Every operation receives the current state snapshot and returns a
`CasDecision<T, R, E>`:

- `CasDecision::update(next, output)` attempts to apply a replacement state
  from an owned value.
- `CasDecision::update_arc(next, output)` attempts to apply a replacement
  state from `Arc<T>` when the shared pointer is already available.
- If another writer wins first, the executor retries according to its retry
  configuration.
- `CasDecision::finish(output)` completes successfully without writing a new
  state. Use it when the current snapshot already satisfies the operation.
- `CasDecision::retry(error)` marks the attempt as a retryable business failure.
  The final error is `CasErrorKind::RetryExhausted` if retry limits are reached.
- `CasDecision::abort(error)` stops the flow immediately and returns
  `CasErrorKind::Abort`.

`execute*` returns `CasOutcome<T, R, E>`. It contains the business
`Result<CasSuccess<T, R>, CasError<T, E>>` plus the `CasExecutionReport`, so
callers can read conflict counts and ratios without registering hooks.

## State and Operation Guidelines

CAS operations may be invoked more than once because conflicts and retryable
business failures restart the flow from a fresh snapshot. Keep the operation
closure deterministic and side-effect-free whenever possible. If a side effect
is required, perform it after `execute*` returns success, or make the side effect
idempotent and tied to an external operation id.

The shared value should be cheap enough to clone into a replacement `Arc<T>`.
For large states, prefer persistent data structures, internal `Arc` fields, or a
smaller state object that points to larger immutable data.

## Error Handling

Terminal failures are returned as `CasError<T, E>` and classified by
`CasErrorKind`:

- `Abort`: the operation returned `CasDecision::abort`.
- `Conflict`: compare-and-swap conflicts exhausted the retry policy.
- `RetryExhausted`: retryable business failures exhausted the retry policy.
- `AttemptTimeout`: an async attempt timed out and the timeout policy stopped
  the flow, or timeout retries were exhausted.
- `MaxOperationElapsedExceeded`: the cumulative user-operation time budget was
  exceeded.
- `MaxTotalElapsedExceeded`: the whole retry flow, including delays and hooks,
  exceeded its total elapsed-time budget.

Use `error.kind()` for control flow, `error.error()` for the preserved business
error when available, and `error.current()` when the final failure retained the
state snapshot observed by the last attempt.

## Execution Strategies

`qubit-cas` ships with three common strategies you can choose directly:

- `CasExecutor::latency_first()` retries immediately with a small attempt budget.
- `CasExecutor::contention_adaptive()` uses exponential backoff and jitter for
  contended writers.
- `CasExecutor::reliability_first()` uses a longer retry window for operations
  where eventual success matters more than latency.

In practice, start with `latency_first()`. If reports show
`conflict_ratio >= 0.30` and `attempts_total >= 3`, the workload is visibly
contended and should move to `contention_adaptive()`. If your operation
prioritizes "succeed eventually" over "return fast", use `reliability_first()`.

## Retry Configuration

Use the builder when the preset executors are not enough:

```rust
use std::time::Duration;

use qubit_cas::CasExecutor;

let executor = CasExecutor::<usize, &'static str>::builder()
    .max_retries(4)
    .exponential_backoff(Duration::from_millis(2), Duration::from_millis(50))
    .jitter_factor(0.25)
    .max_operation_elapsed(Some(Duration::from_millis(250)))
    .build()
    .expect("valid CAS retry settings");
```

## Contention Observation and Hooks

Hooks are attached to a single execution, so the same executor can be reused
with different observability behavior. By default the executor only returns a
`CasExecutionReport`; enable `event_stream()` when real-time events are needed:

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::{
    CasAttemptFailureKind, CasDecision, CasEvent, CasExecutor, CasHooks, CasObservabilityConfig,
};

let state = AtomicRef::from_value(1usize);
let executor = CasExecutor::<usize, &'static str>::builder()
    .observability(CasObservabilityConfig::event_stream())
    .build_latency_first()
    .expect("valid CAS settings");

let hooks = CasHooks::new().on_event(|event: &CasEvent| {
    if let CasEvent::AttemptFailed { context, kind } = event {
        if *kind == CasAttemptFailureKind::Conflict {
            eprintln!("CAS conflict at attempt {}", context.attempt());
        }
    }
});

let success = executor
    .execute_with_hooks(
        &state,
        |current: &usize| CasDecision::update(*current + 1, *current + 1),
        hooks,
    )
    .expect("CAS should succeed");

assert_eq!(*success.output(), 2);
```

## Detection and Performance Trade-offs

Contention detection also adds work to the hot path, so `qubit-cas` separates
observability into three levels:

- `ReportOnly` (default): aggregate only the final `CasExecutionReport` and do
  not construct attempt events. Use this for most production paths.
- `EventStream`: emit `CasEvent` values to listeners. Use this for real-time
  logs, traces, or metrics.
- `EventStreamWithAlert`: add threshold checks and contention alerts on top of
  event streaming.

Prefer `ReportOnly` by default and export `outcome.report().conflict_ratio()`
periodically. Upgrade to `EventStream` only when investigating hot keys or
feeding traces. Avoid synchronous logging, remote metrics calls, or expensive
formatting inside hooks because high contention multiplies that work by the
number of attempts. A non-blocking channel with a background batch consumer is
the recommended pattern.

## Async Usage

With the `tokio` feature, asynchronous operations receive an `Arc<T>` snapshot.
Per-attempt timeouts can either be retried or used to abort the flow.

```rust
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

#[tokio::main]
async fn main() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(3)
        .attempt_timeout(Some(Duration::from_millis(100)))
        .retry_on_timeout()
        .build()
        .expect("valid CAS settings");

    let success = executor
        .execute_async(&state, |current| async move {
            CasDecision::update(*current + 1, *current + 1)
        })
        .await
        .expect("async CAS should succeed");

    assert_eq!(*success.current().as_ref(), 1);
}
```

## Public API Cheat Sheet

- `CasExecutor<T, E>`: reusable CAS executor bound to a state type `T` and
  business error type `E`.
- `CasBuilder<T, E>`: configures retry attempts, elapsed budgets, delay,
  jitter, async timeouts, timeout policy, observability, and strategy presets.
- `CasDecision<T, R, E>`: per-attempt decision returned by user logic.
- `CasOutcome<T, R, E>`: terminal result plus `CasExecutionReport`.
- `CasSuccess<T, R>`: successful update or no-write finish, including current
  state, optional previous state, output, and attempt context.
- `CasError<T, E>`: terminal failure with a classified `CasErrorKind`.
- `CasHooks`: per-execution lifecycle and alert hooks.
- `CasObservabilityConfig`: selects report-only mode, event stream mode, or
  event stream with contention alerts.
- `ContentionThresholds`: classifies hot contention from attempts, conflicts,
  and conflict ratio.

## Project Layout

- `src/decision`: typed CAS decision values.
- `src/executor`: builder and synchronous/asynchronous CAS executor.
- `src/event`: execution context and lifecycle hooks.
- `src/error`: attempt-level and terminal CAS errors.
- `src/observability`: observability modes, contention thresholds, and alerts.
- `src/options`: timeout policy options.
- `src/outcome` and `src/report`: execution result wrapper and observability
  reports.
- `src/strategy`: built-in execution strategies and strategy profiles.
- `benches`: observability overhead benchmarks.
- `tests`: behavior tests for executor, builder, hooks, errors, and options.

## Testing and CI

Run the fast local checks from the crate root:

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

To match the repository CI environment, run:

```bash
./align-ci.sh
./ci-check.sh
./coverage.sh json
```

`./align-ci.sh` aligns the local toolchain and CI-related configuration before
`./ci-check.sh` runs the same checks used by the pipeline. Use `./coverage.sh`
when changing behavior that should be reflected in coverage reports.

## Contributing

Issues and pull requests are welcome. Please keep changes focused, add or update
tests when behavior changes, and update this README or rustdoc when public API
or user-visible behavior changes.

By contributing, you agree that your contribution is licensed under the same
[Apache License, Version 2.0](LICENSE) as this project.

## License and Copyright

Copyright (c) 2026. Haixing Hu.

This software is licensed under the [Apache License, Version 2.0](LICENSE).

## Author and Maintenance

**Haixing Hu** — Qubit Co. Ltd.

| | |
| --- | --- |
| **Repository** | [github.com/qubit-ltd/rs-cas](https://github.com/qubit-ltd/rs-cas) |
| **API documentation** | [docs.rs/qubit-cas](https://docs.rs/qubit-cas) |
| **Crate** | [crates.io/crates/qubit-cas](https://crates.io/crates/qubit-cas) |
