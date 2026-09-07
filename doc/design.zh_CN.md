# Qubit CAS 设计

本文描述当前实现及其不变量。英文版本见 [`design.md`](design.md)。

## 模块边界

- `cas_decision` 定义强类型 operation 结果。
- `executor` 负责 builder 校验、重试准入、同步/异步执行，以及投影为公开成功/错误值。
- `event` 定义执行上下文与生命周期 hooks。
- `report`、`observability` 和 `strategy` 分别负责聚合、事件策略和预设配置。
- `error` 负责 attempt 失败、retry 终态投影和诊断信息。

公共 API 仍从 crate 根导出。为便于维护，可以拆分内部模块，但不应改变这层接口。

## 两条执行路径

Rich 执行会为单次调用构造报告，并可分发事件和告警。result-only 执行跳过这些构造，
但返回相同的业务结果和终态错误语义。两条路径使用相同的状态快照和决策规则。只有在
不需要挂接单次 hooks 或报告状态时，才可以复用缓存的 retry 对象。

## 决策投影

| Operation 决策 | Retry 层 | CAS 结果 |
| --- | --- | --- |
| `Update(next, output)` 且 CAS 成功 | 成功 | 带旧/当前状态的 `CasSuccess` |
| `Finish(output)` | 成功 | 不替换状态的 `CasSuccess` |
| `Retry(error)` | 可重试失败 | 重试或 `RetryExhausted` |
| CAS 冲突 | 可重试失败 | 重试或 `Conflict` |
| `Abort(error)` | 终态失败 | `Abort` |
| 超时或 retry 基础设施失败 | 终态失败 | `CasErrorKind` 与 `CasRetryFailure` |

Rich 与 result-only 路径必须共享同一个分类器。修改此表时，必须同时补充同步、异步和
错误映射测试。

## 线性化与状态所有权

更新在线性化成功的 compare-and-swap 处生效；finish 在线程观测快照时线性化。返回的
`CasSuccess` 持有或引用与该时刻对应的快照，不保证返回后该快照仍是全局最新值。
operation closure 可被重放，不能依赖某个副作用恰好只发生一次。

## 超时状态与优先级

`max_operation_elapsed` 测量用户 operation 时间。`max_total_elapsed` 是软续试预算，
可以在当前准入 operation 完成后拒绝下一次尝试。`flow_timeout` 是异步硬墙钟边界，
`attempt_timeout` 限制一次异步 attempt。错误投影保留超时范围，并在可用时保留超时前
观测到的最新状态。同步执行忽略 `flow_timeout`。

## 报告与 hooks

报告 builder 累计尝试次数、冲突次数、耗时、终态分类和状态/输出上下文。事件在生命周期
边界发出，与 retry 准入回调相互独立。Hooks 只属于一次执行。listener panic 行为遵循
CAS 配置的策略；retry 控制回调失败仍作为 retry 诊断保留。执行发生重试时，不能假设
某个 hook 只运行一次。

## 错误投影与 retry 契约

`CasRetryFailure` 镜像固定 `qubit-retry` 0.22 的终态细节，使调用方可以读取限额、超时范围、
取消、回调失败和基础设施错误而不丢失上下文。retry 类型通过 `qubit_cas::retry` 重导出，
把应用依赖边界保持在 `qubit-cas`。升级 retry 时必须同步更新适配器、映射测试、锁文件和迁移说明。

## 线程安全与异步不变量

执行器配置构造后不可变，可以跨线程复用。每次执行的报告和 hook 状态与其他执行隔离。
异步执行不能在 await 期间持有同步 mutex；硬流程超时同时适用于 rich 和 result-only
异步路径。

## 性能假设与扩展约束

result-only 是低开销路径。Report-only 避免每次 attempt 构造事件；事件流和告警会按尝试
次数增加工作量。新增策略必须保持决策表、预算语义和可重放契约。新增公开类型必须补充
Rustdoc 与行为测试；新增序列化或诊断结构时还需要迁移说明。
