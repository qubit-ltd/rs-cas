/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Ultra-light compare-and-swap primitives for hot `usize` state paths.
//!
//! This module provides [`FastCas`], a minimal executor over [`FastCasState`]
//! (`Atomic<usize>`). Use it when shared machine or protocol state fits in a
//! single integer code and you want **no allocation**, **no reporting hooks**,
//! and **retry logic that only reacts to CAS conflicts** (lost races), not
//! business validation failures.
//!
//! ## Compared with [`CasExecutor`](crate::CasExecutor)
//!
//! [`CasExecutor`](crate::CasExecutor) carries typed state `T`, optional timeouts, observability,
//! and richer outcomes. [`FastCas`] trades those features for predictable
//! overhead: the operation closure returns [`FastCasDecision`] and may run
//! **multiple times** after conflicts, so it must stay cheap and side-effect
//! free except through the returned decision.
//!
//! ## Operation shapes
//!
//! - [`FastCas::execute`] — full control via [`FastCasDecision`] (`Update` /
//!   `Finish` / `Abort`).
//! - [`FastCas::update_by`] — convenience when you already express logic as
//!   `Result<(next, output), error>`.
//! - [`FastCas::compare_update`] / [`FastCas::compare_update_with`] — a single
//!   fixed transition `expected → next`; **one** atomic compare-and-swap (no
//!   spin/yield policy).
//!
//! Retry policies ([`FastCasPolicy`]) apply to [`FastCas::execute`] and
//! [`FastCas::update_by`] only.

mod fast_cas;
mod fast_cas_decision;
mod fast_cas_error;
mod fast_cas_policy;
mod fast_cas_state;
mod fast_cas_success;

pub use fast_cas::FastCas;
pub use fast_cas_decision::FastCasDecision;
pub use fast_cas_error::FastCasError;
pub use fast_cas_policy::FastCasPolicy;
pub use fast_cas_state::FastCasState;
pub use fast_cas_success::FastCasSuccess;
