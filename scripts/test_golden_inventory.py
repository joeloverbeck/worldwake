from __future__ import annotations

import importlib.util
import pathlib
import tempfile
import textwrap
import unittest
from collections import OrderedDict


SCRIPT_PATH = pathlib.Path(__file__).with_name("golden_inventory.py")
SPEC = importlib.util.spec_from_file_location("golden_inventory", SCRIPT_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class GoldenInventoryTests(unittest.TestCase):
    def test_parse_source_inventory_reads_per_file_golden_functions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            (root / "golden_alpha.rs").write_text(
                "fn golden_one() {}\nfn helper() {}\nfn golden_two() {}\n"
            )
            (root / "golden_beta.rs").write_text("fn helper() {}\n")

            inventory = MODULE.parse_source_inventory(root)

        self.assertEqual(
            inventory,
            OrderedDict(
                [
                    ("golden_alpha.rs", ["golden_one", "golden_two"]),
                    ("golden_beta.rs", []),
                ]
            ),
        )

    def test_parse_cargo_test_list_output_groups_tests_by_binary(self) -> None:
        output = textwrap.dedent(
            """
                Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
                 Running tests/golden_alpha.rs (target/debug/deps/golden_alpha-123)
                golden_one: test
                helper_test: test
                golden_two: test
                 Running tests/golden_beta.rs (target/debug/deps/golden_beta-456)
                golden_three: test
                 Running tests/not_golden.rs (target/debug/deps/not_golden-789)
                ignored: test
            """
        ).strip()

        inventory = MODULE.parse_cargo_test_list_output(output)

        self.assertEqual(
            inventory,
            OrderedDict(
                [
                    ("golden_alpha.rs", ["golden_one", "golden_two"]),
                    ("golden_beta.rs", ["golden_three"]),
                ]
            ),
        )

    def test_validate_doc_test_references_flags_stale_names(self) -> None:
        inventory = OrderedDict(
            [
                ("golden_alpha.rs", ["golden_one"]),
                ("golden_beta.rs", ["golden_two"]),
            ]
        )

        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            original_root = MODULE.ROOT
            MODULE.ROOT = root
            good_doc = root / "good.md"
            bad_doc = root / "bad.md"
            good_doc.write_text("See `golden_one` and `golden_two`.")
            bad_doc.write_text("Stale ref: `golden_missing`.")

            errors = MODULE.validate_doc_test_references(
                inventory, [good_doc, bad_doc]
            )
            MODULE.ROOT = original_root

        self.assertEqual(errors, ["bad.md: missing references ['golden_missing']"])

    def test_render_inventory_markdown_reports_summary_and_files(self) -> None:
        inventory = OrderedDict(
            [
                ("golden_alpha.rs", ["golden_one", "golden_two"]),
                ("golden_beta.rs", []),
            ]
        )

        markdown = MODULE.render_inventory_markdown(inventory)

        self.assertIn("- Golden test files: 2", markdown)
        self.assertIn("- Files contributing `golden_*` tests: 1", markdown)
        self.assertIn("- Total `golden_*` tests: 2", markdown)
        self.assertIn("| `golden_alpha.rs` | 2 |", markdown)
        self.assertIn("- No `golden_*` tests", markdown)


if __name__ == "__main__":
    unittest.main()
