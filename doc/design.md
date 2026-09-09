# Qubit CAS Design

This document describes qubit-cas 0.13. [中文版](design.zh_CN.md).

## Responsibilities

`CasDecision` represents update, finish, retry, and abort. `CasExecutor` adapts
those decisions to the single Retry/TokioRetry control engine. The builder owns
validated settings; the executor caches a result-only retry configuration in a
shared OnceLock. Cloning configuration never requires T or E to implement Clone.

The CAS adapter owns atomic snapshot publication and CAS-specific termination.
The retry dependency owns admission, backoff, clocks, and timeout precedence.
There is no separate immediate loop and no duplicate backoff-dispatch flag.

## Publication invariants

Update linearizes at successful compare_set. Returned previous/current snapshots
and output belong to that successful attempt. Conflicting attempts discard their
output and reload state before recomputing. A same-value replacement remains an
Update. Finish linearizes at its snapshot read and does not revalidate it.

The snapshot is not promised to remain globally current after return. Operations
must be replayable. External effects require separate idempotency or placement
after success; CAS does not provide a transaction across resources.

## Execution and observation

Result-only calls skip report construction and hook dispatch. Rich execution
creates a per-execution report and retry observers. Both use the same decision
projection, retry classifier, and terminal mapper. Hooks are per call and never
run while holding the report mutex. Listener panics are caught with unwinding
enabled and appended to the final returned report. Operation panics propagate.

Finished-event and alert reports are snapshots taken before their callbacks.
The final returned report additionally includes failures from those callbacks.
No global event order is imposed across executions. Internal observers do not
register completion callbacks; successful retry completion diagnostics must
remain empty, checked by a debug assertion at the projection boundary.

## Budget and cancellation boundaries

Operation and total elapsed budgets prevent later attempts; they never revoke
an admitted success. Attempt time includes CAS adapter work. Flow timeout is an
independent cooperative async deadline spanning attempts and backoff, excluding
the complete cost of start/finish hooks. Synchronous execution ignores hard
attempt and flow timeouts. Blocking operation code cannot be forcibly interrupted.

Async operations borrow a stack-owned Mutex snapshot slot only when a timeout
is configured. No guard crosses await. Without timeouts, the slot is not locked
and no extra snapshot reference is retained for diagnostics. Finalization receives
an owned optional Arc rather than the slot. Dropping the future drops its
in-flight operation without rolling back committed or external effects.

Attempt timeouts retain the attempt's original snapshot. Backoff timeouts retain
the last failure. A flow stopped before its first attempt may have no snapshot.
Finalization never loads a new value to attach to an older failure.

## Error boundary

CasErrorKind is a compact classification, CasTermination states why execution
ended, and last_failure retains the last business/CAS attempt. These are
independent: a budget or flow timeout takes precedence over the last business
error. CasDiagnostic retains a CAS-owned category and full infrastructure text;
completion_diagnostics retains post-terminal callback failures in order.
Message text is not a stable parsing protocol. Public APIs do not require retry
implementation types and provide no From<RetryError> compatibility bridge.

## Presets and downstream use

ContentionBackoff is fixed exponential backoff with jitter, not an adaptive
controller. Strategy setters replace attempts, soft budgets, and backoff, but
preserve async timeout configuration. Subsequent setters override individual
fields. A plain builder defaults to five attempts; LatencyFirst is a different
preset with 100 attempts and explicit time budgets.

The standard qubit-state-machine 0.8 builder accepts a configured CasExecutor.
Its success callback runs only after a committed transition. The fast variant
and qubit-progress retain their separate qubit-fast-cas implementation.

## Performance and validation

Warm result-only finish bookkeeping avoids heap allocation. Owned Update still
allocates the replacement Arc; report construction, deadlines, and user work can
add allocations. Allocation tests distinguish constructor, first-use cache,
warm execution, owned/preallocated updates, and async timeout configurations.

The contention benchmark measures 1/2/4/8 writers, all three presets, successful
throughput, conflict counts, failed calls, and p50/p95/p99 latency including failed
calls. It does not hide exhausted calls behind unbounded retries. Absolute
latency is machine-specific and not enforced as a shared-CI timing threshold.

See the [user guide](user_guide.md) and [migration note](migration-0.13.md).
