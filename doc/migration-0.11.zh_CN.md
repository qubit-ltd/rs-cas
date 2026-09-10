# 迁移到 `qubit-cas` 0.11

> 历史记录：本文仅描述 0.11 版本。当前 API 请参阅 [0.14 迁移说明](migration-0.14.zh_CN.md)。

英文版本见 [`migration-0.11.md`](migration-0.11.md)。本文说明升级已有应用时需要关注的行为和依赖变化。

## 1. 显式配置异步硬边界

`max_total_elapsed` 是软续试预算，可以拒绝后续重试，但已经准入的 operation 仍可能完成。
需要独立的异步硬墙钟限制时，请配置 `flow_timeout`：

```rust
let executor = qubit_cas::CasExecutor::<usize, ()>::builder()
    .max_total_elapsed(Some(std::time::Duration::from_secs(10)))
    .flow_timeout(Some(std::time::Duration::from_secs(10)))
    .build()
    .expect("valid CAS configuration");
```

`flow_timeout` 默认是 `None`，同步执行会忽略它。`attempt_timeout` 仍是单次异步 attempt
的限制；需要超时后继续时，与 `retry_on_timeout()` 一起使用。

## 2. 使用 retry 重导出

从 `qubit_cas::retry` 导入高级策略类型：

```rust
use qubit_cas::retry::{BackoffPolicy, RetryPolicy};
```

适配器固定依赖 `qubit-retry = 0.22.0`，因为 `CasRetryFailure` 保留了该版本的终态诊断契约。
如果应用也直接使用 `qubit-retry`，请将直接依赖和锁文件同步到 0.22。

## 3. 保留 retry 诊断

`CasError::into_parts()` 返回分类、retry 失败、CAS 上下文、最后一次业务失败和诊断信息。
`completion_callback_failures()` 借用保留的完成观察者失败。旧的
`into_parts_with_diagnostics()` 名称不应再使用。

`CasRetryFailure` 保留超时范围、取消、回调失败和基础设施细节。
`CasError::current()` 可能保留超时前观测到的状态快照。

## 4. 事件统计

`CasEvent::RetryRequested` 表示重试规则的意图，不代表新的 attempt 已准入。请使用
`CasExecutionReport::attempts_total()` 或成功结果中的尝试次数统计实际执行的 operation。
当 `max_attempts = 1` 时，一次冲突仍可能产生 `AttemptFailed`、`RetryRequested` 和
`ExecutionFinished`，但实际执行次数是一次。

## 5. 选择匹配的执行路径

异步调用只需要终态结果时使用 `execute_async_result`，需要报告时使用 `execute_async`。
两条路径都遵循 `flow_timeout`。对于不需要报告或业务重试的数值型、无分配状态迁移，
请改用独立的 `qubit-fast-cas` crate。

迁移完成后运行 `cargo test --all-features`，并在部署前检查超时和事件映射测试。
