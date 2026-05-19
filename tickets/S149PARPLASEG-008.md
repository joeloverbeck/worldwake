# S149PARPLASEG-008: Observer barrier rendering

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — observer planning-diagnostic rendering (read-only tooling)
**Deps**: S149PARPLASEG-001, S149PARPLASEG-003, S149PARPLASEG-005

## Problem

D9 makes barriers debuggable: the observer renders, per plan attempt, the typed terminal, the barrier fact, the resume condition, and the abandon status. This makes "what stopped this plan, and what would un-stop it?" answerable from the observer dump (FND-29).

## Assumption Reassessment (2026-05-20)

1. The spec's original "Observer Section 7 (planning)" is wrong — Section 7 is "End-State Inventory & Resources" (`crates/worldwake-cli/src/bin/observer.rs:4975`). The planning-diagnostic sections are Section 9 "Budget Exhaustion Snapshots" (1418) and Section 13 "Scenario Diagnostics" (4002, which carries `terminal_kind_distribution`). Section header convention: `## Section <N> — <Title>\n`.
2. `terminal_kind_distribution` is keyed by `PlanTerminalKindDiscriminant` after ticket 001 (`scenario_diagnostics/mod.rs:43`); Section 13 already aggregates it, so the seven typed kinds appear there with no observer change beyond label text. The new work is per-attempt barrier rendering (terminal + barrier fact + resume/abandon), which reads `PartialPlanSegment` fields (tickets 002/003) populated by ticket 005.
3. Shared boundary under audit: the observer's planning-diagnostic section formatting and the read-only `PartialPlanSegment` surface. This is observer-only tooling — no engine/simulation-state mutation; items 4–15 of the template are inapplicable.
4. Output-format fidelity: new lines must follow the existing section's formatting (indented sub-lines under a `Plan terminal:` header, matching the example in the spec). Render the resume condition as the derived `IntentionResumeCondition` (e.g. `BeliefStatusChanged(...)`), not the spec's original `BeliefUpdated` text.

## Architecture Check

1. Rendering reads `PartialPlanSegment` and the discriminant-keyed distribution through existing public surfaces — a read-only consumer, no new accessor or authoritative state (FND-27/FND-29).
2. Placing the per-attempt barrier detail in the existing planning-diagnostic sections (9/13) avoids a new section and keeps the dump's structure stable.

## Verification Layers

1. Barrier detail renders for a suspended attempt → headless observer render test asserting the `Plan terminal: <typed>` block with barrier fact + resume/abandon lines appears for a scenario that raises a barrier.
2. Distribution shows typed discriminants → headless render test asserting Section 13 lists the typed kinds (post-001 keying). Tooling-only ticket: headless render tests are the proof surface; no action-trace/event-log mapping applies.

## What to Change

### 1. Per-attempt barrier rendering

In the observer's planning-diagnostic section (Section 9 and/or 13 per the segment data available), render for each suspended attempt: `Plan terminal: <typed terminal>`, `Barrier fact: <BarrierFact>`, `Resume on: <IntentionResumeCondition>`, `Abandon if: <abandon condition> (<remaining attempts> resume attempts left)`.

### 2. Distribution label text

Ensure the Section 13 `terminal_kind_distribution` rendering labels the seven discriminant kinds (the keying change already landed in 001).

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify) — barrier rendering in Section 9/13 + distribution labels + render tests

## Out of Scope

- Any change to the typed-terminal taxonomy or discriminant (ticket 001).
- Producing/storing segments (tickets 002/003/005).
- Golden E2E coverage (ticket 009).

## Acceptance Criteria

### Tests That Must Pass

1. New: a headless observer render of a barrier-raising scenario shows the `Plan terminal:` block with barrier fact, resume condition, and abandon status.
2. New: Section 13 renders the typed discriminant kinds in `terminal_kind_distribution`.
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. The observer reads `PartialPlanSegment`/distribution through existing public surfaces; it introduces no new authoritative state and mutates nothing.
2. New lines preserve the `## Section <N> — <Title>` section structure and the indented-sub-line format.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline render tests) — barrier-detail block + distribution labels.

### Commands

1. `cargo test -p worldwake-cli`
2. `scripts/verify.sh`
