# S138OPPCOM-002: Universal-on-Agent components — RiskWeightProfile and LawAbidingProfile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds two universal components on `EntityKind::Agent`; bumps `SAVE_FORMAT_VERSION` 75 → 76
**Deps**: None

## Problem

S138's compile-time ranking of opportunities (buy / steal / beg / wait) requires two per-agent profiles that the AI crate can read via `GoalBeliefView`: `RiskWeightProfile` (theft_aversion, exposure_aversion, threat_aversion) and `LawAbidingProfile` (criminal_threshold, social_norm_weight). Both are universal on `EntityKind::Agent` so every agent has a default. The ticket lands the full Section 5 scenario contract — core component definition, schema registration, `create_agent` seeding, `world_txn.rs` delta assertion update, `AgentDef` field, `spawn_agent` set-call, `Default` impl, and `GoalBeliefView` accessor — for both components in one shot so the workspace builds at every intermediate state.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-core/src/world.rs` has agent-bootstrap tests at lines 1270 (`create_agent_produces_correct_entity`), 1292 (`create_agent_components_queryable`), 1312 (`create_agent_attaches_belief_store_perception_profile_and_tell_profile`) — these will pick up the two new universal components and must be extended to assert their presence.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "New universal-on-Agent components and their scenario contract"; `docs/spec-drafting-rules.md` Section 5 for the profile contract checklist.
3. Shared abstraction boundary under audit: the cross-crate accessor surface — core defines the component, sim exposes it via `GoalBeliefView` / `RuntimeBeliefView`, cli scenarios author it through `AgentDef`. No direct cross-system call.
4. `World::create_agent` at `crates/worldwake-core/src/world.rs:183` currently seeds 18 universal profiles via `XProfile::default()`; the two new components join that list (precedent: `CognitiveProfile::default()` at line 203, `PreferenceProfile::default()` at line 227). The `with_component_schema_entries!` macro at `crates/worldwake-core/src/component_schema.rs` generates accessors that must compile at `delta.rs`, `world.rs`, `component_tables.rs`, `world_txn.rs` (per `tickets/README.md` check #13).
5. Save-format bump: `SAVE_FORMAT_VERSION = 75` at `crates/worldwake-sim/src/save_load.rs:6`. Adding components to authoritative state requires bump to 76 (cascade with ticket 003 which bumps 76→77).

## Architecture Check

1. Two sibling component files (`risk_weight_profile.rs`, `law_abiding_profile.rs`) follow the established profile-per-file precedent (`cognitive_profile.rs`, `metabolism_profile.rs`, `preference_profile.rs`). Keeps the per-component surface inspectable.
2. Universal classification matches `Default`-with-zero semantics — `Permille::default() == ZERO`, so a default-seeded agent's risk-aversion and law-abiding weights are zero, which is the "neutral" baseline scenario authors can override via RON.
3. No backward-compatibility shims: the SAVE_FORMAT bump is mandatory; older save files cannot deserialize against the new schema, matching the project's no-back-compat invariant (CLAUDE.md "Critical Invariants").
4. `GoalBeliefView` accessor placement: add to the existing `ProfileBeliefView` sub-trait (`crates/worldwake-sim/src/belief_view.rs`, accessor cluster around the existing per-agent profile accessors), so `RuntimeBeliefView`'s blanket impl at `belief_view.rs:1416` continues to forward via the sub-trait.

## Verification Layers

1. Component definition + Default impl — focused unit test (per-file inline `#[cfg(test)]`)
2. Component registration generates accessors — focused unit test calling `txn.insert_component_risk_weight_profile(...)` and `txn.get_component_risk_weight_profile(...)`
3. `create_agent` seeds both components — extension of existing `create_agent_components_queryable` test in `world.rs:1292`
4. `GoalBeliefView::risk_weight_profile(agent)` returns the seeded default for a freshly-created agent — focused unit test in `belief_view.rs`
5. Scenario authoring through `AgentDef` — focused test in `scenario/types.rs` deserializing a RON scenario that overrides one field
6. Save-format roundtrip survives the bump — `cargo test -p worldwake-sim save_load`

## What to Change

### 1. Two new core component files

Create `crates/worldwake-core/src/risk_weight_profile.rs`:

```rust
use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskWeightProfile {
    pub theft_aversion: Permille,
    pub exposure_aversion: Permille,
    pub threat_aversion: Permille,
}

impl Component for RiskWeightProfile {}
```

Create `crates/worldwake-core/src/law_abiding_profile.rs` with the parallel structure (fields: `criminal_threshold`, `social_norm_weight`).

Expose both in `crates/worldwake-core/src/lib.rs` next to existing profile re-exports.

### 2. Component schema registration

Modify `crates/worldwake-core/src/component_schema.rs` to add two `with_component_schema_entries!` entries with `|kind| kind == EntityKind::Agent`, mirroring the existing `cognitive_profile` and `metabolism_profile` entries. Verify the macro-generated accessors compile at all four expansion sites (`delta.rs`, `world.rs`, `world_txn.rs`, `component_tables.rs`).

### 3. `create_agent` seeding

Modify `crates/worldwake-core/src/world.rs:183` (`create_agent`):

Add two `insert_component_*` calls inside the `create_entity_with` closure, alongside the existing 18 universal profile seeds:

```rust
world.insert_component_risk_weight_profile(entity, RiskWeightProfile::default())?;
world.insert_component_law_abiding_profile(entity, LawAbidingProfile::default())?;
```

Update the existing `create_agent_components_queryable` test (line 1292) to assert both components are present and default-valued.

### 4. `world_txn.rs` delta assertion

Update the delta assertion in `crates/worldwake-core/src/world_txn.rs` to account for two additional component insertions on agent creation (the existing assertion counts component-write deltas).

### 5. `AgentDef` scenario field

Modify `crates/worldwake-cli/src/scenario/types.rs` (struct at line 571). Add adjacent to `cognitive_profile` (line 592) and `metabolism_profile` (line 618):

```rust
#[serde(default)]
pub risk_weight_profile: Option<RiskWeightProfile>,
#[serde(default)]
pub law_abiding_profile: Option<LawAbidingProfile>,
```

### 6. `spawn_agent` set-calls

Modify `crates/worldwake-cli/src/scenario/mod.rs:616-654` block. Add adjacent to the existing `cognitive` / `metabolism` unwrap-or-default pattern:

```rust
let risk_weight = agent_def.risk_weight_profile.unwrap_or_default();
txn.set_component_risk_weight_profile(agent_id, risk_weight)?;
let law_abiding = agent_def.law_abiding_profile.unwrap_or_default();
txn.set_component_law_abiding_profile(agent_id, law_abiding)?;
```

### 7. `GoalBeliefView` accessor

Modify `crates/worldwake-sim/src/belief_view.rs` (`ProfileBeliefView` sub-trait, near existing per-agent profile accessors):

```rust
fn risk_weight_profile(&self, agent: EntityId) -> &RiskWeightProfile;
fn law_abiding_profile(&self, agent: EntityId) -> &LawAbidingProfile;
```

Modify `crates/worldwake-sim/src/per_agent_belief_view.rs:1493` and the sibling impls — add the two accessors. The blanket impl at `belief_view.rs:1416` should continue to forward via the sub-trait without modification.

### 8. SAVE_FORMAT bump

Modify `crates/worldwake-sim/src/save_load.rs:6`: `SAVE_FORMAT_VERSION = 76`. Update the version-assertion test at `save_load.rs:1198`.

### 9. Profile-docs regeneration

Run `python3 scripts/profile_docs.py` and commit the regenerated `docs/profiles/all-profiles.md`.

## Files to Touch

- `crates/worldwake-core/src/risk_weight_profile.rs` (new)
- `crates/worldwake-core/src/law_abiding_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — two new registration entries)
- `crates/worldwake-core/src/world.rs` (modify — create_agent seeding + test extensions at lines 1270-1330)
- `crates/worldwake-core/src/world_txn.rs` (modify — delta assertion count)
- `crates/worldwake-core/src/delta.rs` (likely modify — macro expansion site; confirm during implementation)
- `crates/worldwake-core/src/component_tables.rs` (likely modify — macro expansion site; confirm during implementation)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — AgentDef fields)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — spawn_agent set-calls)
- `crates/worldwake-sim/src/belief_view.rs` (modify — two accessor methods on ProfileBeliefView)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — backing impls)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION 75→76 and tests)
- `docs/profiles/all-profiles.md` (regenerate via `scripts/profile_docs.py`)

## Out of Scope

- Reading `RiskWeightProfile` / `LawAbidingProfile` in candidate ranking — lands in ticket 006 (the compiler) and downstream ranking consumers
- Adding fields to `PerceptionProfile` / `CognitiveProfile` — lands in ticket 003 (carries its own SAVE_FORMAT bump 76→77)
- Defining `Opportunity` and the typed enums that the new profiles will be consulted against — lands in ticket 001

## Acceptance Criteria

### Tests That Must Pass

1. New test in `risk_weight_profile.rs`: `RiskWeightProfile::default()` produces zero-valued `Permille` fields; bincode roundtrip preserves identity
2. New test in `law_abiding_profile.rs`: parallel coverage for `LawAbidingProfile`
3. Extended `create_agent_components_queryable` test (`world.rs:1292`): asserts both new components are present on a freshly-created agent
4. New test in `belief_view.rs`: `GoalBeliefView::risk_weight_profile(agent)` returns the seeded default
5. New test in `scenario/types.rs`: scenario RON authoring overrides one field of `RiskWeightProfile` and round-trips
6. Save-format roundtrip test in `save_load.rs:1198` passes with new version 76
7. Existing suite: `cargo test --workspace`

### Invariants

1. Every `EntityKind::Agent` created via `World::create_agent` has both `RiskWeightProfile` and `LawAbidingProfile` queryable immediately after creation
2. Older save files (with SAVE_FORMAT_VERSION 75) fail to load with `SaveError::UnsupportedVersion` (no silent backward-compat shim)
3. Both components live in `worldwake-core` per the core-residence constraint for ECS components

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/risk_weight_profile.rs` (inline `#[cfg(test)]`) — default + roundtrip
2. `crates/worldwake-core/src/law_abiding_profile.rs` (inline `#[cfg(test)]`) — default + roundtrip
3. `crates/worldwake-core/src/world.rs` (extend existing tests at 1292) — both new components present after `create_agent`
4. `crates/worldwake-sim/src/belief_view.rs` (inline `#[cfg(test)]`) — accessor returns default for new agent
5. `crates/worldwake-cli/src/scenario/types.rs` (inline `#[cfg(test)]`) — RON deserialization round-trips a scenario with overridden values
6. `crates/worldwake-sim/src/save_load.rs` (extend existing test at 1198) — version 76 round-trip

### Commands

1. `cargo test -p worldwake-core risk_weight_profile law_abiding_profile create_agent`
2. `cargo test -p worldwake-sim save_load belief_view`
3. `cargo test -p worldwake-cli scenario`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `python3 scripts/profile_docs.py` then `git diff docs/profiles/all-profiles.md` (verify regen captures the new components)

Merge note: This ticket bumps `SAVE_FORMAT_VERSION` 75→76; ticket 003 bumps 76→77 — see Step 6 Merge-Order Constraints in the spec-to-tickets summary.
