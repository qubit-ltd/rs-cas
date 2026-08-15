// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Panic handling policy for event and alert listeners.

/// Policy for panics raised by event or alert listeners.
///
/// The default is [`Self::Propagate`], which exposes a listener panic to the
/// boundary that owns that listener invocation. Retry-owned
/// `AttemptFailed`/`RetryRequested` invocations are owned by `qubit-retry`, so
/// their panics become structured [`crate::CasRetryFailure::CallbackFailed`]
/// values. Outer `ExecutionStarted`/`ExecutionFinished` event invocations and
/// alert invocations are owned by the CAS execution call and therefore unwind
/// through it. Select [`Self::Isolate`] to catch listener panics at dispatch
/// and allow the CAS flow to continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerPanicPolicy {
    /// Exposes a listener panic to its owning execution boundary.
    Propagate,
    /// Listener panics are isolated so the CAS flow can continue.
    Isolate,
}

impl Default for ListenerPanicPolicy {
    /// Returns the default listener panic policy.
    ///
    /// # Returns
    /// [`ListenerPanicPolicy::Propagate`], which exposes a panic to the
    /// boundary that owns the listener invocation.
    #[inline]
    fn default() -> Self {
        Self::Propagate
    }
}
