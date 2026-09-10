#!/usr/bin/env python3
################################################################################
#
#    Copyright (c) 2025 - 2026 Haixing Hu.
#
#    SPDX-License-Identifier: Apache-2.0
#
#    Licensed under the Apache License, Version 2.0.
#
################################################################################

"""Check that every integration test source is reachable from lib_tests.rs."""

from __future__ import annotations

import re
import sys
from pathlib import Path


MODULE_DECLARATION = re.compile(r"^\s*mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;\s*$")


def module_candidates(parent: Path, name: str) -> tuple[Path, Path]:
    """Return the two Rust file layouts accepted for a child module."""
    directory = parent.parent
    return directory / f"{name}.rs", directory / name / "mod.rs"


def walk_modules(source: Path, reachable: set[Path], errors: list[str]) -> None:
    """Visit one module and record missing child declarations."""
    source = source.resolve()
    if source in reachable:
        return
    if not source.is_file():
        errors.append(f"declared module does not exist: {source}")
        return
    reachable.add(source)
    for line in source.read_text(encoding="utf-8").splitlines():
        match = MODULE_DECLARATION.match(line)
        if match is None:
            continue
        name = match.group(1)
        if name == "tests":
            continue
        first, second = module_candidates(source, name)
        child = first if first.is_file() else second
        walk_modules(child, reachable, errors)


def check(test_root: Path, harness: Path) -> int:
    """Validate the test module graph and return a process status."""
    test_root = test_root.resolve()
    harness = harness.resolve()
    reachable: set[Path] = set()
    errors: list[str] = []
    walk_modules(harness, reachable, errors)

    expected = {
        path.resolve()
        for path in test_root.rglob("*_tests.rs")
        if "support" not in path.parts
    }
    expected.discard(harness)
    orphaned = sorted(expected - reachable)
    if errors or orphaned:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        for path in orphaned:
            print(f"error: test file is not reachable from {harness}: {path}", file=sys.stderr)
        return 1

    print(f"Checked {len(expected)} integration test files from {harness}")
    return 0


def main() -> int:
    """Parse optional paths and run the manifest check."""
    root = Path(__file__).resolve().parents[1]
    test_root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "tests"
    harness = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else test_root / "lib_tests.rs"
    return check(test_root, harness)


if __name__ == "__main__":
    raise SystemExit(main())
