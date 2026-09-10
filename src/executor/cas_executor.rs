// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS executor implementation.

// Asynchronous result and observed execution entry points.
#[path = "cas_executor/async_execution.rs"]
#[cfg(feature = "tokio")]
mod async_execution;
// Shared application-decision publication logic.
#[path = "cas_executor/decision.rs"]
mod decision;
// Listener dispatch and report finalization.
#[path = "cas_executor/dispatch.rs"]
mod dispatch;
// Conversion of retry terminal values to CAS results.
#[path = "cas_executor/finalization.rs"]
mod finalization;
// Private retry observer and lifecycle dispatch bridge.
#[path = "cas_executor/internal/mod.rs"]
mod internal;
// Retry configuration, rule classification, and shared lazy cache.
#[path = "cas_executor/retry_adapter.rs"]
mod retry_adapter;
// Synchronous result and observed execution entry points.
#[path = "cas_executor/sync_execution.rs"]
mod sync_execution;

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use qubit_retry::RetryConfig;
use qubit_retry::RetryPolicy;

use super::cas_builder::CasBuilder;
use super::internal::AttemptTimeoutAction;
use crate::error::CasAttemptFailure;
use crate::error::CasBoxError;
use crate::strategy::CasStrategy;

/// Executor for retry-aware compare-and-swap workflows.
///
/// # Type Parameters
/// - `T`: Shared application state; cloning the executor does not clone it.
/// - `E`: Business failure type; it need not implement Clone.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
///
/// let state = AtomicRef::from_value(3usize);
/// let executor = CasExecutor::<usize, ()>::builder().build().unwrap();
/// let success = executor.execute_result(&state, |current: &usize| {
///     CasDecision::update(*current - 1, "reserved")
/// }).unwrap();
/// assert_eq!(*state.load(), 2);
/// assert_eq!(*success.output(), "reserved");
/// assert_eq!(executor.max_attempts(), 5);
/// ```
///
/// Retry implementation types are not part of the configuration API:
///
/// ```compile_fail
/// use qubit_cas::CasExecutor;
///
/// let executor = CasExecutor::<usize, ()>::builder().build().unwrap();
/// let _ = executor.retry_policy();
/// ```
pub struct CasExecutor<T, E = CasBoxError> {
    /// Pure policy used by the retry facades.
    policy: RetryPolicy,
    /// Optional hard wall-clock timeout for asynchronous retry flows.
    flow_timeout: Option<Duration>,
    /// Optional hard timeout applied to each async attempt.
    attempt_timeout: Option<Duration>,
    /// Action selected after a configured attempt timeout.
    attempt_timeout_action: AttemptTimeoutAction,
    /// Result-only retry definition initialized on its first use.
    result_retry: Arc<OnceLock<RetryConfig<CasAttemptFailure<T, E>>>>,
    /// Marker preserving `T` and `E`.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> Clone for CasExecutor<T, E> {
    /// Shares the cached retry configuration without cloning application data.
    ///
    /// # Returns
    /// An executor sharing lazy retry state, without cloning application state
    /// or errors.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            flow_timeout: self.flow_timeout,
            attempt_timeout: self.attempt_timeout,
            attempt_timeout_action: self.attempt_timeout_action,
            result_retry: Arc::clone(&self.result_retry),
            marker: PhantomData,
        }
    }
}

impl<T, E> fmt::Debug for CasExecutor<T, E> {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CasExecutor")
            .field("policy", &self.policy)
            .field("flow_timeout", &self.flow_timeout)
            .field("attempt_timeout", &self.attempt_timeout)
            .field("attempt_timeout_action", &self.attempt_timeout_action)
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

    /// Creates an executor tuned for low-latency workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    #[inline]
    #[must_use]
    pub fn latency_first() -> Self {
        Self::builder()
            .build_latency_first()
            .expect("latency-first CAS strategy must be valid")
    }

    /// Creates an executor tuned for hot-contention workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    #[inline]
    #[must_use]
    pub fn contention_backoff() -> Self {
        Self::builder()
            .build_contention_backoff()
            .expect("contention-backoff CAS strategy must be valid")
    }

    /// Creates an executor tuned for reliability-first workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    #[inline]
    #[must_use]
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
    #[inline]
    #[must_use]
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
    /// - `attempt_timeout`: `Some` cooperative async attempt deadline, or
    ///   `None` when disabled.
    /// - `flow_timeout`: `Some` cooperative async flow deadline, or `None` when
    ///   disabled.
    /// - `attempt_timeout_action`: Action selected for attempt timeouts.
    ///
    /// # Returns
    /// A configured executor.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        policy: RetryPolicy,
        attempt_timeout: Option<Duration>,
        flow_timeout: Option<Duration>,
        attempt_timeout_action: AttemptTimeoutAction,
    ) -> Self {
        Self {
            policy,
            flow_timeout,
            attempt_timeout,
            attempt_timeout_action,
            result_retry: Arc::new(OnceLock::new()),
            marker: PhantomData,
        }
    }

    /// Returns the maximum number of admitted attempts, including the first.
    ///
    /// Reading configuration does not allocate or initialize execution state.
    ///
    /// # Returns
    /// The configured positive attempt limit.
    #[must_use]
    #[inline(always)]
    pub fn max_attempts(&self) -> u32 {
        self.policy.admission_limits().max_attempts().get()
    }

    /// Returns the maximum number of retries after the first attempt.
    ///
    /// # Returns
    /// The attempt limit minus one; zero means no retries are allowed.
    #[must_use]
    #[inline(always)]
    pub fn max_retries(&self) -> u32 {
        self.max_attempts().saturating_sub(1)
    }

    /// Returns the soft budget for accumulated attempt execution time.
    ///
    /// # Returns
    /// `Some` limits admission using elapsed operation and adapter work;
    /// `None` disables this budget. An admitted successful commit is retained.
    #[must_use]
    #[inline(always)]
    pub fn max_operation_elapsed(&self) -> Option<Duration> {
        self.policy.admission_limits().operation_time_budget()
    }

    /// Returns the soft total-time budget, including backoff.
    ///
    /// # Returns
    /// `Some` limits retry scheduling and admission; `None` disables this
    /// budget. It does not cancel an already admitted operation.
    #[must_use]
    #[inline(always)]
    pub fn max_total_elapsed(&self) -> Option<Duration> {
        self.policy.admission_limits().total_time_budget()
    }

    /// Returns the cooperative timeout for each asynchronous attempt.
    ///
    /// # Returns
    /// `Some` enables the timeout; `None` disables it. Synchronous execution
    /// ignores this setting, and blocking operation code cannot be preempted.
    #[must_use]
    #[inline(always)]
    pub fn attempt_timeout(&self) -> Option<Duration> {
        self.attempt_timeout
    }

    /// Returns the cooperative deadline for asynchronous attempts and backoff.
    ///
    /// # Returns
    /// `Some(Duration)` when a hard async flow timeout was configured; `None`
    /// otherwise. It is independent of the soft total-time admission budget,
    /// excludes the full cost of start/finish hooks, and cannot preempt
    /// blocking operation code. Synchronous execution ignores this setting.
    #[must_use]
    #[inline(always)]
    pub fn flow_timeout(&self) -> Option<Duration> {
        self.flow_timeout
    }
}
