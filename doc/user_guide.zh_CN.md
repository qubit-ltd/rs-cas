# Qubit CAS 用户指南

本指南说明如何在生产应用中选择和使用 `qubit-cas`。英文版本见
[`user_guide.md`](user_guide.md)。

## 1. CAS 模型、快照与线性化

`CasExecutor<T, E>` 读取不可变的 `Arc<T>` 快照，调用 operation，并有条件地安装
`CasDecision::update` 返回的新值。只有观测到的快照仍然是当前值时，compare-and-swap
才会成功。成功的 compare-and-swap 是本次更新的线性化点。`finish` 决策在线程观测当前
快照时线性化，不写入新值。

## 2. 决策

使用 `update(next, output)` 替换状态，使用 `finish(output)` 表示无需写入，使用
`retry(error)` 表示可重试的业务条件，使用 `abort(error)` 表示立即终止的业务错误。
发生冲突或可重试失败后 operation 可能再次运行，因此它必须确定且可以安全重放。

## 3. Rich 与 result-only 执行

调用方只需要 `Result<CasSuccess<...>, CasError<...>>` 时，使用 `execute_result` 或
`execute_async_result`；这两条路径不会构造报告和 hooks。需要尝试次数、冲突、耗时或
终态时使用 `execute`/`execute_async`。需要单次执行的事件或告警时使用 `*_with_hooks`。

## 4. 策略与 builder

`latency_first`、`contention_adaptive` 和 `reliability_first` 是起点，不是适用于所有
负载的默认答案。修改预设前应先测量实际负载。builder 可以配置尝试次数、重试延迟、
抖动、elapsed 预算、可观测性和异步超时行为。高级重试类型可从 `qubit_cas::retry`
导入。

## 5. 软预算与硬超时

`max_operation_elapsed` 限制用户 operation 的累计耗时。\
`max_total_elapsed` 是覆盖重试、延迟和 hooks 的软续试预算：已经准入的 operation 可以完成。\
`flow_timeout` 是异步执行独立的硬墙钟超时，对同步执行无效。\
`attempt_timeout` 限制一次异步 operation；需要超时后重试时配置 `retry_on_timeout()`。

## 6. 同步与异步执行

同步延迟会阻塞调用线程。异步方法需要 `tokio` feature，并会在等待期间让出执行权。
result-only 异步 operation 与同步版本具有相同的决策语义：

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

## 7. 报告、事件、告警与 listener panic

`CasExecutionReport` 记录尝试次数、冲突次数、冲突率、耗时和终态。
`CasHooks` 观察一次执行中的 `CasEvent`。默认模式是 `ReportOnly`；`EventStream` 增加
事件；`EventStreamWithAlert` 进一步增加争用阈值。Hook 在执行路径中运行，应保持轻量且
非阻塞。当 listener 可能失败时要显式配置 panic 策略；不要在 hooks 中执行不可幂等的状态修改。

## 8. 错误与诊断所有权

使用 `CasError::kind()` 做控制流判断，使用 `CasError::error()` 读取保留的业务错误。
`CasError::current()` 可能包含最后的快照，也包括异步超时前保留的快照。对于固定的
`qubit-retry` 0.22 契约，`CasRetryFailure` 保留重试限额、超时范围、取消、回调失败和基础设施诊断。

## 9. 性能与 `qubit-fast-cas`

不需要报告的热路径优先使用 result-only 执行。优先使用 `ReportOnly`，如果必须导出事件，
请将事件投递到无阻塞 channel。对于不需要报告、hooks、异步支持和业务重试的紧凑 `u64`
状态机，请使用独立的 [`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas)。

## 10. 排障与限制

- 意外出现 `RetryExhausted`：检查 operation 是否把永久性业务错误错误地返回成 `retry`。
- 意外出现 `MaxTotalElapsedExceeded`：它是软续试预算；检查保留的 retry failure 以确认具体原因。
- 意外出现 `AttemptTimeout`：区分单次 attempt 超时和 `flow_timeout`。
- 冲突率很高：尝试 `ContentionAdaptive`，缩小共享热点，或拆分不可变状态。
- 副作用重复执行：移到成功返回之后，或使用外部 operation id 使其幂等。

执行器不能为多个原子值提供跨对象事务，也不能让不可重放的 operation 变安全。当更新需要
长临界区或外部协调时，应使用锁或数据库事务。
