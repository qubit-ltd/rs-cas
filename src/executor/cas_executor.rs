/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! CAS executor implementation.

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_common::BoxError;
use qubit_function::{BiConsumer, Consumer, Function};
use qubit_retry::{
    AttemptFailure, AttemptFailureDecision, Retry, RetryContext, RetryError, RetryOptions,
};

use crate::decision::CasDecision;
use crate::error::{CasAttemptFailure, CasError};
use crate::event::{CasContext, CasHooks};
use crate::options::CasTimeoutPolicy;
use crate::success::CasSuccess;

use super::cas_builder::CasBuilder;

/// Executor for retry-aware compare-and-swap workflows.
#[derive(Debug, Clone)]
pub struct CasExecutor<T, E = BoxError> {
    /// Immutable retry options shared by every execution.
    options: RetryOptions,
    /// Optional timeout for each async CAS attempt.
    attempt_timeout: Option<Duration>,
    /// Policy used when one async attempt times out.
    timeout_policy: CasTimeoutPolicy,
    /// Whether retry-layer listener panics should be isolated.
    isolate_listener_panics: bool,
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
    /// A configured executor using the default timeout policy.
    ///
    /// # Errors
    /// Returns the retry-layer validation error when `options` are invalid.
    pub fn from_options(options: RetryOptions) -> Result<Self, qubit_retry::RetryConfigError> {
        Self::builder().options(options).build()
    }

    /// Creates an executor tuned for high-concurrency workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in preset is always valid.
    pub fn high_concurrency() -> Self {
        Self::builder()
            .build_high_concurrency()
            .expect("high-concurrency CAS preset must be valid")
    }

    /// Creates an executor tuned for low-latency workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in preset is always valid.
    pub fn low_latency() -> Self {
        Self::builder()
            .build_low_latency()
            .expect("low-latency CAS preset must be valid")
    }

    /// Creates an executor tuned for high-reliability workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in preset is always valid.
    pub fn high_reliability() -> Self {
        Self::builder()
            .build_high_reliability()
            .expect("high-reliability CAS preset must be valid")
    }

    /// Creates one executor from validated parts.
    ///
    /// # Parameters
    /// - `options`: Validated retry options.
    /// - `attempt_timeout`: Optional async attempt timeout.
    /// - `timeout_policy`: Policy used when one attempt exceeds the timeout.
    /// - `isolate_listener_panics`: Whether retry-layer listener panics should
    ///   be isolated.
    ///
    /// # Returns
    /// A configured executor.
    #[inline]
    pub(crate) fn new(
        options: RetryOptions,
        attempt_timeout: Option<Duration>,
        timeout_policy: CasTimeoutPolicy,
        isolate_listener_panics: bool,
    ) -> Self {
        Self {
            options,
            attempt_timeout,
            timeout_policy,
            isolate_listener_panics,
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

    /// Returns the configured async attempt timeout.
    ///
    /// # Returns
    /// `Some(Duration)` when async attempts have a timeout.
    #[inline]
    pub fn attempt_timeout(&self) -> Option<Duration> {
        self.attempt_timeout
    }

    /// Returns the timeout policy.
    ///
    /// # Returns
    /// Policy used when an async attempt exceeds the timeout.
    #[inline]
    pub fn timeout_policy(&self) -> CasTimeoutPolicy {
        self.timeout_policy
    }

    /// Executes one synchronous CAS operation.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and returns
    ///   a CAS decision.
    ///
    /// # Returns
    /// A terminal success or error for the CAS flow.
    pub fn execute<R, O>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>>
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
    /// A terminal success or error for the CAS flow.
    pub fn execute_with_hooks<R, O>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
        hooks: CasHooks<T, R, E>,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        let success_context = Arc::new(Mutex::new(None));
        let retry = self.build_retry(&hooks, Arc::clone(&success_context));
        let attempt = retry.run(|| self.run_sync_attempt(state, &operation));
        self.finish_execution(attempt, hooks, success_context)
    }

    /// Executes one asynchronous CAS operation.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// A terminal success or error for the CAS flow.
    #[cfg(feature = "tokio")]
    pub async fn execute_async<R, O, Fut>(
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
    /// A terminal success or error for the CAS flow.
    #[cfg(feature = "tokio")]
    pub async fn execute_async_with_hooks<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
        hooks: CasHooks<T, R, E>,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let success_context = Arc::new(Mutex::new(None));
        let retry = self.build_retry(&hooks, Arc::clone(&success_context));
        let attempt = retry
            .run_async(|| self.run_async_attempt(state, &operation))
            .await;
        self.finish_execution(attempt, hooks, success_context)
    }

    /// Builds one retry policy for a single CAS execution.
    ///
    /// # Parameters
    /// - `hooks`: Hook registrations for the current execution.
    /// - `success_context`: Shared slot used to capture the retry success
    ///   context.
    ///
    /// # Returns
    /// A retry policy configured for one CAS execution.
    fn build_retry<R>(
        &self,
        hooks: &CasHooks<T, R, E>,
        success_context: Arc<Mutex<Option<RetryContext>>>,
    ) -> Retry<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        let retry_hook = hooks.retry_hook();
        let abort_hook = hooks.abort_hook();
        let timeout_policy = self.timeout_policy;
        let attempt_timeout = self.attempt_timeout;

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
                        AttemptFailure::Error(failure) => failure,
                        AttemptFailure::Timeout => {
                            unreachable!("CAS executor manages async timeouts explicitly")
                        }
                    };
                    let cas_context = CasContext::new(context, attempt_timeout);
                    match failure {
                        CasAttemptFailure::Conflict { .. } | CasAttemptFailure::Retry { .. } => {
                            if let Some(hook) = &retry_hook {
                                hook.accept(&cas_context, failure);
                            }
                            AttemptFailureDecision::Retry
                        }
                        CasAttemptFailure::Abort { .. } => {
                            if let Some(hook) = &abort_hook {
                                hook.accept(&cas_context, failure);
                            }
                            AttemptFailureDecision::Abort
                        }
                        CasAttemptFailure::Timeout { .. } => match timeout_policy {
                            CasTimeoutPolicy::Retry => {
                                if let Some(hook) = &retry_hook {
                                    hook.accept(&cas_context, failure);
                                }
                                AttemptFailureDecision::Retry
                            }
                            CasTimeoutPolicy::Abort => {
                                if let Some(hook) = &abort_hook {
                                    hook.accept(&cas_context, failure);
                                }
                                AttemptFailureDecision::Abort
                            }
                        },
                    }
                },
            );

        if self.isolate_listener_panics {
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
    ) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
    where
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let current = state.load();
        let decision = if let Some(timeout) = self.attempt_timeout {
            match tokio::time::timeout(timeout, operation(Arc::clone(&current))).await {
                Ok(decision) => decision,
                Err(_) => return Err(CasAttemptFailure::timeout(current)),
            }
        } else {
            operation(Arc::clone(&current)).await
        };

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
        hooks: CasHooks<T, R, E>,
        success_context: Arc<Mutex<Option<RetryContext>>>,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>> {
        match attempt {
            Ok(success) => {
                let context = success_context
                    .lock()
                    .expect("CAS success context slot should be lockable")
                    .take()
                    .expect("retry success hook must capture CAS success context");
                let success = self.enrich_success(success, context);
                if let Some(hook) = hooks.success_hook() {
                    hook.accept(&success);
                }
                Ok(success)
            }
            Err(error) => Err(CasError::new(error, self.attempt_timeout)),
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
        let context = CasContext::new(&context, self.attempt_timeout);
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
}
