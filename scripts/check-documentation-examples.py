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

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = (
    "README.md", "README.zh_CN.md",
    "doc/user_guide.md", "doc/user_guide.zh_CN.md",
)
TOOLCHAIN = os.environ.get("RS_CI_BUILD_TOOLCHAIN", "1.94.0")

def main():
    """Run each current Rust example in an isolated consumer package."""
    blocks = []
    for document in DOCUMENTS:
        text = (ROOT / document).read_text(encoding="utf-8")
        matches = list(re.finditer(r"^```rust\s*\n(.*?)^```\s*$", text, re.M | re.S))
        if not matches:
            raise SystemExit(f"No executable Rust example: {document}")
        for index, match in enumerate(matches):
            blocks.append((f"example_{len(blocks)}", document, index + 1, match.group(1)))
    with tempfile.TemporaryDirectory(prefix="rs-cas-doc-examples-") as directory:
        work = Path(directory)
        sources = work / "src" / "bin"
        sources.mkdir(parents=True)
        manifest = '\n'.join((
            '[package]', 'name = "rs-cas-documentation-examples"',
            'version = "0.0.0"', 'edition = "2024"', 'publish = false',
            '[workspace]', '[dependencies]',
            'qubit-cas = { path = ' + json.dumps(str(ROOT)) + ', features = ["tokio"] }',
            'qubit-atomic = "0.17"',
            'tokio = { version = "1.52", features = ["macros", "rt-multi-thread", "time"] }',
            '',
        ))
        (work / "Cargo.toml").write_text(manifest, encoding="utf-8")
        for name, document, index, code in blocks:
            (sources / f"{name}.rs").write_text(code, encoding="utf-8")
        cargo = ["cargo", f"+{TOOLCHAIN}"]
        env = dict(os.environ)
        env["CARGO_TARGET_DIR"] = str(work / "target")
        subprocess.run(cargo + ["generate-lockfile", "--manifest-path", str(work / "Cargo.toml")],
                       check=True, env=env)
        for name, document, index, code in blocks:
            print(f"Validating {document} Rust block {index}", flush=True)
            subprocess.run(cargo + ["run", "--locked", "--quiet", "--manifest-path",
                           str(work / "Cargo.toml"), "--bin", name], check=True, env=env)
    print(f"Validated {len(blocks)} current documentation examples")

if __name__ == "__main__":
    main()
