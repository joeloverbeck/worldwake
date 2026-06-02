# S178PERFOOSPO-002: MetabolismProfile spoiled_food_hunger_threshold field

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `MetabolismProfile` gains `spoiled_food_hunger_threshold: Permille`; compiler-confirmed constructor fallout resolved; `SAVE_FORMAT_VERSION` 116→117.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`

## Problem

Before this ticket, D7's per-agent desperation threshold for spoiled-food Eat candidates had no profile field. The threshold now lives on the universal `MetabolismProfile` per the spec's universal-profile reuse decision — no new component, no new `AgentDef` field, no new `spawn_agent` call site. The implementation resolved the compiler-confirmed construction fallout, regenerated `docs/profiles/all-profiles.md`, and bumped `SAVE_FORMAT_VERSION` 116→117 to cover the format-breaking `MetabolismProfile` change.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MetabolismProfile` at `crates/worldwake-core/src/needs.rs:142` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` and has a `Default` impl at lines 283-309 covering all 21 current fields. `AgentDef.metabolism_profile: Option<MetabolismProfile>` exists at `crates/worldwake-cli/src/scenario/types.rs:645` with `#[serde(default)]`. `spawn_agent` at `crates/worldwake-cli/src/scenario/mod.rs:980-981` applies via `unwrap_or_default()`. Universal-profile contract per `docs/spec-drafting-rules.md` §"Agent Profile Scenario Contract" — field addition rides the existing path. Construction sites enumerated via `rg '^\s*MetabolismProfile\s*\{$' crates/`: 18 literal-opening forms workspace-wide, 0 spread-syntax sites. Production: 1 site at `crates/worldwake-sim/src/save_load.rs:354` (load path), 3 sites in `crates/worldwake-systems/src/needs.rs` (lines 1861, 1913, 2003 — confirm cfg(test) boundary; if production, treat as production). Test: 14 sites across `crates/worldwake-ai/tests/`. Authored RON configs: 5 scenarios reference `metabolism_profile:` (`scenarios/cognitive-archetypes-divergence.ron`, `survival-preferences.ron`, `survival-baseline.ron`, `survival-basin-dirty-dirty.ron`, `cli-evaluation.ron`); adding `#[serde(default)]` on the new field absorbs these without RON edits.
2. Spec D7 verified against current `archive/specs/S178-perishable-food-spoilage.md`. FOUNDATIONS Alignment row FND-22 mandates per-agent profile parameter for diversity; FND-2 mandates concrete profile-authored values (no magic numbers). `world_txn.rs` at `crates/worldwake-core/src/world_txn.rs:2515` has `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` — assess whether this test field-enumerates `MetabolismProfile` during implementation and extend if so.
3. Shared abstraction boundary: the universal-profile contract from `docs/spec-drafting-rules.md` §"Agent Profile Scenario Contract" — `Default` impl + `unwrap_or_default()` path remains the only seeding mechanism; no new `set_component_*` call needed, no `AgentDef` field, no `*Def` wrapper.
4. The new field's `Permille` Default value must be meaningful (FND-2). Pin Default to `Permille::new_unchecked(800)` — agents need hunger ≥ 80% before spoiled-food Eat candidates are emitted. The 800 baseline matches the spec's Design-Goal language that "a well-fed or cautious agent will not" and the Section H #9d dampener language that "fresh stock is consumed before it ages when supply is tight."

## Architecture Check

1. Reusing the existing universal `MetabolismProfile` (rather than introducing a new component) keeps agent metabolic profile centralized and respects the `docs/spec-drafting-rules.md` §"Agent Profile Scenario Contract" universal-profile pattern. No new `AgentDef` field, no new `spawn_agent` call site, no new component schema entry. The change is purely additive on an existing universal profile.
2. `#[serde(default)]` on the new field ensures forward compatibility for authored RON scenarios — those that omit `spoiled_food_hunger_threshold` continue to load. The `SAVE_FORMAT_VERSION` 116→117 bump covers the bincode positional reads on `SimulationState`'s serialized `MetabolismProfile` components, which break atomically when the field count changes (the bincode bump axis is independent of the RON serde-default axis per the spec-to-tickets sub-check (b) rule).

## Verified Layers

1. `MetabolismProfile` round-trips through save/load post-117 → focused save-load test extension in `crates/worldwake-sim/src/save_load.rs`.
2. `unwrap_or_default()` path in `spawn_agent` produces an agent with `spoiled_food_hunger_threshold = Permille::new_unchecked(800)` when `AgentDef.metabolism_profile` is `None` → focused scenario-spawn unit test.
3. `world_txn.rs::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (line 2515) extended if it field-enumerates `MetabolismProfile`'s shape in the produced agent's delta assertion.
4. Single-layer ticket: field-addition plumbing only; downstream gating behavior emerges in ticket 006.

## Landed Changes

### 1. `MetabolismProfile` threshold field

In `crates/worldwake-core/src/needs.rs`, `MetabolismProfile` now carries:

```rust
#[serde(default = "default_spoiled_food_hunger_threshold")]
pub spoiled_food_hunger_threshold: Permille,
```

`Default` and `MetabolismProfile::new()` both seed the field from
`default_spoiled_food_hunger_threshold()` (`Permille::new_unchecked(800)`).

### 2. Constructor fallout

`cargo test --workspace --no-run` found one exhaustive struct literal that required an explicit default threshold:

- `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs`

Other planned construction sites compiled because they use struct update syntax or helper constructors.

### 3. Save/load version and witness

`crates/worldwake-sim/src/save_load.rs` now uses `SAVE_FORMAT_VERSION = 117`. The full non-default save/load fixture sets `spoiled_food_hunger_threshold` to a non-default value and asserts it round-trips.

### 4. Scenario default path

`crates/worldwake-cli/src/scenario/mod.rs::test_spawn_minimal_scenario` now asserts an omitted `AgentDef.metabolism_profile` receives the default spoiled-food threshold through the existing `unwrap_or_default()` path.

### 5. Generated profile docs

`docs/profiles/all-profiles.md` was regenerated with `python3 scripts/profile_docs.py --write`, adding the new `MetabolismProfile` field.

## Landed Files

- `crates/worldwake-core/src/needs.rs` (modify — add field + Default extension)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 116→117 and save/load witness)
- `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs` (modify — 1 site)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — default spawn-path assertion)
- `docs/profiles/all-profiles.md` (modify — regenerated via `python3 scripts/profile_docs.py`)

## Out of Scope

- Reading `spoiled_food_hunger_threshold` in candidate generation (ticket 006).
- New `AgentDef` field or new `spawn_agent` call site (not needed — universal-profile path carries the addition automatically).
- Touching authored `.ron` scenarios (the `#[serde(default)]` annotation absorbs missing-field deserialization).
- Per-commodity desperation thresholds (the field is per-agent only; commodity-aware desperation is a future-spec scope).

## Acceptance Result

### Tests Passed

1. `metabolism_profile_default_includes_spoiled_food_hunger_threshold` — asserts `MetabolismProfile::default().spoiled_food_hunger_threshold == Permille::new_unchecked(800)`.
2. `metabolism_profile_serde_default_absorbs_missing_field` — asserts a partial RON serialization that omits `spoiled_food_hunger_threshold` deserializes with the default value.
3. `save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state` — asserts save/load equivalence at `SAVE_FORMAT_VERSION=117` and proves the non-default threshold round-trips.
4. `scenario::tests::test_spawn_minimal_scenario` — asserts the `unwrap_or_default()` spawn path applies the default threshold.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. `MetabolismProfile` remains a universal profile applied via `unwrap_or_default()` in `spawn_agent` — no conditional application.
2. The `spoiled_food_hunger_threshold` Default value is `Permille::new_unchecked(800)` — concrete and profile-authored, not magic-numbered (FND-2).
3. `world_txn.rs` `create_agent` delta assertion remains valid post-field-addition (verified during implementation).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/needs.rs` `#[cfg(test)]` — added default and serde-default tests for `spoiled_food_hunger_threshold`.
2. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — extended the full non-default round-trip test for the new field and version 117.
3. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — extended `test_spawn_minimal_scenario` to prove the universal-profile default path.
4. `crates/worldwake-core/src/world_txn.rs` required no edit because its agent-create delta assertion uses `MetabolismProfile::default()` rather than field-enumerating the profile.

### Commands Run

1. `cargo test -p worldwake-core --lib metabolism_profile_`
2. `cargo test -p worldwake-sim --lib save_to_bytes_roundtrip_preserves_full_nondefault_state`
3. `cargo test -p worldwake-cli --lib test_spawn_minimal_scenario`
4. `python3 scripts/profile_docs.py --write`
5. `cargo fmt --all`
6. `cargo test --workspace --no-run`
7. `cargo test --workspace`

## Outcome

Completed on 2026-06-02.

- Added `MetabolismProfile.spoiled_food_hunger_threshold` with `#[serde(default = "default_spoiled_food_hunger_threshold")]` and default value `Permille::new_unchecked(800)`.
- Kept the existing universal profile path: `AgentDef.metabolism_profile: Option<MetabolismProfile>` still flows through `unwrap_or_default()` in scenario spawning; no new `AgentDef` field or spawn call was added.
- Bumped `SAVE_FORMAT_VERSION` from 116 to 117 and extended the full non-default save/load witness with a non-default spoiled-food threshold.
- Regenerated `docs/profiles/all-profiles.md` with the new profile field.
- Resolved the only compiler-confirmed exhaustive `MetabolismProfile` literal in `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs`.

## Deviations

- The drafted "18 construction sites" note was stale. `cargo test --workspace --no-run` found only one exhaustive literal missing the new field; other listed sites either used struct update syntax or did not require explicit fallout. The completed ticket records the compiler-confirmed truth.
- The save/load proof reused and extended `save_to_bytes_roundtrip_preserves_full_nondefault_state` instead of creating a narrowly named `metabolism_profile_round_trips_through_save_load_post_117` test. This stronger witness proves the new field inside the persisted `SimulationState`.
- The drafted focused Cargo command used two test filters, which Cargo rejects. Focused coverage was run with valid filters instead.
- `./scripts/verify.sh` is waived for this per-ticket closeout because the implement-spec-tickets harness owns the final full pre-PR verification after all S178 tickets land. This ticket ran `cargo test --workspace`.

## Verification Result

- Passed `cargo test -p worldwake-core --lib metabolism_profile_`.
- Passed `cargo test -p worldwake-sim --lib save_to_bytes_roundtrip_preserves_full_nondefault_state`.
- Passed `cargo test -p worldwake-cli --lib test_spawn_minimal_scenario`.
- Passed `python3 scripts/profile_docs.py --write` (wrote `docs/profiles/all-profiles.md`; reported unrelated existing documentation gaps).
- Passed `cargo fmt --all`.
- Passed `cargo test --workspace --no-run`.
- Passed `cargo test --workspace`.
