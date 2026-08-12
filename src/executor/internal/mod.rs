// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal execution state shared by the CAS executor implementation.

mod attempt_success;
mod attempt_timeout_action;
mod cas_report_finish_context;

pub(super) use attempt_success::AttemptSuccess;
pub(super) use attempt_timeout_action::AttemptTimeoutAction;
pub(super) use cas_report_finish_context::CasReportFinishContext;
