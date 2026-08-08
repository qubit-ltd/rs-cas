// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Built-in CAS preset constants.

use std::time::Duration;

use qubit_retry::constants::DEFAULT_RETRY_MAX_ATTEMPTS;

/// Default maximum attempts inherited from `qubit-retry`.
pub const DEFAULT_CAS_MAX_ATTEMPTS: u32 = DEFAULT_RETRY_MAX_ATTEMPTS;

/// Maximum attempts for the contention-adaptive strategy.
pub const CONTENTION_ADAPTIVE_MAX_ATTEMPTS: u32 = 64;

/// Initial retry delay for the contention-adaptive strategy.
pub const CONTENTION_ADAPTIVE_INITIAL_DELAY: Duration =
    Duration::from_micros(50);

/// Maximum retry delay for the contention-adaptive strategy.
pub const CONTENTION_ADAPTIVE_MAX_DELAY: Duration = Duration::from_millis(5);

/// Cumulative user operation elapsed-time budget for the contention-adaptive
/// strategy.
pub const CONTENTION_ADAPTIVE_MAX_ELAPSED: Duration = Duration::from_millis(50);

/// Monotonic total retry-flow elapsed-time ceiling for the contention-adaptive
/// strategy.
///
/// Includes user operation time, retry sleeps, and control-path listener work.
/// The value is above [`CONTENTION_ADAPTIVE_MAX_ELAPSED`]
/// so exponential backoff can use part of the wall-time budget.
pub const CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED: Duration =
    Duration::from_millis(250);

/// Jitter factor for the contention-adaptive strategy.
pub const CONTENTION_ADAPTIVE_JITTER_FACTOR: f64 = 0.25;

/// Maximum attempts for the latency-first strategy.
pub const LATENCY_FIRST_MAX_ATTEMPTS: u32 = 100;

/// Cumulative user operation elapsed-time budget for the latency-first
/// strategy.
pub const LATENCY_FIRST_MAX_ELAPSED: Duration = Duration::from_millis(5);

/// Monotonic total retry-flow elapsed-time ceiling for the latency-first
/// strategy.
pub const LATENCY_FIRST_MAX_TOTAL_ELAPSED: Duration = Duration::from_millis(20);

/// Maximum attempts for the reliability-first strategy.
pub const RELIABILITY_FIRST_MAX_ATTEMPTS: u32 = 128;

/// Initial retry delay for the reliability-first strategy.
pub const RELIABILITY_FIRST_INITIAL_DELAY: Duration = Duration::from_millis(1);

/// Maximum retry delay for the reliability-first strategy.
pub const RELIABILITY_FIRST_MAX_DELAY: Duration = Duration::from_millis(100);

/// Cumulative user operation elapsed-time budget for the reliability-first
/// strategy.
pub const RELIABILITY_FIRST_MAX_ELAPSED: Duration = Duration::from_secs(5);

/// Monotonic total retry-flow elapsed-time ceiling for the reliability-first
/// strategy.
///
/// Set above [`RELIABILITY_FIRST_MAX_ELAPSED`] so long
/// exponential backoff windows can still fit under a hard end-to-end cap.
pub const RELIABILITY_FIRST_MAX_TOTAL_ELAPSED: Duration =
    Duration::from_secs(10);

/// Jitter factor for the reliability-first strategy.
pub const RELIABILITY_FIRST_JITTER_FACTOR: f64 = 0.1;
