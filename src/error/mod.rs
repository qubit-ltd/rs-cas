/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! CAS error types.

mod cas_attempt_failure;
mod cas_attempt_failure_kind;
mod cas_error;

pub use cas_attempt_failure::CasAttemptFailure;
pub use cas_attempt_failure_kind::CasAttemptFailureKind;
pub use cas_error::{CasError, CasErrorKind};
