/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Built-in CAS preset constants.

use std::time::Duration;

use qubit_retry::constants::DEFAULT_RETRY_MAX_ATTEMPTS;

/// Default maximum attempts inherited from `qubit-retry`.
pub const DEFAULT_CAS_MAX_ATTEMPTS: u32 = DEFAULT_RETRY_MAX_ATTEMPTS;

/// Maximum attempts for the high-concurrency preset.
pub const HIGH_CONCURRENCY_MAX_ATTEMPTS: u32 = 1000;

/// Initial retry delay for the high-concurrency preset.
pub const HIGH_CONCURRENCY_INITIAL_DELAY: Duration = Duration::from_millis(50);

/// Maximum retry delay for the high-concurrency preset.
pub const HIGH_CONCURRENCY_MAX_DELAY: Duration = Duration::from_secs(30);

/// Total elapsed-time budget for the high-concurrency preset.
pub const HIGH_CONCURRENCY_MAX_ELAPSED: Duration = Duration::from_secs(60);

/// Jitter factor for the high-concurrency preset.
pub const HIGH_CONCURRENCY_JITTER_FACTOR: f64 = 0.25;

/// Maximum attempts for the low-latency preset.
pub const LOW_LATENCY_MAX_ATTEMPTS: u32 = 100;

/// Total elapsed-time budget for the low-latency preset.
pub const LOW_LATENCY_MAX_ELAPSED: Duration = Duration::from_secs(5);

/// Maximum attempts for the high-reliability preset.
pub const HIGH_RELIABILITY_MAX_ATTEMPTS: u32 = 5000;

/// Initial retry delay for the high-reliability preset.
pub const HIGH_RELIABILITY_INITIAL_DELAY: Duration = Duration::from_secs(1);

/// Maximum retry delay for the high-reliability preset.
pub const HIGH_RELIABILITY_MAX_DELAY: Duration = Duration::from_secs(300);

/// Total elapsed-time budget for the high-reliability preset.
pub const HIGH_RELIABILITY_MAX_ELAPSED: Duration = Duration::from_secs(600);

/// Jitter factor for the high-reliability preset.
pub const HIGH_RELIABILITY_JITTER_FACTOR: f64 = 0.1;
