/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

/// Policy for panics raised by event or alert listeners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerPanicPolicy {
    /// Listener panics propagate to the caller.
    Propagate,
    /// Listener panics are isolated so the CAS flow can continue.
    Isolate,
}

impl Default for ListenerPanicPolicy {
    /// Returns the default listener panic policy.
    ///
    /// # Returns
    /// [`ListenerPanicPolicy::Propagate`] (panics bubble up to caller).
    #[inline]
    fn default() -> Self {
        Self::Propagate
    }
}
