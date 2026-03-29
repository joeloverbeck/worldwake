# S35OBSACTSIG-008: Save/load round-trip for `BelievedActivity`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` save format version and save/load tests
**Deps**: S35 observable activity implementation in current head (`crates/worldwake-core/src/belief.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, `specs/S35-observable-activity-signals.md`)

## Problem

`BelievedActivity` is now persisted inside `AgentBeliefStore` via `BelievedEntityState`. The ticket must prove that current-head save/load preserves that data inside `SimulationState`. The original ticket also claimed old saves would continue to deserialize through `#[serde(default)]`, but that does not match the live code or the repository's forward-only save-format practice.

## Assumption Reassessment (2026-03-29)

1. `BelievedActivity` is already live in current head, not merely planned. `crates/worldwake-core/src/belief.rs` defines `BelievedActivity` and `BelievedEntityState { believed_activity: Option<BelievedActivity>, ... }`, and `crates/worldwake-sim/src/per_agent_belief_view.rs` already has focused runtime tests for `believed_activity_of()` and `agents_active_at()`.
2. `save()` / `load()` in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) serialize the entire `SimulationState` with `bincode`, gated by explicit `SAVE_FORMAT_VERSION`. This is the authoritative persistence boundary under audit for this ticket.
3. `BelievedEntityState.believed_activity` does not have `#[serde(default)]` in [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs). Current-head `bincode` deserialization of older bytes that lack this field would fail rather than default to `None`.
4. The repository already treats persisted-schema changes as explicit format changes, not silent backward-compatible field additions. [specs/IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) records prior save-format bumps for persisted-state changes, and [specs/S35-observable-activity-signals.md](/home/joeloverbeck/projects/worldwake/specs/S35-observable-activity-signals.md) explicitly rejects backward-compatibility shims for adjacent persisted S35 state.
5. Current save/load coverage in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) exercises broad non-default round-trip, runtime-payload round-trip, legacy v5 acceptance, and rejection of previous/current versions after schema bumps. It does not currently prove that a populated `AgentBeliefStore` with `BelievedActivity` survives save/load.
6. Mismatch + correction: the original ticket's "Engine Changes: None", "`#[serde(default)]`", "old-format deserialization yields `None`", and "no `SAVE_FORMAT_VERSION` bump required" assumptions are all stale or incorrect. Correct scope is a forward-only current-head save-format bump plus focused proof that `BelievedActivity` survives save/load.
7. Adjacent contradiction classification: previous save bytes written under version 10 are no longer trustworthy for current-head `SimulationState` because the persisted schema changed. That is a required consequence of this ticket's audited boundary, not a separate bug.

## Architecture Check

1. The clean architecture is an explicit save-format bump plus targeted persistence coverage. That matches the existing `save_load.rs` design, keeps schema changes honest, and avoids pretending byte-compatible deserialization exists when it does not.
2. Adding `#[serde(default)]` or another compatibility shim here would be worse than the current architecture. It would hide a persisted-schema change behind an implicit alias path, weaken format-version meaning, and create split semantics between "current schema" and "legacy missing-field schema" inside a bincode blob that is otherwise treated as versioned.
3. No backwards-compatibility aliasing or migration shim is introduced. If older version-10 saves are no longer valid after the schema change, the format version must say so explicitly.

## Verification Layers

1. `BelievedActivity` bytes survive current-head save/load intact -> focused `save_load.rs` unit test over `SimulationState`
2. Save header reflects the new persisted schema boundary -> focused `save_load.rs` version assertion
3. Previous current version is rejected after the bump -> existing `save_load.rs` wrong-version rejection test
4. Single-layer ticket: this is a persistence-boundary contract, so additional action-trace / decision-trace / event-log mapping is not applicable

## What to Change

### 1. Bump the save format

Increase `SAVE_FORMAT_VERSION` in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) to reflect the persisted `BelievedEntityState` schema change. Keep the existing explicit version-gate behavior.

### 2. Add `BelievedActivity` save/load coverage

Extend the `save_load.rs` test module so the populated `SimulationState` includes an agent belief store whose known-entity snapshot carries non-default `believed_activity: Some(BelievedActivity { ... })`. Assert that current-head save/load round-trips that data exactly.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify)

## Out of Scope

- Adding backward-compatible deserialization for pre-`BelievedActivity` version-10 save bytes
- Changing `BelievedActivity` semantics or `BelievedEntityState` shape
- Refactoring unrelated save/load architecture

## Acceptance Criteria

### Tests That Must Pass

1. Save/load round-trip preserves `BelievedActivity` with all fields (`action_domain`, `target`, `observed_tick`) through `SimulationState`.
2. Save output writes the bumped `SAVE_FORMAT_VERSION`.
3. Previous-current-version bytes are rejected after the bump.
4. Existing suite: `cargo test -p worldwake-sim save_load`
5. Existing suite: `cargo test --workspace`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Persisted belief state must survive save/load without semantic loss (FOUNDATIONS P11).
2. Save-format versions must reflect persisted schema changes honestly; version numbers cannot imply compatibility that current code does not provide.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` — extend the populated-state round-trip to include a non-default `BelievedActivity`, proving authoritative save/load persistence for the new belief field.
2. `crates/worldwake-sim/src/save_load.rs` — keep version-header and prior-version rejection coverage as the proof surface for the explicit format bump.

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-03-29

What actually changed:
- Bumped `SAVE_FORMAT_VERSION` from 10 to 11 in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) to match the persisted `BelievedEntityState` schema change.
- Extended the `save_load.rs` populated-state fixture so an agent belief store carries a non-default `BelievedActivity`.
- Strengthened the existing save/load round-trip test to assert that the restored belief store still contains the expected `BelievedActivity`.

Deviations from original plan:
- Did not implement backward-compatible deserialization for older saves missing `believed_activity`. Reassessment showed the original `#[serde(default)]` plan was incorrect for current code and weaker than the repository's explicit save-format architecture.
- Ticket scope was corrected from "test only" to "save-format bump plus test coverage."

Verification results:
- `cargo test -p worldwake-sim save_load` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
