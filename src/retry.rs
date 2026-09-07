// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Public retry configuration and diagnostic types used by `qubit-cas`.
//!
//! The re-exports in this module keep the retry types used by the CAS public
//! API available through one dependency. They preserve the underlying
//! `qubit_retry` type identities, so existing calls such as
//! [`crate::CasExecutor::from_policy`] remain source compatible.

pub use qubit_retry::BackoffPolicy;
pub use qubit_retry::RetryCallbackFailure;
pub use qubit_retry::RetryCancellationPhase;
pub use qubit_retry::RetryInfrastructureFailure;
pub use qubit_retry::RetryLimitKind;
pub use qubit_retry::RetryPolicy;
pub use qubit_retry::RetryPolicyError;
pub use qubit_retry::RetryTimeoutScope;
