---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

## Workflow

### 0. Classify ticket shape and pick the right path

Before running the full workflow, classify the ticket:

**Small/local tickets** (fast path) — single-file additive CLI/tooling/reporting/action-registry change, narrow helper extraction, formatting update, or other owned-module additive change with no shared type/planner/golden/persistence/cross-crate fallout expected. Typical examples include a single-file transport/action registration, local handler addition, narrow helper extraction, or bin-local coverage for factored logic:
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

For cross-crate accessor, trait-surface, or API-surface tickets, verify the real downstream caller-facing boundary before coding, not just the immediate trait or type named in the ticket. If live callers consume the data through a broader wrapper, supertrait, blanket impl, or facade surface, correct the ticket to that owned boundary before editing code.

When a ticket relies on an existing helper or accessor, verify not only that the symbol exists on the expected boundary but that its live implementation returns the intended semantic quantity for the concrete subject type under test. Do not trust plausible naming alone when helpers can be overloaded, entity-type-specific, or historically repurposed; if the live helper computes a different concept than the ticket assumes, correct the ticket to the lawful contract before editing code.

For planner-visible belief, profile, or snapshot-completeness tickets, verify the full carriage path before coding: runtime belief view -> snapshot builder -> snapshot storage -> `PlanningState`/planner-facing view surface. Do not stop at the final accessor if planner-visible data can be dropped earlier in the pipeline.

For dedicated goal-root, planner-root, or golden-isolation tickets, verify that the claimed downstream effect is uniquely attributable to the named goal/root rather than already reachable through a more generic operator family. If a generic path can already lawfully produce the same outcome, narrow the ticket and scenario so they prove the dedicated goal's distinct contract instead of over-claiming a broader downstream chain.

Load `references/reassessment-checks.md`.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md`.

When reassessment shows that part of the ticket's claimed substrate is already present in live code, update the ticket before coding so it describes only the remaining owned delta. Reflect that narrowed scope in the ticket's `Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria` sections instead of leaving stale "add X" language in place.

After narrowing a ticket because substrate is already live, re-sweep the adjacent fallout that commonly remains owned by the current ticket: declaration/dispatch tables, snapshot/state carriers, local test stubs/helpers, synthetic candidate/root helpers, and the broadened verification selectors that should now prove only the remaining live delta.

If focused proof added during implementation reveals a production contradiction that reassessment did not yet expose, stop and correct the ticket before proceeding further. Update the same sections (`Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria`) so the ticket no longer claims "tests only" or `Engine Changes: None` when the live invariant actually requires production changes.

If focused proof instead falsifies the suspected production contradiction and shows the live fix is narrower (for example, golden-scenario isolation or fixture recalibration), stop and narrow the ticket before proceeding further. Update the same sections (`Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria`) so the ticket no longer claims production ownership when the honest contract is test-only or fixture-only.

### 4. Extract the implementation scope

Load `references/scope-extraction.md`.

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md`.

When the clean fix requires extracting a helper out of an existing module into a neutral shared location, explicitly sweep sibling and transitive import sites for the old module path before relying on compile fallout alone. Shared-helper extraction often leaves behind stale `use crate::old_module::helper` assumptions even when the owned behavioral change is otherwise correct.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Load `references/verification.md`.

If reassessment revealed that additive substrate from an earlier ticket already landed, include repository-wide live-contract fallout in the broadened verification sweep, not just the ticket's newly edited file set. Typical fallout includes stale `ALL` lists, exhaustiveness fixtures, representative-goal inventories, explicit length assertions that still reflect the pre-addition shape, and adjacent registry/declaration surfaces such as feasibility or invalidation strategies, provenance-family mappings, and other dispatch-table contracts that must now treat the additive shape as live behavior rather than inert scaffolding.

For additive planner-root tickets, also sweep helpers keyed by shared planner transitions or op-family semantics rather than only declaration tables and enum matches. Typical fallout includes planner-only synthetic candidate builders, search helpers that expand candidates from shared `PlannerTransitionKind` behavior, and exhaustive `PlannerOpKind` matches in non-obvious support modules such as observation/runtime reconciliation, blocker classification, or related-place/related-entity helpers.

For behavior-expanding tickets, expect broadened golden fallout to include stale scenario isolation, not just compile or enum-shape fallout. If an existing golden now reaches a newly lawful branch, tighten the scenario so it still proves its intended invariant using explicit local belief seeding, profile/perception overrides, or other lawful setup constraints rather than silently preserving the old behavior.

When broadened verification fails, treat each failure as current-ticket fallout and continue the fix-and-rerun loop until the broadened target passes or you hit a real 1-3-1 blocker. Do not stop after the first full-suite failure if the next step is a straightforward fallout fix within the ticket's live scope.

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
