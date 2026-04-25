# Qubit CAS

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-cas.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-cas)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-cas/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-cas?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-cas.svg?color=blue)](https://crates.io/crates/qubit-cas)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Doc](https://img.shields.io/badge/docs-English-blue.svg)](README.md)

面向 Rust 的强类型 compare-and-swap（CAS）执行器。`qubit-cas` 将常见的
「读取共享快照、根据快照生成新值、通过 compare-and-swap 原子写入新值、遇到竞争后重试」
流程封装为可复用的 `CasExecutor`。

CAS 机制可以理解为“先比较、再交换”：只有当共享状态仍等于你读取到的旧快照时，
新值才会被原子写入并生效；若期间被其他线程改动，本次写入会失败并可按策略重试。
它的优点是无锁路径延迟低、并发冲突时不会产生写丢失；代价是高竞争下可能出现
较多重试，进而带来 CPU 开销增加与尾延迟上升。

本 crate 基于 [`qubit-atomic`](https://crates.io/crates/qubit-atomic)、
[`qubit-function`](https://crates.io/crates/qubit-function) 与
[`qubit-retry`](https://crates.io/crates/qubit-retry)。它适合共享状态以
不可变 `Arc<T>` 快照保存、并且每次更新都希望用显式类型表达结果决策的场景。

## 特性

- **强类型决策**：业务操作返回 `CasDecision::update`、`finish`、`retry`
  或 `abort` 后，`CasExecutor` 会按决策自动执行对应流程：写入新状态、无写入成功、
  继续重试或立即终止。
- **带重试语义的 CAS 循环**：compare-and-swap 冲突与业务层 `retry`
  决策会交给 `qubit-retry` 处理，可配置尝试次数、总耗时、延迟和抖动。
- **同步与异步 API**：`execute` 不依赖异步运行时；启用 `tokio` feature 后可使用
  `execute_async`。
- **异步超时控制**：可为每次异步尝试设置超时时间，并选择超时后继续重试或立即中止。
- **生命周期 Hooks**：`CasHooks` 可在单次执行中观察成功、重试和中止事件，不需要污染业务逻辑。
- **预置执行器**：内置高并发、低延迟、高可靠三种常见重试配置。
- **结构化结果**：`CasSuccess`、`CasError` 与 `CasAttemptFailure` 暴露最终状态、旧状态、
  业务输出、尝试次数、错误分类和最后一次失败原因。

## 安装

```toml
[dependencies]
qubit-cas = "0.1.0"
qubit-atomic = "0.10"
```

`qubit-cas` 使用 `qubit_atomic::AtomicRef<T>` 保存共享状态。应用代码如果需要构造或持有该状态，应直接依赖 `qubit-atomic`。

启用异步执行：

```toml
[dependencies]
qubit-cas = { version = "0.1.0", features = ["tokio"] }
qubit-atomic = "0.10"
```

## 快速开始

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

    let result = executor.execute(&state, |current: &Inventory| {
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

    match result {
        Ok(success) => {
            println!("stock updated successfully, remaining: {}", success.output());
            assert!(success.is_updated());
            assert_eq!(*success.output(), 2);
            assert_eq!(state.load().stock, 2);
        }
        Err(error) => {
            // 缺货是业务结果，不应直接 panic。
            eprintln!("order rejected: {error}");
        }
    }
}
```

这段示例展示了一个“下单扣减库存”的 CAS 更新流程：

- `AtomicRef::from_value(Inventory { stock: 3 })` 初始化共享库存快照，初始库存是 `3`。
- `execute` 每次尝试都会读取当前快照 `current`：
  - 若库存为 `0`，返回 `CasDecision::abort(OrderError::OutOfStock)`，立即终止，不再重试。
  - 否则返回 `CasDecision::update(...)`，把库存减 `1` 并把“扣减后的库存值”作为业务输出。
- 这次写入通过 CAS（compare-and-swap）更新共享状态：若并发竞争导致本次写入失败，执行器会基于最新快照重试，避免并发更新时的写丢失。
- 示例通过 `match` 显式处理结果：成功时校验写入与输出；失败时按业务分支处理（例如记录缺货）。

## 决策模型

每次业务操作都会收到当前状态快照，并返回 `CasDecision<T, R, E>`：

- `CasDecision::update(next, output)`：从所有权值创建并写入新状态。
- `CasDecision::update_arc(next, output)`：当你已经有 `Arc<T>` 时直接写入新状态。
- 如果其他写入者先完成更新，本次 CAS 会按重试配置再次尝试。
- `CasDecision::finish(output)`：成功结束但不写入新状态。适合当前快照已经满足操作目标的场景。
- `CasDecision::retry(error)`：表示可重试的业务失败；如果重试次数耗尽，最终错误分类为
  `CasErrorKind::RetryExhausted`。
- `CasDecision::abort(error)`：立即终止流程，并返回 `CasErrorKind::Abort`。

成功时返回 `CasSuccess<T, R>`。它可以判断是否真的发生写入、读取当前状态、读取更新前的状态、
取得业务输出，并查看实际尝试次数。

## 预置执行器

`qubit-cas` 提供三种常见重试配置，方便按场景直接选用：

- `CasExecutor::high_concurrency()`：指数退避 + 抖动，适合写竞争较高的场景。
- `CasExecutor::low_latency()`：立即重试 + 较小尝试次数，适合延迟敏感场景。
- `CasExecutor::high_reliability()`：更长重试窗口，适合更看重最终成功率的操作。

通常可以先用 `low_latency()` 起步；如果冲突频繁再切到
`high_concurrency()`；如果业务更看重“尽量成功”而非“尽快返回”，可选
`high_reliability()`。

## 重试配置

预置执行器不满足需求时，可以使用 builder：

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

## Hooks

Hook 绑定到单次执行，因此同一个 executor 可以在不同调用中使用不同的观测逻辑：

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
        |current: &usize| CasDecision::update(*current + 1, *current + 1),
        hooks,
    )
    .expect("CAS should succeed");

assert_eq!(*success.output(), 2);
```

## 异步用法

启用 `tokio` feature 后，异步操作会收到一个 `Arc<T>` 快照。每次尝试可以设置超时，
超时后可继续重试，也可直接中止。

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

## 项目结构

- `src/decision`：强类型 CAS 决策值。
- `src/executor`：builder、同步 CAS 执行器与异步 CAS 执行器。
- `src/event`：执行上下文与生命周期 hooks。
- `src/error`：尝试级失败和终止级 CAS 错误。
- `src/options`：超时处理策略。
- `tests`：executor、builder、hooks、错误与选项的行为测试。

## 质量检查

```bash
./align-ci.sh
./ci-check.sh
./coverage.sh json
```

## 许可证

Apache-2.0
