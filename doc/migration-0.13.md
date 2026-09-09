# Migrating to qubit-cas 0.13

[中文版](migration-0.13.zh_CN.md).

This release makes breaking naming and dependency-boundary changes. Update all
callers together; there are no deprecated compatibility aliases.

| Previous API | Current API |
| --- | --- |
| CasStrategy::ContentionAdaptive | CasStrategy::ContentionBackoff |
| CasExecutor::contention_adaptive | CasExecutor::contention_backoff |
| CasBuilder::build_contention_adaptive | CasBuilder::build_contention_backoff |
| CONTENTION_ADAPTIVE_* | CONTENTION_BACKOFF_* |
| From<RetryError<...>> for CasError | Removed; execution returns CAS-owned errors |

Configure limits and backoff on CasExecutor::builder(). No retry re-export module
is available. CasError::diagnostic() retains terminal infrastructure details;
completion_diagnostics() retains post-terminal callback failures. Ordinary
business errors remain available through error().

Executor clones no longer require Clone on T or E. Result-only execution shares
the same retry engine as rich execution. It skips reports, but owned Update still
allocates a new Arc. Async snapshot bookkeeping no longer allocates an Arc slot.
Hard timeouts remain cooperative and do not roll back external effects.

qubit-state-machine 0.8 uses this release and accepts CasExecutor through
StateMachineBuilder::cas_executor. Fast-only consumers still do not enable CAS.

See the [user guide](user_guide.md) for configuration order, snapshots, and hooks.
