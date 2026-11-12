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

"""Check that every integration test source is reachable from lib_tests.rs.

This lightweight check recognizes standalone ``mod name;`` declarations. It
does not evaluate ``cfg`` conditions, expand macros, or model inline modules
and ``#[path]`` attributes, and it does not skip files based on those forms.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


MODULE_DECLARATION = re.compile(r"^\s*mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;\s*$")


def mask_non_code(source: str) -> str:
    """Mask comments and literals while preserving source offsets and lines."""
    masked = list(source)
    length = len(source)

    def blank(start: int, end: int, preserve_boundary: bool = False) -> None:
        for position in range(start, end):
            if source[position] not in "\r\n":
                masked[position] = " "
        if preserve_boundary:
            first = next((i for i in range(start, end) if source[i] not in "\r\n"), None)
            last = next((i for i in range(end - 1, start - 1, -1) if source[i] not in "\r\n"), None)
            if first is not None:
                masked[first] = "X"
            if last is not None:
                masked[last] = "X"

    def raw_start(position: int) -> tuple[int, int] | None:
        for prefix in ("br", "cr", "r"):
            if not source.startswith(prefix, position):
                continue
            cursor = position + len(prefix)
            while cursor < length and source[cursor] == "#":
                cursor += 1
            if cursor < length and source[cursor] == '"':
                return cursor, cursor - (position + len(prefix))
        return None

    def char_end(position: int) -> int | None:
        """Return the end of a simple char literal, avoiding Rust lifetimes."""
        cursor = position + 1
        if cursor >= length or source[cursor] in "\r\n'":
            return None
        if source[cursor] == "\\":
            cursor += 1
            if cursor >= length or source[cursor] in "\r\n":
                return None
            if source[cursor] == "u" and cursor + 1 < length and source[cursor + 1] == "{":
                close = source.find("}", cursor + 2)
                if close < 0:
                    return None
                cursor = close + 1
            else:
                cursor += 1
        else:
            cursor += 1
        return cursor + 1 if cursor < length and source[cursor] == "'" else None

    index = 0
    while index < length:
        if source.startswith("//", index):
            end = index + 2
            while end < length and source[end] not in "\r\n":
                end += 1
            blank(index, end)
            index = end
            continue
        if source.startswith("/*", index):
            start = index
            depth = 1
            index += 2
            while index < length and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("unclosed block comment")
            blank(start, index)
            continue

        literal_start = index
        raw = raw_start(index)
        if raw is None and index + 1 < length and source[index] in "bc" and source[index + 1] == '"':
            quote = index + 1
            raw = None
            ordinary = True
        elif raw is not None:
            quote, hashes = raw
            terminator = '"' + "#" * hashes
            end_quote = source.find(terminator, quote + 1)
            if end_quote < 0:
                raise ValueError("unclosed raw string literal")
            index = end_quote + len(terminator)
            blank(literal_start, index, preserve_boundary=True)
            continue
        elif source[index] == '"':
            quote = index
            ordinary = True
        else:
            ordinary = False

        if ordinary:
            index = quote + 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("unclosed string literal")
            blank(literal_start, index, preserve_boundary=True)
            continue

        char_position = index + 1 if source.startswith("b'", index) else index
        if source[char_position] == "'":
            end = char_end(char_position)
            if end is not None:
                blank(index, end, preserve_boundary=True)
                index = end
                continue
        index += 1

    return "".join(masked)


def module_candidates(parent: Path, name: str, harness: Path) -> tuple[Path, Path]:
    """Return the two Rust file layouts accepted for a child module."""
    directory = parent.parent if parent == harness or parent.name == "mod.rs" else parent.parent / parent.stem
    return directory / f"{name}.rs", directory / name / "mod.rs"


def walk_modules(source: Path, harness: Path, reachable: set[Path], errors: list[str]) -> None:
    """Visit one module and record missing child declarations."""
    source = source.resolve()
    if source in reachable:
        return
    if not source.is_file():
        errors.append(f"declared module does not exist: {source}")
        return
    reachable.add(source)
    try:
        lines = mask_non_code(source.read_text(encoding="utf-8")).splitlines()
    except ValueError as error:
        errors.append(f"cannot lex {source}: {error}")
        return
    for line in lines:
        match = MODULE_DECLARATION.match(line)
        if match is None:
            continue
        name = match.group(1)
        first, second = module_candidates(source, name, harness)
        child = first if first.is_file() else second
        walk_modules(child, harness, reachable, errors)


def check(test_root: Path, harness: Path) -> int:
    """Validate the test module graph and return a process status."""
    test_root = test_root.resolve()
    harness = harness.resolve()
    reachable: set[Path] = set()
    errors: list[str] = []
    walk_modules(harness, harness, reachable, errors)

    expected = {
        path.resolve()
        for path in test_root.rglob("*.rs")
        if path.relative_to(test_root).parts[0] != "support"
    }
    expected.discard(harness)
    orphaned = sorted(expected - reachable)
    if errors or orphaned:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        for path in orphaned:
            print(f"error: test file is not reachable from {harness}: {path}", file=sys.stderr)
        return 1

    print(f"Checked {len(expected)} integration test Rust source files from {harness}")
    return 0


def main() -> int:
    """Parse optional paths and run the manifest check."""
    root = Path(__file__).resolve().parents[1]
    test_root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "tests"
    harness = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else test_root / "lib_tests.rs"
    return check(test_root, harness)


if __name__ == "__main__":
    raise SystemExit(main())
