# Qubit CAS 用户指南

本指南适用于 `qubit-cas` 0.9，面向需要并发更新不可变状态的 Rust 应用开发者。英文版本见
[`user_guide.md`](user_guide.md)。

## 库存预留场景与安装

多个订单共享剩余库存。先安装以下依赖；使用 README 的库存例子从 3 扣到 2，库存为 0 时返回
OutOfStock。只有成功提交的 output 交给调用者。发送通知或扣款应安排在成功返回后；
在重试闭包内执行外部 I/O 时，即便请求可幂等，也必须考虑取消不会回滚它的效果。

```toml
[dependencies]
qubit-cas = { version = "0.9", features = ["tokio"] }
qubit-atomic = "0.13"
tokio = { version = "1.52", features = ["macros", "rt-multi-thread", "time"] }
```

## 1. CAS 模型、快照与线性化

`CasExecutor<T, E>` 读取不可变的 `Arc<T>` 快照，调用 operation，并有条件地安装
`CasDecision::update` 返回的新值。只有观测到的快照仍然是当前值时，compare-and-swap
才会成功。成功的 compare-and-swap 是本次更新的线性化点。`finish` 决策在线程观测当前
快照时线性化，不写入新值。

比较依据是 `Arc::ptr_eq`，不是 `T` 的值。分别分配、内容相同的两个 `Arc` 仍属于不同快照，
可能彼此冲突。使用 `update_arc` 时，应根据 operation 得到的观测快照生成替换值。不要把历史
`Arc` 当作 A-B-A 检测器：一个分配身份可以在中间更新之后再次被安装，这段历史不会被 CAS 暴露。
`T` 内的 `Mutex`、`Cell`、原子字段和其他内部可变性也不受身份 CAS 保护。需要共同协调的状态
应放进不可变、带版本的快照中。`CasSuccess::current()` 返回 `update` 已发布或 `finish` 已观测
的快照；调用方拿到结果前，全局状态可能已经被其他写者替换。

## 2. 决策

使用 `update(next, output)` 替换状态，使用 `finish(output)` 表示无需写入，使用
`retry(error)` 表示可重试的业务条件，使用 `abort(error)` 表示立即终止的业务错误。
发生冲突或可重试失败后 operation 可能再次运行，因此它必须确定且可以安全重放。

## 3. Rich 与 result-only 执行

调用方只需要 `Result<CasSuccess<...>, CasError<...>>` 时，使用 `execute_result` 或
`execute_async_result`；这两条路径不会构造报告和 hooks。需要尝试次数、冲突、耗时或
终态时使用 `execute`/`execute_async`。需要单次执行的事件或告警时使用 `*_with_hooks`。

## 0.9 的默认错误与 Clone

`CasExecutor<T>` 和 `CasBuilder<T>` 的默认业务错误改为 `CasBoxError`。它实现
`std::error::Error`，并把被包装的错误保留为 source，因此默认的终态 `CasError` 可以接入
普通错误链。具体错误必须显式装箱；若提供 blanket `From<E>`，会与 Rust 已有的
`From<T> for T` 实现重叠。

```rust
use std::error::Error;

use qubit_cas::CasBoxError;

fn main() {
    let error = CasBoxError::new(Box::new(std::io::Error::other("reservation failed")));
    assert_eq!(error.source().unwrap().to_string(), "reservation failed");
}
```

`CasDecision`、`CasSuccess`、`CasAttemptFailure`、`CasError` 和 `CasOutcome` 克隆时共享
`Arc<T>`。它们不再要求 `T: Clone`；只有按值持有的 output 或业务错误需要 `Clone`。

## 4. 策略与 builder

`latency_first`、`contention_backoff` 和 `reliability_first` 提供三种预设。
`ContentionBackoff` 使用固定参数的指数退避和 jitter，不会依据观测到的竞争率动态调参。
builder 默认是 5 次尝试、无延迟、无软预算；LatencyFirst 是 100 次尝试、5ms 操作预算、20ms 总预算。

配置按调用顺序最后写入生效：strategy 替换次数、软预算和退避，但保留异步 timeout/action；
随后调用 max_attempts/no_delay 可覆盖对应配置。次数包括首次尝试。

四个 executor getter 读取实际生效的次数和软预算，不执行 CAS，也不初始化执行缓存。
退避在 builder 上配置；getter 不暴露 retry 内部策略对象。下面先选择延迟优先预设，
再把次数改成 7，时间预算仍沿用预设：

```rust
use std::time::Duration;

use qubit_cas::CasExecutor;
use qubit_cas::CasStrategy;

fn main() {
    let executor = CasExecutor::<usize, ()>::builder()
        .strategy(CasStrategy::LatencyFirst)
        .max_attempts(7)
        .build()
        .expect("valid configuration");
    assert_eq!(executor.max_attempts(), 7);
    assert_eq!(executor.max_retries(), 6);
    assert_eq!(executor.max_operation_elapsed(), Some(Duration::from_millis(5)));
    assert_eq!(executor.max_total_elapsed(), Some(Duration::from_millis(20)));
    assert_eq!(executor.attempt_timeout(), None);
    assert_eq!(executor.flow_timeout(), None);
}
```

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
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

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

### 告警与事件时序

只有 `on_contention_alert(thresholds, callback)` 能注册告警，三个阈值必须全部满足。
重复调用时，阈值和回调一起替换；使用默认阈值时显式传入 `ContentionThresholds::default()`。
下例模拟一次库存竞争，第二次尝试基于新库存 7 扣减到 6，并触发一次告警：

```rust
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAlert;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::ContentionThresholds;

fn main() {
    let state = AtomicRef::from_value(3usize);
    let first = Cell::new(true);
    let alerts = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&alerts);
    let hooks = CasHooks::new().on_contention_alert(
        ContentionThresholds::new(2, 1, 0.5),
        move |alert: &CasAlert| {
            assert_eq!(alert.report().conflicts(), 1);
            observed.fetch_add(1, Ordering::SeqCst);
        },
    );
    let executor = CasExecutor::<usize, ()>::builder().max_attempts(2).build().unwrap();
    let outcome = executor.execute_with_hooks(&state, |current: &usize| {
        if first.replace(false) {
            state.store(Arc::new(7));
        }
        CasDecision::update(*current - 1, ())
    }, hooks);
    assert!(outcome.is_ok());
    assert_eq!(outcome.report().attempts_total(), 2);
    assert_eq!(*state.load(), 6);
    assert_eq!(alerts.load(Ordering::SeqCst), 1);
}
```

这里的 `state.store` 仅用来模拟另一写者，不推荐在真实业务重试闭包中执行外部副作用。

`RetryScheduled` 表示调度检查已通过并选定延迟，后续 deadline、取消或准入检查仍可能阻止
下一次操作开始。最终次数耗尽或调度预算不足时不发送该事件；实际次数读取终态 attempts。

| 场景 | 事件顺序 | 实际尝试次数 |
| --- | --- | --- |
| 第一次成功 | Started → Finished | 1 |
| 一次 Retry 后成功 | Started → AttemptFailed → RetryScheduled → Finished | 2 |
| max_attempts=1，返回 Retry | Started → AttemptFailed → Finished | 1 |
| 第一次 Retry 的退避被总预算拒绝 | Started → AttemptFailed → Finished | 1 |
| 已调度，但退避期间触发 flow timeout | Started → AttemptFailed → RetryScheduled → Finished | 1 |

表中 Started/Finished 分别是 `ExecutionStarted`/`ExecutionFinished` 的简写。

## 8. 错误与诊断所有权

使用 `kind()` 判断错误类别，`termination()` 区分次数/预算/超时终止原因，
`error()` 读取原始业务错误。终止原因与最后失败相互独立，例如业务 Retry 后在退避期间
触发 FlowTimeout，仍保留之前的业务错误。

`current()` 返回与最后失败关联的快照，不保证是调用时最新状态。尝试超时保留该次开始时的快照；
退避超时保留已有失败；尚未开始第一次尝试时可以是 None。终止时不会重新 load 拼接较新的状态。

`diagnostic()` 可返回 Cancellation、Callback、Clock、Timer 等 CAS 自有分类及原始诊断文本；
`completion_diagnostics()` 按顺序保留终态冻结后的回调失败。程序判断使用 kind，message 仅用于排障，
不依赖其文本格式。普通 Abort/冲突/预算/超时没有基础设施诊断。drop 取消 future 不会返回 CasError。

库存不足属于业务终止，应检查结构化分类并读取原始错误：

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;
use qubit_cas::CasTermination;

fn main() {
    let inventory = AtomicRef::from_value(0usize);
    let error = CasExecutor::<usize, &'static str>::builder().build().unwrap()
        .execute_result(&inventory, |stock: &usize| {
            if *stock == 0 {
                CasDecision::abort("out of stock")
            } else {
                CasDecision::update(*stock - 1, ())
            }
        })
        .expect_err("empty inventory aborts");
    assert_eq!(error.kind(), CasErrorKind::Abort);
    assert_eq!(error.termination(), CasTermination::Aborted);
    assert_eq!(error.error(), Some(&"out of stock"));
    assert_eq!(**error.current().unwrap(), 0);
    assert!(error.diagnostic().is_none());
}
```

## 9. 性能与 `qubit-fast-cas`

不需要报告的热路径优先使用 result-only 执行。需要报告时使用 `execute`；如果必须导出事件，
请将事件投递到无阻塞 channel。对于不需要报告、hooks、异步支持和业务重试的紧凑 `u64`
状态机，请使用独立的 [`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas)。

标准状态机 `qubit-state-machine` 0.9 使用 CAS 0.9，支持通过
`StateMachineBuilder::cas_executor` 注入配置。仅启用 fast 时不引入 CAS。
`AtomicRef`、`Function`/`Consumer` 和默认错误类型 `CasBoxError` 是有意保留的公共协作边界；
隐藏 retry 内部类型不代表隐藏全部上游类型。

## 10. 排障与限制

- 意外出现 `RetryExhausted`：检查 operation 是否把永久性业务错误错误地返回成 `retry`。
- 意外出现 `TotalBudgetExceeded`：它是软续试预算；检查 termination 与最后失败以确认具体原因。
- 意外出现 `AttemptTimeout`：区分单次 attempt 超时和 `flow_timeout`。
- 冲突率很高：尝试 `ContentionBackoff`，缩小共享热点，或拆分不可变状态。
- 副作用重复执行：移到成功返回之后，或使用外部 operation id 使其幂等。

执行器不能为多个原子值提供跨对象事务，也不能让不可重放的 operation 变安全。当更新需要
长临界区或外部协调时，应使用锁或数据库事务。

`update` 为每个新快照分配 Arc；`update_arc` 可复用预先分配的快照。预先分配不能把历史快照变成
版本标记，替换值仍应从当前观测推导。热 finish 的执行器记账不分配，但不能把它推广成任意 CAS
操作无分配。运行 `cargo bench --bench contention` 评估竞争与尾延迟。

返回 [README](../README.zh_CN.md) 或参阅 [API](https://docs.rs/qubit-cas)。
