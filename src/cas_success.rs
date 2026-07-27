// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Successful CAS execution results.

use std::sync::Arc;

use crate::event::CasContext;

/// Successful result returned by [`crate::CasExecutor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CasSuccess<T, R> {
    /// The executor installed a new state by compare-and-swap.
    Updated {
        /// Previous state observed by the successful attempt.
        previous: Arc<T>,
        /// Current state after the successful update.
        current: Arc<T>,
        /// Business output returned by the operation.
        output: R,
        /// Retry context captured when the flow completed.
        context: CasContext,
    },

    /// The executor finished successfully without writing a new state.
    Finished {
        /// Current state that remained installed.
        current: Arc<T>,
        /// Business output returned by the operation.
        output: R,
        /// Retry context captured when the flow completed.
        context: CasContext,
    },
}

impl<T, R> CasSuccess<T, R> {
    /// Creates an updated success result.
    ///
    /// # Parameters
    /// - `previous`: Previous state.
    /// - `current`: Current state after the update.
    /// - `output`: Business output.
    /// - `context`: Retry context for the completed flow.
    ///
    /// # Returns
    /// A [`CasSuccess::Updated`] value.
    #[inline]
    pub(crate) fn updated(
        previous: Arc<T>,
        current: Arc<T>,
        output: R,
        context: CasContext,
    ) -> Self {
        Self::Updated {
            previous,
            current,
            output,
            context,
        }
    }

    /// Creates a finished success result.
    ///
    /// # Parameters
    /// - `current`: Current state.
    /// - `output`: Business output.
    /// - `context`: Retry context for the completed flow.
    ///
    /// # Returns
    /// A [`CasSuccess::Finished`] value.
    #[inline]
    pub(crate) fn finished(current: Arc<T>, output: R, context: CasContext) -> Self {
        Self::Finished {
            current,
            output,
            context,
        }
    }

    /// Returns whether this success performed a write.
    ///
    /// # Returns
    /// `true` for [`CasSuccess::Updated`], `false` for
    /// [`CasSuccess::Finished`].
    #[inline(always)]
    pub fn is_updated(&self) -> bool {
        matches!(self, Self::Updated { .. })
    }

    /// Returns the previous state when a write occurred.
    ///
    /// # Returns
    /// `Some(&Arc<T>)` for [`CasSuccess::Updated`], or `None` when no write
    /// occurred.
    #[inline(always)]
    pub fn previous(&self) -> Option<&Arc<T>> {
        match self {
            Self::Updated { previous, .. } => Some(previous),
            Self::Finished { .. } => None,
        }
    }

    /// Returns the current state after success.
    ///
    /// # Returns
    /// Shared reference to the current state.
    #[inline(always)]
    pub fn current(&self) -> &Arc<T> {
        match self {
            Self::Updated { current, .. } | Self::Finished { current, .. } => current,
        }
    }

    /// Returns the business output.
    ///
    /// # Returns
    /// Shared reference to the business output.
    #[inline(always)]
    pub fn output(&self) -> &R {
        match self {
            Self::Updated { output, .. } | Self::Finished { output, .. } => output,
        }
    }

    /// Consumes the success result and returns the business output.
    ///
    /// # Returns
    /// The owned business output.
    #[inline(always)]
    pub fn into_output(self) -> R {
        match self {
            Self::Updated { output, .. } | Self::Finished { output, .. } => output,
        }
    }

    /// Returns the retry context captured at success.
    ///
    /// # Returns
    /// Retry context for the completed flow.
    #[inline(always)]
    pub fn context(&self) -> CasContext {
        match self {
            Self::Updated { context, .. } | Self::Finished { context, .. } => *context,
        }
    }

    /// Returns the number of attempts that were executed.
    ///
    /// # Returns
    /// One-based attempt count.
    #[inline(always)]
    pub fn attempts(&self) -> u32 {
        self.context().attempt()
    }
}
