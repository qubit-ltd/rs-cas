/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Typed compare-and-swap executor for synchronous and asynchronous workflows.
//!
//! `CasExecutor<T, E>` binds the shared state type `T` and operation error type
//! `E`. Each execution call introduces its own business output type `R`, so one
//! executor configuration can serve multiple CAS operations over the same state.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod cas_decision;
mod cas_outcome;
mod cas_success;
pub mod constants;
pub mod error;
pub mod event;
pub mod executor;
pub mod observability;
pub mod options;
pub mod report;
pub mod strategy;

pub use cas_decision::CasDecision;
pub use cas_outcome::CasOutcome;
pub use cas_success::CasSuccess;
pub use error::{CasAttemptFailure, CasAttemptFailureKind, CasError, CasErrorKind};
pub use event::{CasContext, CasEvent, CasHooks};
pub use executor::{CasBuilder, CasExecutor};
pub use observability::{
    CasAlert, CasObservabilityConfig, CasObservabilityMode, ContentionThresholds,
    ListenerPanicPolicy,
};
pub use options::CasTimeoutPolicy;
pub use report::{CasExecutionOutcome, CasExecutionReport};
pub use strategy::{CasStrategy, CasStrategyProfile};
