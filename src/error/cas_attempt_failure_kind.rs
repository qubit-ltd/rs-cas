/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Lightweight kind of attempt-level CAS failure.

/// Lightweight kind of attempt-level CAS failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasAttemptFailureKind {
    /// Compare-and-swap failed because another writer changed the state first.
    Conflict,
    /// Business logic requested another attempt.
    Retry,
    /// Business logic aborted the flow.
    Abort,
    /// An async attempt exceeded its timeout.
    Timeout,
}
