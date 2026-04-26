/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Shared hook type for CAS lifecycle events.

use qubit_function::ArcConsumer;

use super::CasEvent;

/// Shared hook invoked for CAS lifecycle events.
pub type CasEventHook = ArcConsumer<CasEvent>;
