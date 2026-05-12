#!/usr/bin/env python3
"""Lightweight structural closeout checks for Worldwake tickets."""

from __future__ import annotations

import re
import sys
from pathlib import Path


SECTION_RE = re.compile(r"^##\s+(.+?)\s*$", re.MULTILINE)
STATUS_RE = re.compile(r"^\s*\**Status\**:\s*(.+?)\s*$", re.MULTILINE)
VERIFICATION_ITEM_RE = re.compile(r"^\s*(?:-\s*|\d+\.\s+)")
VERIFICATION_LABEL_RE = re.compile(r"^\s*(?:-\s*|\d+\.\s+)(Passed|Waived|Blocked)\b")
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
        for section in ("Outcome", "Verification Result"):
            if section_body(text, section) is None:
                warnings.append(f"completed ticket is missing ## {section}")

        verification = section_body(text, "Verification Result")
        verification_commands = command_refs(verification)
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
        proof_sections = (
            "Acceptance Criteria",
            "Test Plan",
            "New/Modified Tests",
            "Verification Layers",
        )
        for section in proof_sections:
            required_commands.update(command_refs(section_body(text, section)))

        missing_commands = sorted(required_commands - verification_commands)
        if missing_commands:
            warnings.append(
                "command-like proof references are not mirrored in Verification Result: "
                + ", ".join(f"`{command}`" for command in missing_commands)
            )

        for section in proof_sections:
            body = section_body(text, section)
            if body and re.search(r"\b(new|will|should|planned|TODO|TBD)\b", body, re.IGNORECASE):
                warnings.append(f"completed ticket has future/draft wording in ## {section}")

    if warnings:
        for warning in warnings:
            print(f"warning: {warning}")
        return 1

    print("closeout structural checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
