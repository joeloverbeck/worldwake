# DOCTRAC-001: Layer Verification Contracts and Ground Coverage Claims

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, `specs/S13-political-emergence-golden-suites.md`

## Problem

Recent cross-system ticket work exposed two documentation failures:

1. verification language was specified at the scenario level instead of the architectural-layer level, which blurred action-lifecycle assertions together with authoritative system-mutation assertions
2. golden-suite coverage claims drifted away from the real test inventory and understated existing focused/system coverage

That makes tickets easier to misread and weakens the repo's stated contract that assumptions must be grounded in current code and tests before implementation.

## Assumption Reassessment (2026-03-18)

1. `tickets/README.md` already requires assumption reassessment, exact test naming, and distinguishing AI/planning logic from authoritative/system logic, but it does not yet require per-invariant verification-layer mapping for mixed cross-system scenarios.
2. `tickets/_TEMPLATE.md` has no explicit slot for "this invariant is checked via decision trace vs action trace vs event-log/world-state". Contributors can satisfy the template while still writing vague assertion surfaces.
3. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` are manually maintained and can drift from the real golden inventory. Current counts were recently stale relative to `cargo test -p worldwake-ai -- --list`.
4. `specs/S13-political-emergence-golden-suites.md` currently describes Scenario 21 as exercising AI political behavior (`ClaimOffice`, `DeclareSupport`) even though force-law succession is actually authoritative system behavior in `crates/worldwake-systems/src/offices.rs:13`.
5. The gap here is documentation/authoring precision, not missing engine behavior.

## Architecture Check

1. Tightening the ticket/spec contract is cleaner than compensating with ad hoc reviewer memory. The repo should encode how to specify mixed-layer invariants instead of relying on implementers to infer it every time.
2. Grounding coverage claims in `cargo test -p worldwake-ai -- --list` and exact focused/system test names is more robust than maintaining hand-wavy counts or scenario summaries.
3. No backward-compatibility shims or aliasing are involved; this is a precision upgrade to ticket/spec authoring rules and S13 wording.

## What to Change

### 1. Tighten ticket authoring rules

Update `tickets/README.md` and `tickets/_TEMPLATE.md` so mixed-system tickets must explicitly map each important invariant to its verification layer, for example:

- AI reasoning or candidate absence: decision trace / focused runtime test
- action lifecycle ordering: action trace
- authoritative mutation ordering: event-log deltas and/or authoritative world state

The contract should explicitly forbid writing a single vague assertion surface for a scenario that spans multiple layers.

### 2. Ground golden-suite claims in real inventory

Update the authoring guidance to require `cargo test -p worldwake-ai -- --list` before claiming golden-suite counts or gaps, and require exact existing focused/system tests to be named when a ticket claims a missing coverage area.

### 3. Correct S13 political-emergence wording

Update `specs/S13-political-emergence-golden-suites.md` so the force-law succession scenario names the actual architecture:

- combat -> `DeadAt`
- politics -> `succession_system()`
- no `ClaimOffice` / `DeclareSupport` path for force-law installation

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `specs/S13-political-emergence-golden-suites.md` (modify)
- `docs/golden-e2e-coverage.md` (modify only if wording references the old blurred verification model)
- `docs/golden-e2e-scenarios.md` (modify only if wording references the old blurred verification model)

## Out of Scope

- Adding new runtime traces or harness helpers
- Changing political, combat, or AI behavior
- Auto-generating the golden docs

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- --list`
2. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. Ticket/spec language for cross-system scenarios must distinguish verification layers instead of collapsing them into one generic "trace" assertion surface.
2. Coverage-gap claims must be grounded in the current repository state, including existing focused/system coverage where applicable.

## Test Plan

### New/Modified Tests

1. No new Rust tests expected — this ticket strengthens ticket/spec/docs precision.
2. Re-read the updated docs/tickets against at least one mixed-layer scenario (`S13` force-law succession) to confirm the wording now maps invariant -> verification layer explicitly.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_emergent`

