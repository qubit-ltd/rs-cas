# 迁移到 qubit-cas 0.15

[English](migration-0.15.md)。本文适用于 0.15。

0.15 有意引入破坏性的公共契约变更，不提供 deprecated 别名、兼容 feature 或转发桥梁。

## 默认业务错误

`CasExecutor<T>` 和 `CasBuilder<T>` 的默认业务错误参数改为 `CasBoxError`。它包装
`qubit_error::BoxError`，实现 `std::error::Error`，并通过 `source()` 保留被包装错误。
因此默认的终态 `CasError<T, CasBoxError>` 可以进入普通错误 source 链。

| 旧用法 | 0.15 用法 |
| --- | --- |
| 将默认错误推导或标注为 `BoxError` | 推导或标注为 `CasBoxError` |
| 通过泛型转换传入具体错误 | `CasBoxError::new(Box::new(error))` |
| 将默认终态错误匹配为 `CasError<T, BoxError>` | 匹配为 `CasError<T, CasBoxError>` |

这里有意不提供 `impl<E: Error> From<E> for CasBoxError`。它会与 Rust 的
`From<T> for T` 实现重叠。请在边界处显式装箱具体错误；需要集成时可使用包装器的
`as_inner()` 或 `into_inner()` 访问保留的 boxed error。

## Clone 约束

克隆 `CasDecision`、`CasAttemptFailure`、`CasSuccess`、`CasError` 或 `CasOutcome` 时，
不再要求 `T: Clone`，因为它们共享 `Arc<T>` 快照。若实际调用类型本身的 clone，按值保存的
output（`R`）或业务错误（`E`）仍需实现 `Clone`。这是约束放宽，调用方可以移除不必要的
`T: Clone` 限制，行为不变。

## listener panic 隔离

在 unwind 构建中，listener panic 隔离现在也能处理第一个 panic payload 在析构时再次 panic
的情况。已确定的 CAS 结果和最终报告仍可用，listener failure 会被保留为诊断。这不会捕获
operation panic，也不能用于 `panic = "abort"` 构建。

## 快照身份

CAS 用 `Arc::ptr_eq` 比较 `Arc` 身份，从不比较 `T` 的值。内容相同、来自不同分配的对象仍是
不同快照。调用 `update_arc` 时，应从 operation 当前观测推导替换值，不要把历史 `Arc` 当作
A-B-A 检测器。`Mutex`、`Cell`、原子字段等内部可变性不在身份检查范围内；如果这些修改也要
参与协调，应把状态建模为不可变并带显式版本。最后，`CasSuccess::current()` 是成功尝试发布或
观测到的快照，不能保证它在返回时仍是全局当前值。

配置与示例见[用户指南](user_guide.zh_CN.md)、[设计文档](design.zh_CN.md)和历史
[0.14 迁移说明](migration-0.14.zh_CN.md)。
