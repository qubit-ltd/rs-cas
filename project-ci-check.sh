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
python3 scripts/check-test-manifest.py
python3 scripts/check-documentation-examples.py
RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -D warnings -D missing-docs" \
    cargo +"${RS_CI_BUILD_TOOLCHAIN:-1.94.0}" doc \
    --locked --all-features --no-deps --document-private-items

downstream_root="${RS_CI_DOWNSTREAM_ROOT:-$cas_project_root/../rs-state-machine}"
if [ -f "$downstream_root/Cargo.toml" ]; then
    downstream_target="${RS_CI_DOWNSTREAM_TARGET_DIR:-${CARGO_TARGET_DIR:-$cas_project_root/target/rs-cas-downstream-ci}/rs-state-machine}"
    CARGO_TARGET_DIR="$downstream_target" \
        cargo +"${RS_CI_BUILD_TOOLCHAIN:-1.94.0}" test \
        --manifest-path "$downstream_root/Cargo.toml" \
        --no-default-features --features standard --locked
else
    echo "No local rs-state-machine downstream found; skipping downstream compatibility check."
fi
