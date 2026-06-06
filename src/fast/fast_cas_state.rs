// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared atomic state for fast compare-and-swap operations.

use qubit_atomic::Atomic;

/// Shared atomic state used by [`crate::FastCas`].
///
/// This is a semantic alias over [`Atomic<usize>`]. The stored value is usually
/// a compact state code assigned by an upper-level abstraction such as a state
/// machine.
pub type FastCasState = Atomic<usize>;
