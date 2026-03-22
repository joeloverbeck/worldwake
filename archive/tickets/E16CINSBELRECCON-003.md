# E16CINSBELRECCON-003: Extend AgentBeliefStore + PerceptionProfile for Institutional Knowledge

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new fields on existing components in worldwake-core
**Deps**: E16CINSBELRECCON-001 (institutional types must exist)

## Problem

Agents need storage for institutional beliefs and per-agent consultation parameters. `AgentBeliefStore` must gain an `institutional_beliefs` field, and `PerceptionProfile` must gain consultation-related fields (`institutional_memory_capacity`, `consultation_speed_factor`, `contradiction_tolerance`).

## Assumption Reassessment (2026-03-21)

1. `AgentBeliefStore` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) currently contains only `known_entities`, `social_observations`, `told_beliefs`, and `heard_beliefs`. No institutional-belief lane exists yet.
2. `PerceptionProfile` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) currently contains only `memory_capacity`, `memory_retention_ticks`, `observation_fidelity`, and `confidence_policy`. No consultation-related parameters exist yet.
3. The dependency ticket is already completed and archived at [archive/tickets/E16CINSBELRECCON-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E16CINSBELRECCON-001.md). `InstitutionalBeliefKey` and `BelievedInstitutionalClaim` already exist in [institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), so this ticket should consume those concrete types rather than redefining them.
4. The original ticket understated the current verification surface. Core already has focused serde/default tests such as `belief::tests::new_creates_empty_store`, `belief::tests::perception_profile_roundtrips_through_bincode`, and `belief::tests::default_perception_profile_carries_default_confidence_policy`, plus broader schema roundtrips in `component_tables`, `delta`, and `world`. This ticket should extend those existing tests and fixtures rather than adding redundant parallel coverage.
5. The original files-to-touch list was incomplete. In addition to [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) and [delta.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs), the current core sample constructors in [component_tables.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs) and [world.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) will need updates because they construct these values directly for serialization/component roundtrip tests.
6. Workspace-wide `PerceptionProfile` struct literals already exist across `worldwake-core`, `worldwake-systems`, `worldwake-sim`, and `worldwake-ai`. This ticket should expect compile-fix updates at those call sites as part of the schema change, not treat them as optional helper cleanup.
7. Both types derive `Serialize`/`Deserialize`. Adding fields intentionally breaks old save payloads, which is acceptable under Principle 26 and the repo's no-backward-compatibility rule.
8. `AgentBeliefStore::new()` returns `Self::default()`, so adding `institutional_beliefs` to the struct and deriving `Default` remains the clean path. No bespoke constructor logic is needed in this ticket.
9. No mismatch on the architectural direction: extending the existing belief/perception components is still cleaner than introducing parallel institutional-only components for this phase.
10. Scope correction: this ticket should remain a schema-and-defaults ticket. It should not introduce projection logic, capacity enforcement for institutional beliefs, consultation timing semantics, or AI read helpers; those belong to later E16c tickets.
11. The proposed `consultation_speed_factor` field is acceptable for now because the active E16c spec places consultation parameters on `PerceptionProfile`, but this is a boundary to watch. If more non-sensory cognitive parameters accumulate later, the cleaner long-term move would be a deliberate profile split or rename, not continued silent overloading.
12. Current narrow verification command names are real and discoverable. `cargo test -p worldwake-core -- --list` confirms the existing belief/core test targets, so the commands below can be made exact instead of approximate.

## Architecture Check

1. Extending `AgentBeliefStore` is cleaner than creating a parallel `InstitutionalBeliefStore`. Institutional claims are one more subjective knowledge lane, not a separate authority model, so they belong beside existing entity/social/tell memory rather than in a detached component.
2. Extending `PerceptionProfile` is acceptable for this phase because consultation throughput and contradiction tolerance are per-agent knowledge-acquisition parameters and the live E16c spec explicitly places them there. This is still better than introducing a one-off `RecordConsultationProfile` component that would fragment agent configuration and force wider plumbing.
3. The current architecture is only worth preserving if we keep the boundary honest: no alias layer, no compatibility shims, and no planner-only shortcuts. If this profile keeps accumulating non-perceptual cognition knobs later, the cleaner durable move would be a deliberate redesign of the profile model, not incremental clutter hidden behind aliases.
4. No backward-compatibility shims. Old save files will not deserialize, which is acceptable per Principle 26.

## Verification Layers

1. `AgentBeliefStore` schema/defaults remain valid -> focused tests in `belief.rs` (`new_creates_empty_store`, roundtrip assertions)
2. `PerceptionProfile` schema/defaults remain valid -> focused tests in `belief.rs` (`perception_profile_roundtrips_through_bincode`, default assertions)
3. Component/delta/world serialization still accepts the widened structs -> existing roundtrip tests in `component_tables.rs`, `delta.rs`, and `world.rs`
4. Cross-crate struct-literal fallout is resolved -> compile-checked by `cargo test -p worldwake-core`, then `cargo clippy --workspace` / `cargo test --workspace`
5. Single authoritative layer change only. No runtime, planner, or action-trace verification is required in this ticket.

## What to Change

### 1. Extend `AgentBeliefStore` in `belief.rs`

Add field:
```rust
pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
```

Continue using the derived/default empty `BTreeMap` path; do not add a custom constructor unless this ticket actually needs behavior beyond schema/default initialization.

### 2. Extend `PerceptionProfile` in `belief.rs`

Add fields:
```rust
pub institutional_memory_capacity: u32,       // default: 20
pub consultation_speed_factor: Permille,      // default: Permille(500)
pub contradiction_tolerance: Permille,        // default: Permille(300)
```

Update `Default` impl with the defaults above.

### 3. Update delta test samples in `delta.rs`

Update `ComponentValue::AgentBeliefStore(...)` and `ComponentValue::PerceptionProfile(...)` samples to include the new fields.

### 4. Update core sample fixtures that serialize these types

Update direct constructors in:
- [component_tables.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs)
- [world.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs)

### 5. Update downstream struct literals across the workspace

Grep for `PerceptionProfile {` and `AgentBeliefStore {` across the workspace and add the new fields where literals are used directly. These are compile-fix updates required by the schema change, not optional cleanup.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add fields, defaults, and focused assertions)
- `crates/worldwake-core/src/delta.rs` (modify — update component sample payloads)
- `crates/worldwake-core/src/component_tables.rs` (modify — update sample roundtrip fixtures)
- `crates/worldwake-core/src/world.rs` (modify — update populated-world sample fixtures)
- Workspace files with direct `PerceptionProfile` / `AgentBeliefStore` literals as required for compilation (modify — schema follow-through only)

## Out of Scope

- Derivation helpers on institutional beliefs (ticket -009)
- WorldTxn projection helpers (ticket -004)
- AI reading institutional beliefs (tickets -010 through -014)
- Capacity enforcement logic (part of projection in ticket -004)
- Any action definitions or handlers

## Acceptance Criteria

### Tests That Must Pass

1. `AgentBeliefStore` with populated `institutional_beliefs` roundtrips through bincode
2. `PerceptionProfile` with new fields roundtrips through bincode
3. Default `PerceptionProfile` has `institutional_memory_capacity == 20`, `consultation_speed_factor == Permille(500)`, `contradiction_tolerance == Permille(300)`
4. Default `AgentBeliefStore` has empty `institutional_beliefs`
5. Existing schema roundtrip tests in core continue to pass after the field additions
6. Existing suite: `cargo test --workspace`

### Invariants

1. `institutional_beliefs` uses `BTreeMap` (deterministic iteration)
2. `consultation_speed_factor` and `contradiction_tolerance` use `Permille` (no floats)
3. All existing tests pass with new fields added to struct literals

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — extended `new_creates_empty_store` to assert empty `institutional_beliefs`
Rationale: proves the widened default belief store stays empty and deterministic after adding the new lane.
2. `crates/worldwake-core/src/belief.rs` — added `agent_belief_store_roundtrips_through_bincode_with_institutional_beliefs`
Rationale: covers the new `institutional_beliefs` payload with a populated roundtrip instead of relying on empty/default serialization only.
3. `crates/worldwake-core/src/belief.rs` — extended `default_perception_profile_carries_default_confidence_policy`
Rationale: pins the spec-mandated defaults for `institutional_memory_capacity`, `consultation_speed_factor`, and `contradiction_tolerance`.
4. `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/component_tables.rs`, and `crates/worldwake-core/src/world.rs` — updated existing roundtrip fixtures that serialize widened component values
Rationale: keeps the schema-change verification at the real serialization boundaries already used by the core event/world/component tests.

### Commands

1. `cargo test -p worldwake-core belief::tests::new_creates_empty_store`
2. `cargo test -p worldwake-core belief::tests::perception_profile_roundtrips_through_bincode`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed: extended `AgentBeliefStore` with `institutional_beliefs`; extended `PerceptionProfile` with `institutional_memory_capacity`, `consultation_speed_factor`, and `contradiction_tolerance`; updated core serialization fixtures and downstream `PerceptionProfile` literals across the workspace so the schema compiled cleanly.
- Deviations from original plan: the ticket was corrected before implementation to reflect the already-archived `-001` dependency, the existing core verification surface, and the additional core fixture files that needed widening. During verification, `cargo clippy --workspace` surfaced `StateDelta` as a `large_enum_variant`; instead of forcing heap boxing into the event-log delta path, a targeted allow was added to preserve the existing flat, allocation-free delta architecture.
- Verification results: `cargo test -p worldwake-core belief::tests::new_creates_empty_store`, `cargo test -p worldwake-core belief::tests::perception_profile_roundtrips_through_bincode`, `cargo test -p worldwake-core`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
