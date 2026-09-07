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

use qubit_error::BoxError;
use qubit_retry::BackoffPolicy;
use qubit_retry::RetryPolicy;
use qubit_retry::RetryPolicyError;

use super::cas_executor::CasExecutor;
use super::internal::AttemptTimeoutAction;
use crate::constants::DEFAULT_CAS_MAX_ATTEMPTS;
use crate::error::CasBuildError;
use crate::strategy::CasStrategy;

/// Builder for [`CasExecutor`](crate::CasExecutor).
///
/// ```compile_fail
/// #![deny(unused_must_use)]
///
/// use qubit_cas::CasExecutor;
///
/// CasExecutor::<usize, ()>::builder();
/// ```
#[must_use = "a CAS builder must be configured or built"]
pub struct CasBuilder<T, E = BoxError> {
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
    immediate_backoff: bool,
    /// Marker preserving the executor type parameters.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> CasBuilder<T, E> {
    /// Creates a builder with default retry policy values.
    ///
    /// # Returns
    /// A [`CasBuilder`] using immediate retries and the CAS default limit.
    pub fn new() -> Self {
        Self {
            max_attempts: DEFAULT_CAS_MAX_ATTEMPTS,
            max_operation_elapsed: None,
            max_total_elapsed: None,
            flow_timeout: None,
            backoff: Ok(BackoffPolicy::immediate()),
            attempt_timeout: None,
            attempt_timeout_action: AttemptTimeoutAction::Abort,
            immediate_backoff: true,
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
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets the maximum retries after the initial attempt.
    ///
    /// # Parameters
    /// - `max_retries`: Maximum retries after the first attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_retries(self, max_retries: u32) -> Self {
        self.max_attempts(max_retries.saturating_add(1))
    }

    /// Sets the maximum cumulative user operation elapsed-time budget.
    ///
    /// # Parameters
    /// - `max_operation_elapsed`: Optional cumulative user operation time
    ///   budget.
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
    /// - `max_total_elapsed`: Optional total retry-flow time budget.
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
    /// This timeout cancels an admitted attempt when reached. The retry policy
    /// `max_total_elapsed` setting remains a soft continuation budget that only
    /// controls admission of later attempts.
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
        self.immediate_backoff = false;
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
        self.immediate_backoff = false;
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
        builder.immediate_backoff = false;
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
        self.immediate_backoff = false;
        self
    }

    /// Sets the async per-attempt timeout.
    ///
    /// # Parameters
    /// - `attempt_timeout`: Timeout applied to each async CAS attempt.
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
    /// Returns a [`CasBuildError`] when a setting is invalid.
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
            self.immediate_backoff,
        ))
    }

    /// Builds one executor with the contention-adaptive strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] suitable for contended writers.
    pub fn build_contention_adaptive(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        self.strategy(CasStrategy::ContentionAdaptive).build()
    }

    /// Builds one executor with the latency-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for low latency.
    pub fn build_latency_first(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        self.strategy(CasStrategy::LatencyFirst).build()
    }

    /// Builds one executor with the reliability-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for long retry windows.
    pub fn build_reliability_first(self) -> Result<CasExecutor<T, E>, CasBuildError> {
        self.strategy(CasStrategy::ReliabilityFirst).build()
    }
}

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
