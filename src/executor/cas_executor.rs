/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! CAS executor implementation.

use std::marker::PhantomData;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::{
    Arc,
    Mutex,
};
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_error::BoxError;
use qubit_function::{
    Consumer,
    Function,
};
use qubit_retry::{
    AttemptFailure,
    AttemptFailureDecision,
    AttemptTimeoutPolicy,
    AttemptTimeoutSource,
    Retry,
    RetryContext,
    RetryError,
    RetryOptions,
};

use crate::cas_decision::CasDecision;
use crate::cas_outcome::CasOutcome;
use crate::cas_success::CasSuccess;
use crate::error::{
    CasAttemptFailure,
    CasError,
    CasErrorKind,
};
use crate::event::{
    CasContext,
    CasEvent,
    CasHooks,
};
use crate::observability::{
    CasAlert,
    CasObservabilityConfig,
    CasObservabilityMode,
    ListenerPanicPolicy,
};
use crate::report::{
    CasExecutionOutcome,
    CasExecutionReport,
    CasReportBuilder,
};
use crate::strategy::CasStrategy;

use super::cas_builder::CasBuilder;

/// Executor for retry-aware compare-and-swap workflows.
#[derive(Debug, Clone)]
pub struct CasExecutor<T, E = BoxError> {
    /// Immutable retry options shared by every execution.
    options: RetryOptions,
    /// Observability settings shared by executions.
    observability: CasObservabilityConfig,
    /// Marker preserving `T` and `E`.
    marker: PhantomData<fn() -> (T, E)>,
}

/// Success payload produced by one successful attempt before context enrichment.
enum AttemptSuccess<T, R> {
    /// One compare-and-swap write succeeded.
    Updated {
        previous: Arc<T>,
        current: Arc<T>,
        output: R,
    },
    /// The operation completed successfully without writing.
    Finished { current: Arc<T>, output: R },
}

/// Snapshot of retry-layer limits plus terminal outcome for finalizing [`CasReportBuilder`].
struct CasReportFinishContext {
    attempts_total: u32,
    max_attempts: u32,
    max_operation_elapsed: Option<Duration>,
    max_total_elapsed: Option<Duration>,
    outcome: CasExecutionOutcome,
}

impl CasReportFinishContext {
    #[inline]
    fn new(
        attempts_total: u32,
        max_attempts: u32,
        max_operation_elapsed: Option<Duration>,
        max_total_elapsed: Option<Duration>,
        outcome: CasExecutionOutcome,
    ) -> Self {
        Self {
            attempts_total,
            max_attempts,
            max_operation_elapsed,
            max_total_elapsed,
            outcome,
        }
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Creates a CAS builder.
    ///
    /// # Returns
    /// A builder configured with default retry settings.
    #[inline]
    pub fn builder() -> CasBuilder<T, E> {
        CasBuilder::new()
    }

    /// Creates an executor from retry options.
    ///
    /// # Parameters
    /// - `options`: Retry options to validate and install.
    ///
    /// # Returns
    /// A configured executor using the supplied retry options.
    ///
    /// # Errors
    /// Returns the retry-layer validation error when `options` are invalid.
    pub fn from_options(options: RetryOptions) -> Result<Self, qubit_retry::RetryConfigError> {
        Self::builder().options(options).build()
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
    /// - `options`: Validated retry options.
    /// - `observability`: Observability settings shared by executions.
    ///
    /// # Returns
    /// A configured executor.
    #[inline]
    pub(crate) fn new(options: RetryOptions, observability: CasObservabilityConfig) -> Self {
        Self {
            options,
            observability,
            marker: PhantomData,
        }
    }

    /// Returns the immutable retry options used by this executor.
    ///
    /// # Returns
    /// Shared retry options.
    #[inline]
    pub fn options(&self) -> &RetryOptions {
        &self.options
    }

    /// Returns observability settings used by this executor.
    ///
    /// # Returns
    /// Shared observability configuration.
    #[inline]
    pub fn observability(&self) -> &CasObservabilityConfig {
        &self.observability
    }

    /// Executes one synchronous CAS operation.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and returns
    ///   a CAS decision.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    pub fn execute<R, O>(&self, state: &AtomicRef<T>, operation: O) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        self.execute_with_hooks(state, operation, CasHooks::new())
    }

    /// Executes one synchronous CAS operation with lifecycle hooks.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and returns
    ///   a CAS decision.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
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
        let success_context = Arc::new(Mutex::new(None));
        let attempt_snapshot = Arc::new(Mutex::new(None));
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(
            &hooks,
            Arc::clone(&success_context),
            Arc::clone(&report_builder),
        );
        let attempt = retry.run(|| self.run_sync_attempt(state, &operation));
        self.finish_execution(
            attempt,
            hooks,
            success_context,
            attempt_snapshot,
            report_builder,
        )
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

    /// Executes one asynchronous CAS operation with lifecycle hooks.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
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
        let success_context = Arc::new(Mutex::new(None));
        let attempt_snapshot = Arc::new(Mutex::new(None));
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(
            &hooks,
            Arc::clone(&success_context),
            Arc::clone(&report_builder),
        );
        let attempt_snapshot_for_attempt = Arc::clone(&attempt_snapshot);
        let attempt = retry
            .run_async(|| {
                self.run_async_attempt(state, &operation, Arc::clone(&attempt_snapshot_for_attempt))
            })
            .await;
        self.finish_execution(
            attempt,
            hooks,
            success_context,
            attempt_snapshot,
            report_builder,
        )
    }

    /// Builds one retry policy for a single CAS execution.
    ///
    /// # Parameters
    /// - `hooks`: Hook registrations for the current execution.
    /// - `success_context`: Shared slot used to capture the retry success
    ///   context.
    /// # Returns
    /// A retry policy configured for one CAS execution.
    fn build_retry(
        &self,
        hooks: &CasHooks,
        success_context: Arc<Mutex<Option<RetryContext>>>,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> Retry<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        let event_hook = hooks.event_hook();
        let retry_timeout_policy = self
            .options
            .attempt_timeout()
            .map(|attempt_timeout| attempt_timeout.policy());
        let observability = self.observability.clone();

        let mut builder = Retry::<CasAttemptFailure<T, E>>::builder()
            .options(self.options.clone())
            .on_success(move |context: &RetryContext| {
                *success_context
                    .lock()
                    .expect("CAS success context slot should be lockable") = Some(*context);
            })
            .on_failure(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, context: &RetryContext| {
                    let failure = match failure {
                        AttemptFailure::Panic(_) | AttemptFailure::Executor(_) => {
                            return AttemptFailureDecision::UseDefault;
                        }
                        AttemptFailure::Error(failure) => failure,
                        AttemptFailure::Timeout => {
                            let cas_context = CasContext::new(context);
                            report_builder
                                .lock()
                                .expect("CAS report builder should be lockable")
                                .record_timeout();
                            if Self::should_emit_events(&observability, &event_hook) {
                                let hook = event_hook
                                    .as_ref()
                                    .expect("event hook should exist when events are emitted");
                                Self::dispatch_event(
                                    &observability,
                                    hook,
                                    CasEvent::AttemptFailed {
                                        context: cas_context,
                                        kind: crate::error::CasAttemptFailureKind::Timeout,
                                    },
                                );
                                if context.attempt_timeout_source()
                                    == Some(AttemptTimeoutSource::Configured)
                                    && retry_timeout_policy == Some(AttemptTimeoutPolicy::Retry)
                                {
                                    Self::dispatch_event(
                                        &observability,
                                        hook,
                                        CasEvent::RetryRequested {
                                            context: cas_context,
                                        },
                                    );
                                }
                            }
                            return AttemptFailureDecision::UseDefault;
                        }
                    };
                    let cas_context = CasContext::new(context);
                    {
                        let mut report = report_builder
                            .lock()
                            .expect("CAS report builder should be lockable");
                        match failure {
                            CasAttemptFailure::Conflict { .. } => report.record_conflict(),
                            CasAttemptFailure::Retry { .. } => report.record_retry_error(),
                            CasAttemptFailure::Abort { .. } => report.record_abort(),
                            CasAttemptFailure::Timeout { .. } => report.record_timeout(),
                        }
                    }
                    if Self::should_emit_events(&observability, &event_hook) {
                        Self::dispatch_event(
                            &observability,
                            event_hook
                                .as_ref()
                                .expect("event hook should exist when events are emitted"),
                            CasEvent::AttemptFailed {
                                context: cas_context,
                                kind: Self::failure_kind(failure),
                            },
                        );
                    }
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
                            AttemptFailureDecision::Retry
                        }
                        CasAttemptFailure::Abort { .. } => AttemptFailureDecision::Abort,
                        CasAttemptFailure::Timeout { .. } => AttemptFailureDecision::UseDefault,
                    }
                },
            );

        if self.observability.listener_panic_policy() == ListenerPanicPolicy::Isolate {
            builder = builder.isolate_listener_panics();
        }
        builder
            .build()
            .expect("validated CAS executor configuration must build retry policy")
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
            CasDecision::Finish { output } => Ok(AttemptSuccess::Finished { current, output }),
            CasDecision::Retry(error) => Err(CasAttemptFailure::retry(current, error)),
            CasDecision::Abort(error) => Err(CasAttemptFailure::abort(current, error)),
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
            .expect("CAS attempt snapshot slot should be lockable") = Some(Arc::clone(&current));
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
            CasDecision::Finish { output } => Ok(AttemptSuccess::Finished { current, output }),
            CasDecision::Retry(error) => Err(CasAttemptFailure::retry(current, error)),
            CasDecision::Abort(error) => Err(CasAttemptFailure::abort(current, error)),
        }
    }

    /// Finalizes one retry execution into the public CAS result type.
    ///
    /// # Parameters
    /// - `attempt`: Retry-layer terminal success or error.
    /// - `hooks`: Hook registrations for the current execution.
    /// - `success_context`: Shared slot storing the success context.
    ///
    /// # Returns
    /// Public CAS success or error.
    fn finish_execution<R>(
        &self,
        attempt: Result<AttemptSuccess<T, R>, RetryError<CasAttemptFailure<T, E>>>,
        hooks: CasHooks,
        success_context: Arc<Mutex<Option<RetryContext>>>,
        attempt_snapshot: Arc<Mutex<Option<Arc<T>>>>,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
    {
        match attempt {
            Ok(success) => {
                let context = success_context
                    .lock()
                    .expect("CAS success context slot should be lockable")
                    .take()
                    .expect("retry success hook must capture CAS success context");
                let attempts_total = context.attempt();
                let max_attempts = context.max_attempts();
                let max_operation_elapsed = context.max_operation_elapsed();
                let max_total_elapsed = context.max_total_elapsed();
                let outcome = match success {
                    AttemptSuccess::Updated { .. } => CasExecutionOutcome::SuccessUpdated,
                    AttemptSuccess::Finished { .. } => CasExecutionOutcome::SuccessFinished,
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
                let timeout_current = attempt_snapshot
                    .lock()
                    .expect("CAS attempt snapshot slot should be lockable")
                    .clone();
                let error = CasError::new(error, timeout_current);
                let context = error.context();
                let outcome = Self::error_outcome(error.kind());
                let report = self.finish_report(
                    &hooks,
                    report_builder,
                    CasReportFinishContext::new(
                        context.attempt(),
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
    fn emit_started(&self, hooks: &CasHooks, report_builder: &Arc<Mutex<CasReportBuilder>>)
    where
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
        if self.observability.mode() == CasObservabilityMode::EventStreamWithAlert
            && let Some(thresholds) = self.observability.contention_thresholds()
            && report.is_contention_hot(&thresholds)
        {
            Self::dispatch_alert(
                &self.observability,
                &hooks.alert_hook(),
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
            CasErrorKind::Conflict => CasExecutionOutcome::ErrorConflictExhausted,
            CasErrorKind::RetryExhausted => CasExecutionOutcome::ErrorRetryExhausted,
            CasErrorKind::AttemptTimeout => CasExecutionOutcome::ErrorAttemptTimeout,
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
    fn failure_kind(failure: &CasAttemptFailure<T, E>) -> crate::error::CasAttemptFailureKind {
        failure.kind()
    }

    /// Dispatches one lifecycle event if event streaming is enabled.
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
        observability.mode() != CasObservabilityMode::ReportOnly && hook.is_some()
    }

    /// Dispatches one alert if an alert listener is registered.
    fn dispatch_alert(
        observability: &CasObservabilityConfig,
        hook: &Option<crate::event::CasAlertHook>,
        alert: CasAlert,
    ) {
        if let Some(hook) = hook {
            match observability.listener_panic_policy() {
                ListenerPanicPolicy::Propagate => hook.accept(&alert),
                ListenerPanicPolicy::Isolate => {
                    let _ = catch_unwind(AssertUnwindSafe(|| hook.accept(&alert)));
                }
            }
        }
    }
}
