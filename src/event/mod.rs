/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! CAS event context and hook types.

mod cas_context;
mod cas_hooks;

pub use cas_context::CasContext;
pub use cas_hooks::{CasAbortHook, CasHooks, CasRetryHook, CasSuccessHook};
