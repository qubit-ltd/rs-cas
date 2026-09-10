// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for `qubit-cas`.

mod error;
mod event;
mod executor;
mod observability;
mod report;
mod strategy;
mod support;

// Public contracts share one harness; allocation counting remains thread-local.
mod allocation_tests;
mod async_contract_tests;
mod cas_concurrency_tests;
mod cas_decision_tests;
mod cas_outcome_tests;
mod cas_success_tests;
mod constants_tests;
mod documented_usage_tests;
mod execution_contract_tests;
mod executor_config_tests;
mod listener_isolation_tests;
