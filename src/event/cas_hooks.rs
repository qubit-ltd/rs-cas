/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! CAS hook registrations.

use qubit_function::{ArcBiConsumer, ArcConsumer, BiConsumer, Consumer};

use crate::error::CasAttemptFailure;
use crate::success::CasSuccess;

use super::CasContext;

/// Shared hook invoked after a successful CAS flow completes.
pub type CasSuccessHook<T, R> = ArcConsumer<CasSuccess<T, R>>;

/// Shared hook invoked when an attempt failure will be retried.
pub type CasRetryHook<T, E> = ArcBiConsumer<CasContext, CasAttemptFailure<T, E>>;

/// Shared hook invoked when an attempt failure aborts the CAS flow.
pub type CasAbortHook<T, E> = ArcBiConsumer<CasContext, CasAttemptFailure<T, E>>;

/// Per-execution hooks for observing CAS lifecycle events.
#[derive(Clone)]
pub struct CasHooks<T, R, E> {
    /// Hook invoked after a successful CAS flow completes.
    on_success: Option<CasSuccessHook<T, R>>,
    /// Hook invoked when an attempt failure will be retried.
    on_retry: Option<CasRetryHook<T, E>>,
    /// Hook invoked when an attempt failure aborts the CAS flow.
    on_abort: Option<CasAbortHook<T, E>>,
}

impl<T, R, E> Default for CasHooks<T, R, E> {
    /// Creates an empty hook set.
    ///
    /// # Returns
    /// A [`CasHooks`] value with every hook unset.
    #[inline]
    fn default() -> Self {
        Self {
            on_success: None,
            on_retry: None,
            on_abort: None,
        }
    }
}

impl<T, R, E> CasHooks<T, R, E> {
    /// Creates an empty hook set.
    ///
    /// # Returns
    /// A [`CasHooks`] value with every hook unset.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a success hook.
    ///
    /// # Parameters
    /// - `hook`: Hook receiving the final CAS success value.
    ///
    /// # Returns
    /// The updated hook set.
    pub fn on_success<C>(mut self, hook: C) -> Self
    where
        C: Consumer<CasSuccess<T, R>> + Send + Sync + 'static,
    {
        self.on_success = Some(hook.into_arc());
        self
    }

    /// Registers a retry hook.
    ///
    /// # Parameters
    /// - `hook`: Hook receiving the retry context and attempt failure that
    ///   triggered another attempt.
    ///
    /// # Returns
    /// The updated hook set.
    pub fn on_retry<C>(mut self, hook: C) -> Self
    where
        C: BiConsumer<CasContext, CasAttemptFailure<T, E>> + Send + Sync + 'static,
    {
        self.on_retry = Some(hook.into_arc());
        self
    }

    /// Registers an abort hook.
    ///
    /// # Parameters
    /// - `hook`: Hook receiving the context and attempt failure that
    ///   aborted the CAS flow.
    ///
    /// # Returns
    /// The updated hook set.
    pub fn on_abort<C>(mut self, hook: C) -> Self
    where
        C: BiConsumer<CasContext, CasAttemptFailure<T, E>> + Send + Sync + 'static,
    {
        self.on_abort = Some(hook.into_arc());
        self
    }

    /// Returns the registered success hook.
    ///
    /// # Returns
    /// Optional shared success hook.
    #[inline]
    pub(crate) fn success_hook(&self) -> Option<CasSuccessHook<T, R>> {
        self.on_success.clone()
    }

    /// Returns the registered retry hook.
    ///
    /// # Returns
    /// Optional shared retry hook.
    #[inline]
    pub(crate) fn retry_hook(&self) -> Option<CasRetryHook<T, E>> {
        self.on_retry.clone()
    }

    /// Returns the registered abort hook.
    ///
    /// # Returns
    /// Optional shared abort hook.
    #[inline]
    pub(crate) fn abort_hook(&self) -> Option<CasAbortHook<T, E>> {
        self.on_abort.clone()
    }
}
