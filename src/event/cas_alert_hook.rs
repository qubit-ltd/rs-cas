/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Shared hook type for CAS alerts.

use qubit_function::ArcConsumer;

use crate::observability::CasAlert;

/// Shared hook invoked for CAS alerts.
pub type CasAlertHook = ArcConsumer<CasAlert>;
