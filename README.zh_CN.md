# Qubit CAS

[![Rust CI](https://github.com/qubit-ltd/rs-cas/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-cas/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-cas/coverage-badge.json)](https://qubit-ltd.github.io/rs-cas/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-cas.svg?color=blue)](https://crates.io/crates/qubit-cas)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

## 概览

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
- **同步与异步 API**：`execute` 与 `execute_result` 不依赖异步运行时；启用
  `tokio` feature 后可使用 `execute_async` 与 `execute_async_result`。
- **异步超时控制**：可为每次异步尝试设置超时时间，并选择超时后继续重试或立即中止。
- **可观测执行报告**：生成报告的执行会返回 `CasOutcome`，其中的
  `CasExecutionReport` 汇总尝试次数、冲突次数、冲突率、耗时和终止结果。
- **生命周期事件流**：`CasHooks` 可在单次执行中观察统一的 `CasEvent`，不需要污染业务逻辑。
- **策略化执行器**：内置 `LatencyFirst`、`ContentionAdaptive`、`ReliabilityFirst`
  三种策略画像。
- **结构化结果**：`CasSuccess`、`CasError` 与 `CasAttemptFailure` 暴露最终状态、旧状态、
  业务输出、错误分类和最后一次失败原因。
- **清晰的 crate 所有权**：轻量 `u64` CAS 类型位于独立的
  [`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas) crate，
  `qubit-cas` 不再重导出这些外部类型。

## 安装

```toml
[dependencies]
qubit-cas = "0.10"
```

`qubit-cas` 使用 `qubit_atomic::AtomicRef<T>` 保存共享状态。应用代码如果需要构造或
持有该状态，应直接依赖 `qubit-atomic`。

高级 builder 方法会暴露 `qubit-retry` 的选项类型；使用这些方法时应直接依赖
`qubit-retry`。

启用异步执行：

```toml
[dependencies]
qubit-cas = { version = "0.10", features = ["tokio"] }
```

可选 feature：

- `tokio`：启用 `CasExecutor::execute_async` 与
  `CasExecutor::execute_async_result`，并通过 Tokio 支持异步单次 attempt 超时处理。

默认 feature 为空，因此同步 CAS 执行不会引入异步运行时。

## 适用场景

当一次更新可以表达为“从当前不可变快照推导出一个类型化决策”时，适合使用
`qubit-cas`：

- 小型共享状态保存在 `AtomicRef<T>` 中，并以整体替换方式更新。
- 存在并发写入，但不允许出现写丢失。
- 基于最新快照重试的成本低于在整个操作期间持有锁。
- 调用方需要结构化观测 attempt、冲突、可重试业务失败、abort、timeout 和 elapsed 预算。

如果临界区耗时较长、更新逻辑包含无法安全重放的副作用，或状态无法表示为不可变替换值，
应优先考虑 mutex、数据库事务或领域内专用锁。

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
            eprintln!("order rejected: {error:?}");
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

`execute`、`execute_with_hooks`、`execute_async` 与 `execute_async_with_hooks` 返回
`CasOutcome<T, R, E>`。它包含业务层 `Result<CasSuccess<T, R>, CasError<T, E>>` 与本次执行的
`CasExecutionReport`，调用方可以在不注册 Hook 的情况下读取冲突次数和冲突率。
`execute_result` 与 `execute_async_result` 会跳过报告和 Hook 构造，适合只需要终态结果的调用方。

## 状态与操作建议

CAS 操作可能被调用多次，因为冲突和可重试业务失败都会让流程基于新快照重新尝试。
尽量保持 operation closure 确定且无副作用。如果必须执行副作用，建议在 `execute*`
成功返回后再执行，或让副作用具备幂等性并绑定外部 operation id。

共享值应当足够轻量，便于克隆为新的 `Arc<T>` 替换值。对于较大的状态，可以考虑持久化数据结构、
内部 `Arc` 字段，或只把指向大型不可变数据的小型状态对象放入 CAS。

`CasSuccess` 保存的是操作线性化时刻安装或观察到的快照，不保证方法返回时该值仍是全局最新状态。

## 错误处理

终止失败会以 `CasError<T, E>` 返回，并通过 `CasErrorKind` 分类：

- `Abort`：operation 返回了 `CasDecision::abort`。
- `Conflict`：compare-and-swap 冲突耗尽了重试策略。
- `RetryExhausted`：可重试业务失败耗尽了重试策略。
- `AttemptTimeout`：异步 attempt 超时并按 retry 层超时策略终止，或超时重试已耗尽。
- `MaxOperationElapsedExceeded`：累计用户 operation 执行时间超过预算。
- `MaxTotalElapsedExceeded`：整个 retry flow（包含延迟与 hook）超过总耗时预算。

控制分支建议使用 `error.kind()`；需要读取保留下来的业务错误时使用 `error.error()`；
如果最后一次失败保留了当时观察到的状态快照，可通过 `error.current()` 读取。

## 执行策略

`qubit-cas` 提供三种常见策略，方便按场景直接选用：

- `CasExecutor::latency_first()`：在很短总时间预算内立即重试，适合延迟敏感场景。
- `CasExecutor::contention_adaptive()`：使用微秒到毫秒级、有上界的指数退避与抖动，适合写竞争场景。
- `CasExecutor::reliability_first()`：使用毫秒级、有上界的退避窗口，适合允许适度等待的操作。

通常先用 `latency_first()`，再根据真实工作负载的测量结果选择其他预设。CAS 冲突与
显式业务 `CasDecision::retry` 共享同一 retry delay；如果二者需要不同时间策略，应通过
builder 配置与工作负载匹配的策略。同步 retry delay 会阻塞调用线程；需要在等待期间让出
执行权时应使用异步 API。

## 相关的 Fast CAS Crate

独立的 `qubit-fast-cas` crate 提供面向 `u64` 状态码的低层 CAS 路径，
适合状态机、executor、线程池内部状态和其他高频热路径。它假设共享状态
已经被编码成紧凑数字，状态迁移必须保持无分配。

常规 `CasExecutor` 基于不可变 `Arc<T>` 快照工作，提供业务重试、hooks、
执行报告、异步执行、超时处理和争用观测。`FastCas` 刻意不包含这些能力：
每次尝试只读取当前 `u64`，让调用方给出状态迁移决策，然后对刚才观测到的值
执行一次原子 compare-and-set。更小的接口让热路径更直接，也更适合紧凑状态机。

| 需求 | 推荐选择 |
| --- | --- |
| 需要丰富快照、执行报告、hooks、异步支持、超时处理或业务级重试 | `CasExecutor` |
| 状态已编码为 `u64`，需要无分配执行、无报告构造，并且只重试 CAS 冲突 | `FastCas` |

核心类型包括：

- `CasCell`：持有一个原子 `u64`，提供基础原子操作及无界的 `update`、
  `try_update` CAS 循环。
- `FastCasState`：`CasCell` 的兼容别名。
- `FastCas`：只携带 `FastCasPolicy` 的可复用执行器。
- `FastCasPolicy`：单次尝试、有界自旋或有界自旋后 yield。
- `FastCasDecision`：每次观测状态后返回 `Update`、`Finish` 或 `Abort`。
- `FastCasSuccess`：包含旧状态、当前状态、业务输出和尝试次数。
- `FastCasError`：表示调用方主动 `Abort`，或重试预算耗尽后的 `Conflict`。

如果希望把冲突作为内部并发细节，可以直接使用 `CasCell`：

```rust
use qubit_fast_cas::CasCell;

let state = CasCell::new(10);
let previous = state.update(|current| (current + 1, current));

assert_eq!(previous, 10);
assert_eq!(state.load(), 11);
```

发生并发冲突后，`CasCell` 的更新闭包可能执行多次，因此闭包应保持轻量，
并避免不可重复的副作用。

```rust
use qubit_fast_cas::{
    FastCas,
    FastCasState,
};

let state = FastCasState::new(0);
let cas = FastCas::spin(8);

let success = cas
    .update_by(&state, |current| {
        let next = current + 1;
        Ok::<_, &'static str>((next, next))
    })
    .expect("state code should update");

assert_eq!(success.previous(), 0);
assert_eq!(success.current(), 1);
assert_eq!(success.into_output(), 1);
assert_eq!(state.load(), 1);
```

显式状态机可以直接返回 `FastCasDecision`：

```rust
use qubit_fast_cas::{
    FastCas,
    FastCasDecision,
    FastCasState,
};

const IDLE: u64 = 0;
const RUNNING: u64 = 1;
const DONE: u64 = 2;

let state = FastCasState::new(IDLE);
let cas = FastCas::spin(8);

cas.compare_update(&state, IDLE, RUNNING)
    .expect("IDLE should transition to RUNNING");

let success = cas
    .execute(&state, |current| match current {
        RUNNING => FastCasDecision::<_, &'static str>::update(DONE, DONE),
        DONE => FastCasDecision::finish(DONE),
        _ => FastCasDecision::abort("invalid state"),
    })
    .expect("transition should be valid");

assert_eq!(success.current(), DONE);
assert_eq!(success.into_output(), DONE);
```

`FastCas` 只重试 CAS 冲突，不重试调用方返回的业务错误，不构造执行报告，也不
调用 hooks。传入的操作闭包是 `Fn`，当其他写入者先完成更新时，同一个闭包可能
被多次调用，因此闭包应保持确定性，并避免不可重复的副作用。调用方已经知道期望
状态码、只需要固定 `expected -> next` 迁移时，可以使用 `compare_update` 或
`compare_update_with`，这样不会在观测到其他状态后重新计算迁移。

`FastCasPolicy::once()` 最多执行一次 compare-and-set。
`FastCasPolicy::spin(max_attempts)` 在有界循环中紧凑重试冲突。
`FastCasPolicy::spin_yield(spin_attempts, max_attempts)` 先自旋，超过自旋前缀后
在后续尝试前调用 `thread::yield_now()`。尝试次数为 0 时会规范化为 1，
因此每种策略都至少能基于一次观测状态尝试推进。

### 迁移到 qubit-fast-cas

Fast CAS 状态值从 `usize` 改为 `u64`。`FastCasState` 也从
`qubit_atomic::Atomic<usize>` 的别名改为 `CasCell` 的别名。`load`、`store`、
`swap` 和 `compare_set` 仍可直接使用；原先使用 `fetch_add`、
`compare_set_weak`、`inner` 等其他 `Atomic` 方法的代码，应改用
`CasCell::update`/`try_update`，或者在确实需要这些底层操作时自行持有独立的
原子类型。

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
            eprintln!("CAS conflict at attempt {}", context.attempts());
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
        .execute_async_result(&state, |current| async move {
            CasDecision::update(*current + 1, *current + 1)
        })
        .await
        .expect("async CAS should succeed");

    assert_eq!(*success.current().as_ref(), 1);
}
```

## 公共 API 速览

- `CasExecutor<T, E>`：可复用的 CAS 执行器，绑定状态类型 `T` 和业务错误类型 `E`。
- `CasExecutor::execute_result` 与 `CasExecutor::execute_async_result`：跳过报告和 Hook 构造的
  result-only 执行路径。
- `CasBuilder<T, E>`：配置重试次数、elapsed 预算、延迟、抖动、异步超时选项、可观测能力和策略预设。
- `CasDecision<T, R, E>`：用户逻辑在每次 attempt 中返回的决策。
- `CasOutcome<T, R, E>`：终态结果与 `CasExecutionReport` 的组合。
- `CasSuccess<T, R>`：成功更新或无写入完成，包含当前状态、可选旧状态、业务输出和 attempt 上下文。
- `CasError<T, E>`：带 `CasErrorKind` 分类的终止失败。
- `CasRetryFailure`：精确保留固定 0.20.0 契约中的限额、超时、取消、回调失败与基础设施失败
  细节；若替换后的本地 path 源扩展了该契约，则以防御性的 `Unknown` 分类承接。
- `CasHooks`：单次执行的生命周期事件 hook 和告警 hook。
- `CasObservabilityConfig`：选择仅报告、事件流或带争用告警的事件流。
- `ContentionThresholds`：基于 attempt 数、冲突数和冲突率识别热点争用。

## 项目结构

- `src/cas_decision.rs`：强类型 CAS 决策值。
- `src/executor`：builder、同步 CAS 执行器与异步 CAS 执行器。
- `src/event`：执行上下文与生命周期 hooks。
- `src/error`：尝试级失败和终止级 CAS 错误。
- `src/observability`：可观测模式、争用阈值和告警类型。
- `src/cas_outcome.rs`、`src/cas_success.rs` 与 `src/report`：执行结果包装与可观测报告。
- `src/strategy`：内置执行策略和策略画像。
- `benches`：观测模式开销基准测试。
- `tests`：executor、builder、hooks、错误与选项的行为测试。

## 重试事件与次数

CAS 已迁移到 qubit-retry 0.20。`CasEvent::RetryRequested` 表示 CAS 规则请求重试，
即使最后一次允许的尝试发生冲突，也可能发出该事件；随后预算仍可能拒绝继续执行。
统计实际操作次数时，请读取最终报告的 `attempts_total()` 或结果中的尝试次数。
规则请求、重试调度回调和实际准入是三个不同阶段。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-cas](https://github.com/qubit-ltd/rs-cas)
