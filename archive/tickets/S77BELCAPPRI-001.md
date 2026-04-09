# S77BELCAPPRI-001: Add `believed_kind` to belief state structs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `ObservedEntitySnapshot` and `BelievedEntityState` gain a new field
**Deps**: None

## Problem

The belief store's capacity enforcement needs to distinguish entity kinds (Place, Facility, Agent vs ItemLot, etc.) to prioritize infrastructure beliefs over transient ground-item beliefs. Currently `BelievedEntityState` has no `believed_kind` field, so entity kind cannot be determined from beliefs alone. `build_observed_entity_snapshot()` already calls `world.entity_kind(entity)` but discards the return value.

## Assumption Reassessment (2026-04-09)

1. `ObservedEntitySnapshot` defined at `crates/worldwake-core/src/belief.rs:1210` with 10 fields, no `believed_kind`. 4 construction sites: `belief.rs:1730`, `belief.rs:3877` (test), `event_record.rs:683`, `perception.rs:1241`.
2. `BelievedEntityState` defined at `crates/worldwake-core/src/belief.rs:1297` with 13 fields, no `believed_kind`. ~100+ construction sites across all crates (majority in test code). Macro expansion sites at `component_tables.rs:211`, `world.rs:701`, `delta.rs:453`.
3. Shared boundary: `build_observed_entity_snapshot()` at `belief.rs:1708` — captures world state into snapshot. `to_believed_entity_state()` at `belief.rs:1228` — converts snapshot to belief. `derive_entity_summary()` at `belief.rs:1838` — reconstructs belief from claims (cannot derive `believed_kind` from claims alone).
4. `EntityKind` derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` at `entity.rs:7`. `Option<EntityKind>` satisfies all existing trait bounds on `BelievedEntityState` (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`).
5. Auto-correction: the ticket named `enforce_entity_claim_capacity()` / `enforce_capacity()` as the only post-`derive_entity_summary()` preservation path. Live code also re-derives summaries through `refresh_entity_summary_from_claims()`, so that caller must preserve prior `believed_kind` too or ordinary claim refresh erases the stored kind.

## Architecture Check

1. Adding `believed_kind` as concrete stored state aligns with P3 (Concrete State Over Abstract Scores). The alternative — inferring entity kind from proxy fields (resource_source, workstation_tag) — cannot reliably detect Place entities and would be fragile.
2. No backward-compatibility shims. The `#[serde(default)]` annotation allows existing serialized data to deserialize with `believed_kind: None`; this is a migration-forward pattern, not a shim.

## Verification Layers

1. `believed_kind` populated at observation time -> focused unit test asserting `build_observed_entity_state()` returns snapshot with correct `believed_kind`
2. `believed_kind` survives snapshot-to-belief conversion -> focused unit test asserting `to_believed_entity_state()` preserves `believed_kind`
3. `derive_entity_summary()` preserves prior `believed_kind` -> focused unit test (this function can't derive kind from claims; it must fall back to prior known_entities entry)
4. Single-layer ticket: changes are internal to belief data structures in worldwake-core, no cross-system interaction

## What to Change

### 1. Add `believed_kind` to `ObservedEntitySnapshot`

In `crates/worldwake-core/src/belief.rs`, add `#[serde(default)] pub believed_kind: Option<EntityKind>` to `ObservedEntitySnapshot` (after line 1223).

### 2. Populate `believed_kind` in `build_observed_entity_snapshot()`

In `build_observed_entity_snapshot()` (belief.rs:1708), capture the `entity_kind()` return value instead of discarding it. Change line 1712 from `world.entity_kind(entity)?;` to `let kind = world.entity_kind(entity)?;` and include `believed_kind: Some(kind)` in the returned struct.

### 3. Add `believed_kind` to `BelievedEntityState`

Add `#[serde(default)] pub believed_kind: Option<EntityKind>` to `BelievedEntityState` (after line 1311).

### 4. Thread through `to_believed_entity_state()`

In `to_believed_entity_state()` (belief.rs:1228), add `believed_kind: self.believed_kind` to the constructed `BelievedEntityState`.

### 5. Propagate in `derive_entity_summary()`

In `derive_entity_summary()` (belief.rs:1838), this function cannot derive entity kind from claims. Set `believed_kind: None` in the initial struct construction. The callers that re-derive summaries from claims (`refresh_entity_summary_from_claims()` and `enforce_entity_claim_capacity()`) must preserve the prior `believed_kind` from `known_entities` after `derive_entity_summary()` returns, by copying it from the previous entry if one exists.

### 6. Update all construction sites

Add `believed_kind: None` to every `BelievedEntityState { ... }` and `ObservedEntitySnapshot { ... }` literal across the workspace. This is a mechanical change — no behavioral difference for existing code. Key files:

**worldwake-core** (production): `communication.rs:148,177,206`, `world_txn.rs:4601`, `component_tables.rs:211`, `world.rs:701`, `delta.rs:453`
**worldwake-core** (tests): `belief.rs` (~10 sites in test helpers)
**worldwake-sim**: `save_load.rs:275`, `social_relay.rs:288`, `per_agent_belief_view.rs:1536,1562`
**worldwake-systems** (production): `investigate_actions.rs:552`, `perception.rs:1241,1332`
**worldwake-systems** (tests): `perception.rs` (~12 sites), `tell_actions.rs` (~13 sites), `office_actions.rs`, `justice_actions.rs`, `artifact_actions.rs`
**worldwake-ai** (tests): `route_threat.rs`, `exhaustion.rs`, `plan_revalidation.rs`, `planning_state.rs`, `candidate_generation.rs`, `pursuit_belief.rs`, `ranking.rs`, `planning_snapshot.rs`, `search/tests.rs`, `goal_model.rs`
**ObservedEntitySnapshot**: `event_record.rs:683`, `perception.rs:1241`, `belief.rs:3877` (test)

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/communication.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `crates/worldwake-sim/src/social_relay.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-systems/src/justice_actions.rs` (modify)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify)
- `crates/worldwake-ai/src/route_threat.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/pursuit_belief.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changing eviction logic (that is S77BELCAPPRI-002 and S77BELCAPPRI-003)
- Removing the SceneEvidence gate (that is S77BELCAPPRI-004)
- Adding any new perception or planning behavior based on `believed_kind`
- Changing belief capacity parameters

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `build_observed_entity_snapshot` populates `believed_kind` with correct `EntityKind` for Place, Agent, ItemLot entities
2. New unit test: `to_believed_entity_state` preserves `believed_kind` from snapshot
3. Existing suite: `cargo test -p worldwake-core`
4. Existing suite: `cargo test --workspace`

### Invariants

1. `believed_kind` is `Some(kind)` for all beliefs created through `build_believed_entity_state()` / `build_observed_entity_snapshot()`
2. `believed_kind` is `None` for beliefs reconstructed by `derive_entity_summary()` (until the caller patches it from prior state)
3. All existing tests pass unchanged — the new field defaults to `None` and has no behavioral impact in this ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — `build_observed_entity_snapshot_captures_believed_kind` — confirms entity kind flows from world state into snapshot
2. `crates/worldwake-core/src/belief.rs` — `to_believed_entity_state_preserves_believed_kind` — confirms snapshot-to-belief conversion preserves kind

### Commands

1. `cargo test -p worldwake-core -- believed_kind`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Added `believed_kind: Option<EntityKind>` to `ObservedEntitySnapshot` and `BelievedEntityState`.
- Populated `believed_kind` from `build_observed_entity_snapshot()` and threaded it through `to_believed_entity_state()`.
- Preserved prior `believed_kind` when claim-only summary re-derivation runs through both `refresh_entity_summary_from_claims()` and `enforce_entity_claim_capacity()`.
- Updated workspace constructor fallout to seed `believed_kind: None` where no concrete observed kind is available in test/setup literals.
- Added focused tests proving observation captures entity kind and claim refresh preserves prior kind.

## Verification Result

- Passed `cargo test -p worldwake-core -- believed_kind`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
