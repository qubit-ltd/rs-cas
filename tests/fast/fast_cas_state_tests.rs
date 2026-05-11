/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_cas::FastCasState;

#[test]
fn test_fast_cas_state_alias_uses_atomic_usize_api() {
    let state = FastCasState::new(1);

    assert_eq!(state.load(), 1);
    assert!(state.compare_set(1, 2).is_ok());
    assert_eq!(state.load(), 2);
}
