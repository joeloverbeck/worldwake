#!/usr/bin/env python3
"""Lightweight structural closeout checks for Worldwake tickets."""

from __future__ import annotations

import re
import sys
from pathlib import Path


SECTION_RE = re.compile(r"^##\s+(.+?)\s*$", re.MULTILINE)
STATUS_RE = re.compile(r"^Status:\s*(.+?)\s*$", re.MULTILINE)
VERIFICATION_LABEL_RE = re.compile(r"^\s*-\s*(Passed|Waived|Blocked)\b")


def section_body(text: str, name: str) -> str | None:
    matches = list(SECTION_RE.finditer(text))
    for index, match in enumerate(matches):
        if match.group(1).strip().lower() != name.lower():
            continue
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        return text[start:end]
    return None


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
        if verification is not None:
            bullets = [line for line in verification.splitlines() if line.lstrip().startswith("-")]
            unlabeled = [line.strip() for line in bullets if not VERIFICATION_LABEL_RE.match(line)]
            if unlabeled:
                warnings.append(
                    "Verification Result has bullets that do not start with Passed, Waived, or Blocked"
                )

        for section in ("Acceptance Criteria", "Test Plan", "New/Modified Tests", "Verification Layers"):
            body = section_body(text, section)
            if body and re.search(r"\b(new|will|should|planned|TODO|TBD)\b", body, re.IGNORECASE):
                warnings.append(f"completed ticket has future/draft wording in ## {section}")

        if re.search(r"^\s*-\s*`?(cargo|./scripts/verify\.sh|scripts/verify\.sh)\b", text, re.MULTILINE):
            warnings.append("ticket still contains command-looking bullets; confirm they are observed proof")

    if warnings:
        for warning in warnings:
            print(f"warning: {warning}")
        return 1

    print("closeout structural checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
