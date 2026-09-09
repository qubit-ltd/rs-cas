# Qubit CAS 用户指南

本指南适用于 `qubit-cas` 0.13，面向需要并发更新不可变状态的 Rust 应用开发者。英文版本见
[`user_guide.md`](user_guide.md)。

## 库存预留场景与安装

多个订单共享剩余库存。先安装以下依赖；使用 README 的库存例子从 3 扣到 2，库存为 0 时返回
OutOfStock。只有成功提交的 output 交给调用者。发送通知或扣款应安排在成功返回后；
在重试闭包内执行外部 I/O 时，即便请求可幂等，也必须考虑取消不会回滚它的效果。

```toml
[dependencies]
qubit-cas = { version = "0.13", features = ["tokio"] }
qubit-atomic = "0.13"
tokio = { version = "1.52", features = ["macros", "rt-multi-thread", "time"] }
```

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

`latency_first`、`contention_backoff` 和 `reliability_first` 提供三种预设。
`ContentionBackoff` 使用固定参数的指数退避和 jitter，不会依据观测到的竞争率动态调参。
builder 默认是 5 次尝试、无延迟、无软预算；LatencyFirst 是 100 次尝试、5ms 操作预算、20ms 总预算。

配置按调用顺序最后写入生效：strategy 替换次数、软预算和退避，但保留异步 timeout/action；
随后调用 max_attempts/no_delay 可覆盖对应配置。次数包括首次尝试。

## 5. 软预算与硬超时

`max_operation_elapsed` 限制尝试累计耗时（包括 CAS 适配工作）；`max_total_elapsed`
覆盖 retry flow 的操作、退避和控制回调。这两个软预算只决定是否允许后续尝试，
已经准入的操作可以成功提交，成功结果不会因预算耗尽被撤销。

`flow_timeout` 覆盖异步 retry flow 的尝试和退避，不涵盖开始/结束 hooks 的全部耗时。
`attempt_timeout` 限制单次异步尝试；默认超时终止，`retry_on_timeout()` 则继续重试。
同步入口忽略这两种硬 timeout。它们依赖 future 让出执行权，不能抢占阻塞调用或持续占用 CPU 的代码。

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

`execute` 返回报告，`execute_with_hooks` 还可通过 `CasHooks::on_event` 接收事件；
`on_contention_alert` 同时注册阈值和告警回调。没有额外的观测模式枚举。

报告记录尝试、冲突、业务重试、超时、耗时和终态。listener 在执行线程运行，应快速且非阻塞。
在 unwind 构建中，listener panic 被隔离并记入最终报告的 `listener_failures()`；业务结果不变。
完成事件携带调用该 listener 之前的报告快照，不能包含它自身稍后发生的 panic。
用户 operation 的 panic 继续向外传播；panic=abort 构建不提供隔离保证。

## 8. 错误与诊断所有权

使用 `kind()` 判断错误类别，`termination()` 区分次数/预算/超时终止原因，
`error()` 读取原始业务错误。终止原因与最后失败相互独立，例如业务 Retry 后在退避期间
触发 FlowTimeout，仍保留之前的业务错误。

`current()` 返回与最后失败关联的快照，不保证是调用时最新状态。尝试超时保留该次开始时的快照；
退避超时保留已有失败；尚未开始第一次尝试时可以是 None。终止时不会重新 load 拼接较新的状态。

`diagnostic()` 可返回 Cancellation、Callback、Clock、Timer 等 CAS 自有分类及原始诊断文本；
`completion_diagnostics()` 按顺序保留终态冻结后的回调失败。程序判断使用 kind，message 仅用于排障，
不依赖其文本格式。普通 Abort/冲突/预算/超时没有基础设施诊断。drop 取消 future 不会返回 CasError。

## 9. 性能与 `qubit-fast-cas`

不需要报告的热路径优先使用 result-only 执行。需要报告时使用 `execute`；如果必须导出事件，
请将事件投递到无阻塞 channel。对于不需要报告、hooks、异步支持和业务重试的紧凑 `u64`
状态机，请使用独立的 [`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas)。

## 10. 排障与限制

- 意外出现 `RetryExhausted`：检查 operation 是否把永久性业务错误错误地返回成 `retry`。
- 意外出现 `TotalBudgetExceeded`：它是软续试预算；检查 termination 与最后失败以确认具体原因。
- 意外出现 `AttemptTimeout`：区分单次 attempt 超时和 `flow_timeout`。
- 冲突率很高：尝试 `ContentionBackoff`，缩小共享热点，或拆分不可变状态。
- 副作用重复执行：移到成功返回之后，或使用外部 operation id 使其幂等。

执行器不能为多个原子值提供跨对象事务，也不能让不可重放的 operation 变安全。当更新需要
长临界区或外部协调时，应使用锁或数据库事务。

`update` 为每个新快照分配 Arc；`update_arc` 可复用预先分配的快照。热 finish 的执行器记账不分配，
但不能把它推广成任意 CAS 操作无分配。运行 `cargo bench --bench contention` 评估竞争与尾延迟。

返回 [README](../README.zh_CN.md)；参阅 [API](https://docs.rs/qubit-cas) 和 [0.13 迁移说明](migration-0.13.zh_CN.md)。
