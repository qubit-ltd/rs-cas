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

Use it when concurrent writers must not lose updates and an operation can be
replayed from the latest snapshot. Keep the operation deterministic and make
side effects idempotent; CAS may invoke it more than once.

## Installation

```toml
[dependencies]
qubit-cas = "0.11"
qubit-atomic = "0.13"
```

Enable asynchronous execution with `features = ["tokio"]`. Advanced retry
configuration is available through [`qubit_cas::retry`](https://docs.rs/qubit-cas/latest/qubit_cas/retry/index.html).

## Quick start

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

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
[0.11 migration note](doc/migration-0.11.md). The [API documentation](https://docs.rs/qubit-cas)
contains the complete Rustdoc. `qubit-fast-cas` is a separate compact `u64`
state-machine crate without reports, hooks, async execution, or business retry.

## Testing

```bash
cargo test
cargo test --all-features
./ci-check.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Keep public API documentation and tests current, run
`./align-ci.sh`, and run `./ci-check.sh` before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-cas](https://github.com/qubit-ltd/rs-cas)
