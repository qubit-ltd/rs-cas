# Qubit CAS

[![Rust CI](https://github.com/qubit-ltd/rs-cas/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-cas/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-cas/coverage-badge.json)](https://qubit-ltd.github.io/rs-cas/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-cas.svg?color=blue)](https://crates.io/crates/qubit-cas)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

## Overview

`qubit-cas` is a typed compare-and-swap executor for immutable `Arc<T>` snapshots.
It combines atomic updates, contention-aware retry, business decisions, optional
timeouts, and structured reports in one reusable `CasExecutor`.

For example, concurrent orders can decrement inventory without one writer
overwriting another successful reservation. Use it when concurrent writers must not lose updates and an operation can be
replayed from the latest snapshot. Keep the operation deterministic and make
side effects idempotent; CAS may invoke it more than once.

## Installation

```toml
[dependencies]
qubit-cas = "0.14"
qubit-atomic = "0.13"
```

Enable asynchronous execution with `features = ["tokio"]`. Configure attempts,
budgets, backoff, and timeouts directly on `CasExecutor::builder()`; no retry
implementation types are needed.

## Quick start

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

#[derive(Debug, PartialEq, Eq)]
struct Inventory { stock: u32 }

#[derive(Debug, Clone, PartialEq, Eq)]
enum OrderError { OutOfStock }

fn main() {
    let state = AtomicRef::from_value(Inventory { stock: 3 });
    let executor = CasExecutor::<Inventory, OrderError>::latency_first();
    let result = executor.execute_result(&state, |current: &Inventory| {
        if current.stock == 0 { return CasDecision::abort(OrderError::OutOfStock); }
        CasDecision::update(Inventory { stock: current.stock - 1 }, current.stock - 1)
    });
    let success = result.expect("stock should be available");
    assert_eq!(*success.output(), 2);
    assert_eq!(state.load().stock, 2);
}
```

| Decision | Effect |
| --- | --- |
| `update(next, output)` | CAS-install a replacement snapshot and return output. |
| `finish(output)` | Succeed without writing a new snapshot. |
| `retry(error)` | Retry a business-level failure through the configured policy. |
| `abort(error)` | Stop immediately with a classified business error. |

## Choosing an execution path

| Need | API |
| --- | --- |
| Result only, no report or hooks | `execute_result` / `execute_async_result` |
| Attempts, conflicts, elapsed time, or report | `execute` / `execute_async` |
| Per-execution events or alerts | `execute_with_hooks` / `execute_async_with_hooks` |
| Numeric allocation-free hot path | [`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas) |

Read the [user guide](doc/user_guide.md), [design](doc/design.md), and
[0.15 migration note](doc/migration-0.15.md). The [API documentation](https://docs.rs/qubit-cas)
contains the complete Rustdoc. `qubit-fast-cas` is a separate compact `u64`
state-machine crate without reports, hooks, async execution, or business retry.

Result-only execution skips reports and events; `update` still allocates an `Arc`
for the replacement snapshot. Timeouts are cooperative and cannot preempt blocking
code. Cancellation does not undo committed state or external side effects.
`CasError::diagnostic()` and `completion_diagnostics()` preserve infrastructure
and callback details, separately from the business `error()`.

See the [Chinese guide](doc/user_guide.zh_CN.md). Standard state machines can inject
an executor with `StateMachineBuilder::cas_executor`; compact integer states keep
using the separate fast-cas crate.

Version 0.15 exposes installed limits through `max_attempts()`, `max_retries()`,
`max_operation_elapsed()`, and `max_total_elapsed()`. Register alerts through
`on_contention_alert(thresholds, callback)`; repeated registration replaces both.

## Snapshot identity and default errors

CAS compares snapshot identity with `Arc::ptr_eq`, not the value inside `T`.
Two separately allocated `Arc`s containing equal values are distinct snapshots;
installing either can conflict with an observation of the other. Build an
`update_arc` replacement from the snapshot passed to the operation, and do not
use an old `Arc` to detect an A-B-A history: identity can return to a prior
allocation without exposing the intervening publication. `Mutex`, `Cell`, atomic
fields, and other interior-mutability inside `T` are outside this identity check.
Use immutable, versioned state when those mutations must participate in the CAS
contract. `CasSuccess::current()` is the snapshot published by an `update` or
observed by a `finish`; another writer may replace the global state before the
method returns.

The default business-error type is `CasBoxError`, so terminal default errors
participate in `std::error::Error` source chains. Wrap concrete errors explicitly
with `CasBoxError::new(Box::new(error))`; 0.15 intentionally has no blanket
`From<E>` conversion because it would overlap with Rust's `From<T> for T`.
`CasSuccess`, `CasError`, and `CasOutcome` can now be cloned without `T: Clone`;
only the owned output or business error needs `Clone`.

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-cas](https://github.com/qubit-ltd/rs-cas)
