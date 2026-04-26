/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Shared hook type for CAS alerts.

use qubit_function::ArcConsumer;

use crate::observability::CasAlert;

/// Shared hook invoked for CAS alerts.
pub type CasAlertHook = ArcConsumer<CasAlert>;
