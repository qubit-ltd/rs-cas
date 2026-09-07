# Qubit CAS Design

This document describes the current implementation and its invariants. The
Chinese version is [`design.zh_CN.md`](design.zh_CN.md).

## Module boundaries

- `cas_decision` defines the typed operation result.
- `executor` owns builder validation, retry admission, sync/async execution,
  and projection to public success/error values.
- `event` defines execution context and lifecycle hooks.
- `report`, `observability`, and `strategy` own aggregation, event policy, and
  preset configuration respectively.
- `error` owns attempt failures, retry-terminal projection, and diagnostics.

The public API remains available from the crate root. Internal modules may be
split for maintenance without changing that surface.

## Two execution paths

Rich execution allocates a per-call report builder and can dispatch events and
alerts. Result-only execution avoids that construction and returns the same
business result and terminal error semantics. Both paths use the same state
snapshot and decision rules. A cached retry object may be reused only when no
per-call hooks or report state must be attached.

## Decision projection

| Operation decision | Retry layer | CAS result |
| --- | --- | --- |
| `Update(next, output)` and successful CAS | success | `CasSuccess` with previous/current state |
| `Finish(output)` | success | `CasSuccess` without a replacement state |
| `Retry(error)` | retryable failure | retry or `RetryExhausted` |
| CAS conflict | retryable failure | retry or `Conflict` |
| `Abort(error)` | terminal failure | `Abort` |
| timeout or retry infrastructure failure | terminal failure | mapped `CasErrorKind` plus `CasRetryFailure` |

The classifier must be shared by rich and result-only paths. Any change to the
table requires corresponding sync, async, and error mapping tests.

## Linearization and state ownership

An update linearizes at the successful compare-and-swap. A finish linearizes at
the observation of the snapshot. The returned `CasSuccess` owns or references
the snapshot associated with that point; it does not promise that the snapshot
remains globally current after return. The operation closure is replayable and
must not rely on a side effect occurring exactly once.

## Timeout state and precedence

`max_operation_elapsed` measures user-operation time. `max_total_elapsed` is a
soft continuation budget and can reject a future attempt after the current
admitted operation completes. `flow_timeout` is a hard asynchronous wall-clock
boundary. `attempt_timeout` bounds one async attempt. The error projection
retains timeout scope and, when available, the latest state observed before the
timeout. Synchronous execution ignores `flow_timeout`.

## Reports and hooks

The report builder accumulates attempts, conflicts, elapsed durations, terminal
classification, and state/output context. Events are emitted at lifecycle
boundaries and are distinct from retry admission callbacks. Hooks are scoped to
one execution. Listener panic behavior follows the configured CAS policy;
retry-control callback failures remain retry diagnostics. A hook must not be
assumed to run exactly once for an operation that is retried.

## Error projection and retry contract

`CasRetryFailure` mirrors the pinned `qubit-retry` 0.22 terminal details so
callers can inspect limits, timeout scope, cancellation, callback failures, and
infrastructure errors without losing context. The retry types are re-exported
through `qubit_cas::retry` to keep the application dependency boundary at
`qubit-cas`. Updating retry requires updating this adapter, its mapping tests,
lockfiles, and migration notes together.

## Thread safety and async invariants

The executor configuration is immutable after construction and can be reused
across threads. Per-execution report and hook state is isolated from other
executions. Async execution must not hold a synchronous mutex across an await;
hard flow timeout applies to both report-producing and result-only async paths.

## Performance assumptions and extension constraints

Result-only execution is the low-overhead path. Report-only observability avoids
per-attempt event construction; event streaming and alerts intentionally add
work proportional to attempts. New strategies must preserve the decision table,
budget semantics, and replay contract. New public types require Rustdoc and
behavior tests, and any new serialized or diagnostic shape needs a migration
note.
