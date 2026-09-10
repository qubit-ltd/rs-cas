# 迁移到 qubit-cas 0.14

[English](migration-0.14.md)。

> 历史记录：本文仅描述 0.14 版本。当前 API 请参阅 [0.15 迁移说明](migration-0.15.zh_CN.md)。

0.14 删除两个公共接口，请同步迁移调用方。本版本不提供 deprecated 别名、兼容 feature
或转发桥梁。

| 原用法 | 0.14 用法 |
| --- | --- |
| `hooks.on_alert(callback)` | `hooks.on_contention_alert(ContentionThresholds::default(), callback)` |
| 只替换已有告警回调 | 再次调用 `on_contention_alert`，显式传入要保留的阈值 |
| `retry_policy().admission_limits().max_attempts().get()` | `max_attempts()` |
| 从策略次数推导重试次数 | `max_retries()` |
| `retry_policy().admission_limits().operation_time_budget()` | `max_operation_elapsed()` |
| `retry_policy().admission_limits().total_time_budget()` | `max_total_elapsed()` |

告警注册必须同时提供阈值。再次注册会把阈值和回调一起替换；三个阈值全部满足才触发告警。
旧的独立 `on_alert` 可能没有设置阈值，导致注册回调后仍然不发告警。

四个 executor getter 读取实际生效的配置，包括对预设的覆盖；读取不分配内存，也不初始化
惰性执行状态。`attempt_timeout()` 和 `flow_timeout()` 继续保留。退避仍在 builder 上配置，
不提供退避 getter，也不再公开 retry 策略对象。

`RetryScheduled` 名称不变，但文档已按实际行为修正：通过调度检查并选定延迟后才发送。
最后一次尝试已耗尽，或调度时预算不足，均不发送该事件；发送后也可能因 deadline 或取消
而无法开始下一次操作。实际执行次数应读取终态 attempts，不能统计调度事件来推算。
这次修正文档不会在最终尝试后增加新事件。

标准状态机 qubit-state-machine 0.9 改用 CAS 0.14 和新的配置 getter，默认仍是 16 次尝试、
无软预算。显式选择 LatencyFirst 才是 100 次并带时间预算。仅启用 fast 的构建继续使用
独立的 qubit-fast-cas，不引入 CAS。

AtomicRef、Function/Consumer 和默认 BoxError 仍是公共协作边界。快照提交、合作式超时、
unwind 构建中的 listener panic 隔离，以及 result-only 分配合同均保持。

配置、告警和错误处理的完整示例见[用户指南](user_guide.zh_CN.md)。历史迁移记录继续保留：
[0.11](migration-0.11.zh_CN.md)、[0.12](migration-0.12.zh_CN.md)、[0.13](migration-0.13.zh_CN.md)。
