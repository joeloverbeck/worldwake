# S129PLADIRFAC-001: Place dirtiness and facility wear components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS components on `EntityKind::Place` and `EntityKind::Facility`; bumps `SAVE_FORMAT_VERSION`
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md (D1, D2, D3)

## Problem

Hygiene is currently a per-agent property only. Places do not carry dirtiness from repeated wilderness relief, latrine-tagged places do not track fill state, and washbasin facilities have no per-basin water buffer. Without these state carriers, the consequence chain "wilderness relief → place dirtier → sleep worse there → travel to clean shelter" is impossible to express. This ticket lands the three foundational components and registers them in the ECS schema so all downstream tickets (event tags, handler extensions, AI ranking, scenario authoring, goldens) have something to read and write.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Permille` exists at `crates/worldwake-core/src/numerics.rs:25` with `new_unchecked` constructor (line 43), `saturating_add` (line 56), `saturating_sub` (line ~62), and `ZERO` constant (line 28). No `FULL` constant exists — components rely on `saturating_add` clamping at 1000. Existing inline tests for the registration macro at `crates/worldwake-core/src/component_tables.rs` and `world.rs` are the proof surface for new component accessor generation.
2. `Component` trait at `crates/worldwake-core/src/traits.rs:15` requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned`. The proposed three components all satisfy these via `derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)`. `Default` is mandatory because `PlaceDirtiness` is universal-on-Place (every place implicitly carries one), but the S129 D1/D2/D3 defaults are non-zero for rates/thresholds/capacity; implement manual `Default` impls instead of deriving zero defaults.
3. The shared abstraction boundary under audit is the component-schema macro `with_component_schema_entries!` (referenced from `world_txn.rs:1142, 2286, 2900`, `delta.rs:37, 69, 70`, `world.rs:658, 659`, and `component_tables.rs:138, 142`). The macro generates `insert_component_*`, `get_component_*`, `set_component_*`, `entities_with_*`, `query_*`, `count_with_*`, etc. accessors per-component and counts authoritative components for the `ComponentKind::ALL` table. Adding three components extends this surface at every macro call site and bumps the count constant.
4. `EntityKind::Place` precedent for kind-filtered registration: `BanditCamp`, `SceneEvidence`, `PlaceVisibilityProfile`, `SleepQualityProfile` all use `|kind| kind == EntityKind::Place,` (`component_schema.rs:1660, 1685`). `EntityKind::Facility`-only registration: `MerchantStorage`, `ContentionPolicy` etc. — confirm filter shape during implementation. `WorkstationTag::WashBasin` exists at `production.rs:15`; `PlaceTag::Latrine` exists at `topology.rs:18`.
5. `SAVE_FORMAT_VERSION = 55` at `crates/worldwake-sim/src/save_load.rs:6`. Adding three new component types to the schema changes the serialized world surface — must bump to `56`. No backward-compat shim is appropriate per FND-28.

## Architecture Check

1. Co-locating `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` in a single new module `place_dirtiness.rs` keeps the hygiene-state domain cohesive — sibling modules like `sleep_episode.rs` (S128's home for `SleepQualityProfile`) set the precedent of one-domain-per-module. Splitting into three modules would scatter the hygiene domain without benefit; merging into existing modules (e.g., `topology.rs` for place state) would couple unrelated concerns.
2. No backwards-compatibility aliasing: the per-agent dirtiness pathway in `HomeostaticNeeds.dirtiness` is preserved unchanged (it is correct — agents do still get dirty); the new place/facility state is additive. No shim layer translates between agent-side and place-side dirtiness.

## Verification Layers

1. Component registration produces all expected accessors → focused unit tests in `place_dirtiness.rs` mod tests asserting `World::set_component_place_dirtiness`, `get_component_place_dirtiness`, `entities_with_place_dirtiness`, etc. compile and round-trip.
2. `EntityKind` filter rejects components on wrong-kind entities → focused unit tests assert wrong-kind insertion returns the live schema error, `WorldError::InvalidOperation`.
3. Save/load round-trips the new components → focused save/load test seeds a world with one Place carrying `PlaceDirtiness`, one Place carrying `LatrineFullness`, and one WashBasin facility carrying `WashBasinState`, then `save → load → assert` all three components present with correct field values. Latrine tag-conditional spawning remains ticket 011 scope.
4. `SAVE_FORMAT_VERSION` bump is observable → save_load test asserts `SAVE_FORMAT_VERSION == 56` after this ticket.

## What to Change

### 1. New module `crates/worldwake-core/src/place_dirtiness.rs`

Define three components per the spec D1/D2/D3 shapes. Implement manual `Default` for each so omitted scenario/spawn values use the spec defaults (`PlaceDirtiness`: 0/2/80; `LatrineFullness`: 0/80/800; `WashBasinState`: 10/10/1/2/0/50). Implement `Component` for each. Add a `pub mod place_dirtiness;` line to `lib.rs` and re-export the three structs alongside existing core types.

### 2. Schema registration in `component_schema.rs`

Add three entries to the `with_component_schema_entries!` macro invocation that lists authoritative components. For each, declare the table name, type, full accessor list, registration string, and the kind filter:

- `PlaceDirtiness`: `|kind| kind == EntityKind::Place,` (universal-on-Place — every place implicitly has one)
- `LatrineFullness`: `|kind| kind == EntityKind::Place,` (role-specific by `PlaceTag::Latrine`; the kind filter is still Place since it lives on a place; tag-conditional spawning happens at the scenario layer per ticket 011)
- `WashBasinState`: `|kind| kind == EntityKind::Facility,` (role-specific by `WorkstationTag::WashBasin`; tag-conditional spawning happens at the scenario layer per ticket 011)

Mirror the SleepQualityProfile registration block (`component_schema.rs:1718–1737`) for the universal-on-Place pattern.

### 3. Macro expansion site imports

Per `tickets/README.md` check #13, the `with_component_schema_entries!` macro generates code that references new types by bare name at most expansion sites. Add imports for the expansion sites that compile generated bare component type names:

- `crates/worldwake-core/src/delta.rs` (three macro invocations)
- `crates/worldwake-core/src/world.rs` (two macro invocations)
- `crates/worldwake-core/src/component_tables.rs` (two macro invocations)

No change is needed in `world_txn.rs`: the live `select_txn_simple_set_components` expansion uses crate-qualified transaction component types for this surface.

### 4. SAVE_FORMAT_VERSION bump

Edit `crates/worldwake-sim/src/save_load.rs:6` to set `pub const SAVE_FORMAT_VERSION: u32 = 56;`. Update the test asserting the format constant (if one exists in `save_load.rs`'s `#[cfg(test)]` block).

### 5. Component-kind count update

If `delta.rs:37` declares a `ComponentKind::ALL` array indexed by a count derived from `with_component_schema_entries!(forward_authoritative_components, count_authoritative_components)`, no manual count change is needed — the macro recounts automatically. Verify the count macro recompiles after the three additions.

## Files to Touch

- `crates/worldwake-core/src/place_dirtiness.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add `pub mod place_dirtiness;` and re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — three new schema entries)
- `crates/worldwake-core/src/delta.rs` (modify — imports at macro expansion sites)
- `crates/worldwake-core/src/world.rs` (modify — imports at macro expansion sites)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports at macro expansion sites)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 55→56 and any version-asserting test)

## Out of Scope

- Per-tick decay of `PlaceDirtiness` (deferred to ticket 008).
- Basin natural refill from `ResourceSource` (deferred to ticket 008).
- Action handler integration (deferred to tickets 005, 006, 007).
- Scenario authoring of the new fields (deferred to ticket 011 — components are universally registered with `Default` here, so existing scenarios continue to load).
- AI ranking reads (deferred to tickets 004, 009, 010).

## Acceptance Criteria

### Tests That Must Pass

1. New focused tests in `place_dirtiness.rs`'s `#[cfg(test)]` block cover spec defaults, component bounds/bincode round-trips, accessors, and wrong-kind rejection. The save/load seam cannot live in `worldwake-core` because `save_load` is owned by `worldwake-sim`.
2. New focused save/load assertions in `crates/worldwake-sim/src/save_load.rs` seed one Place with `PlaceDirtiness`, one Place with `LatrineFullness`, and one Facility with `WashBasinState`, then save/load/assert equal.
3. New focused test: `place_dirtiness_kind_filter_rejects_facility` — attempt to set `PlaceDirtiness` on an `EntityKind::Facility` and assert the world rejects it with the appropriate error.
4. New focused test: `wash_basin_state_kind_filter_rejects_place` — analogous (basin state is facility-only).
5. Existing `dispatch_table_routes_item_decay_system` and other inline tests in `item_decay.rs` continue to pass (no behavior change here; just confirms the new components don't break the existing dispatch).
6. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`.

### Invariants

1. Every `EntityKind::Place` entity may have a `PlaceDirtiness` component (universal); attempting to set it on any other kind returns the appropriate `WorldError`.
2. `LatrineFullness` may be set on `EntityKind::Place` only; tag-conditional application to latrine-tagged places is enforced at the scenario layer (ticket 011), not by the schema kind filter.
3. `WashBasinState` may be set on `EntityKind::Facility` only; tag-conditional application to WashBasin-tagged facilities is enforced at the scenario layer (ticket 011).
4. `SAVE_FORMAT_VERSION == 56` after this ticket; loads of older format values fail with `SaveError::UnsupportedVersion` per existing save-load semantics.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/place_dirtiness.rs` (new) — inline `#[cfg(test)]` block covering defaults, bincode round-trip per component, generated accessors, and kind-filter rejection.
2. `crates/worldwake-sim/src/save_load.rs` — extend the non-default state round-trip and format-version assertion tests to cover the new persisted components and `SAVE_FORMAT_VERSION == 56`.

### Commands

1. `cargo test -p worldwake-core place_dirtiness`
2. `cargo test -p worldwake-sim save_load`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`

Merge note: This ticket bumps `SAVE_FORMAT_VERSION` 55→56. Ticket 002 may also need a bump if EventTag/DecisionEventPayload variants are part of the saved-state surface — to be confirmed during 002's reassessment. If 002 also bumps, the merge order is 001 (→56) then 002 (→57); landing them out of order produces a value collision.

## Outcome

Completed on 2026-04-29.

- Added `PlaceDirtiness`, `LatrineFullness`, and `WashBasinState` in `crates/worldwake-core/src/place_dirtiness.rs` with manual spec-defined non-zero defaults.
- Registered all three authoritative components in the ECS schema, exported them from `worldwake-core`, and updated `ComponentKind` / `ComponentValue` sample coverage for the new persisted component variants.
- Bumped `SAVE_FORMAT_VERSION` from 55 to 56 and extended the save/load non-default round-trip to preserve all three new components.
- Updated the active S129 spec snippets so the documented component defaults match the live manual `Default` implementations.

## Deviations

- The drafted `worldwake-core` save/load tests were moved to the live `worldwake-sim` save/load seam because `worldwake-core` cannot depend on `worldwake-sim`.
- `world_txn.rs` did not need a landed import change; its selected component-setter macro path uses crate-qualified component types.

## Verification Result

- Passed `cargo test --workspace --no-run`.
- Passed `cargo test -p worldwake-core --lib place_dirtiness` (6 tests).
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_writes_current_format_version -- --exact`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo test -p worldwake-sim`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`, whose live gate set is `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
