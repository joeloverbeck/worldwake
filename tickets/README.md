# Ticket Authoring Contract

This directory contains active implementation tickets.

To keep architecture clean, robust, and extensible, every new ticket must be created from `tickets/_TEMPLATE.md` and must satisfy the checks below.

## Core Architectural Contract

1. No backwards-compatibility shims or alias paths in new work.
2. If current code and ticket assumptions diverge, update the ticket first before implementation.

## Required Ticket Sections

1. `Assumption Reassessment (YYYY-MM-DD)`:
   - Validate ticket assumptions against current code/tests.
   - Explicitly call out mismatches and corrected scope.
2. `Architecture Check`:
   - Explain why the proposed design is cleaner than alternatives.
3. `Tests`:
   - List new/modified tests and rationale per test.
   - Include targeted and full-suite verification commands.

## Mandatory Pre-Implementation Checks

1. Dependency references point to existing repository files (active or archived paths are both valid when explicit).
2. Type and data contracts match current code.
3. Files-to-touch list matches current file layout and ownership.
4. Scope does not duplicate already-delivered architecture.

## Archival Reminder

Follow `docs/archival-workflow.md` as the canonical process.
