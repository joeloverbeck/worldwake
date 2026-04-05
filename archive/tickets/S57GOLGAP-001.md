# S57GOLGAP-001: Golden secondary-jurisdiction punishment closeout

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — golden E2E coverage + generated golden docs
**Deps**: None (S50 rights lattice fully implemented; Scenario 110 already proves in/out-jurisdiction gating)

## Problem

Scenario 110 proves jurisdiction-gated punishment (in-jurisdiction succeeds, out-of-jurisdiction suppressed), but its positive branch does not require a distinct seat and secondary jurisdiction place. No golden test proves that the S50 `seat` / `jurisdiction` split allows lawful punishment at a place inside the office's jurisdiction that is NOT the office seat. This is the remaining cross-system contract distinguishing the multi-place jurisdiction model from the earlier single-place office model.

## Assumption Reassessment (2026-04-05)

1. `GoalKind::PunishAccused` exists at `crates/worldwake-core/src/goal.rs:95` — confirmed.
2. `RightKind::JurisdictionalAuthority` exists and is used in `crates/worldwake-core/src/world/ownership.rs:272` for rights lattice queries — confirmed.
3. Scenario 110 exists at `crates/worldwake-ai/tests/golden_emergent.rs:7331` ("Jurisdiction-Gated Punishment") — confirmed. It proves in/out-jurisdiction but not seat-vs-secondary-jurisdiction.
4. `golden_emergent.rs` exists at `crates/worldwake-ai/tests/golden_emergent.rs` — confirmed. This is the target file per the spec.
5. `OfficeData` has `seat` and `jurisdiction` fields — implied by S50 implementation. The spec references `OfficeData { seat, jurisdiction }` split.
6. Focused tests in `crates/worldwake-systems/src/offices.rs` and `crates/worldwake-systems/src/office_actions.rs` prove `.contains()` membership and seat-local political constraints separately — lower-layer coverage exists but no E2E golden.
7. `emit_punishment_candidates()` in candidate generation uses office-specific jurisdiction gate — referenced by spec as exercised system.
8. The existing `run_jurisdiction_gated_punishment_branch()` helper in `crates/worldwake-ai/tests/golden_emergent.rs` already owns the punishment-ready accusation setup for this contract. The honest implementation boundary is to widen that helper for a seat-distinct in-jurisdiction branch, not to add a second bespoke punishment harness.
9. Adding a new `// Scenario` block changes the repo-global golden inventory surface, so `python3 scripts/golden_inventory.py --write --check-docs` is part of the owned verification path.

## Architecture Check

1. This is a golden-only ticket. No production code changes. The golden test exercises the cross-crate contract: rights lattice (worldwake-core) → office substrate (worldwake-core) → justice candidate generation (worldwake-ai) → punishment action (worldwake-systems). All interaction through shared state per Principle 26.
2. No backward-compatibility shims.
3. Reusing the existing Scenario 110 helper keeps the proof surface honest and avoids drift between two near-identical punishment harnesses whose only intended difference is the seat-vs-jurisdiction split.

## Verification Layers

1. Office seat is distinct from punishment place → authoritative world state (`OfficeData.seat != punishment_place`)
2. Punishment place is inside office jurisdiction → authoritative world state (`OfficeData.jurisdiction.contains(punishment_place)`)
3. `PunishAccused` goal generated at secondary jurisdiction place → decision trace
4. Punishment action committed at secondary jurisdiction place → action trace
5. `JurisdictionalAuthority` right carried via office, not via seat co-location → authoritative world state (rights lattice query)
6. Deterministic replay → world hash and event-log hash comparison

## What to Change

### 1. Extend the existing punishment golden with a seat-distinct in-jurisdiction branch

In `crates/worldwake-ai/tests/golden_emergent.rs`:

**Setup**:
- Reuse the existing `run_jurisdiction_gated_punishment_branch()` harness.
- Add a branch where:
  - office `seat = RulersHall`
  - office `jurisdiction = { RulersHall, GeneralStore }`
  - authority and accused start at `GeneralStore`
  - accusation state and rights knowledge are already available to the authority there
- Keep the existing Scenario 110 in/out-jurisdiction proof intact; add a new scenario block for the seat-distinct positive case rather than replacing the old contract.

**Execution**: Tick simulation with bounded limit until punishment commits.

**Assertions**:
- Office seat is RulersHall, not GeneralStore (authoritative world state).
- GeneralStore is in office jurisdiction (authoritative world state).
- Office holder generated `PunishAccused` at GeneralStore (decision trace).
- Punishment action committed at GeneralStore (action trace).
- Office holder did NOT travel to RulersHall before punishing (action trace absence of travel to seat).
- `JurisdictionalAuthority` right derived from office, not seat co-location (rights lattice).

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical world hash and event-log hash.

### 3. Refresh generated golden docs

Run `python3 scripts/golden_inventory.py --write --check-docs` after adding the new scenario metadata so the inventory, scenario map, and coverage matrix stay current.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)

## Out of Scope

- In/out-jurisdiction gating (already covered by Scenario 110)
- Seat-local political actions (covered by focused tests)
- Rights lattice production code changes
- Office seat relocation scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Golden: punishment commits at a place inside jurisdiction that is NOT the office seat
2. Assertions confirm seat/jurisdiction split is load-bearing (seat ≠ punishment place, punishment place ∈ jurisdiction)
3. Deterministic replay companion reproduces identical outcome
4. Generated golden docs refresh cleanly
5. Existing suite: `cargo test --workspace`

### Invariants

1. Punishment legality depends on `JurisdictionalAuthority` via office — not on seat co-location
2. Office seat remains unchanged after punishment at secondary jurisdiction place
3. Accusation and rights knowledge arrive through lawful perception/belief paths (Principle 7)
4. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — secondary-jurisdiction punishment golden scenario + replay companion
2. `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-map.md` — generated refresh after new scenario metadata

### Commands

1. `cargo test -p worldwake-ai golden_jurisdiction_gated_punishment -- --nocapture`
2. `cargo test -p worldwake-ai secondary_jurisdiction_punishment -- --nocapture`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace -q`

## Outcome

Completed: 2026-04-05

Added Scenario 111 in `crates/worldwake-ai/tests/golden_emergent.rs` by extending the existing Scenario 110 punishment harness instead of creating a second setup path. The new golden proves that punishment can commit at `GeneralStore` while the issuing office seat remains `RulersHall`, as long as the same office's jurisdiction includes the secondary place and the acting holder's `JurisdictionalAuthority` right is carried via that office.

The only deviation from the original plan was a lawful fixture correction inside the reused harness: the local crime register had to live at the punishment place, because authoritative punishment validation resolves against a local register there. No production code changed.

Verification:
- `cargo test -p worldwake-ai golden_jurisdiction_gated_punishment -- --nocapture`
- `cargo test -p worldwake-ai secondary_jurisdiction_punishment -- --nocapture`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace -q`
