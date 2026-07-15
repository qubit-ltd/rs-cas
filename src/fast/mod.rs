// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compatibility exports for lightweight `u64` compare-and-swap primitives.
//!
//! The implementation lives in the standalone `qubit-fast-cas` crate. This
//! module preserves the established `qubit_cas::fast` import path while new
//! consumers that only need the lightweight primitives can depend directly on
//! `qubit-fast-cas`.
//!
//! ## Compared with [`CasExecutor`](crate::CasExecutor)
//!
//! [`CasExecutor`](crate::CasExecutor) carries typed state `T`, optional
//! timeouts, observability, and richer outcomes. [`FastCas`] adds bounded
//! conflict policy to a [`CasCell`], while [`CasCell`] itself supplies reusable
//! unbounded functional updates over one atomic `u64` state word.

pub use qubit_fast_cas::{
    CasCell,
    FastCas,
    FastCasDecision,
    FastCasError,
    FastCasPolicy,
    FastCasState,
    FastCasSuccess,
};
