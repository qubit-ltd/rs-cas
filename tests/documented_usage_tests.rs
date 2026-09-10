// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Executable examples for inventory and diagnostics.

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

#[test]
fn test_inventory_example_retains_business_errors() {
    let stock = AtomicRef::from_value(1usize);
    let cas = CasExecutor::<usize, &'static str>::latency_first();
    let first = cas
        .execute_result(&stock, |n: &usize| {
            if *n == 0 {
                CasDecision::abort("out of stock")
            } else {
                CasDecision::update(*n - 1, *n - 1)
            }
        })
        .expect("stock available");
    assert_eq!(*first.output(), 0);
    let error = cas
        .execute_result(&stock, |_: &usize| {
            CasDecision::<usize, (), &'static str>::abort("out of stock")
        })
        .expect_err("stock exhausted");
    assert_eq!(error.error(), Some(&"out of stock"));
    assert!(error.diagnostic().is_none());
    assert!(error.completion_diagnostics().is_empty());
}
