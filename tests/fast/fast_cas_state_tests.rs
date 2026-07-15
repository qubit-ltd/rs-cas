// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::fast::CasCell as ModuleCasCell;
use qubit_cas::{
    CasCell,
    FastCasState,
};

#[test]
fn test_fast_cas_state_alias_uses_atomic_u64_api() {
    let state = FastCasState::new(1);

    assert_eq!(state.load(), 1);
    assert!(state.compare_set(1, 2).is_ok());
    assert_eq!(state.load(), 2);
}

/// Verifies that both compatibility export paths expose `CasCell`.
#[test]
fn test_cas_cell_is_reexported_from_root_and_fast_module() {
    let root_cell = CasCell::new(3);
    let module_cell = ModuleCasCell::new(4);

    assert_eq!(root_cell.load(), 3);
    assert_eq!(module_cell.load(), 4);
}
