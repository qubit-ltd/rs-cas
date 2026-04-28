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
- **可观测执行报告**：每次执行都会返回 `CasOutcome`，其中的
  `CasExecutionReport` 汇总尝试次数、冲突次数、冲突率、耗时和终止结果。
- **生命周期事件流**：`CasHooks` 可在单次执行中观察统一的 `CasEvent`，不需要污染业务逻辑。
- **策略化执行器**：内置 `LatencyFirst`、`ContentionAdaptive`、`ReliabilityFirst`
  三种策略画像。
- **结构化结果**：`CasSuccess`、`CasError` 与 `CasAttemptFailure` 暴露最终状态、旧状态、
  业务输出、错误分类和最后一次失败原因。

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

`execute*` 返回 `CasOutcome<T, R, E>`。它包含业务层 `Result<CasSuccess<T, R>, CasError<T, E>>`
以及本次执行的 `CasExecutionReport`，调用方可以在不注册 Hook 的情况下读取冲突次数和冲突率。

## 执行策略

`qubit-cas` 提供三种常见策略，方便按场景直接选用：

- `CasExecutor::latency_first()`：立即重试 + 较小尝试次数，适合延迟敏感场景。
- `CasExecutor::contention_adaptive()`：指数退避 + 抖动，适合写竞争较高的场景。
- `CasExecutor::reliability_first()`：更长重试窗口，适合更看重最终成功率的操作。

通常可以先用 `latency_first()` 起步；如果报告中
`conflict_ratio >= 0.30` 且 `attempts_total >= 3`，说明出现明显热点争用，
可以切到 `contention_adaptive()`；如果业务更看重“尽量成功”而非“尽快返回”，
可选 `reliability_first()`。

## 重试配置

预置执行器不满足需求时，可以使用 builder：

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

## 冲突观测与 Hooks

Hook 绑定到单次执行，因此同一个 executor 可以在不同调用中使用不同的观测逻辑。
默认情况下只返回 `CasExecutionReport`，如果需要实时事件流，可开启 `event_stream()`：

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

let outcome = executor
    .execute_with_hooks(
        &state,
        |current: &usize| CasDecision::update(*current + 1, *current + 1),
        hooks,
    )
    .expect("CAS should succeed");

assert_eq!(*outcome.output(), 2);
```

## 检测能力与性能权衡

冲突检测本身也会增加热路径成本，因此 `qubit-cas` 将可观测能力分为三个层级：

- `ReportOnly`（默认）：只聚合 `CasExecutionReport`，不构造 attempt 事件，适合大多数生产路径。
- `EventStream`：向 listener 发送 `CasEvent`，适合需要实时日志、trace 或指标上报的路径。
- `EventStreamWithAlert`：在事件流基础上做阈值判定，适合热点争用告警。

建议默认使用 `ReportOnly`，通过 `outcome.report().conflict_ratio()` 做周期性指标上报。
只有在需要定位热点或接入 trace 时再开启 `EventStream`。不要在 Hook 中同步写日志、
同步请求远端 metrics 或执行复杂格式化；高冲突时这些操作会被按 attempt 次数放大。
更稳妥的方式是把事件投递到无阻塞 channel，由后台任务批量消费。

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
- `src/observability`：可观测模式、争用阈值和告警类型。
- `src/options`：超时处理策略。
- `src/outcome` 与 `src/report`：执行结果包装与可观测报告。
- `src/strategy`：内置执行策略和策略画像。
- `benches`：观测模式开销基准测试。
- `tests`：executor、builder、hooks、错误与选项的行为测试。

## 质量检查

```bash
./align-ci.sh
./ci-check.sh
./coverage.sh json
```

## 许可证

Apache-2.0
