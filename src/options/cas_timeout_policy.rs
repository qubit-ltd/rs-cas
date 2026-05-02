/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Timeout handling policy for async CAS operations.

/// Policy used when one async CAS attempt exceeds the configured timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasTimeoutPolicy {
    /// Retry the timed-out attempt while retry limits still allow it.
    Retry,
    /// Abort the CAS flow immediately when a timeout occurs.
    Abort,
}

impl Default for CasTimeoutPolicy {
    /// Returns the default timeout policy.
    ///
    /// # Returns
    /// [`CasTimeoutPolicy::Retry`].
    #[inline]
    fn default() -> Self {
        Self::Retry
    }
}
