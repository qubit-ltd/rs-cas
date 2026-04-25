# Qubit CAS

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-cas.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-cas)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-cas/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-cas?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-cas.svg?color=blue)](https://crates.io/crates/qubit-cas)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

A typed compare-and-swap executor for Rust. `qubit-cas` packages the usual
"load a shared snapshot, derive a new value, install it by compare-and-swap,
retry on contention" loop into a reusable `CasExecutor`.

The crate builds on [`qubit-atomic`](https://crates.io/crates/qubit-atomic),
[`qubit-function`](https://crates.io/crates/qubit-function), and
[`qubit-retry`](https://crates.io/crates/qubit-retry). It is useful when shared
state is stored as an immutable `Arc<T>` snapshot and every update should be
expressed as an explicit, typed decision.

## Features

- **Typed decisions**: user operations return `CasDecision::update`,
  `finish`, `retry`, or `abort`, making write, no-write success, retryable
  failure, and terminal failure explicit.
- **Retry-aware CAS loop**: compare-and-swap conflicts and business-level
  retry decisions are retried through `qubit-retry` with configurable attempts,
  elapsed-time budgets, delays, and jitter.
- **Synchronous and asynchronous APIs**: `execute` works without an async
  runtime; `execute_async` is available with the `tokio` feature.
- **Async timeout control**: per-attempt timeouts can be retried or converted
  into immediate aborts with `CasTimeoutPolicy`.
- **Lifecycle hooks**: per-execution `CasHooks` can observe success, retry, and
  abort events without changing the business operation.
- **Preset executors**: built-in high-concurrency, low-latency, and
  high-reliability presets cover common retry profiles.
- **Structured results**: `CasSuccess`, `CasError`, and `CasAttemptFailure`
  expose the final state, previous state, output, attempt count, error kind, and
  last failure.

## Installation

```toml
[dependencies]
qubit-cas = "0.1.0"
qubit-atomic = "0.10"
```

`qubit-cas` expects the shared state to be held in `qubit_atomic::AtomicRef<T>`.
Add `qubit-atomic` as a direct dependency when your application constructs or
stores that state.

Enable asynchronous execution with:

```toml
[dependencies]
qubit-cas = { version = "0.1.0", features = ["tokio"] }
qubit-atomic = "0.10"
```

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
    let executor = CasExecutor::<Inventory, OrderError>::low_latency();

    let success = executor
        .execute(&state, |current: &Inventory| {
            if current.stock == 0 {
                return CasDecision::abort(OrderError::OutOfStock);
            }

            CasDecision::update_value(
                Inventory {
                    stock: current.stock - 1,
                },
                current.stock - 1,
            )
        })
        .expect("stock should be updated");

    assert!(success.is_updated());
    assert_eq!(*success.output(), 2);
    assert_eq!(state.load().stock, 2);
}
```

## Decision Model

Every operation receives the current state snapshot and returns a
`CasDecision<T, R, E>`:

- `CasDecision::update(next, output)` or `update_value(next, output)` attempts
  to install a replacement state. If another writer wins first, the executor
  retries according to its retry configuration.
- `CasDecision::finish(output)` completes successfully without writing a new
  state. Use it when the current snapshot already satisfies the operation.
- `CasDecision::retry(error)` marks the attempt as a retryable business failure.
  The final error is `CasErrorKind::RetryExhausted` if retry limits are reached.
- `CasDecision::abort(error)` stops the flow immediately and returns
  `CasErrorKind::Abort`.

Successful executions return `CasSuccess<T, R>`. It can tell whether an update
happened, expose the current state, expose the previous state for real updates,
return the business output, and report the number of attempts.

## Retry Configuration

Use the builder when the preset executors are not enough:

```rust
use std::time::Duration;

use qubit_cas::CasExecutor;

let executor = CasExecutor::<usize, &'static str>::builder()
    .max_retries(4)
    .exponential_backoff(Duration::from_millis(2), Duration::from_millis(50))
    .jitter_factor(0.25)
    .max_elapsed(Some(Duration::from_millis(250)))
    .build()
    .expect("valid CAS retry settings");
```

Common shortcuts:

- `CasExecutor::high_concurrency()` uses exponential backoff and jitter for
  contended writers.
- `CasExecutor::low_latency()` retries immediately with a small attempt budget.
- `CasExecutor::high_reliability()` uses a longer retry window for operations
  where eventual success is more important than latency.

## Hooks

Hooks are attached to a single execution, so the same executor can be reused
with different observability behavior:

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::{CasAttemptFailure, CasContext, CasDecision, CasExecutor, CasHooks};

let state = AtomicRef::from_value(1usize);
let executor = CasExecutor::<usize, &'static str>::low_latency();

let hooks = CasHooks::new().on_retry(
    |context: &CasContext, failure: &CasAttemptFailure<usize, &'static str>| {
        eprintln!("retry attempt {} after {failure}", context.attempt());
    },
);

let success = executor
    .execute_with_hooks(
        &state,
        |current: &usize| CasDecision::update_value(*current + 1, *current + 1),
        hooks,
    )
    .expect("CAS should succeed");

assert_eq!(*success.output(), 2);
```

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
            CasDecision::update_value(*current + 1, *current + 1)
        })
        .await
        .expect("async CAS should succeed");

    assert_eq!(*success.current().as_ref(), 1);
}
```

## Project Layout

- `src/decision`: typed CAS decision values.
- `src/executor`: builder and synchronous/asynchronous CAS executor.
- `src/event`: execution context and lifecycle hooks.
- `src/error`: attempt-level and terminal CAS errors.
- `src/options`: timeout policy options.
- `tests`: behavior tests for executor, builder, hooks, errors, and options.

## Quality Checks

```bash
./align-ci.sh
./ci-check.sh
./coverage.sh json
```

## License

Apache-2.0
