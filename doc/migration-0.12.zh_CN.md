# qubit-cas 0.12 迁移指南

> 历史记录：本文仅描述 0.12 版本。当前 API 请参阅 [0.15 迁移说明](migration-0.15.zh_CN.md)。

0.12 版本改为暴露 CAS 原生终止错误，重试实现类型和可观测性配置不再公开。请直接使用 `CasExecutor::builder()` 配置尝试次数、预算、延迟和流程超时。

`CasErrorKind` 明确区分冲突耗尽、重试耗尽、单次尝试超时、流程超时、操作预算超限和总预算超限。监听器 panic 会被隔离，并通过 `CasExecutionReport::listener_failures()` 报告。
