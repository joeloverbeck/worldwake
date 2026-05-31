# S178PERFOOSPO-002: MetabolismProfile spoiled_food_hunger_threshold field

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `MetabolismProfile` gains `spoiled_food_hunger_threshold: Permille`; 18 construction sites updated; `SAVE_FORMAT_VERSION` 116→117.
**Deps**: 001

## Problem

D7 introduces a per-agent profile-driven desperation threshold that gates whether candidate generation emits a spoiled-food Eat candidate (D5, consumed by ticket 006). The threshold lives on the universal `MetabolismProfile` per the spec's universal-profile reuse decision — no new component, no new `AgentDef` field, no new `spawn_agent` call site. Field addition requires updating 18 literal-opening construction sites (no spread syntax in current code per spot-check (d)), regenerating `docs/profiles/all-profiles.md`, and bumping `SAVE_FORMAT_VERSION` 116→117 to cover the format-breaking `MetabolismProfile` change.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MetabolismProfile` at `crates/worldwake-core/src/needs.rs:142` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` and has a `Default` impl at lines 283-309 covering all 21 current fields. `AgentDef.metabolism_profile: Option<MetabolismProfile>` exists at `crates/worldwake-cli/src/scenario/types.rs:645` with `#[serde(default)]`. `spawn_agent` at `crates/worldwake-cli/src/scenario/mod.rs:980-981` applies via `unwrap_or_default()`. Universal-profile contract per `docs/spec-drafting-rules.md` §"Agent Profile Scenario Contract" — field addition rides the existing path. Construction sites enumerated via `rg '^\s*MetabolismProfile\s*\{$' crates/`: 18 literal-opening forms workspace-wide, 0 spread-syntax sites. Production: 1 site at `crates/worldwake-sim/src/save_load.rs:354` (load path), 3 sites in `crates/worldwake-systems/src/needs.rs` (lines 1861, 1913, 2003 — confirm cfg(test) boundary; if production, treat as production). Test: 14 sites across `crates/worldwake-ai/tests/`. Authored RON configs: 5 scenarios reference `metabolism_profile:` (`scenarios/cognitive-archetypes-divergence.ron`, `survival-preferences.ron`, `survival-baseline.ron`, `survival-basin-dirty-dirty.ron`, `cli-evaluation.ron`); adding `#[serde(default)]` on the new field absorbs these without RON edits.
2. Spec D7 verified against current `specs/S178-perishable-food-spoilage.md`. FOUNDATIONS Alignment row FND-22 mandates per-agent profile parameter for diversity; FND-2 mandates concrete profile-authored values (no magic numbers). `world_txn.rs` at `crates/worldwake-core/src/world_txn.rs:2515` has `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` — assess whether this test field-enumerates `MetabolismProfile` during implementation and extend if so.
3. Shared abstraction boundary: the universal-profile contract from `docs/spec-drafting-rules.md` §"Agent Profile Scenario Contract" — `Default` impl + `unwrap_or_default()` path remains the only seeding mechanism; no new `set_component_*` call needed, no `AgentDef` field, no `*Def` wrapper.
4. The new field's `Permille` Default value must be meaningful (FND-2). Pin Default to `Permille::new_unchecked(800)` — agents need hunger ≥ 80% before spoiled-food Eat candidates are emitted. The 800 baseline matches the spec's Design-Goal language that "a well-fed or cautious agent will not" and the Section H #9d dampener language that "fresh stock is consumed before it ages when supply is tight."

## Architecture Check

1. Reusing the existing universal `MetabolismProfile` (rather than introducing a new component) keeps agent metabolic profile centralized and respects the `docs/spec-drafting-rules.md` §"Agent Profile Scenario Contract" universal-profile pattern. No new `AgentDef` field, no new `spawn_agent` call site, no new component schema entry. The change is purely additive on an existing universal profile.
2. `#[serde(default)]` on the new field ensures forward compatibility for authored RON scenarios — those that omit `spoiled_food_hunger_threshold` continue to load. The `SAVE_FORMAT_VERSION` 116→117 bump covers the bincode positional reads on `SimulationState`'s serialized `MetabolismProfile` components, which break atomically when the field count changes (the bincode bump axis is independent of the RON serde-default axis per the spec-to-tickets sub-check (b) rule).

## Verification Layers

1. `MetabolismProfile` round-trips through save/load post-117 → focused save-load test extension in `crates/worldwake-sim/src/save_load.rs`.
2. `unwrap_or_default()` path in `spawn_agent` produces an agent with `spoiled_food_hunger_threshold = Permille::new_unchecked(800)` when `AgentDef.metabolism_profile` is `None` → focused scenario-spawn unit test.
3. `world_txn.rs::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (line 2515) extended if it field-enumerates `MetabolismProfile`'s shape in the produced agent's delta assertion.
4. Single-layer ticket: field-addition plumbing only; downstream gating behavior emerges in ticket 006.

## What to Change

### 1. Add field to `MetabolismProfile`

In `crates/worldwake-core/src/needs.rs:142`:

```rust
pub struct MetabolismProfile {
    // ... 21 existing fields ...
    #[serde(default)]
    pub spoiled_food_hunger_threshold: Permille,
}
```

Extend the `Default` impl at lines 283-309 to add `spoiled_food_hunger_threshold: Permille::new_unchecked(800)`.

### 2. Update 18 construction sites

Production code:
- `crates/worldwake-sim/src/save_load.rs:354` — load path constructor.
- `crates/worldwake-systems/src/needs.rs:1861, 1913, 2003` — test fixtures (confirm `#[cfg(test)]` boundary during implementation; treat as production if outside cfg(test)).

Test code (all in `crates/worldwake-ai/tests/`):
- `golden_harness/soak_world.rs:146, 278`
- `planner_pathology_harness/mod.rs:321`
- `golden_harness/mod.rs:353`
- `scenarios/place_dirtiness.rs:29`
- `scenarios/survival_drive_escalation.rs:495`
- `scenarios/exploration.rs:355`
- `scenarios/ai_decisions.rs:2139`
- `scenarios/offices.rs:1279`
- `scenarios/simulation_gaps.rs:685`
- `scenarios/sleep_episode.rs:100`
- `scenarios/scaled_contention.rs:109`
- `scenarios/survival_self_care_interruption.rs:43, 330`

At each site, add the new field with the Default value `Permille::new_unchecked(800)`. Scenario-tuned overrides (testing the candidate-gate behavior with specific thresholds) belong in ticket 006's and 008's test additions, not here.

### 3. `world_txn.rs::create_agent` delta assertion

In `crates/worldwake-core/src/world_txn.rs`, locate `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` at line 2515. If the test field-enumerates `MetabolismProfile`'s shape (asserting specific field values in the delta), extend with the new field. If the test only asserts component presence/type, no update needed. The decision is verifiable mid-implementation.

### 4. Regenerate `docs/profiles/all-profiles.md`

Run `python3 scripts/profile_docs.py` (the canonical regen command for profile documentation). Commit the regenerated `docs/profiles/all-profiles.md` alongside the code changes.

### 5. `SAVE_FORMAT_VERSION` bump 116→117

In `crates/worldwake-sim/src/save_load.rs`, bump `SAVE_FORMAT_VERSION` from 116 (set by ticket 001) to 117 to cover the `MetabolismProfile` field addition.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify — add field + Default extension)
- `crates/worldwake-core/src/world_txn.rs` (modify — extend `create_agent` delta test at line 2515 if field-enumerated)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 116→117; construction site at line 354)
- `crates/worldwake-systems/src/needs.rs` (modify — 3 construction sites at lines 1861, 1913, 2003)
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs` (modify — 2 sites)
- `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/place_dirtiness.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/exploration.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/ai_decisions.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/offices.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/simulation_gaps.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` (modify — 1 site)
- `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs` (modify — 2 sites)
- `docs/profiles/all-profiles.md` (modify — regenerated via `python3 scripts/profile_docs.py`)

## Out of Scope

- Reading `spoiled_food_hunger_threshold` in candidate generation (ticket 006).
- New `AgentDef` field or new `spawn_agent` call site (not needed — universal-profile path carries the addition automatically).
- Touching authored `.ron` scenarios (the `#[serde(default)]` annotation absorbs missing-field deserialization).
- Per-commodity desperation thresholds (the field is per-agent only; commodity-aware desperation is a future-spec scope).

## Acceptance Criteria

### Tests That Must Pass

1. `metabolism_profile_default_includes_spoiled_food_hunger_threshold` — asserts `MetabolismProfile::default().spoiled_food_hunger_threshold == Permille::new_unchecked(800)`.
2. `metabolism_profile_round_trips_through_save_load_post_117` — asserts save/load equivalence at `SAVE_FORMAT_VERSION=117`.
3. `metabolism_profile_serde_default_absorbs_missing_field` — asserts a partial RON serialization that omits `spoiled_food_hunger_threshold` deserializes with the default value.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. `MetabolismProfile` remains a universal profile applied via `unwrap_or_default()` in `spawn_agent` — no conditional application.
2. The `spoiled_food_hunger_threshold` Default value is `Permille::new_unchecked(800)` — concrete and profile-authored, not magic-numbered (FND-2).
3. `world_txn.rs` `create_agent` delta assertion remains valid post-field-addition (verified during implementation).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` `#[cfg(test)]` — add 2 new unit tests (Default + serde-default).
2. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — extend existing `MetabolismProfile` round-trip test for the new field.
3. `crates/worldwake-core/src/world_txn.rs` `#[cfg(test)]` — extend `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (line 2515) only if field-enumerated.

### Commands

1. `cargo test -p worldwake-core -- needs::tests::metabolism_profile_default_includes_spoiled_food_hunger_threshold needs::tests::metabolism_profile_serde_default_absorbs_missing_field`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
