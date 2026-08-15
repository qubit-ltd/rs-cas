// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS error types.

mod cas_attempt_failure;
mod cas_attempt_failure_kind;
mod cas_error;
mod cas_error_kind;
mod cas_retry_failure;
mod internal;

pub use cas_attempt_failure::CasAttemptFailure;
pub use cas_attempt_failure_kind::CasAttemptFailureKind;
pub use cas_error::CasError;
pub use cas_error_kind::CasErrorKind;
pub use cas_retry_failure::CasRetryFailure;
