/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Execution reports produced by CAS flows.

mod cas_execution_outcome;
mod cas_execution_report;
mod cas_report_builder;

pub use cas_execution_outcome::CasExecutionOutcome;
pub use cas_execution_report::CasExecutionReport;
pub(crate) use cas_report_builder::CasReportBuilder;
