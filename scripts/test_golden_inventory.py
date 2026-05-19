from __future__ import annotations

import importlib.util
import pathlib
import sys
import tempfile
import textwrap
import unittest
from collections import OrderedDict


SCRIPT_PATH = pathlib.Path(__file__).with_name("golden_inventory.py")
SPEC = importlib.util.spec_from_file_location("golden_inventory", SCRIPT_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class GoldenInventoryTests(unittest.TestCase):
    def test_parse_source_inventory_reads_post_move_scenarios(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            scenarios = root / "scenarios"
            scenarios.mkdir()
            (scenarios / "alpha.rs").write_text(
                "#[test]\nfn golden_one() {}\nfn helper() {}\n#[test]\nfn golden_two() {}\n"
            )
            (scenarios / "beta.rs").write_text("fn helper() {}\n")
            (scenarios / "mod.rs").write_text("pub mod alpha;\npub mod beta;\n")

            inventory = MODULE.parse_source_inventory(root)

        self.assertEqual(
            inventory,
            OrderedDict(
                [
                    ("alpha.rs", ["golden_one", "golden_two"]),
                    ("beta.rs", []),
                ]
            ),
        )

    def test_parse_cargo_test_list_output_groups_golden_ai_by_scenario_module(
        self,
    ) -> None:
        output = textwrap.dedent(
            """
                Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
                 Running tests/golden_ai.rs (target/debug/deps/golden_ai-123)
                scenarios::alpha::golden_one: test
                scenarios::alpha::golden_two: test
                scenarios::beta::golden_three: test
                 Running tests/integration_ai.rs (target/debug/deps/integration_ai-456)
                integration::ignored::test_name: test
            """
        ).strip()

        inventory = MODULE.parse_cargo_test_list_output(output)

        self.assertEqual(
            inventory,
            OrderedDict(
                [
                    ("alpha.rs", ["golden_one", "golden_two"]),
                    ("beta.rs", ["golden_three"]),
                ]
            ),
        )

    def test_parse_source_scenarios_reads_identifier_title_and_tests(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            scenarios_dir = root / "scenarios"
            scenarios_dir.mkdir()
            (scenarios_dir / "alpha.rs").write_text(
                textwrap.dedent(
                    """
                    // ---------------------------------------------------------------------------
                    // Scenario 33: Remote Record Travel + Consultation + Political Action
                    // ---------------------------------------------------------------------------
                    fn helper() {}
                    #[test]
                    fn golden_remote_record_consultation_political_action() {}
                    #[test]
                    fn golden_remote_record_consultation_political_action_replays_deterministically() {}
                    // ---------------------------------------------------------------------------
                    // Scenario 34: Knowledge Asymmetry Race
                    // ---------------------------------------------------------------------------
                    #[test]
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
                    file_name="alpha.rs",
                    line_number=2,
                    tests=(
                        "golden_remote_record_consultation_political_action",
                        "golden_remote_record_consultation_political_action_replays_deterministically",
                    ),
                ),
                MODULE.ScenarioEntry(
                    identifier="34",
                    title="Knowledge Asymmetry Race",
                    file_name="alpha.rs",
                    line_number=10,
                    tests=("golden_knowledge_asymmetry_race_informed_wins_office",),
                ),
            ],
        )

    def test_parse_source_scenarios_accepts_letter_suffix_identifiers(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = pathlib.Path(tmp_dir)
            scenarios_dir = root / "scenarios"
            scenarios_dir.mkdir()
            (scenarios_dir / "alpha.rs").write_text(
                textwrap.dedent(
                    """
                    // Scenario 11b: Deterministic Replay
                    #[test]
                    fn golden_simple_office_claim_deterministic_replay() {}
                    // Scenario 2c-self: Self Care
                    #[test]
                    fn golden_self_care_with_medicine() {}
                    // Scenario S03a: Multi-Corpse Loot Binding
                    #[test]
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
                ("alpha.rs", ["golden_one"]),
                ("beta.rs", ["golden_two"]),
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
                file_name="alpha.rs",
                line_number=2,
                tests=("golden_remote_record_consultation_political_action",),
            ),
            MODULE.ScenarioEntry(
                identifier="33",
                title="Duplicate Remote Record",
                file_name="beta.rs",
                line_number=5,
                tests=("golden_duplicate",),
            ),
            MODULE.ScenarioEntry(
                identifier="34",
                title="Knowledge Asymmetry",
                file_name="beta.rs",
                line_number=10,
                tests=(),
            ),
            MODULE.ScenarioEntry(
                identifier="35",
                title="Missing Compiled Test",
                file_name="gamma.rs",
                line_number=12,
                tests=("golden_missing",),
            ),
        ]
        inventory = OrderedDict(
            [("alpha.rs", ["golden_remote_record_consultation_political_action"])]
        )

        errors = MODULE.validate_scenarios(scenarios, inventory)

        self.assertEqual(
            errors,
            [
                "duplicate scenario identifier '33': alpha.rs:2 and beta.rs:5",
                "beta.rs:10: Scenario 34 has no test functions",
                "gamma.rs:12: Scenario 35 references missing compiled tests ['golden_missing']",
            ],
        )

    def test_render_inventory_markdown_reports_summary_and_files(self) -> None:
        inventory = OrderedDict(
            [
                ("alpha.rs", ["golden_one", "golden_two"]),
                ("beta.rs", []),
            ]
        )

        markdown = MODULE.render_inventory_markdown(inventory)

        self.assertIn("- Golden scenario source files: 2", markdown)
        self.assertIn("- Files contributing `golden_*` tests: 1", markdown)
        self.assertIn("- Total `golden_*` tests: 2", markdown)
        self.assertIn("| `alpha.rs` | 2 |", markdown)
        self.assertIn("- No `golden_*` tests", markdown)

    def test_render_scenario_markdown_reports_primary_and_replay_tests(self) -> None:
        markdown = MODULE.render_scenario_detail_markdown(
            "alpha.rs",
            [
                MODULE.ScenarioEntry(
                    identifier="33",
                    title="Remote Record",
                    file_name="alpha.rs",
                    line_number=2,
                    tests=(
                        "golden_remote_record_consultation_political_action",
                        "golden_remote_record_consultation_political_action_replays_deterministically",
                    ),
                )
            ]
        )

        self.assertIn("Scenarios: 1", markdown)
        self.assertIn("- Source: `alpha.rs:2`", markdown)
        self.assertIn("`golden_remote_record_consultation_political_action`", markdown)
        self.assertIn(
            "`golden_remote_record_consultation_political_action_replays_deterministically`",
            markdown,
        )


if __name__ == "__main__":
    unittest.main()
