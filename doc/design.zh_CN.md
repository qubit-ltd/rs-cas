# Qubit CAS 设计

本文描述 qubit-cas 0.13。[English](design.md)。

## 职责划分

CasDecision 表达更新、完成、重试和终止。CasExecutor 将决策适配到唯一的
Retry/TokioRetry 执行内核。Builder 负责校验配置，executor 通过共享 OnceLock
缓存仅返回结果的 retry 配置；克隆 executor 不要求 T 或 E 实现 Clone。

CAS 层负责快照提交与领域终态，retry 层负责准入、退避、时钟和超时优先级。
实现不再包含独立 immediate 循环或重复的退避分派标记。

## 提交不变量

Update 在 compare_set 成功时线性化，返回的新旧快照及 output 均来自该次成功尝试。
冲突尝试的 output 被丢弃，下一次尝试重新读取状态并计算。值相等的新快照提交仍属于 Update。
Finish 的线性化点是其使用的快照读取，不会重新验证当前状态。

返回快照不保证在返回后仍是全局最新值。Operation 必须可重放；外部副作用需要独立的幂等机制，
或放在成功提交后执行。CAS 不提供跨资源事务。

## 执行与观测

Result-only 路径不构造报告、不发送 hook 事件。完整路径创建每次执行独立的报告和 retry observer。
两条路径共享决策提交、重试分类及错误映射。Hooks 属于单次执行，不在持有报告锁时调用。
在 unwind 构建中，listener panic 被捕获并记录到返回报告；operation 的 panic 向外传播。

完成事件和告警收到的是调用自身回调之前的报告快照；调用者收到的最终报告还包括这些回调的失败。
不同执行之间不保证全局事件顺序。内部 observer 不注册 completion callbacks，
成功路径的 retry completion diagnostics 应为空，投影边界通过 debug_assert 检查这个不变量。

## 预算与取消边界

操作与总耗时软预算只阻止后续尝试，不撤销已准入的成功结果。操作耗时包含 CAS 适配工作。
Flow timeout 是独立的合作式异步 deadline，覆盖尝试及退避，但不覆盖开始/结束 hooks 的全部成本。
同步入口忽略两种硬 timeout，阻塞的 operation 也不能被强制中断。

异步入口只有配置 timeout 时才借用栈内 Mutex 快照槽，不持锁跨 await。
未配置 timeout 时既不加这把锁，也不为诊断额外保留快照引用。Finalization 接收
Option<Arc<T>>，不再接收槽本身。丢弃 future 会丢弃进行中的 operation，不回滚已提交状态或外部效果。

尝试超时保留该次开始时的快照；退避超时保留已有失败；第一次尝试前停止可以没有快照。
终止时不会重新读取状态，避免把新快照和旧错误拼在一起。

## 错误边界

CasErrorKind 提供简洁分类，CasTermination 描述停止原因，last_failure 保留最后一次业务/CAS 失败。
三者相互独立：预算超限或流程超时优先于最后一次业务错误。CasDiagnostic 保留 CAS 自有类别和
完整基础设施文本；completion_diagnostics 按顺序保留终态冻结后的回调失败。
诊断文本不作为稳定的解析协议。公共 API 不要求 retry 类型，也没有 From<RetryError> 兼容桥梁。

## 预设与下游

ContentionBackoff 是固定指数退避加 jitter，不会学习竞争率。Strategy 覆盖次数、软预算和退避，
保留异步 timeout 配置；后续 setter 覆盖单个字段。普通 builder 默认五次尝试，
LatencyFirst 则是 100 次尝试并带时间预算的另一套预设。

qubit-state-machine 0.8 的标准 builder 接受配置好的 CasExecutor，成功回调仍只在提交后执行。
Fast 版和 qubit-progress 继续使用独立的 qubit-fast-cas。

## 性能与验证

预热后的 result-only Finish 记账不分配堆内存；owned Update 仍需为快照分配 Arc，
报告、deadline 和用户操作还可能增加分配。分配测试区分构造、首次缓存初始化、热执行、
owned/预分配更新和有无异步 timeout。

竞争基准使用 1/2/4/8 个写者、三种预设，记录成功吞吐、冲突数、失败调用和包含失败的
p50/p95/p99 延迟；不会通过无限重试隐藏耗尽的调用。绝对耗时取决于机器，不作为共享 CI 的时间门槛。

参阅[用户指南](user_guide.zh_CN.md)和[迁移说明](migration-0.13.zh_CN.md)。
