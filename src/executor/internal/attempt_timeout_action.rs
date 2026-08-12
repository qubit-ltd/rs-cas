// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS action selected for configured attempt timeouts.

/// Action selected after one configured attempt timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttemptTimeoutAction {
    /// Schedule another attempt when continuation budgets allow it.
    Retry,
    /// Terminate the CAS flow immediately.
    Abort,
}
