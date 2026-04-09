---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

## Workflow

### 0. Classify ticket shape and pick the right path

Before running the full workflow, classify the ticket:

**Small/local tickets** (fast path) — single-file additive CLI/tooling/reporting change, narrow helper extraction, formatting update, no shared type/planner/golden/persistence/cross-crate fallout expected:
1. Resolve the exact live ticket/spec path, including typos or shorthand.
2. Confirm the dependency path and the exact owned symbol/file boundary.
3. Run a constructor/usage sweep for the changed shape (see Step 4, Type-change scope).
4. Implement the owned change with focused proof first.
5. Use all-target compile fallout to catch remaining shared-shape literals/helpers.
6. Close out the ticket with the actual verification set and tracked-vs-untracked note.

For CLI/tooling-only tickets, if the owned logic can be factored into local helpers, prefer bin-local `#[cfg(test)]` coverage over command-only validation.

Do not skip reassessment for small tickets, but scale it down: read the ticket, cited references, and owned symbol/file; confirm the dependency path is present; run a narrow existence/fallout sweep for prior implementation or obvious constructor/usage fallout. Do not force the full Step 2 matrix when the owned surface is genuinely small and local.

**All other tickets** — use the full workflow below (Steps 1-8).

When the ticket was authored by `/spec-to-tickets` in the current session from a freshly reassessed spec, scale reassessment to a targeted sweep: confirm the ticket's owned types still exist at stated paths, check for exhaustive matchers on modified enums, verify trait bounds on any types used in new test code, check for manual struct literals of modified types (constructors, test helpers, `from_*_for_test` patterns) that would need updating for new fields, and before adding new test-only accessors or helpers, check whether existing test infrastructure (e.g., `ActualWorldState::from_world`, test harness methods) already provides the needed capability.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
4. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow.
5. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

Load `references/reassessment-checks.md`.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md`.

### 4. Extract the implementation scope

Load `references/scope-extraction.md`.

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md`.

When the clean fix requires extracting a helper out of an existing module into a neutral shared location, explicitly sweep sibling and transitive import sites for the old module path before relying on compile fallout alone. Shared-helper extraction often leaves behind stale `use crate::old_module::helper` assumptions even when the owned behavioral change is otherwise correct.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Load `references/verification.md`.

### 7. Close out the ticket honestly

Load `references/closeout.md`.

### 8. Close the loop on the ticket

Covered in `references/closeout.md` (Step 8 section).

## Guardrails

- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- Architectural contradictions: solve or escalate with 1-3-1 (see mismatch-handling.md, Escalation decision tree). Do not patch around them.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
