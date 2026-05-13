#!/usr/bin/env python3
"""Lightweight structural closeout checks for Worldwake tickets."""

from __future__ import annotations

import re
import sys
from pathlib import Path


SECTION_RE = re.compile(r"^##\s+(.+?)\s*$", re.MULTILINE)
HEADING_RE = re.compile(r"^(#{2,6})\s+(.+?)\s*$", re.MULTILINE)
STATUS_RE = re.compile(r"^\s*\**Status\**:\s*(.+?)\s*$", re.MULTILINE)
VERIFICATION_ITEM_RE = re.compile(r"^\s*(?:-\s*|\d+\.\s+)")
VERIFICATION_LABEL_RE = re.compile(r"^\s*(?:-\s*|\d+\.\s+)(Passed|Waived|Blocked)\b")
VERIFICATION_LABELED_ITEM_RE = re.compile(r"^\s*(?:-\s*|\d+\.\s+)(Passed|Waived|Blocked)\b(.*)$")
BACKTICK_RE = re.compile(r"`([^`\n]+)`")
COMMAND_START_RE = re.compile(
    r"^(?:"
    r"cargo|"
    r"\./scripts/verify\.sh|"
    r"scripts/verify\.sh|"
    r"python3|"
    r"node|"
    r"bash|"
    r"pnpm|"
    r"npm"
    r")\b"
)
PROOF_SECTIONS = (
    "Acceptance Criteria",
    "Test Plan",
    "New/Modified Tests",
    "Verification Layers",
)
NARRATIVE_SECTIONS = (
    "Problem",
    "Assumption Reassessment",
    "What to Change",
)
PLANNING_ONLY_SECTIONS = (
    "What to Change",
    "Files to Touch",
    "Test Plan",
    "New/Modified Tests",
    "Verification Layers",
)
PLANNING_ONLY_SUBSECTIONS = (
    "Commands",
    "Tests That Must Pass",
)
RESULT_TENSE_ALLOWLIST_RE = re.compile(
    r"\b(landed|completed|actual|result|verified|added|modified|no-change)\b",
    re.IGNORECASE,
)
PROOF_DRAFT_RE = re.compile(r"\b(new|will|should|planned|TODO|TBD)\b", re.IGNORECASE)
NARRATIVE_STALE_RE = re.compile(
    r"\b(today|current|leaves|will|should|planned|TODO|TBD)\b",
    re.IGNORECASE,
)


def section_body(text: str, name: str) -> str | None:
    matches = list(SECTION_RE.finditer(text))
    for index, match in enumerate(matches):
        if match.group(1).strip().lower() != name.lower():
            continue
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        return text[start:end]
    return None


def command_refs(body: str | None) -> set[str]:
    if body is None:
        return set()

    refs: set[str] = set()
    for match in BACKTICK_RE.finditer(body):
        command = match.group(1).strip()
        if COMMAND_START_RE.match(command):
            refs.add(command)
    return refs


def labeled_verification_commands(body: str | None) -> dict[str, set[str]]:
    commands_by_label: dict[str, set[str]] = {
        "Passed": set(),
        "Waived": set(),
        "Blocked": set(),
    }
    if body is None:
        return commands_by_label

    for line in body.splitlines():
        label_match = VERIFICATION_LABELED_ITEM_RE.match(line)
        if label_match is None:
            continue
        label = label_match.group(1)
        item_text = label_match.group(2)
        commands_by_label[label].update(command_refs(item_text))
    return commands_by_label


def line_number_for_offset(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def draft_matches(text: str, section: str, pattern: re.Pattern[str]) -> list[str]:
    matches = list(SECTION_RE.finditer(text))
    for index, match in enumerate(matches):
        if match.group(1).strip().lower() != section.lower():
            continue
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        body = text[start:end]
        results: list[str] = []
        for draft_match in pattern.finditer(body):
            absolute = start + draft_match.start()
            line_no = line_number_for_offset(text, absolute)
            line_start = text.rfind("\n", 0, absolute) + 1
            line_end = text.find("\n", absolute)
            if line_end == -1:
                line_end = len(text)
            line = text[line_start:line_end].strip()
            token = draft_match.group(0)
            results.append(f"line {line_no} (`{token}`): {line}")
        return results
    return []


def planning_heading_warnings(text: str) -> list[str]:
    warnings: list[str] = []
    matches = list(HEADING_RE.finditer(text))
    for index, match in enumerate(matches):
        level = len(match.group(1))
        title = match.group(2).strip()
        start = match.end()
        end = len(text)
        for next_match in matches[index + 1 :]:
            if len(next_match.group(1)) <= level:
                end = next_match.start()
                break
        body = text[start:end]
        if title in PLANNING_ONLY_SECTIONS:
            if not RESULT_TENSE_ALLOWLIST_RE.search(body):
                warnings.append(
                    f"completed ticket retains planning-only heading ## {title}; "
                    "rename it to a result-tense heading such as Landed Files, "
                    "Landed Changes, Verified Layers, or Test Plan Result"
                )
        elif level >= 3 and title in PLANNING_ONLY_SUBSECTIONS:
            if not RESULT_TENSE_ALLOWLIST_RE.search(body):
                line_no = line_number_for_offset(text, match.start())
                warnings.append(
                    f"completed ticket retains planning-only subsection line {line_no} "
                    f"({match.group(1)} {title}); rename or fold it into Verification Result"
                )
    return warnings


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check_closeout.py <ticket-path>", file=sys.stderr)
        return 2

    ticket_path = Path(sys.argv[1])
    text = ticket_path.read_text(encoding="utf-8")
    warnings: list[str] = []

    status_match = STATUS_RE.search(text)
    status = status_match.group(1).strip() if status_match else ""
    is_completed = status.upper() == "COMPLETED"

    if is_completed:
        warnings.extend(planning_heading_warnings(text))

        for section in ("Outcome", "Verification Result"):
            if section_body(text, section) is None:
                warnings.append(f"completed ticket is missing ## {section}")

        verification = section_body(text, "Verification Result")
        verification_commands = command_refs(verification)
        verification_commands_by_label = labeled_verification_commands(verification)
        if verification is not None:
            items = [
                line
                for line in verification.splitlines()
                if VERIFICATION_ITEM_RE.match(line)
            ]
            unlabeled = [line.strip() for line in items if not VERIFICATION_LABEL_RE.match(line)]
            if unlabeled:
                warnings.append(
                    "Verification Result has list items that do not start with Passed, Waived, or Blocked"
                )

        required_commands: set[str] = set()
        for section in PROOF_SECTIONS:
            required_commands.update(command_refs(section_body(text, section)))

        missing_commands = sorted(required_commands - verification_commands)
        if missing_commands:
            warnings.append(
                "command-like proof references are not mirrored in Verification Result: "
                + ", ".join(f"`{command}`" for command in missing_commands)
            )

        waived_required_commands = sorted(
            required_commands & verification_commands_by_label["Waived"]
        )
        if waived_required_commands:
            warnings.append(
                "waived commands are still listed as required proof: "
                + ", ".join(f"`{command}`" for command in waived_required_commands)
            )

        for section in PROOF_SECTIONS:
            body = section_body(text, section)
            if body and PROOF_DRAFT_RE.search(body):
                details = "; ".join(draft_matches(text, section, PROOF_DRAFT_RE))
                warnings.append(
                    f"completed ticket has future/draft wording in ## {section}: {details}"
                )

        for section in NARRATIVE_SECTIONS:
            body = section_body(text, section)
            if body and NARRATIVE_STALE_RE.search(body):
                details = "; ".join(draft_matches(text, section, NARRATIVE_STALE_RE))
                warnings.append(
                    f"completed ticket may have stale present/future wording in ## {section}; "
                    "use result tense or explicit before-this-ticket framing: "
                    + details
                )

    if warnings:
        for warning in warnings:
            print(f"warning: {warning}")
        return 1

    print("closeout structural checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
