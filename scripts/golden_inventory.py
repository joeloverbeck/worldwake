#!/usr/bin/env python3

from __future__ import annotations

import argparse
import pathlib
import re
import subprocess
import sys
from collections import OrderedDict
from dataclasses import dataclass, field
from typing import Iterable

ROOT = pathlib.Path(__file__).resolve().parents[1]
TESTS_DIR = ROOT / "crates" / "worldwake-ai" / "tests"
OUTPUT_PATH = ROOT / "docs" / "generated" / "golden-e2e-inventory.md"
SCENARIO_OUTPUT_PATH = ROOT / "docs" / "generated" / "golden-scenario-map.md"
COVERAGE_OUTPUT_PATH = ROOT / "docs" / "generated" / "golden-coverage-matrix.md"
DOCS_TO_VALIDATE = (
    ROOT / "docs" / "golden-e2e-coverage.md",
    ROOT / "docs" / "golden-e2e-testing.md",
)

SOURCE_TEST_RE = re.compile(r"(?m)^fn (golden_[a-z0-9_]+)\s*\(")
SCENARIO_HEADER_RE = re.compile(
    r"^// Scenario (?P<identifier>[A-Za-z0-9_-]+)(?::| —) (?P<title>.+)$"
)
DOC_TEST_REF_RE = re.compile(r"`(golden_[a-z0-9_]+)`")
RUNNING_GOLDEN_BINARY_RE = re.compile(r"^\s*Running tests/(golden_[^ ]+\.rs) ")
RUNNING_ANY_BINARY_RE = re.compile(r"^\s*Running ")
LISTED_TEST_RE = re.compile(r"^(golden_[a-z0-9_]+): test$", re.MULTILINE)
REPLAY_TEST_RE = re.compile(
    r"_(?:replays_deterministically|deterministic_replay)$"
)

# Structured metadata keys within scenario comment blocks.
METADATA_KEYS = (
    "Systems",
    "GoalKinds",
    "ActionDomains",
    "Places",
    "Principles",
    "Setup",
    "Proves",
    "Chain",
)
STRUCTURED_KEY_RE = re.compile(
    r"^//\s+(?P<key>" + "|".join(METADATA_KEYS) + r"):\s*(?P<value>.*)$"
)
# Continuation: 3+ spaces after // (not a new key, not a separator).
CONTINUATION_RE = re.compile(r"^//\s{3,}(?P<value>\S.*)$")
# Blank or separator lines within scenario comment blocks.
BLANK_OR_SEP_RE = re.compile(r"^//\s*$|^// -{3,}$")


@dataclass
class ScenarioEntry:
    identifier: str
    title: str
    file_name: str
    line_number: int
    tests: tuple[str, ...]
    metadata: dict[str, str] = field(default_factory=dict)

    @property
    def primary_tests(self) -> tuple[str, ...]:
        return tuple(test for test in self.tests if not REPLAY_TEST_RE.search(test))

    @property
    def replay_tests(self) -> tuple[str, ...]:
        return tuple(test for test in self.tests if REPLAY_TEST_RE.search(test))


def parse_source_inventory(tests_dir: pathlib.Path) -> OrderedDict[str, list[str]]:
    inventory: OrderedDict[str, list[str]] = OrderedDict()
    for path in sorted(tests_dir.glob("golden_*.rs")):
        inventory[path.name] = SOURCE_TEST_RE.findall(path.read_text())
    return inventory


def parse_source_scenarios(tests_dir: pathlib.Path) -> list[ScenarioEntry]:
    scenarios: list[ScenarioEntry] = []
    for path in sorted(tests_dir.glob("golden_*.rs")):
        current_identifier: str | None = None
        current_title: str | None = None
        current_line_number: int | None = None
        current_tests: list[str] = []
        current_metadata: dict[str, str] = {}
        current_key: str | None = None

        def finish_current() -> None:
            nonlocal current_identifier, current_title, current_line_number
            nonlocal current_tests, current_metadata, current_key
            if current_identifier is None:
                return
            scenarios.append(
                ScenarioEntry(
                    identifier=current_identifier,
                    title=current_title or "",
                    file_name=path.name,
                    line_number=current_line_number or 1,
                    tests=tuple(current_tests),
                    metadata=dict(current_metadata),
                )
            )
            current_identifier = None
            current_title = None
            current_line_number = None
            current_tests = []
            current_metadata = {}
            current_key = None

        for line_number, raw_line in enumerate(path.read_text().splitlines(), start=1):
            header_match = SCENARIO_HEADER_RE.match(raw_line)
            if header_match:
                finish_current()
                current_identifier = header_match.group("identifier")
                current_title = header_match.group("title").strip()
                current_line_number = line_number
                continue

            if current_identifier is None:
                continue

            # Try structured metadata key.
            key_match = STRUCTURED_KEY_RE.match(raw_line)
            if key_match:
                current_key = key_match.group("key")
                value = key_match.group("value").strip()
                current_metadata[current_key] = value
                continue

            # Try continuation of current key.
            if current_key is not None:
                cont_match = CONTINUATION_RE.match(raw_line)
                if cont_match:
                    prev = current_metadata.get(current_key, "")
                    current_metadata[current_key] = (
                        prev + " " + cont_match.group("value").strip()
                    ).strip()
                    continue

            # Blank/separator lines don't reset the current key.
            if BLANK_OR_SEP_RE.match(raw_line):
                continue

            # Non-metadata comment line (e.g. old-style prose) — reset key.
            if raw_line.startswith("//"):
                current_key = None
                continue

            # Test function.
            test_match = SOURCE_TEST_RE.match(raw_line)
            if test_match:
                current_key = None
                current_tests.append(test_match.group(1))

        finish_current()

    return scenarios


def parse_cargo_test_list_output(output: str) -> OrderedDict[str, list[str]]:
    inventory: OrderedDict[str, list[str]] = OrderedDict()
    current_file: str | None = None
    for raw_line in output.splitlines():
        line = raw_line.rstrip()
        running_match = RUNNING_GOLDEN_BINARY_RE.match(line)
        if running_match:
            current_file = running_match.group(1)
            inventory.setdefault(current_file, [])
            continue
        if RUNNING_ANY_BINARY_RE.match(line):
            current_file = None
            continue
        if current_file is None:
            continue
        listed_match = LISTED_TEST_RE.match(line)
        if listed_match:
            inventory[current_file].append(listed_match.group(1))
    return inventory


def run_cargo_test_list(root: pathlib.Path) -> OrderedDict[str, list[str]]:
    inventory: OrderedDict[str, list[str]] = OrderedDict()
    for path in sorted(TESTS_DIR.glob("golden_*.rs")):
        result = subprocess.run(
            [
                "cargo",
                "test",
                "-p",
                "worldwake-ai",
                "--test",
                path.stem,
                "--",
                "--list",
            ],
            cwd=root,
            capture_output=True,
            text=True,
            check=True,
        )
        combined = result.stdout + "\n" + result.stderr
        inventory[path.name] = LISTED_TEST_RE.findall(combined)
    return inventory


def flatten_inventory(inventory: OrderedDict[str, list[str]]) -> list[str]:
    return [name for names in inventory.values() for name in names]


def count_files_with_goldens(inventory: OrderedDict[str, list[str]]) -> int:
    return sum(1 for names in inventory.values() if names)


def compare_inventories(
    source_inventory: OrderedDict[str, list[str]],
    cargo_inventory: OrderedDict[str, list[str]],
) -> list[str]:
    errors: list[str] = []
    all_files = sorted(set(source_inventory) | set(cargo_inventory))
    for file_name in all_files:
        source_names = source_inventory.get(file_name, [])
        cargo_names = cargo_inventory.get(file_name, [])
        if sorted(source_names) != sorted(cargo_names):
            errors.append(
                f"{file_name}: source={source_names!r} cargo_list={cargo_names!r}"
            )
    return errors


def validate_scenarios(
    scenarios: list[ScenarioEntry],
    inventory: OrderedDict[str, list[str]],
) -> list[str]:
    errors: list[str] = []
    by_identifier: dict[str, ScenarioEntry] = {}
    compiled_tests = set(flatten_inventory(inventory))

    for scenario in scenarios:
        previous = by_identifier.get(scenario.identifier)
        if previous is not None:
            errors.append(
                "duplicate scenario identifier "
                f"{scenario.identifier!r}: "
                f"{previous.file_name}:{previous.line_number} and "
                f"{scenario.file_name}:{scenario.line_number}"
            )
            continue
        by_identifier[scenario.identifier] = scenario

        if not scenario.tests:
            errors.append(
                f"{scenario.file_name}:{scenario.line_number}: "
                f"Scenario {scenario.identifier} has no `golden_*` tests"
            )
            continue

        missing_tests = sorted(test for test in scenario.tests if test not in compiled_tests)
        if missing_tests:
            errors.append(
                f"{scenario.file_name}:{scenario.line_number}: "
                f"Scenario {scenario.identifier} references missing compiled tests {missing_tests}"
            )

    return errors


def render_inventory_markdown(inventory: OrderedDict[str, list[str]]) -> str:
    total_files = len(inventory)
    contributing_files = count_files_with_goldens(inventory)
    total_tests = len(flatten_inventory(inventory))

    lines = [
        "# Generated Golden E2E Inventory",
        "",
        "This file is generated by `python3 scripts/golden_inventory.py --write --check-docs`.",
        "Do not hand-edit it.",
        "",
        "## Summary",
        "",
        f"- Golden test files: {total_files}",
        f"- Files contributing `golden_*` tests: {contributing_files}",
        f"- Total `golden_*` tests: {total_tests}",
        "",
        "## Per-File Inventory",
        "",
        "| File | `golden_*` tests |",
        "|------|------------------|",
    ]

    for file_name, tests in inventory.items():
        lines.append(f"| `{file_name}` | {len(tests)} |")

    for file_name, tests in inventory.items():
        lines.extend(
            [
                "",
                f"### `{file_name}`",
                "",
            ]
        )
        if not tests:
            lines.append("- No `golden_*` tests")
            continue
        for test_name in tests:
            lines.append(f"- `{test_name}`")

    lines.append("")
    return "\n".join(lines)


def _split_csv(value: str) -> list[str]:
    """Split a comma-separated metadata value into trimmed, non-empty items."""
    return [item.strip() for item in value.split(",") if item.strip()]


def render_scenario_markdown(scenarios: list[ScenarioEntry]) -> str:
    contributing_files = len({scenario.file_name for scenario in scenarios})
    total_tests = sum(len(scenario.tests) for scenario in scenarios)

    lines = [
        "# Generated Golden Scenario Map",
        "",
        "This file is generated by `python3 scripts/golden_inventory.py --write --check-docs`.",
        "Do not hand-edit it.",
        "",
        "This map covers only source-declared `// Scenario ...` blocks in `crates/worldwake-ai/tests/golden_*.rs`.",
        "It does not claim that planned spec scenarios already exist in live test source.",
        "",
        "## Summary",
        "",
        f"- Scenario blocks with explicit metadata: {len(scenarios)}",
        f"- Files contributing scenario metadata: {contributing_files}",
        f"- `golden_*` tests associated with scenario blocks: {total_tests}",
        "",
        "## Scenario Inventory",
        "",
        "| Scenario | Title | File | Primary tests | Replay tests |",
        "|----------|-------|------|---------------|--------------|",
    ]

    for scenario in scenarios:
        primary = "<br>".join(f"`{name}`" for name in scenario.primary_tests) or "—"
        replay = "<br>".join(f"`{name}`" for name in scenario.replay_tests) or "—"
        lines.append(
            f"| `{scenario.identifier}` | {scenario.title} | "
            f"`{scenario.file_name}:{scenario.line_number}` | {primary} | {replay} |"
        )

    for scenario in scenarios:
        lines.extend(
            [
                "",
                f"### Scenario {scenario.identifier}: {scenario.title}",
                "",
                f"- Source: `{scenario.file_name}:{scenario.line_number}`",
            ]
        )
        # Render structured metadata if present.
        for key in ("Systems", "GoalKinds", "ActionDomains", "Places", "Principles"):
            if key in scenario.metadata:
                lines.append(f"- {key}: {scenario.metadata[key]}")
        lines.append(
            "- Primary tests: "
            + (
                ", ".join(f"`{name}`" for name in scenario.primary_tests)
                if scenario.primary_tests
                else "None"
            )
        )
        lines.append(
            "- Replay tests: "
            + (
                ", ".join(f"`{name}`" for name in scenario.replay_tests)
                if scenario.replay_tests
                else "None"
            )
        )
        lines.append("- All tests: " + ", ".join(f"`{name}`" for name in scenario.tests))
        # Render prose metadata if present.
        for key, heading in (
            ("Setup", "Setup"),
            ("Proves", "Proves"),
            ("Chain", "Cross-system chain"),
        ):
            if key in scenario.metadata:
                lines.extend(["", f"**{heading}**: {scenario.metadata[key]}"])

    lines.append("")
    return "\n".join(lines)


def render_coverage_matrix_markdown(scenarios: list[ScenarioEntry]) -> str:
    """Render a coverage matrix derived from structured scenario metadata."""
    # Collect per-key mappings: value -> list of scenario identifiers.
    matrix_keys = (
        ("GoalKinds", "GoalKind Coverage"),
        ("ActionDomains", "ActionDomain Coverage"),
        ("Systems", "Systems Exercised"),
        ("Places", "Topology Coverage"),
        ("Principles", "Foundation Principles Tested"),
    )
    coverage: dict[str, dict[str, list[str]]] = {
        key: {} for key, _ in matrix_keys
    }

    annotated_count = 0
    for scenario in scenarios:
        if not scenario.metadata:
            continue
        annotated_count += 1
        for key, _ in matrix_keys:
            if key not in scenario.metadata:
                continue
            for item in _split_csv(scenario.metadata[key]):
                coverage[key].setdefault(item, []).append(scenario.identifier)

    lines = [
        "# Generated Golden Coverage Matrix",
        "",
        "This file is generated by `python3 scripts/golden_inventory.py --write --check-docs`.",
        "Do not hand-edit it.",
        "",
        f"Derived from structured metadata annotations in {annotated_count} of {len(scenarios)} scenario blocks.",
        f"Scenarios without annotations are not reflected here.",
        "",
    ]

    for key, heading in matrix_keys:
        data = coverage[key]
        lines.extend([f"## {heading}", ""])
        if not data:
            lines.extend(["No annotations yet.", ""])
            continue
        col_name = key.rstrip("s") if key != "Systems" else "System"
        if key == "Places":
            col_name = "Place"
        lines.extend(
            [
                f"| {col_name} | Scenarios |",
                "|" + "-" * (len(col_name) + 2) + "|-----------|",
            ]
        )
        for item in sorted(data):
            scenario_ids = ", ".join(sorted(data[item]))
            lines.append(f"| {item} | {scenario_ids} |")
        lines.append("")

    return "\n".join(lines)


def validate_doc_test_references(
    inventory: OrderedDict[str, list[str]],
    docs: Iterable[pathlib.Path],
) -> list[str]:
    valid_tests = set(flatten_inventory(inventory))
    errors: list[str] = []
    for doc_path in docs:
        if not doc_path.exists():
            continue
        refs = DOC_TEST_REF_RE.findall(doc_path.read_text())
        missing = sorted({name for name in refs if name not in valid_tests})
        if missing:
            errors.append(f"{doc_path.relative_to(ROOT)}: missing references {missing}")
    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate and validate the golden E2E inventory."
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write the generated markdown inventory artifact.",
    )
    parser.add_argument(
        "--check-docs",
        action="store_true",
        help="Validate that golden docs only reference existing `golden_*` tests.",
    )
    parser.add_argument(
        "--skip-cargo-list",
        action="store_true",
        help="Skip the compiled `cargo test -p worldwake-ai -- --list` cross-check.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source_inventory = parse_source_inventory(TESTS_DIR)
    source_scenarios = parse_source_scenarios(TESTS_DIR)

    errors: list[str] = []
    cargo_inventory: OrderedDict[str, list[str]] = OrderedDict()
    if not args.skip_cargo_list:
        cargo_inventory = run_cargo_test_list(ROOT)
        errors.extend(compare_inventories(source_inventory, cargo_inventory))
        errors.extend(validate_scenarios(source_scenarios, cargo_inventory))
    else:
        errors.extend(validate_scenarios(source_scenarios, source_inventory))

    if args.check_docs:
        errors.extend(validate_doc_test_references(source_inventory, DOCS_TO_VALIDATE))

    markdown = render_inventory_markdown(source_inventory)
    scenario_markdown = render_scenario_markdown(source_scenarios)
    coverage_markdown = render_coverage_matrix_markdown(source_scenarios)
    if args.write:
        OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT_PATH.write_text(markdown)
        SCENARIO_OUTPUT_PATH.write_text(scenario_markdown)
        COVERAGE_OUTPUT_PATH.write_text(coverage_markdown)

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        "golden inventory ok:"
        f" {len(source_inventory)} files,"
        f" {count_files_with_goldens(source_inventory)} contributing files,"
        f" {len(flatten_inventory(source_inventory))} tests,"
        f" {len(source_scenarios)} scenario blocks"
    )
    if not args.write:
        print(markdown)
        print()
        print(scenario_markdown)
        print()
        print(coverage_markdown)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
