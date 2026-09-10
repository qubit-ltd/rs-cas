# Migrating to qubit-cas 0.14

[中文版](migration-0.14.zh_CN.md). This page applies to 0.14.

Version 0.14 removes two public interfaces. Update callers together; no deprecated
aliases, compatibility features, or forwarding bridges are provided.

| Previous usage | 0.14 replacement |
| --- | --- |
| `hooks.on_alert(callback)` | `hooks.on_contention_alert(ContentionThresholds::default(), callback)` |
| Replace only an existing alert callback | Call `on_contention_alert` again with the thresholds to retain |
| `retry_policy().admission_limits().max_attempts().get()` | `max_attempts()` |
| Derive retries from the policy attempt limit | `max_retries()` |
| `retry_policy().admission_limits().operation_time_budget()` | `max_operation_elapsed()` |
| `retry_policy().admission_limits().total_time_budget()` | `max_total_elapsed()` |

Alert registration now always includes thresholds. Repeated registration replaces
both the threshold set and callback; all three thresholds must be met. The old
standalone `on_alert` registration could leave thresholds unset and emit nothing.

The four executor getters read installed configuration without allocating or
initializing lazy execution state. They describe overrides as well as presets.
`attempt_timeout()` and `flow_timeout()` remain available. Configure backoff on
the builder; no backoff getter or public retry policy object is exposed.

The `RetryScheduled` name is unchanged. Its documentation now matches execution:
the event follows accepted scheduling with a selected delay. It is not emitted
when the attempt limit is exhausted or a scheduling-time budget rejects the retry.
A later deadline or cancellation can still prevent another operation. Count
operations with terminal attempts, not scheduled events. This documentation
correction does not introduce a new event on the final attempt.

The standard qubit-state-machine 0.9 path now uses CAS 0.14 and its configuration
getters. Its default remains 16 attempts without soft budgets. A LatencyFirst
preset instead uses 100 attempts and time budgets. Fast-only builds retain their
separate qubit-fast-cas dependency and do not enable CAS.

The AtomicRef, Function/Consumer, and default BoxError collaboration boundaries
remain public. Snapshot publication, cooperative timeouts, listener panic
isolation in unwind builds, and result-only allocation contracts are unchanged.

See the [user guide](user_guide.md) for executable configuration, alert, and error
examples. Older migrations remain linked as historical records:
[0.11](migration-0.11.md), [0.12](migration-0.12.md), [0.13](migration-0.13.md).
