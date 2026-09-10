// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Retry policy construction for CAS executions.

use std::sync::Arc;
use std::sync::Mutex;

use qubit_retry::AttemptFailure;
use qubit_retry::RetryConfig;
use qubit_retry::RetryContext;
use qubit_retry::RetryDecision;
use qubit_retry::RetryFallback;
use qubit_retry::RetryTimeoutScope;

use super::CasExecutor;
use super::internal::CasRetryObserver;
use crate::error::CasAttemptFailure;
use crate::error::CasAttemptFailureKind;
use crate::event::CasContext;
use crate::event::CasEvent;
use crate::event::CasHooks;
use crate::executor::cas_executor::retry_adapter;
use crate::executor::internal::AttemptTimeoutAction;
use crate::report::CasReportBuilder;

/// Classifies one retry-layer failure without producing side effects.
///
/// # Type Parameters
/// - `T`: State snapshot retained by application failures.
/// - `E`: Business failure type.
///
/// # Parameters
/// - `failure`: Retry failure being considered for another attempt.
/// - `attempt_timeout_action`: Whether an attempt timeout retries or aborts.
///
/// # Returns
/// Retry for conflicts and retryable business errors, Abort for explicit
/// aborts, or the retry facade's default handling for infrastructure and other
/// failures.
pub(super) fn retry_decision<T, E>(
    failure: &AttemptFailure<CasAttemptFailure<T, E>>,
    attempt_timeout_action: AttemptTimeoutAction,
) -> RetryDecision {
    match failure {
        AttemptFailure::Error(CasAttemptFailure::Conflict { .. })
        | AttemptFailure::Error(CasAttemptFailure::Retry { .. }) => RetryDecision::Retry,
        AttemptFailure::Error(CasAttemptFailure::Abort { .. }) => RetryDecision::Abort,
        AttemptFailure::Error(CasAttemptFailure::Timeout { .. }) => RetryDecision::UseDefault,
        AttemptFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt,
        } if attempt_timeout_action == AttemptTimeoutAction::Retry => RetryDecision::Retry,
        AttemptFailure::TimedOut { .. } | AttemptFailure::Panicked { .. } => RetryDecision::UseDefault,
        _ => RetryDecision::UseDefault,
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Builds one retry policy for a single CAS execution.
    ///
    /// # Parameters
    /// - `hooks`: Hook registrations for the current execution.
    /// - `report_builder`: Shared accumulator used after callback dispatch.
    ///
    /// # Returns
    /// A retry policy configured for one CAS execution.
    ///
    /// # Panics
    /// Panics if the previously validated configuration is no longer valid.
    pub(super) fn build_retry(
        &self,
        hooks: &CasHooks,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> RetryConfig<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        let event_hook = hooks.event_hook();
        let attempt_timeout_action = self.attempt_timeout_action;
        let observer_event_hook = event_hook.clone();
        let observer_report_builder = Arc::clone(&report_builder);

        RetryConfig::<CasAttemptFailure<T, E>>::builder()
            .policy(self.policy.clone())
            .fallback(RetryFallback::Retry)
            .observer(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, context: &RetryContext| {
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
                            Some(CasAttemptFailureKind::Timeout)
                        }
                        AttemptFailure::Panicked { .. } => None,
                        _ => None,
                    };
                    if let Some(kind) = kind
                        && Self::should_emit_events(&observer_event_hook)
                        && let Some(listener_failure) = Self::dispatch_event(
                            observer_event_hook
                                .as_ref()
                                .expect("event hook should exist when events are emitted"),
                            CasEvent::AttemptFailed {
                                context: CasContext::new(context),
                                kind,
                            },
                        )
                    {
                        observer_report_builder
                            .lock()
                            .expect("CAS report builder should be lockable")
                            .record_listener_failure(listener_failure);
                    }
                },
            )
            .observer(CasRetryObserver::new(event_hook, report_builder))
            .rule(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, _context: &RetryContext| {
                    super::retry_adapter::retry_decision(failure, attempt_timeout_action)
                },
            )
            .build()
            .expect("validated CAS retry configuration")
    }

    /// Returns the cached retry definition used by result-only execution.
    ///
    /// # Returns
    /// An immutable retry definition initialized once and shared by executor
    /// clones. The first call allocates its rule; later calls reuse the cache.
    ///
    /// # Panics
    /// Panics if the previously validated configuration is no longer valid.
    pub(super) fn result_retry(&self) -> &RetryConfig<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        self.result_retry.get_or_init(|| {
            let attempt_timeout_action = self.attempt_timeout_action;
            RetryConfig::<CasAttemptFailure<T, E>>::builder()
                .policy(self.policy.clone())
                .fallback(RetryFallback::Retry)
                .rule(
                    move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, _context: &RetryContext| {
                        retry_adapter::retry_decision(failure, attempt_timeout_action)
                    },
                )
                .build()
                .expect("validated CAS retry configuration")
        })
    }
}
