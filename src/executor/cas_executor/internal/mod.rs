// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private observer owned by the CAS executor.

mod cas_retry_observer;

pub(super) use cas_retry_observer::CasRetryObserver;
