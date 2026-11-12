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

"""Regression tests for the integration test source manifest checker."""

import contextlib
import io
import runpy
import tempfile
import unittest
from pathlib import Path


CHECKER = runpy.run_path(str(Path(__file__).with_name("check-test-manifest.py")))


class TestManifestTests(unittest.TestCase):
    def check_sources(self, files):
        with tempfile.TemporaryDirectory(prefix="test-manifest-") as temporary:
            root = Path(temporary)
            for name, contents in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(contents, encoding="utf-8")
            output = io.StringIO()
            errors = io.StringIO()
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                status = CHECKER["check"](root, root / "lib_tests.rs")
            return status, output.getvalue(), errors.getvalue()

    def test_root_file_module_candidate(self):
        self.assertEqual(self.check_sources({"lib_tests.rs": "mod alpha;\n", "alpha.rs": ""})[0], 0)

    def test_mod_rs_module_candidate(self):
        self.assertEqual(self.check_sources({
            "lib_tests.rs": "mod alpha;\n", "alpha/mod.rs": "mod child;\n", "alpha/child.rs": ""
        })[0], 0)

    def test_module_below_ordinary_file(self):
        self.assertEqual(self.check_sources({
            "lib_tests.rs": "mod group;\n", "group.rs": "mod child_tests;\n",
            "group/child_tests.rs": "#[test] fn example() {}\n"
        })[0], 0)

    def test_module_below_ordinary_file_directory_layout(self):
        self.assertEqual(self.check_sources({
            "lib_tests.rs": "mod group;\n", "group.rs": "mod child_tests;\n",
            "group/child_tests/mod.rs": "#[test] fn example() {}\n"
        })[0], 0)

    def test_non_suffix_test_file_must_be_reachable(self):
        status, _, errors = self.check_sources({
            "lib_tests.rs": "", "regression.rs": "#[test] fn regression() {}\n"
        })
        self.assertEqual(status, 1)
        self.assertIn("regression.rs", errors)

    def assert_ghost_is_unreachable(self, snippet):
        status, _, errors = self.check_sources({
            "lib_tests.rs": snippet,
            "ghost.rs": "#[test] fn regression() {}\n",
        })
        self.assertEqual(status, 1, snippet)
        self.assertIn("ghost.rs", errors)

    def test_commented_module_is_not_reachable(self):
        self.assert_ghost_is_unreachable("/*\nmod ghost;\n*/\n")

    def test_nested_block_commented_module_is_not_reachable(self):
        self.assert_ghost_is_unreachable("/* outer /*\nmod ghost;\n*/ end */\n")

    def test_line_commented_module_is_not_reachable(self):
        self.assert_ghost_is_unreachable("// mod ghost;\n")

    def test_string_literals_do_not_declare_modules(self):
        for literal in (
            '"first line\\nmod ghost;"',
            '"first line\nmod ghost;\nlast line"',
            'r#"first line\nmod ghost;\nlast line"#',
            'r##"first line\nmod ghost;\nlast line"##',
            'b"/* mod ghost; */"',
            'c"/* mod ghost; */"',
            'b"/*\nmod ghost;\n*/"',
            'c"/*\nmod ghost;\n*/"',
            'br#"/*\nmod ghost;\n*/"#',
            'cr##"/*\nmod ghost;\n*/"##',
        ):
            with self.subTest(literal=literal):
                self.assert_ghost_is_unreachable(f"const TEXT: &str = {literal};\n")

    def test_character_literals_do_not_hide_real_module(self):
        for literal in ("'" + '"' + "'", "'/'", r"'\\'", "b'/'", "b'\\\"'"):
            with self.subTest(literal=literal):
                status, _, errors = self.check_sources({
                    "lib_tests.rs": f"const CH: char = {literal};\nmod real;\n",
                    "real.rs": "",
                })
                self.assertEqual((status, errors), (0, ""))

    def test_lifetimes_do_not_hide_real_module(self):
        for declaration in (
            "fn borrow<'a>(x: &'a str) {}\n",
            "const VALUE: &\'static str = \"\";\n",
        ):
            with self.subTest(declaration=declaration):
                status, _, errors = self.check_sources({
                    "lib_tests.rs": declaration + "mod real;\n",
                    "real.rs": "",
                })
                self.assertEqual((status, errors), (0, ""))

    def test_raw_string_ends_only_at_matching_hash_count(self):
        status, _, errors = self.check_sources({
            "lib_tests.rs": 'const TEXT: &str = r##"/*\nmod ghost;\n"#\n*/\n"##;\nmod real;\n',
            "real.rs": "",
            "ghost.rs": "",
        })
        self.assertEqual(status, 1)
        self.assertIn("ghost.rs", errors)

    def test_declaration_after_comments_and_strings_is_reachable(self):
        for source in (
            '"a \\\" quoted string containing /* */";\nmod real;\n',
            "'/' ; // comment\nmod real;\n",
            'r#"/* */"#;\nmod real;\n',
            "/* closed */\nmod real;\n",
        ):
            with self.subTest(source=source):
                status, _, errors = self.check_sources({"lib_tests.rs": source, "real.rs": ""})
                self.assertEqual((status, errors), (0, ""))

    def test_unclosed_lexical_construct_fails_with_file_path(self):
        for source in ("/* open", '"open string', 'r##"open raw'):
            with self.subTest(source=source):
                status, _, errors = self.check_sources({"lib_tests.rs": source, "real.rs": ""})
                self.assertEqual(status, 1)
                self.assertIn("lib_tests.rs", errors)

    def test_mask_preserves_shape_and_literal_boundaries(self):
        source = 'const TEXT: &str = "multi\nline"; mod ghost;\n/* note\n*/ mod real;\n'
        masked = CHECKER["mask_non_code"](source)
        self.assertEqual(len(masked), len(source))
        self.assertEqual(masked.count("\n"), source.count("\n"))
        self.assertIn("mod ghost;", masked)
        self.assertIn("mod real;", masked)
        self.assertIsNone(CHECKER["MODULE_DECLARATION"].match(masked.splitlines()[0]))

    def test_intermediate_module_file_must_be_reachable(self):
        status, _, errors = self.check_sources({"lib_tests.rs": "", "unused/mod.rs": ""})
        self.assertEqual(status, 1)
        self.assertIn("unused/mod.rs", errors)

    def test_tests_named_module_is_traversed(self):
        status, _, errors = self.check_sources({"lib_tests.rs": "mod tests;\n", "tests.rs": ""})
        self.assertEqual((status, errors), (0, ""))

    def test_missing_module_declaration_is_reported(self):
        status, _, errors = self.check_sources({"lib_tests.rs": "mod missing;\n"})
        self.assertEqual(status, 1)
        self.assertIn("declared module does not exist", errors)

    def test_top_level_support_subtree_is_excluded(self):
        self.assertEqual(self.check_sources({"lib_tests.rs": "", "support/helper.rs": ""})[0], 0)

    def test_nested_support_subtree_is_not_excluded(self):
        status, _, errors = self.check_sources({"lib_tests.rs": "", "group/support/helper.rs": ""})
        self.assertEqual(status, 1)
        self.assertIn("group/support/helper.rs", errors)


if __name__ == "__main__":
    unittest.main()
