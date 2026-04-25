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
pub mod options;
mod success;

pub use decision::CasDecision;
pub use error::{CasAttemptFailure, CasError, CasErrorKind};
pub use event::{CasContext, CasHooks};
pub use executor::{CasBuilder, CasExecutor};
pub use options::CasTimeoutPolicy;
pub use success::CasSuccess;
