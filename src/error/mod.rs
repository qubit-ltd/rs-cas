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
mod cas_build_error;
mod cas_diagnostic;
mod cas_diagnostic_kind;
mod cas_error;
mod cas_error_kind;
mod cas_limit_kind;
mod cas_termination;
mod cas_timeout_scope;
mod internal;

pub use cas_attempt_failure::CasAttemptFailure;
pub use cas_attempt_failure_kind::CasAttemptFailureKind;
pub use cas_build_error::CasBuildError;
pub use cas_diagnostic::CasDiagnostic;
pub use cas_diagnostic_kind::CasDiagnosticKind;
pub use cas_error::CasError;
pub use cas_error_kind::CasErrorKind;
pub use cas_limit_kind::CasLimitKind;
pub use cas_termination::CasTermination;
pub use cas_timeout_scope::CasTimeoutScope;
