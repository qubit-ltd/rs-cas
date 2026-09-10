// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for [`crate::CasExecutor`].

use std::marker::PhantomData;
use std::time::Duration;

use qubit_retry::BackoffPolicy;
use qubit_retry::RetryPolicy;
use qubit_retry::RetryPolicyError;

use super::cas_executor::CasExecutor;
use super::internal::AttemptTimeoutAction;
use crate::constants::DEFAULT_CAS_MAX_ATTEMPTS;
use crate::error::CasBoxError;
use crate::error::CasBuildError;
use crate::strategy::CasStrategy;

/// Builder for [`CasExecutor`].
///
/// ```compile_fail
/// #![deny(unused_must_use)]
///
/// use qubit_cas::CasExecutor;
///
/// CasExecutor::<usize, ()>::builder();
/// ```
///
/// # Type Parameters
/// - `T`: Shared application state used by the executor.
/// - `E`: Business failure returned by operations.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
///
/// let state = AtomicRef::from_value(3usize);
/// let success = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute_result(&state, |current: &usize| CasDecision::update(*current - 1, "reserved"))
///     .unwrap();
/// assert!(success.is_updated());
/// assert_eq!(**success.previous().unwrap(), 3);
/// assert_eq!(**success.current(), 2);
/// assert_eq!(*success.output(), "reserved");
/// assert!(CasExecutor::<usize, ()>::builder().max_attempts(0).build().is_err());
/// ```
#[must_use = "a CAS builder must be configured or built"]
pub struct CasBuilder<T, E = CasBoxError> {
    /// Maximum total attempts, including the initial operation.
    max_attempts: u32,
    /// Optional cumulative operation-time continuation budget.
    max_operation_elapsed: Option<Duration>,
    /// Optional end-to-end continuation budget.
    max_total_elapsed: Option<Duration>,
    /// Optional hard wall-clock timeout for asynchronous retry flows.
    flow_timeout: Option<Duration>,
    /// Validated backoff policy or its deferred construction error.
    backoff: Result<BackoffPolicy, RetryPolicyError>,
    /// Optional hard timeout applied to each async attempt.
    attempt_timeout: Option<Duration>,
    /// Action selected after a configured attempt timeout.
    attempt_timeout_action: AttemptTimeoutAction,
    /// Marker preserving the executor type parameters.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> CasBuilder<T, E> {
    /// Creates a builder with default retry policy values.
    ///
    /// # Returns
    /// A [`CasBuilder`] using immediate retries and the CAS default limit.
    #[inline]
    pub fn new() -> Self {
        Self {
            max_attempts: DEFAULT_CAS_MAX_ATTEMPTS,
            max_operation_elapsed: None,
            max_total_elapsed: None,
            flow_timeout: None,
            backoff: Ok(BackoffPolicy::immediate()),
            attempt_timeout: None,
            attempt_timeout_action: AttemptTimeoutAction::Abort,
            marker: PhantomData,
        }
    }

    /// Sets the maximum total attempts.
    ///
    /// # Parameters
    /// - `max_attempts`: Maximum attempts, including the initial attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets the maximum retries after the initial attempt.
    ///
    /// # Parameters
    /// - `max_retries`: Maximum retries after the first attempt. Conversion to
    ///   total attempts saturates at `u32::MAX`, so the largest input permits
    ///   at most `u32::MAX - 1` retries.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_retries(self, max_retries: u32) -> Self {
        self.max_attempts(max_retries.saturating_add(1))
    }

    /// Sets the maximum cumulative attempt elapsed-time budget.
    ///
    /// # Parameters
    /// - `max_operation_elapsed`: `Some` soft budget for cumulative operation
    ///   and CAS adapter work; `None` disables this admission check. Admitted
    ///   successful commits are retained even when they exceed the budget.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_operation_elapsed(mut self, max_operation_elapsed: Option<Duration>) -> Self {
        self.max_operation_elapsed = max_operation_elapsed;
        self
    }

    /// Sets the maximum monotonic elapsed-time budget for the whole retry flow.
    ///
    /// # Parameters
    /// - `max_total_elapsed`: `Some` soft budget including backoff; `None`
    ///   disables this check. It cannot cancel an already admitted operation.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_total_elapsed(mut self, max_total_elapsed: Option<Duration>) -> Self {
        self.max_total_elapsed = max_total_elapsed;
        self
    }

    /// Sets the hard wall-clock timeout for asynchronous retry flows.
    ///
    /// This cooperative timeout drops an admitted future when it yields
    /// control. It cannot preempt blocking work or roll back external side
    /// effects. The retry policy `max_total_elapsed` setting remains a soft
    /// continuation budget that only controls admission of later attempts.
    /// Synchronous execution ignores it.
    ///
    /// # Parameters
    /// - `flow_timeout`: `Some` enables the deadline; `None` disables it.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn flow_timeout(mut self, flow_timeout: Option<Duration>) -> Self {
        self.flow_timeout = flow_timeout;
        self
    }

    /// Uses immediate retries with no sleep.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn no_delay(mut self) -> Self {
        self.backoff = Ok(BackoffPolicy::immediate());
        self
    }

    /// Uses one fixed retry delay.
    ///
    /// # Parameters
    /// - `delay`: Delay slept before each retry.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn fixed_delay(mut self, delay: Duration) -> Self {
        self.backoff = Ok(BackoffPolicy::fixed(delay));
        self
    }

    /// Uses one random retry delay range.
    ///
    /// # Parameters
    /// - `min`: Inclusive minimum delay.
    /// - `max`: Inclusive maximum delay.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn random_delay(mut self, min: Duration, max: Duration) -> Self {
        self.backoff = BackoffPolicy::uniform(min, max);
        self
    }

    /// Uses exponential backoff with multiplier `2.0`.
    ///
    /// # Parameters
    /// - `initial`: Initial retry delay.
    /// - `max`: Maximum retry delay.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn exponential_backoff(self, initial: Duration, max: Duration) -> Self {
        self.exponential_backoff_with_multiplier(initial, max, 2.0)
    }

    /// Uses exponential backoff with a custom multiplier.
    ///
    /// # Parameters
    /// - `initial`: Initial retry delay.
    /// - `max`: Maximum retry delay.
    /// - `multiplier`: Multiplier applied after each failed attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn exponential_backoff_with_multiplier(self, initial: Duration, max: Duration, multiplier: f64) -> Self {
        let mut builder = self;
        builder.backoff = BackoffPolicy::exponential(initial, multiplier, max);
        builder
    }

    /// Sets relative jitter by factor.
    ///
    /// # Parameters
    /// - `factor`: Relative jitter factor in `[0.0, 1.0]`.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn jitter_factor(mut self, factor: f64) -> Self {
        self.backoff = self.backoff.and_then(|backoff| backoff.with_bounded_jitter(factor));
        self
    }

    /// Sets the async per-attempt timeout.
    ///
    /// # Parameters
    /// - `attempt_timeout`: `Some` enables a cooperative async deadline; `None`
    ///   disables it. Synchronous calls ignore it; blocking operation code
    ///   cannot be preempted, and cancellation does not undo side effects.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn attempt_timeout(mut self, attempt_timeout: Option<Duration>) -> Self {
        self.attempt_timeout = attempt_timeout;
        self
    }

    /// Retries attempts that exceed the configured timeout.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn retry_on_timeout(mut self) -> Self {
        self.attempt_timeout_action = AttemptTimeoutAction::Retry;
        self
    }

    /// Aborts the CAS flow when one attempt exceeds the timeout.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn abort_on_timeout(mut self) -> Self {
        self.attempt_timeout_action = AttemptTimeoutAction::Abort;
        self
    }

    /// Applies a built-in CAS strategy to this builder.
    ///
    /// Replaces attempts, soft budgets, and the entire backoff configuration,
    /// including a deferred backoff error. Async timeouts and their action
    /// remain unchanged. Later setters override the installed preset.
    ///
    /// # Parameters
    /// - `strategy`: Strategy profile to install.
    ///
    /// # Returns
    /// The updated builder.
    pub fn strategy(self, strategy: CasStrategy) -> Self {
        let profile = strategy.profile();
        let builder = self
            .max_attempts(profile.max_attempts())
            .max_operation_elapsed(Some(profile.max_operation_elapsed()))
            .max_total_elapsed(profile.max_total_elapsed());
        if let Some((initial, max, jitter)) = strategy.backoff() {
            builder.exponential_backoff(initial, max).jitter_factor(jitter)
        } else {
            builder.no_delay()
        }
    }

    /// Builds one executor after validating the settings.
    ///
    /// # Returns
    /// A validated [`CasExecutor`].
    ///
    /// # Errors
    /// Returns a [`CasBuildError`] for zero attempts, reversed delay bounds,
    /// a non-finite or less-than-one exponential multiplier, or jitter outside
    /// the finite range `[0, 1]`. Backoff errors are deferred until this call;
    /// a later backoff setter or strategy can replace an earlier error.
    pub fn build(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        let backoff = self.backoff.map_err(map_retry_policy_error)?;
        let policy = RetryPolicy::builder()
            .max_attempts(self.max_attempts)
            .operation_time_budget_opt(self.max_operation_elapsed)
            .total_time_budget_opt(self.max_total_elapsed)
            .backoff(backoff)
            .build()
            .map_err(map_retry_policy_error)?;
        Ok(CasExecutor::new(
            policy,
            self.attempt_timeout,
            self.flow_timeout,
            self.attempt_timeout_action,
        ))
    }

    /// Builds one executor with the contention-backoff strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] suitable for contended writers.
    ///
    /// # Errors
    /// Preset construction uses the same validated result as [`Self::build`].
    /// The installed built-in settings are always valid and replace any
    /// deferred retry-policy error, so current presets return `Ok`.
    #[inline(always)]
    pub fn build_contention_backoff(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        self.strategy(CasStrategy::ContentionBackoff).build()
    }

    /// Builds one executor with the latency-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for low latency.
    ///
    /// # Errors
    /// Preset construction uses the same validated result as [`Self::build`].
    /// The installed built-in settings are always valid and replace any
    /// deferred retry-policy error, so current presets return `Ok`.
    #[inline(always)]
    pub fn build_latency_first(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        self.strategy(CasStrategy::LatencyFirst).build()
    }

    /// Builds one executor with the reliability-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for long retry windows.
    ///
    /// # Errors
    /// Preset construction uses the same validated result as [`Self::build`].
    /// The installed built-in settings are always valid and replace any
    /// deferred retry-policy error, so current presets return `Ok`.
    #[inline(always)]
    pub fn build_reliability_first(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        self.strategy(CasStrategy::ReliabilityFirst).build()
    }
}

/// Retains the retry validator message under the CAS policy configuration
/// boundary.
///
/// # Parameters
/// - `error`: Upstream validation error consumed during builder validation.
///
/// # Returns
/// A CAS-owned construction error; the upstream message remains diagnostic
/// text.
#[inline]
fn map_retry_policy_error(error: RetryPolicyError) -> CasBuildError {
    CasBuildError::new("retry_policy", error.to_string())
}

impl<T, E> Default for CasBuilder<T, E> {
    /// Creates a default CAS builder.
    ///
    /// # Returns
    /// A builder equivalent to [`CasBuilder::new`].
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
