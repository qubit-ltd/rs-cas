// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS executor implementation.

use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use qubit_atomic::AtomicRef;
use qubit_error::BoxError;
use qubit_function::Consumer;
use qubit_function::Function;
use qubit_retry::AttemptFailure;
use qubit_retry::Retry;
use qubit_retry::RetryContext;
use qubit_retry::RetryDecision;
use qubit_retry::RetryError;
use qubit_retry::RetryPolicy;
use qubit_retry::RetrySuccess;
use qubit_retry::RetryTimeoutScope;

use super::cas_builder::CasBuilder;
use super::internal::AttemptSuccess;
use super::internal::AttemptTimeoutAction;
use super::internal::CasReportFinishContext;
use crate::cas_decision::CasDecision;
use crate::cas_outcome::CasOutcome;
use crate::cas_success::CasSuccess;
use crate::error::CasAttemptFailure;
use crate::error::CasError;
use crate::error::CasErrorKind;
use crate::event::CasContext;
use crate::event::CasEvent;
use crate::event::CasHooks;
use crate::observability::CasAlert;
use crate::observability::CasObservabilityConfig;
use crate::observability::CasObservabilityMode;
use crate::observability::ListenerPanicPolicy;
use crate::report::CasExecutionOutcome;
use crate::report::CasExecutionReport;
use crate::report::CasReportBuilder;
use crate::strategy::CasStrategy;

/// Executor for retry-aware compare-and-swap workflows.
#[derive(Clone)]
pub struct CasExecutor<T, E = BoxError> {
    /// Pure policy used by the retry facades.
    policy: RetryPolicy,
    /// Optional hard timeout applied to each async attempt.
    attempt_timeout: Option<std::time::Duration>,
    /// Action selected after a configured attempt timeout.
    attempt_timeout_action: AttemptTimeoutAction,
    /// Observability settings shared by executions.
    observability: CasObservabilityConfig,
    /// Result-only retry definition initialized on its first use.
    result_retry: Arc<OnceLock<Retry<CasAttemptFailure<T, E>>>>,
    /// Marker preserving `T` and `E`.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> std::fmt::Debug for CasExecutor<T, E> {
    /// Formats the reusable executor configuration without forcing retry rules
    /// or application types to implement [`std::fmt::Debug`].
    ///
    /// # Parameters
    /// - `f`: Formatter provided by the standard formatting machinery.
    ///
    /// # Returns
    /// `fmt::Result` from the formatter.
    ///
    /// # Errors
    /// Returns a formatting error if the formatter fails.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CasExecutor")
            .field("policy", &self.policy)
            .field("attempt_timeout", &self.attempt_timeout)
            .field("attempt_timeout_action", &self.attempt_timeout_action)
            .field("observability", &self.observability)
            .finish()
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Creates a CAS builder.
    ///
    /// # Returns
    /// A builder configured with default retry settings.
    #[inline(always)]
    pub fn builder() -> CasBuilder<T, E> {
        CasBuilder::new()
    }

    /// Creates an executor from a pure retry policy.
    ///
    /// # Parameters
    /// - `policy`: Retry continuation and backoff policy to install.
    ///
    /// # Returns
    /// A configured executor using the supplied retry policy.
    pub fn from_policy(policy: RetryPolicy) -> Self {
        Self::builder()
            .policy(policy)
            .build()
            .expect("an existing retry policy is already validated")
    }

    /// Creates an executor tuned for low-latency workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    pub fn latency_first() -> Self {
        Self::builder()
            .build_latency_first()
            .expect("latency-first CAS strategy must be valid")
    }

    /// Creates an executor tuned for hot-contention workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    pub fn contention_adaptive() -> Self {
        Self::builder()
            .build_contention_adaptive()
            .expect("contention-adaptive CAS strategy must be valid")
    }

    /// Creates an executor tuned for reliability-first workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    pub fn reliability_first() -> Self {
        Self::builder()
            .build_reliability_first()
            .expect("reliability-first CAS strategy must be valid")
    }

    /// Creates an executor from a built-in strategy.
    ///
    /// # Parameters
    /// - `strategy`: Strategy to install.
    ///
    /// # Returns
    /// A configured executor. Built-in strategies are always valid.
    pub fn with_strategy(strategy: CasStrategy) -> Self {
        Self::builder()
            .strategy(strategy)
            .build()
            .expect("built-in CAS strategy must be valid")
    }

    /// Creates one executor from validated parts.
    ///
    /// # Parameters
    /// - `policy`: Validated retry policy.
    /// - `attempt_timeout`: Optional hard timeout for async attempts.
    /// - `attempt_timeout_action`: Action selected for attempt timeouts.
    /// - `observability`: Observability settings shared by executions.
    ///
    /// # Returns
    /// A configured executor.
    #[inline]
    pub(crate) fn new(
        policy: RetryPolicy,
        attempt_timeout: Option<std::time::Duration>,
        attempt_timeout_action: AttemptTimeoutAction,
        observability: CasObservabilityConfig,
    ) -> Self {
        Self {
            policy,
            attempt_timeout,
            attempt_timeout_action,
            observability,
            result_retry: Arc::new(OnceLock::new()),
            marker: PhantomData,
        }
    }

    /// Returns the immutable retry policy used by this executor.
    ///
    /// # Returns
    /// Shared retry policy.
    #[inline(always)]
    pub fn policy(&self) -> &RetryPolicy {
        &self.policy
    }

    /// Returns the optional hard timeout for each async attempt.
    #[inline(always)]
    pub fn attempt_timeout(&self) -> Option<std::time::Duration> {
        self.attempt_timeout
    }

    /// Returns the hard wall-clock boundary for asynchronous retry flows.
    ///
    /// # Returns
    /// The end-to-end total elapsed budget, when configured. The operation
    /// budget controls whether another attempt may start and never cancels an
    /// admitted attempt.
    #[cfg(feature = "tokio")]
    #[inline(always)]
    fn flow_timeout(&self) -> Option<std::time::Duration> {
        self.policy.limits().max_total_elapsed()
    }

    /// Returns observability settings used by this executor.
    ///
    /// # Returns
    /// Shared observability configuration.
    #[inline(always)]
    pub fn observability(&self) -> &CasObservabilityConfig {
        &self.observability
    }

    /// Executes one synchronous CAS operation.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and
    ///   returns a CAS decision.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Blocking
    /// Configured retry delays block the calling thread until execution ends.
    pub fn execute<R, O>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        self.execute_with_hooks(state, operation, CasHooks::new())
    }

    /// Executes one synchronous CAS operation without constructing a report.
    ///
    /// This path preserves retry, success, and terminal error semantics while
    /// skipping report accumulation and lifecycle hook dispatch. Use
    /// [`Self::execute`] when the caller needs execution metrics.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and
    ///   returns a CAS decision.
    ///
    /// # Returns
    /// The terminal CAS success or error without an execution report.
    ///
    /// # Blocking
    /// Configured retry delays block the calling thread until execution ends.
    pub fn execute_result<R, O>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        let attempt = self
            .result_retry()
            .sync()
            .run(|| self.run_sync_attempt(state, &operation));
        match attempt {
            Ok(success) => {
                let (success, context) = success.into_parts();
                Ok(self.enrich_success(success, context))
            }
            Err(error) => Err(CasError::new(error, None)),
        }
    }

    /// Executes one synchronous CAS operation with lifecycle hooks.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and
    ///   returns a CAS decision.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Blocking
    /// Configured retry delays block the calling thread until execution ends.
    ///
    /// # Panics
    /// With [`ListenerPanicPolicy::Propagate`], panics from outer
    /// `ExecutionStarted`/`ExecutionFinished` listeners and alert listeners
    /// unwind through this call. Panics from retry-owned `AttemptFailed` and
    /// `RetryRequested` listeners instead return a
    /// [`crate::CasRetryFailure::CallbackFailed`] terminal error.
    /// [`ListenerPanicPolicy::Isolate`] catches every listener
    /// panic at dispatch and allows execution to continue.
    pub fn execute_with_hooks<R, O>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
        hooks: CasHooks,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(&hooks, Arc::clone(&report_builder));
        let attempt = retry
            .sync()
            .run(|| self.run_sync_attempt(state, &operation));
        self.finish_execution(attempt, hooks, None, report_builder)
    }

    /// Executes one asynchronous CAS operation.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    #[cfg(feature = "tokio")]
    pub async fn execute_async<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        self.execute_async_with_hooks(state, operation, CasHooks::new())
            .await
    }

    /// Executes one asynchronous CAS operation without constructing a report.
    ///
    /// This path preserves retry, success, timeout, and terminal error
    /// semantics while skipping report accumulation and lifecycle hook
    /// dispatch. Use [`Self::execute_async`] when the caller needs execution
    /// metrics.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// The terminal CAS success or error without an execution report.
    ///
    /// # Cancellation
    /// Cancelling the returned future cancels the in-flight operation future.
    /// Operations must therefore remain safe to retry or cancel.
    #[cfg(feature = "tokio")]
    pub async fn execute_async_result<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let attempt_snapshot = Arc::new(Mutex::new(None));
        let attempt_snapshot_for_attempt = Arc::clone(&attempt_snapshot);
        let mut async_retry = self.result_retry().asynchronous();
        if let Some(timeout) = self.attempt_timeout {
            async_retry = async_retry.attempt_timeout(timeout);
        }
        if let Some(timeout) = self.flow_timeout() {
            async_retry = async_retry.flow_timeout(timeout);
        }
        let attempt = async_retry
            .run(|| {
                self.run_async_attempt(
                    state,
                    &operation,
                    Arc::clone(&attempt_snapshot_for_attempt),
                )
            })
            .await;
        match attempt {
            Ok(success) => {
                let (success, context) = success.into_parts();
                Ok(self.enrich_success(success, context))
            }
            Err(error) => {
                let timeout_current = attempt_snapshot
                    .lock()
                    .expect("CAS attempt snapshot slot should be lockable")
                    .clone();
                Err(CasError::new(error, timeout_current))
            }
        }
    }

    /// Executes one asynchronous CAS operation with lifecycle hooks.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Panics
    /// With [`ListenerPanicPolicy::Propagate`], panics from outer
    /// `ExecutionStarted`/`ExecutionFinished` listeners and alert listeners
    /// unwind while this future is polled. Panics from retry-owned
    /// `AttemptFailed` and `RetryRequested` listeners instead return a
    /// [`crate::CasRetryFailure::CallbackFailed`] terminal error.
    /// [`ListenerPanicPolicy::Isolate`] catches every listener panic at
    /// dispatch and allows execution to continue.
    #[cfg(feature = "tokio")]
    pub async fn execute_async_with_hooks<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
        hooks: CasHooks,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let attempt_snapshot = Arc::new(Mutex::new(None));
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(&hooks, Arc::clone(&report_builder));
        let attempt_snapshot_for_attempt = Arc::clone(&attempt_snapshot);
        let mut async_retry = retry.asynchronous();
        if let Some(timeout) = self.attempt_timeout {
            async_retry = async_retry.attempt_timeout(timeout);
        }
        if let Some(timeout) = self.flow_timeout() {
            async_retry = async_retry.flow_timeout(timeout);
        }
        let attempt = async_retry
            .run(|| {
                self.run_async_attempt(
                    state,
                    &operation,
                    Arc::clone(&attempt_snapshot_for_attempt),
                )
            })
            .await;
        self.finish_execution(
            attempt,
            hooks,
            Some(attempt_snapshot),
            report_builder,
        )
    }

    /// Builds one retry policy for a single CAS execution.
    ///
    /// # Parameters
    /// - `hooks`: Hook registrations for the current execution.
    /// # Returns
    /// A retry policy configured for one CAS execution.
    fn build_retry(
        &self,
        hooks: &CasHooks,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> Retry<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        let event_hook = hooks.event_hook();
        let attempt_timeout_action = self.attempt_timeout_action;
        let observability = self.observability.clone();
        let observer_event_hook = event_hook.clone();
        let observer_observability = observability.clone();
        let observer_report_builder = Arc::clone(&report_builder);

        Retry::<CasAttemptFailure<T, E>>::builder(self.policy.clone())
            .observer(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>,
                      context: &RetryContext| {
                    let kind = match failure {
                        AttemptFailure::Error(failure) => {
                            let mut report = observer_report_builder
                                .lock()
                                .expect("CAS report builder should be lockable");
                            match failure {
                                CasAttemptFailure::Conflict { .. } => report.record_conflict(),
                                CasAttemptFailure::Retry { .. } => report.record_retry_error(),
                                CasAttemptFailure::Abort { .. } => report.record_abort(),
                                CasAttemptFailure::Timeout { .. } => report.record_timeout(),
                            }
                            Some(Self::failure_kind(failure))
                        }
                        AttemptFailure::TimedOut { .. } => {
                            observer_report_builder
                                .lock()
                                .expect("CAS report builder should be lockable")
                                .record_timeout();
                            Some(crate::error::CasAttemptFailureKind::Timeout)
                        }
                        AttemptFailure::Panicked { .. } => None,
                        _ => None,
                    };
                    if let Some(kind) = kind
                        && Self::should_emit_events(
                            &observer_observability,
                            &observer_event_hook,
                        )
                    {
                        Self::dispatch_event(
                            &observer_observability,
                            observer_event_hook
                                .as_ref()
                                .expect("event hook should exist when events are emitted"),
                            CasEvent::AttemptFailed {
                                context: CasContext::new(context),
                                kind,
                            },
                        );
                    }
                },
            )
            .rule(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, context: &RetryContext| {
                    let failure = match failure {
                        AttemptFailure::Error(failure) => failure,
                        AttemptFailure::TimedOut { scope } => {
                            let cas_context = CasContext::new(context);
                            if *scope == RetryTimeoutScope::Attempt
                                && attempt_timeout_action == AttemptTimeoutAction::Retry
                            {
                                if Self::should_emit_events(&observability, &event_hook) {
                                    Self::dispatch_event(
                                        &observability,
                                        event_hook.as_ref().expect(
                                            "event hook should exist when events are emitted",
                                        ),
                                        CasEvent::RetryRequested {
                                            context: cas_context,
                                        },
                                    );
                                }
                                return RetryDecision::Retry;
                            }
                            return RetryDecision::UseDefault;
                        }
                        AttemptFailure::Panicked { .. } => {
                            return RetryDecision::UseDefault;
                        }
                        _ => return RetryDecision::UseDefault,
                    };
                    let cas_context = CasContext::new(context);
                    match failure {
                        CasAttemptFailure::Conflict { .. } | CasAttemptFailure::Retry { .. } => {
                            if Self::should_emit_events(&observability, &event_hook) {
                                Self::dispatch_event(
                                    &observability,
                                    event_hook
                                        .as_ref()
                                        .expect("event hook should exist when events are emitted"),
                                    CasEvent::RetryRequested {
                                        context: cas_context,
                                    },
                                );
                            }
                            RetryDecision::Retry
                        }
                        CasAttemptFailure::Abort { .. } => RetryDecision::Abort,
                        CasAttemptFailure::Timeout { .. } => RetryDecision::UseDefault,
                    }
                },
            )
            .build()
    }

    /// Returns the cached retry definition used by result-only execution.
    ///
    /// # Returns
    /// An immutable retry definition initialized exactly once per executor.
    fn result_retry(&self) -> &Retry<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        self.result_retry.get_or_init(|| {
            let attempt_timeout_action = self.attempt_timeout_action;
            Retry::<CasAttemptFailure<T, E>>::builder(self.policy.clone())
                .rule(
                    move |failure: &AttemptFailure<CasAttemptFailure<T, E>>,
                          _context: &RetryContext| {
                        match failure {
                            AttemptFailure::Error(
                                CasAttemptFailure::Conflict { .. },
                            )
                            | AttemptFailure::Error(
                                CasAttemptFailure::Retry { .. },
                            ) => RetryDecision::Retry,
                            AttemptFailure::Error(
                                CasAttemptFailure::Abort { .. },
                            ) => RetryDecision::Abort,
                            AttemptFailure::Error(
                                CasAttemptFailure::Timeout { .. },
                            ) => RetryDecision::UseDefault,
                            AttemptFailure::TimedOut {
                                scope: RetryTimeoutScope::Attempt,
                            } if attempt_timeout_action
                                == AttemptTimeoutAction::Retry =>
                            {
                                RetryDecision::Retry
                            }
                            AttemptFailure::TimedOut { .. }
                            | AttemptFailure::Panicked { .. } => {
                                RetryDecision::UseDefault
                            }
                            _ => RetryDecision::UseDefault,
                        }
                    },
                )
                .build()
        })
    }

    /// Runs one synchronous CAS attempt.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation over the current state snapshot.
    ///
    /// # Returns
    /// An attempt success or one attempt failure.
    fn run_sync_attempt<R, O>(
        &self,
        state: &AtomicRef<T>,
        operation: &O,
    ) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
    where
        O: Function<T, CasDecision<T, R, E>>,
    {
        let current = state.load();
        match operation.apply(current.as_ref()) {
            CasDecision::Update { next, output } => {
                match state.compare_set(&current, Arc::clone(&next)) {
                    Ok(()) => Ok(AttemptSuccess::Updated {
                        previous: current,
                        current: next,
                        output,
                    }),
                    Err(actual) => Err(CasAttemptFailure::conflict(actual)),
                }
            }
            CasDecision::Finish { output } => {
                Ok(AttemptSuccess::Finished { current, output })
            }
            CasDecision::Retry(error) => {
                Err(CasAttemptFailure::retry(current, error))
            }
            CasDecision::Abort(error) => {
                Err(CasAttemptFailure::abort(current, error))
            }
        }
    }

    /// Runs one asynchronous CAS attempt.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory over one state snapshot.
    ///
    /// # Returns
    /// An attempt success or one attempt failure.
    #[cfg(feature = "tokio")]
    async fn run_async_attempt<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: &O,
        attempt_snapshot: Arc<Mutex<Option<Arc<T>>>>,
    ) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
    where
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let current = state.load();
        *attempt_snapshot
            .lock()
            .expect("CAS attempt snapshot slot should be lockable") =
            Some(Arc::clone(&current));
        let decision = operation(Arc::clone(&current)).await;

        match decision {
            CasDecision::Update { next, output } => {
                match state.compare_set(&current, Arc::clone(&next)) {
                    Ok(()) => Ok(AttemptSuccess::Updated {
                        previous: current,
                        current: next,
                        output,
                    }),
                    Err(actual) => Err(CasAttemptFailure::conflict(actual)),
                }
            }
            CasDecision::Finish { output } => {
                Ok(AttemptSuccess::Finished { current, output })
            }
            CasDecision::Retry(error) => {
                Err(CasAttemptFailure::retry(current, error))
            }
            CasDecision::Abort(error) => {
                Err(CasAttemptFailure::abort(current, error))
            }
        }
    }

    /// Finalizes one retry execution into the public CAS result type.
    ///
    /// # Parameters
    /// - `attempt`: Retry-layer terminal success or error.
    /// - `hooks`: Hook registrations for the current execution.
    /// - `attempt_snapshot`: Last async operation snapshot, when an async
    ///   execution needs to preserve it for a timeout error.
    ///
    /// # Returns
    /// Public CAS success or error.
    fn finish_execution<R>(
        &self,
        attempt: Result<
            RetrySuccess<AttemptSuccess<T, R>>,
            RetryError<CasAttemptFailure<T, E>>,
        >,
        hooks: CasHooks,
        attempt_snapshot: Option<Arc<Mutex<Option<Arc<T>>>>>,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
    {
        match attempt {
            Ok(success) => {
                let (success, context) = success.into_parts();
                let attempts_total = context.attempts();
                let max_attempts = context.max_attempts();
                let max_operation_elapsed = context.max_operation_elapsed();
                let max_total_elapsed = context.max_total_elapsed();
                let outcome = match success {
                    AttemptSuccess::Updated { .. } => {
                        CasExecutionOutcome::SuccessUpdated
                    }
                    AttemptSuccess::Finished { .. } => {
                        CasExecutionOutcome::SuccessFinished
                    }
                };
                let success = self.enrich_success(success, context);
                let report = self.finish_report(
                    &hooks,
                    report_builder,
                    CasReportFinishContext::new(
                        attempts_total,
                        max_attempts,
                        max_operation_elapsed,
                        max_total_elapsed,
                        outcome,
                    ),
                );
                CasOutcome::new(Ok(success), report)
            }
            Err(error) => {
                let timeout_current = attempt_snapshot.and_then(|snapshot| {
                    snapshot
                        .lock()
                        .expect("CAS attempt snapshot slot should be lockable")
                        .clone()
                });
                let error = CasError::new(error, timeout_current);
                let context = error.context();
                let outcome = Self::error_outcome(error.kind());
                let report = self.finish_report(
                    &hooks,
                    report_builder,
                    CasReportFinishContext::new(
                        context.attempts(),
                        context.max_attempts(),
                        context.max_operation_elapsed(),
                        context.max_total_elapsed(),
                        outcome,
                    ),
                );
                CasOutcome::new(Err(error), report)
            }
        }
    }

    /// Enriches one attempt success with the final CAS context.
    ///
    /// # Parameters
    /// - `success`: Attempt success payload.
    /// - `context`: Retry success context captured by the retry layer.
    ///
    /// # Returns
    /// Public CAS success value with context attached.
    fn enrich_success<R>(
        &self,
        success: AttemptSuccess<T, R>,
        context: RetryContext,
    ) -> CasSuccess<T, R> {
        let context = CasContext::new(&context);
        match success {
            AttemptSuccess::Updated {
                previous,
                current,
                output,
            } => CasSuccess::updated(previous, current, output, context),
            AttemptSuccess::Finished { current, output } => {
                CasSuccess::finished(current, output, context)
            }
        }
    }

    /// Emits the execution-started event when event streaming is enabled.
    ///
    /// # Parameters
    /// - `hooks`: Per-execution hooks (checked for event hook presence).
    /// - `report_builder`: Used to obtain the start instant for the event.
    fn emit_started(
        &self,
        hooks: &CasHooks,
        report_builder: &Arc<Mutex<CasReportBuilder>>,
    ) where
        T: 'static,
        E: 'static,
    {
        if hooks.event_hook().is_none()
            || self.observability.mode() == CasObservabilityMode::ReportOnly
        {
            return;
        }
        let started_at = report_builder
            .lock()
            .expect("CAS report builder should be lockable")
            .started_at();
        let event_hook = hooks.event_hook();
        Self::dispatch_event(
            &self.observability,
            event_hook
                .as_ref()
                .expect("event hook should exist when events are emitted"),
            CasEvent::ExecutionStarted { started_at },
        );
    }

    /// Finishes and emits one execution report (and optional alert).
    ///
    /// Locks the report builder, finalizes the report, emits the
    /// `ExecutionFinished` event if enabled, and dispatches a contention alert
    /// if the mode and thresholds warrant it.
    ///
    /// # Parameters
    /// - `hooks`: Used for event and alert dispatching.
    /// - `report_builder`: Accumulator to finalize.
    /// - `ctx`: Retry limits and terminal outcome for the report.
    ///
    /// # Returns
    /// The finalized [`CasExecutionReport`].
    fn finish_report(
        &self,
        hooks: &CasHooks,
        report_builder: Arc<Mutex<CasReportBuilder>>,
        ctx: CasReportFinishContext,
    ) -> CasExecutionReport
    where
        T: 'static,
        E: 'static,
    {
        let report = report_builder
            .lock()
            .expect("CAS report builder should be lockable")
            .finish(
                ctx.attempts_total,
                ctx.max_attempts,
                ctx.max_operation_elapsed,
                ctx.max_total_elapsed,
                ctx.outcome,
            );
        let event_hook = hooks.event_hook();
        if Self::should_emit_events(&self.observability, &event_hook) {
            Self::dispatch_event(
                &self.observability,
                event_hook
                    .as_ref()
                    .expect("event hook should exist when events are emitted"),
                CasEvent::ExecutionFinished {
                    report: report.clone(),
                },
            );
        }
        let alert_hook = hooks.alert_hook();
        if self.observability.mode()
            == CasObservabilityMode::EventStreamWithAlert
            && let Some(thresholds) = self.observability.contention_thresholds()
            && alert_hook.is_some()
            && report.is_contention_hot(&thresholds)
        {
            Self::dispatch_alert(
                &self.observability,
                &alert_hook,
                CasAlert::contention(report.clone(), thresholds),
            );
        }
        report
    }

    /// Converts a terminal error kind into a report outcome.
    ///
    /// # Parameters
    /// - `kind`: The high-level [`CasErrorKind`].
    ///
    /// # Returns
    /// Corresponding [`CasExecutionOutcome`] variant for the report.
    #[inline]
    fn error_outcome(kind: CasErrorKind) -> CasExecutionOutcome {
        match kind {
            CasErrorKind::Abort => CasExecutionOutcome::ErrorAbort,
            CasErrorKind::Conflict => {
                CasExecutionOutcome::ErrorConflictExhausted
            }
            CasErrorKind::RetryExhausted => {
                CasExecutionOutcome::ErrorRetryExhausted
            }
            CasErrorKind::AttemptTimeout => {
                CasExecutionOutcome::ErrorAttemptTimeout
            }
            CasErrorKind::RetryInfrastructure => {
                CasExecutionOutcome::ErrorRetryInfrastructure
            }
            CasErrorKind::MaxOperationElapsedExceeded => {
                CasExecutionOutcome::ErrorMaxOperationElapsedExceeded
            }
            CasErrorKind::MaxTotalElapsedExceeded => {
                CasExecutionOutcome::ErrorMaxTotalElapsedExceeded
            }
        }
    }

    /// Converts one attempt failure into its lightweight event kind.
    ///
    /// # Parameters
    /// - `failure`: The [`CasAttemptFailure`] to classify.
    ///
    /// # Returns
    /// The [`CasAttemptFailureKind`] for event emission.
    #[inline]
    fn failure_kind(
        failure: &CasAttemptFailure<T, E>,
    ) -> crate::error::CasAttemptFailureKind {
        failure.kind()
    }

    /// Dispatches one lifecycle event to a registered hook.
    ///
    /// # Parameters
    /// - `observability`: Configuration controlling listener panic behavior.
    /// - `hook`: Listener that receives the event.
    /// - `event`: Lifecycle event to dispatch.
    ///
    /// # Panics
    /// With [`ListenerPanicPolicy::Propagate`], exposes a listener panic to the
    /// boundary owning this dispatch. Retry-owned boundaries convert that
    /// panic to a structured callback failure; outer CAS boundaries unwind.
    fn dispatch_event(
        observability: &CasObservabilityConfig,
        hook: &crate::event::CasEventHook,
        event: CasEvent,
    ) where
        T: 'static,
        E: 'static,
    {
        match observability.listener_panic_policy() {
            ListenerPanicPolicy::Propagate => hook.accept(&event),
            ListenerPanicPolicy::Isolate => {
                let _ = catch_unwind(AssertUnwindSafe(|| hook.accept(&event)));
            }
        }
    }

    /// Returns whether lifecycle event construction and dispatch are needed.
    #[inline]
    fn should_emit_events(
        observability: &CasObservabilityConfig,
        hook: &Option<crate::event::CasEventHook>,
    ) -> bool {
        observability.mode() != CasObservabilityMode::ReportOnly
            && hook.is_some()
    }

    /// Dispatches one alert if an alert listener is registered.
    ///
    /// # Parameters
    /// - `observability`: Configuration controlling listener panic behavior.
    /// - `hook`: Optional listener that receives the alert.
    /// - `alert`: Contention alert to dispatch.
    ///
    /// # Panics
    /// Exposes alert listener panics to the owning CAS execution boundary when
    /// [`ListenerPanicPolicy::Propagate`] is configured.
    fn dispatch_alert(
        observability: &CasObservabilityConfig,
        hook: &Option<crate::event::CasAlertHook>,
        alert: CasAlert,
    ) {
        if let Some(hook) = hook {
            match observability.listener_panic_policy() {
                ListenerPanicPolicy::Propagate => hook.accept(&alert),
                ListenerPanicPolicy::Isolate => {
                    let _ =
                        catch_unwind(AssertUnwindSafe(|| hook.accept(&alert)));
                }
            }
        }
    }
}
