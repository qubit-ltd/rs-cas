/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Execution reports produced by CAS flows.

mod cas_execution_outcome;
mod cas_execution_report;
mod cas_report_builder;

pub use cas_execution_outcome::CasExecutionOutcome;
pub use cas_execution_report::CasExecutionReport;
pub(crate) use cas_report_builder::CasReportBuilder;
