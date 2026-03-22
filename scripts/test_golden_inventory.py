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

    def test_parse_source_scenarios_reads_identifier_title_and_tests(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            (root / "golden_alpha.rs").write_text(
                textwrap.dedent(
                    """
                    // ---------------------------------------------------------------------------
                    // Scenario 33: Remote Record Travel + Consultation + Political Action
                    // ---------------------------------------------------------------------------
                    fn helper() {}
                    fn golden_remote_record_consultation_political_action() {}
                    fn golden_remote_record_consultation_political_action_replays_deterministically() {}
                    // ---------------------------------------------------------------------------
                    // Scenario 34: Knowledge Asymmetry Race
                    // ---------------------------------------------------------------------------
                    fn golden_knowledge_asymmetry_race_informed_wins_office() {}
                    """
                ).strip()
                + "\n"
            )

            scenarios = MODULE.parse_source_scenarios(root)

        self.assertEqual(
            scenarios,
            [
                MODULE.ScenarioEntry(
                    identifier="33",
                    title="Remote Record Travel + Consultation + Political Action",
                    file_name="golden_alpha.rs",
                    line_number=2,
                    tests=(
                        "golden_remote_record_consultation_political_action",
                        "golden_remote_record_consultation_political_action_replays_deterministically",
                    ),
                ),
                MODULE.ScenarioEntry(
                    identifier="34",
                    title="Knowledge Asymmetry Race",
                    file_name="golden_alpha.rs",
                    line_number=8,
                    tests=("golden_knowledge_asymmetry_race_informed_wins_office",),
                ),
            ],
        )

    def test_parse_source_scenarios_accepts_letter_suffix_identifiers(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            (root / "golden_alpha.rs").write_text(
                textwrap.dedent(
                    """
                    // Scenario 11b: Deterministic Replay
                    fn golden_simple_office_claim_deterministic_replay() {}
                    // Scenario 2c-self: Self Care
                    fn golden_self_care_with_medicine() {}
                    // Scenario S03a: Multi-Corpse Loot Binding
                    fn golden_multi_corpse_loot_binding() {}
                    """
                ).strip()
                + "\n"
            )

            scenarios = MODULE.parse_source_scenarios(root)

        self.assertEqual(
            [scenario.identifier for scenario in scenarios],
            ["11b", "2c-self", "S03a"],
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

    def test_validate_scenarios_flags_duplicates_empty_blocks_and_missing_compiled_tests(
        self,
    ) -> None:
        scenarios = [
            MODULE.ScenarioEntry(
                identifier="33",
                title="Remote Record",
                file_name="golden_alpha.rs",
                line_number=2,
                tests=("golden_remote_record_consultation_political_action",),
            ),
            MODULE.ScenarioEntry(
                identifier="33",
                title="Duplicate Remote Record",
                file_name="golden_beta.rs",
                line_number=5,
                tests=("golden_duplicate",),
            ),
            MODULE.ScenarioEntry(
                identifier="34",
                title="Knowledge Asymmetry",
                file_name="golden_beta.rs",
                line_number=10,
                tests=(),
            ),
            MODULE.ScenarioEntry(
                identifier="35",
                title="Missing Compiled Test",
                file_name="golden_gamma.rs",
                line_number=12,
                tests=("golden_missing",),
            ),
        ]
        inventory = OrderedDict(
            [("golden_alpha.rs", ["golden_remote_record_consultation_political_action"])]
        )

        errors = MODULE.validate_scenarios(scenarios, inventory)

        self.assertEqual(
            errors,
            [
                "duplicate scenario identifier '33': golden_alpha.rs:2 and golden_beta.rs:5",
                "golden_beta.rs:10: Scenario 34 has no `golden_*` tests",
                "golden_gamma.rs:12: Scenario 35 references missing compiled tests ['golden_missing']",
            ],
        )

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

    def test_render_scenario_markdown_reports_primary_and_replay_tests(self) -> None:
        markdown = MODULE.render_scenario_markdown(
            [
                MODULE.ScenarioEntry(
                    identifier="33",
                    title="Remote Record",
                    file_name="golden_alpha.rs",
                    line_number=2,
                    tests=(
                        "golden_remote_record_consultation_political_action",
                        "golden_remote_record_consultation_political_action_replays_deterministically",
                    ),
                )
            ]
        )

        self.assertIn("- Scenario blocks with explicit metadata: 1", markdown)
        self.assertIn("| `33` | Remote Record | `golden_alpha.rs:2` |", markdown)
        self.assertIn("`golden_remote_record_consultation_political_action`", markdown)
        self.assertIn(
            "`golden_remote_record_consultation_political_action_replays_deterministically`",
            markdown,
        )


if __name__ == "__main__":
    unittest.main()
