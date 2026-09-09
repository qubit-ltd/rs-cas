# 迁移到 qubit-cas 0.13

[English](migration-0.13.md)。

本版本包含破坏性命名和依赖边界变更，请同时更新调用方；不保留 deprecated 兼容别名。

| 原 API | 新 API |
| --- | --- |
| CasStrategy::ContentionAdaptive | CasStrategy::ContentionBackoff |
| CasExecutor::contention_adaptive | CasExecutor::contention_backoff |
| CasBuilder::build_contention_adaptive | CasBuilder::build_contention_backoff |
| CONTENTION_ADAPTIVE_* | CONTENTION_BACKOFF_* |
| From<RetryError<...>> for CasError | 移除；执行入口直接返回 CAS 自有错误 |

次数、预算和退避统一在 CasExecutor::builder() 配置，没有 retry 重导出模块。
CasError::diagnostic() 保留终态基础设施详情，completion_diagnostics() 保留终态后的回调失败。
原始业务错误仍从 error() 读取。

克隆 executor 不再要求 T/E 实现 Clone。Result-only 与完整路径共享 retry 内核，省去报告成本，
但 owned Update 仍会为新快照分配 Arc。异步快照记账不再为共享槽分配 Arc。
硬 timeout 仍是合作式的，不会回滚外部副作用。

qubit-state-machine 0.8 使用本版本，可通过 StateMachineBuilder::cas_executor 注入 executor。
只启用 fast 的消费者仍不引入 CAS。配置顺序、快照与 hooks 详见[用户指南](user_guide.zh_CN.md)。
