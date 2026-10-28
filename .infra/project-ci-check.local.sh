#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$project_root"

python3 -c 'import sys; assert sys.version_info >= (3, 11), "Python 3.11+ is required"'
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check-test-manifest.py
python3 scripts/check-documentation-examples.py
cargo +"${RS_CI_BUILD_TOOLCHAIN:-1.94.0}" rustdoc --locked --lib --all-features -- -D warnings --document-private-items
