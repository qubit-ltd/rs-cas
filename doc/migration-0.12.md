# Migrating to qubit-cas 0.12

Version 0.12 exposes CAS-native terminal errors. Retry implementation types and observability configuration are no longer public. Configure attempts, budgets, delays, and flow timeouts directly on `CasExecutor::builder()`.

`CasErrorKind` distinguishes `ConflictExhausted`, `RetryExhausted`, `AttemptTimeout`, `FlowTimeout`, `OperationBudgetExceeded`, and `TotalBudgetExceeded`. Listener panics are isolated and reported through `CasExecutionReport::listener_failures()`.
