#!/usr/bin/env bash
################################################################################
#
#    Copyright (c) 2025 - 2026 Haixing Hu.
#
#    SPDX-License-Identifier: Apache-2.0
#
#    Licensed under the Apache License, Version 2.0.
#
################################################################################

set -euo pipefail
cas_project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$cas_project_root"
python3 scripts/check-documentation-examples.py
RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -D warnings -D missing-docs" \
    cargo +"${RS_CI_BUILD_TOOLCHAIN:-1.94.0}" doc \
    --locked --all-features --no-deps --document-private-items
