# Qubit CAS

面向 Rust 的强类型 CAS 执行器。

`qubit-cas` 基于 `qubit-atomic`、`qubit-function` 和 `qubit-retry` 提供：

- 强类型 CAS 决策：`update`、`finish`、`retry`、`abort`
- 基于退避策略的 CAS 冲突重试
- 同步与异步执行 API
- 面向单次执行的成功、重试、中止 hooks
- 高并发、低延迟、高可靠三个预置构建配置

## 说明

该 crate 的目录结构、测试组织和 CI 脚本风格对齐 `rs-retry`。
