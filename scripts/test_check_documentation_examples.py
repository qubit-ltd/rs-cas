import importlib.util
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_documentation_examples", Path(__file__).with_name("check-documentation-examples.py")
)
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class DocumentationExamplesTests(unittest.TestCase):
    def document(self, content):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "guide.md"
        path.write_text(content, encoding="utf-8")
        return path

    def test_manifest_preserves_cas_version_and_dependencies(self):
        project = Path("/local/qubit-cas")
        dependencies = {
            "qubit-cas": {"version": "0.12", "features": ["tokio"]},
            "qubit-atomic": "0.13",
        }
        parsed = tomllib.loads(checker.consumer_manifest(project, dependencies))
        self.assertEqual(parsed["dependencies"]["qubit-atomic"], "0.13")
        self.assertEqual(parsed["dependencies"]["qubit-cas"]["features"], ["tokio"])
        self.assertEqual(parsed["dependencies"]["qubit-cas"]["path"], str(project))
        self.assertEqual(parsed["dependencies"]["qubit-cas"]["version"], "0.12")

    def test_cas_simple_string_is_converted(self):
        parsed = tomllib.loads(checker.consumer_manifest(Path("/local/cas"), {"qubit-cas": "0.12"}))
        self.assertEqual(parsed["dependencies"]["qubit-cas"]["path"], "/local/cas")
        self.assertEqual(parsed["dependencies"]["qubit-cas"]["version"], "0.12")

    def test_cargo_rejects_incompatible_documented_cas_version(self):
        with tempfile.TemporaryDirectory(prefix="cas-version-contract-") as directory:
            work = Path(directory)
            project = work / "cas"
            (project / "src").mkdir(parents=True)
            (project / "src" / "lib.rs").write_text("", encoding="utf-8")
            (project / "Cargo.toml").write_text(
                '[package]\nname="qubit-cas"\nversion="0.12.2"\nedition="2024"\n'
                '[workspace]\n', encoding="utf-8"
            )
            for index, version in enumerate(("0.12", "999.0")):
                consumer = work / f"consumer_{index}"
                (consumer / "src").mkdir(parents=True)
                (consumer / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
                manifest = consumer / "Cargo.toml"
                manifest.write_text(
                    checker.consumer_manifest(project, {"qubit-cas": version}),
                    encoding="utf-8"
                )
                result = subprocess.run(
                    ["cargo", f"+{checker.TOOLCHAIN}", "generate-lockfile", "--offline",
                     "--manifest-path", str(manifest)],
                    capture_output=True, text=True, check=False
                )
                if version == "0.12":
                    self.assertEqual(result.returncode, 0, result.stderr)
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("qubit-cas", result.stderr)
                    self.assertIn("999.0", result.stderr)

    def test_default_features_false_is_preserved(self):
        dependencies = {"qubit-cas": {"version": "0.12", "default-features": False}}
        parsed = tomllib.loads(checker.consumer_manifest(Path("/cas"), dependencies))
        self.assertFalse(parsed["dependencies"]["qubit-cas"]["default-features"])

    def test_sync_readme_dependencies_do_not_gain_tokio(self):
        parsed = tomllib.loads(checker.consumer_manifest(Path("/cas"), {"qubit-cas": "0.12", "qubit-atomic": "0.17"}))
        self.assertNotIn("features", parsed["dependencies"]["qubit-cas"])
        self.assertNotIn("tokio", parsed["dependencies"])

    def test_missing_duplicate_and_malformed_installation_blocks_fail_with_context(self):
        cases = (
            ("# guide", "installation"),
            ("```toml\n[dependencies]\nqubit-cas='0.12'\n```\n```toml\n[dependencies]\nqubit-cas='0.12'\n```", "guide.md"),
            ("```toml\n[dependencies\n```", "guide.md"),
        )
        for content, context in cases:
            with self.subTest(content=content):
                path = self.document(content)
                with self.assertRaises(SystemExit) as caught:
                    checker.extract_installation(path)
                self.assertIn(context, str(caught.exception))

    def test_unsupported_dependency_shapes_fail_with_document_context(self):
        path = self.document("```toml\n[dependencies]\nqubit-cas = 7\n```\n")
        with self.assertRaises(SystemExit) as caught:
            checker.extract_installation(path)
        self.assertIn("guide.md", str(caught.exception))

    def test_rust_examples_keep_document_and_block_attribution(self):
        path = self.document("```rust\nfn main() {}\n```\n```rust\nfn main() {}\n```\n")
        examples = checker.extract_examples(path)
        self.assertEqual([entry[0] for entry in examples], [1, 2])
        self.assertEqual([entry[1].strip() for entry in examples], ["fn main() {}", "fn main() {}"])


if __name__ == "__main__":
    unittest.main()
