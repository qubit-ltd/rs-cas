// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Typed compare-and-swap executor for synchronous and asynchronous workflows.
//!
//! `CasExecutor<T, E>` binds the shared state type `T` and operation error type
//! `E`. Each execution call introduces its own business output type `R`, so one
//! executor configuration can serve multiple CAS operations over the same
//! state.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod cas_decision;
mod cas_outcome;
mod cas_success;
pub mod constants;
pub mod error;
pub mod event;
pub mod executor;
mod observability;
pub mod report;
pub mod strategy;

pub use cas_decision::CasDecision;
pub use cas_outcome::CasOutcome;
pub use cas_success::CasSuccess;
pub use error::CasAttemptFailure;
pub use error::CasAttemptFailureKind;
pub use error::CasBoxError;
pub use error::CasBuildError;
pub use error::CasDiagnostic;
pub use error::CasDiagnosticKind;
pub use error::CasError;
pub use error::CasErrorKind;
pub use error::CasLimitKind;
pub use error::CasTermination;
pub use error::CasTimeoutScope;
pub use event::CasContext;
pub use event::CasEvent;
pub use event::CasHooks;
pub use event::CasListenerFailure;
pub use event::CasListenerKind;
pub use executor::CasBuilder;
pub use executor::CasExecutor;
pub use observability::CasAlert;
pub use observability::ContentionThresholds;
pub use report::CasExecutionOutcome;
pub use report::CasExecutionReport;
pub use strategy::CasStrategy;
pub use strategy::CasStrategyProfile;

#[cfg(test)]
mod tests;
