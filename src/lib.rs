/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Typed compare-and-swap executor for synchronous and asynchronous workflows.
//!
//! `CasExecutor<T, E>` binds the shared state type `T` and operation error type
//! `E`. Each execution call introduces its own business output type `R`, so one
//! executor configuration can serve multiple CAS operations over the same state.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod constants;
mod decision;
pub mod error;
pub mod event;
pub mod executor;
pub mod observability;
pub mod options;
mod outcome;
pub mod report;
pub mod strategy;
mod success;

pub use decision::CasDecision;
pub use error::{CasAttemptFailure, CasAttemptFailureKind, CasError, CasErrorKind};
pub use event::{CasContext, CasEvent, CasHooks};
pub use executor::{CasBuilder, CasExecutor};
pub use observability::{
    CasAlert, CasObservabilityConfig, CasObservabilityMode, ContentionThresholds,
    ListenerPanicPolicy,
};
pub use options::CasTimeoutPolicy;
pub use outcome::CasOutcome;
pub use report::{CasExecutionOutcome, CasExecutionReport};
pub use strategy::{CasStrategy, CasStrategyProfile};
pub use success::CasSuccess;
