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

import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = (
    "README.md", "README.zh_CN.md",
    "doc/user_guide.md", "doc/user_guide.zh_CN.md",
)
TOOLCHAIN = os.environ.get("RS_CI_BUILD_TOOLCHAIN", "1.94.0")
FENCE = re.compile(r"^```([\w-]*)\s*\n(.*?)^```\s*$", re.M | re.S)


def _document_text(document):
    try:
        return document.read_text(encoding="utf-8")
    except OSError as error:
        raise SystemExit(f"{document}: cannot read document: {error}") from error


def extract_installation(document: Path) -> dict:
    """Return dependencies from the document's unique TOML installation block."""
    text = _document_text(document)
    candidates = []
    for match in FENCE.finditer(text):
        if match.group(1).lower() != "toml":
            continue
        try:
            parsed = tomllib.loads(match.group(2))
        except tomllib.TOMLDecodeError as error:
            raise SystemExit(f"{document}: malformed TOML installation block: {error}") from error
        if "dependencies" in parsed:
            candidates.append(parsed["dependencies"])
    if len(candidates) != 1:
        reason = "missing" if not candidates else "multiple"
        raise SystemExit(f"{document}: expected one TOML [dependencies] installation block; found {reason}")
    dependencies = candidates[0]
    if not isinstance(dependencies, dict):
        raise SystemExit(f"{document}: [dependencies] must be a TOML table")
    for name, value in dependencies.items():
        if not isinstance(name, str) or not isinstance(value, (str, dict)) or not _supported_value(value):
            raise SystemExit(f"{document}: unsupported dependency value for {name!r}: {value!r}")
    if "qubit-cas" not in dependencies:
        raise SystemExit(f"{document}: [dependencies] must declare qubit-cas")
    return dependencies


def _supported_value(value):
    if isinstance(value, (str, bool, int, float)):
        return True
    if isinstance(value, list):
        return all(_supported_value(item) and not isinstance(item, dict) for item in value)
    if isinstance(value, dict):
        return all(isinstance(key, str) and _supported_value(item) for key, item in value.items())
    return False


def _toml_value(value):
    if isinstance(value, str):
        return '"' + value.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n').replace('\r', '\\r').replace('\t', '\\t') + '"'
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(_toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{_toml_key(key)} = {_toml_value(item)}" for key, item in value.items()) + " }"
    raise TypeError(f"unsupported TOML value: {value!r}")


def _toml_key(key):
    return key if re.fullmatch(r"[A-Za-z0-9_-]+", key) else _toml_value(key)


def consumer_manifest(project: Path, dependencies: dict) -> str:
    """Build a consumer manifest preserving declared dependencies and CAS options."""
    dependencies = dict(dependencies)
    cas = dependencies["qubit-cas"]
    if isinstance(cas, str):
        cas = {"version": cas}
    if not isinstance(cas, dict):
        raise ValueError("qubit-cas must be a version string or dependency table")
    cas = dict(cas)
    cas.pop("version", None)
    cas["path"] = str(project)
    dependencies["qubit-cas"] = cas
    lines = [
        "[package]", 'name = "rs-cas-documentation-examples"',
        'version = "0.0.0"', 'edition = "2024"', 'publish = false',
        "", "[workspace]", "", "[dependencies]",
    ]
    lines.extend(f"{_toml_key(name)} = {_toml_value(value)}" for name, value in dependencies.items())
    return "\n".join(lines) + "\n"


def extract_examples(document: Path) -> list[tuple[int, str]]:
    """Return Rust code with one-based block numbers for source attribution."""
    text = _document_text(document)
    examples = [
        (index, match.group(2))
        for index, match in enumerate(
            (match for match in FENCE.finditer(text) if match.group(1).lower() == "rust"), 1
        )
    ]
    if not examples:
        raise SystemExit(f"{document}: no executable Rust example")
    return examples


def main():
    """Run every document's Rust examples using its declared installation."""
    with tempfile.TemporaryDirectory(prefix="rs-cas-doc-examples-") as directory:
        work = Path(directory)
        target = work / "target"
        cargo = ["cargo", f"+{TOOLCHAIN}"]
        env = dict(os.environ)
        env["CARGO_TARGET_DIR"] = str(target)
        count = 0
        for document_name in DOCUMENTS:
            document = ROOT / document_name
            dependencies = extract_installation(document)
            examples = extract_examples(document)
            consumer = work / f"consumer_{count}"
            sources = consumer / "src" / "bin"
            sources.mkdir(parents=True)
            (consumer / "Cargo.toml").write_text(
                consumer_manifest(ROOT, dependencies), encoding="utf-8"
            )
            for index, code in examples:
                name = f"example_{index}"
                (sources / f"{name}.rs").write_text(code, encoding="utf-8")
            manifest_path = str(consumer / "Cargo.toml")
            try:
                subprocess.run(cargo + ["generate-lockfile", "--manifest-path", manifest_path],
                               check=True, env=env)
                for index, _ in examples:
                    name = f"example_{index}"
                    print(f"Validating {document_name} Rust block {index}", flush=True)
                    subprocess.run(cargo + ["run", "--locked", "--quiet", "--manifest-path",
                                   manifest_path, "--bin", name], check=True, env=env)
                    count += 1
            except (OSError, subprocess.CalledProcessError) as error:
                raise SystemExit(f"{document}: Rust examples failed: {error}") from error
    print(f"Validated {count} current documentation examples")


if __name__ == "__main__":
    main()
