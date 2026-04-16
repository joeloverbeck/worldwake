# S105OBSSALFIL-002: Implement observation priority and budget pipeline

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — perception pipeline in worldwake-systems
**Deps**: archive/tickets/S105OBSSALFIL-001.md (completed)

## Problem

The perception pipeline in `collect_direct_local_observation_batch` iterates ALL co-located entities with no prioritization. At waste-heavy locations (55+ entities), agents waste perception bandwidth observing low-value entities, continuously refreshing belief activation scores and defeating S101's power-law decay. This ticket implements entity-kind-based priority sorting and budget truncation to break the unbounded observation feedback loop (FND-11) and bound per-tick perception cost (FND-12).

## Assumption Reassessment (2026-04-16)

1. `collect_direct_local_observation_batch` is at `crates/worldwake-systems/src/perception.rs:443`. Current signature: `fn collect_direct_local_observation_batch(world: &World, observer: EntityId, place: EntityId, colocated_entities: &[EntityId], tick: Tick, observation_fidelity: u16, rng: &mut DeterministicRng, store: &AgentBeliefStore) -> Option<DirectLocalObservationBatch>`. Two new parameters will be added: `needs: HomeostaticNeeds` and `profile: &PerceptionProfile`.
2. `observe_passive_local_entities` at line 231 is the sole caller. It already retrieves `profile` (line 247) and `HomeostaticNeeds` (line 295, via `get_component_homeostatic_needs`). The needs retrieval must be moved before the `collect_direct_local_observation_batch` call (currently after it, used only by `apply_direct_local_observation_batch`).
3. `world.entity_kind(entity)` at `crates/worldwake-core/src/world.rs:453` returns `Option<EntityKind>`. `EntityKind` has 10 variants: Agent, ItemLot, UniqueItem, Container, Facility, Place, Faction, Office, Record, SocialArtifact.
4. `world.get_component_item_lot(entity)` returns `Option<&ItemLot>`. `ItemLot.commodity` (not `commodity_kind`) is of type `CommodityKind`. `CommodityKind::Waste` exists.
5. `HomeostaticNeeds::max_value()` at `crates/worldwake-core/src/needs.rs:55` returns `u16`. The existing `salience_boost()` at `crates/worldwake-core/src/belief.rs:2513` uses u32 intermediate math: `(u32::from(max_need) * u32::from(boost.value()) / 1000) as u16`. The observation-side boost must follow the same pattern but exclude Waste ItemLots (the retention-side does not exclude Waste).
6. No focused tests exist for `collect_direct_local_observation_batch` in the perception.rs test module (`#[cfg(test)]` boundary at line 1164). The function is tested indirectly through `perception_system`.
7. `passes_observation_check` at line 817 takes `(fidelity: u16, rng: &mut DeterministicRng)` and returns `bool`. Its interface is unchanged by this ticket.

## Architecture Check

1. The priority function is a pure computation over entity kind, commodity type, and agent need state — no stored state, no cross-system calls. Priority scores are transient stack-local values (FND-3, FND-26). The budget is a physical dampener (FND-11) representing finite cognitive attention (FND-20).
2. No backward-compatibility shims. The observation loop changes in-place. The existing `salience_boost()` function in belief.rs is not modified — a new observation-specific priority function is created because the Waste-exclusion semantics differ.

## Verification Layers

1. Priority ordering is deterministic (EntityKind + EntityId tie-breaking) → unit test with known entity composition
2. Budget truncation caps observed entities → unit test asserting truncation at `observation_budget`
3. Need-based boost activates only for non-Waste ItemLots above urgency threshold → unit test with mixed entity types and varying need pressure
4. Pipeline behavioral correctness (high-priority entities always observed first) → unit test verifying Agent/Facility entities appear in batch before Waste
5. Existing perception behavior unchanged at default budget → existing test suite passes (regression)

## What to Change

### 1. Create `compute_observation_priority` function

In `crates/worldwake-systems/src/perception.rs`, add a new private function:

```rust
fn compute_observation_priority(
    world: &World,
    entity: EntityId,
    needs: &HomeostaticNeeds,
    profile: &PerceptionProfile,
) -> u16
```

Logic:
- Read `world.entity_kind(entity)` for base priority (match all 10 EntityKind variants per spec priority table)
- For `EntityKind::ItemLot`: read `world.get_component_item_lot(entity)` to check `commodity == CommodityKind::Waste` → base 100; non-Waste → base 300 with need boost
- Need boost: if `needs.max_value() >= profile.need_salience_urgency_threshold.value()`, compute `(u32::from(needs.max_value()) * u32::from(profile.need_salience_boost.value()) / 1000) as u16` and add to base
- Return `None` entity kind as base 0 (should not happen for well-formed entities)

### 2. Modify `collect_direct_local_observation_batch` signature and body

Add two parameters: `needs: HomeostaticNeeds`, `profile: &PerceptionProfile`.

Insert steps 1-3 before the existing entity loop:
1. Build a `Vec<(u16, EntityId)>` of `(priority, entity)` for each co-located entity (excluding observer)
2. Sort by priority descending, then EntityId ascending for deterministic tie-breaking
3. Truncate to `profile.observation_budget` entries

Replace the existing direct iteration over `colocated_entities` with iteration over the truncated priority list. The fidelity check, snapshot building, place observation, and missing-entity detection remain unchanged.

### 3. Update `observe_passive_local_entities` caller

Move the `get_component_homeostatic_needs` call (currently at line ~295) to before the `collect_direct_local_observation_batch` call (line ~270). Pass `needs` and `&profile` as the two new parameters. Continue passing `needs` to `apply_direct_local_observation_batch` as before.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)

## Out of Scope

- Goal-specific observation filtering (would couple perception to planning, violating FND-26)
- Dynamic budget adjustment based on location density
- Observation priority for evidence entries or scene elements
- Commodity-specific salience mapping beyond Waste detection
- Modifying the retention-side `salience_boost()` in `belief.rs`
- Changes to `passes_observation_check` or `build_believed_entity_state`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` compiles
2. `cargo test --workspace` passes — all existing golden tests produce identical results at default budget (24)
3. `cargo clippy --workspace --all-targets -- -D warnings` passes

### Invariants

1. Priority ordering is deterministic: same seed + same entity set = same observation order
2. Budget truncation: observed entity count per tick ≤ `observation_budget` (excluding the place itself)
3. High-priority entities (Agent, Facility) are always observed before low-priority (Waste ItemLots)
4. Place observation and missing-entity detection are unaffected by priority/budget logic
5. No new cross-system dependency: only `EntityKind`, `ItemLot.commodity`, and `HomeostaticNeeds` are read — all already accessed by the perception system

## Test Plan

### New/Modified Tests

1. None in this ticket — tests are in S105OBSSALFIL-003. This ticket's correctness is verified by existing test suite passing unchanged (behavioral equivalence at default budget).

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
