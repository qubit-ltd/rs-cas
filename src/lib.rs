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
//!
//! # Snapshots and retries
//!
//! The executor operates on immutable `Arc<T>` snapshots. It may call an
//! operation again after a conflict or retryable failure, using a newly loaded
//! snapshot. Keep retryable operations deterministic and safe to recompute:
//! side effects performed inside an operation can happen more than once. Defer
//! external effects such as notifications or payments until execution succeeds;
//! once performed, they are not undone by a later retry, timeout, or
//! cancellation.
//!
//! A synchronous result-only execution looks like this:
//!
//! ```
//! use qubit_atomic::AtomicRef;
//! use qubit_cas::{CasDecision, CasExecutor};
//!
//! let state = AtomicRef::from_value(3usize);
//! let success = CasExecutor::<usize, ()>::builder().build().unwrap()
//!     .execute_result(&state, |current: &usize| CasDecision::update(*current - 1, *current - 1))
//!     .unwrap();
//! assert_eq!(*success.output(), 2);
//! assert_eq!(*state.load(), 2);
//! ```
//!
//! # Choosing an execution API
//!
//! | Need | Synchronous | Asynchronous (requires the `tokio` feature) |
//! | --- | --- | --- |
//! | Result only | [`CasExecutor::execute_result`] | `execute_async_result` |
//! | Execution report ([`CasExecutionReport`]) | [`CasExecutor::execute`] | `execute_async` |
//! | Per-execution hooks ([`CasHooks`]) | [`CasExecutor::execute_with_hooks`] | `execute_async_with_hooks` |
//!
//! Async methods must be polled inside an entered Tokio runtime. Configure a
//! Tokio time driver when using non-zero backoff or a hard timeout (for
//! example, enable `time` or use `enable_all()` on a manually built runtime).
//! Timeouts are cooperative: blocking code or a future that does not yield
//! cannot be preempted reliably. Cancellation drops the in-flight future, but
//! does not roll back committed state or external side effects already
//! performed.
//!
//! # Further reading
//!
//! - [English README](https://github.com/qubit-ltd/rs-cas/blob/main/README.md)
//! - [简体中文 README](https://github.com/qubit-ltd/rs-cas/blob/main/README.zh_CN.md)
//! - [English user guide](https://github.com/qubit-ltd/rs-cas/blob/main/doc/user_guide.md)
//! - [简体中文用户指南](https://github.com/qubit-ltd/rs-cas/blob/main/doc/user_guide.zh_CN.md)
//! - [English design](https://github.com/qubit-ltd/rs-cas/blob/main/doc/design.md)
//! - [简体中文设计说明](https://github.com/qubit-ltd/rs-cas/blob/main/doc/design.zh_CN.md)

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
