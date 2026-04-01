---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code.

Keep the workflow compact and deterministic. Reassess first, then implement. Do not treat a ticket as mechanically executable until its assumptions match the current codebase.

## Workflow

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference named by the ticket:
   - specs
   - docs
   - code symbols
   - test files
3. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all reads, writes, searches, moves, and archival actions.

### 2. Reassess assumptions before coding

1. Verify the ticket against the current codebase, not against stale architectural memory.
2. Check the `Deps` field. Confirm each listed dependency is actually present on the current branch, whether as active planning material or as an archived completed prerequisite.
3. Validate the ticket's concrete claims:
   - referenced files exist
   - referenced types, functions, modules, commands, and tests exist
   - described architecture still matches the live code
   - stated coverage gaps are real and classified correctly
   - for golden-driven tickets, claimed missing scenarios are not already covered by the current `golden_*` suites or the generated golden inventory/docs
   - when shared types change, serialized fixtures, bundled scenarios, schema examples, and other non-Rust deserialization inputs still match the live struct shape
4. Reassess against Worldwake's repo rules:
   - ticket fidelity from [AGENTS.md](../../../AGENTS.md)
   - foundational compliance from [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
   - ticket structure from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md)
5. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit instead of using broad summaries.

### 3. Handle mismatches explicitly

If the ticket and live code disagree, stop and surface the discrepancy before implementation.

For each mismatch, state:
- what the ticket says
- what the codebase currently has
- whether the ticket should be corrected, the implementation should adapt, or the issue is blocked

Apply the 1-3-1 rule from [AGENTS.md](../../../AGENTS.md) when the correct direction is unclear or risky:
- 1 concrete problem
- 3 viable options
- 1 recommendation

Do not silently skip deliverables. Do not "fix" the problem by weakening the ticket without user confirmation.

When the user confirms a direction that changes the ticket's exact architecture boundary, affected files, or proof surface, update the relevant ticket sections before coding so the implementation and eventual archive remain faithful to the chosen plan. This commonly includes `Files to Touch`, `Verification Layers`, and `Test Plan`, not just the prose summary.

### 4. Extract the implementation scope

Turn the ticket into a concrete task list derived from:
- `What to Change`
- `Acceptance Criteria`
- required consequences discovered during reassessment

Separate:
- required in-scope work
- blocked work that needs user direction
- explicit out-of-scope work

If the ticket's requested invariant exposes a production contradiction, correct the scope first instead of pretending it is a tests-only change.

For golden tickets, remove duplicate proof from scope unless the new scenario proves a materially different contract from the existing coverage.

When a shared type changes, treat helper factories, sample fixtures, serialized scenario/config inputs, and schema examples as part of the construction-site sweep, not just direct Rust struct literals.

### 5. Implement with Worldwake discipline

1. Keep edits minimal and targeted.
2. Prefer the existing abstraction boundary instead of duplicating logic.
3. Use TDD for bug fixes:
   - add or update a test that captures the bug
   - confirm it fails for the right reason
   - fix the behavior
4. Never adapt tests to preserve a bug.
5. Do not add backward-compatibility shims, aliases, or dual paths.
6. Preserve critical invariants from [AGENTS.md](../../../AGENTS.md), especially:
   - belief-only planning
   - information locality
   - append-only event log
   - determinism
   - conservation
   - unique location
7. If authoritative validation, control checks, action preconditions, target specs, or other affordance-surface behavior changes, verify the full AI pipeline called out in `Authoritative-To-AI Impact Rule` in [AGENTS.md](../../../AGENTS.md).
8. When widening an existing action into a new custody or state regime, audit all related stored state carriers so the moved entity does not keep stale assignment, listing, queue, or other regime-specific markers after the transition.
9. When adding a new enum variant, search for exhaustive matches, pattern arms, and state validators in dependent crates and update the non-owning handlers explicitly before broad verification.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden when warranted.

Typical order:
1. focused test covering the changed behavior
2. crate-level tests for the affected crate
3. broader workspace validation if the change crosses boundaries or the ticket requires it

When the change touches more than one focused proof surface inside the same crate, run each focused selector needed to cover those boundaries rather than assuming one name filter is sufficient.

If you change code after a broader verification pass, rerun the narrowest affected tests and any broader command whose earlier result is now stale.

Use the repo-approved commands from [AGENTS.md](../../../AGENTS.md) when relevant, especially:

```bash
cargo test -p worldwake-core <test_name>
cargo test -p worldwake-core
cargo test -p worldwake-ai
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For AI, planner, golden, or start-failure work, prove behavior at the strongest available layer instead of relying on a weaker downstream proxy.

When a ticket changes whether an action should be available at all, include at least one focused proof that goes through real affordance enumeration rather than only constructing action instances directly.

When a valid architecture change makes an existing golden scenario stale, update the golden to prove the new lawful contract rather than preserving outdated failure reasons, plan shapes, or scenario narratives.

If the architecture change invalidates the old scenario invariant itself rather than just a timing detail or output shape, rewrite the scenario to prove the new contract and update the scenario header/comments to match.

### 7. Close the loop on the ticket

If the user asked for full ticket completion, update and archive the ticket per [docs/archival-workflow.md](../../../docs/archival-workflow.md).

When archiving:
- mark completion status accurately
- add an `Outcome` section with what changed and how it was verified
- note any approved partial completion and create a follow-up ticket when required

If the user asked only for implementation or analysis, do not archive automatically, but keep the factual completion details current enough that a later archival pass can record outcome and verification without reconstructing the session from scratch.

When golden coverage changes, keep the scenario prose and proof claim aligned with the updated assertions so the documented contract stays traceable.

Do not archive automatically if the user only asked for implementation or for analysis.

## Guardrails

- Correct stale ticket assumptions before coding against them.
- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- If a golden or mixed-layer scenario narrative diverges from live code, correct the narrative first.
- If you hit a real architectural contradiction, solve the contradiction or escalate with 1-3-1. Do not patch around it.
