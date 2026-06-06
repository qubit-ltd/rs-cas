// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared hook type for CAS lifecycle events.

use qubit_function::ArcConsumer;

use super::CasEvent;

/// Shared hook invoked for CAS lifecycle events.
pub type CasEventHook = ArcConsumer<CasEvent>;
