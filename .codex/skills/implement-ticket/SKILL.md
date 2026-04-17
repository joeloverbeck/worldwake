---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. If that doc does not cover the exact planner boundary under audit, record the gap and fall back to the landed implementation/spec/ticket chain instead of treating the contract as unknowable. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

## Workflow

### 0. Classify ticket shape and pick the right path

Load `references/ticket-classification.md`.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
4. When the ticket name implies a numbered family or the user cites a parent spec, search for sibling tickets in the same family and confirm whether adjacent missing substrate is already owned elsewhere before broadening or narrowing the current ticket.
5. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow.
6. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. If a dependency ticket has already been completed and archived, rewrite `Deps` to the live archived path instead of leaving a stale active-ticket reference. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

Load `references/reassessment-checks.md`. For planner-root, snapshot-completeness, planner-traceability, or AI pipeline work, also load `references/reassessment-planner-ai.md`. For golden E2E or observer-motivated tickets, also load `references/reassessment-golden.md`.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md` when reassessment exposes a contradiction, risky ticket/code divergence, or a user decision that requires 1-3-1.

### 4. Extract the implementation scope

Load `references/scope-extraction.md` when the owned edit surface, dependency boundary, or honest verification scope is not already clear from reassessment and ticket classification.

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md` when implementation begins or when reassessment shows a non-mechanical change that needs extra guardrails.

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
- Architectural contradictions: solve or escalate with 1-3-1 (see `references/mismatch-handling.md`, Escalation decision tree). Do not patch around them.
- For focused test commands, verify that the selector actually proves the owned surface. Substring filters can run extra tests or, for integration-test binaries, compile the target while executing zero tests. When exactness matters, prefer the narrowest truthful selector such as an exact unit-test name or `cargo test -p <crate> --test <file_stem>` for integration-test binaries instead of a loose name filter.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
