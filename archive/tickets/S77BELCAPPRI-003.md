# S77BELCAPPRI-003: Tiered entity eviction in `enforce_capacity()`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — entity eviction sort key changes in belief store
**Deps**: S77BELCAPPRI-001

## Problem

When `known_entities` exceeds `entity_memory_capacity`, entities are evicted oldest-first regardless of kind. This causes Place and resource-source entities to be evicted in favor of recently-observed ground items (ItemLot), breaking the planner's ability to reason about remote resource acquisition. After this change, infrastructure-tier entities (Place, Facility, live Agents, resource sources) survive eviction ahead of transient entities.

## Assumption Reassessment (2026-04-09)

1. Entity eviction in `enforce_capacity()` at `belief.rs:197-214`. Current sort: `(state.observed_tick, *entity)` ascending — oldest evicted first. Eviction removes from both `known_entities` and `entity_claims` (lines 212-213).
2. `BelievedEntityState` will have `believed_kind: Option<EntityKind>` after S77BELCAPPRI-001. Also has `resource_source: Option<ResourceSource>` and `alive: bool`.
3. Existing tests: `enforce_capacity_evicts_oldest_entities_deterministically` (line 3149), `enforce_capacity_removes_stale_entities_and_social_observations` (line 3164), `enforce_capacity_clears_entities_when_capacity_is_zero` (line 3182), `enforce_capacity_applies_global_entity_cap_after_claim_pruning` (line 3193), `enforce_capacity_uses_entity_memory_capacity_not_claim_depth` (line 3324).
4. Ticket says the helper should return `0` for infrastructure and `1` for transient entities. Live eviction removes from the start of an ascending sort, so that numeric direction would evict infrastructure first. Correction applied: `entity_eviction_tier()` returns `1` for infrastructure and `0` for transient entities so transient entities sort to the eviction front. Safe because it preserves the ticket's stated behavior and only fixes the contradictory example.

## Architecture Check

1. Tier classification uses concrete `believed_kind` and `resource_source` presence — no abstract priority scores. Aligns with P3.
2. No backward-compatibility shims. Old age-only eviction is replaced. Entities with `believed_kind: None` (pre-migration beliefs) are treated as transient — safe default that avoids silently promoting unknown entities.

## Verification Layers

1. Infrastructure entities survive eviction when competing with transient entities -> focused unit test with mixed-kind entities exceeding capacity
2. Within-tier eviction preserves age ordering -> focused unit test confirming oldest transient entities evicted first
3. Paired removal (known_entities + entity_claims) preserved -> focused unit test confirming claims removed alongside evicted entities
4. Single-layer ticket: changes are internal to belief store capacity enforcement

## What to Change

### 1. Add `entity_eviction_tier()` helper

Add to `crates/worldwake-core/src/belief.rs`:

```rust
fn entity_eviction_tier(state: &BelievedEntityState) -> u8 {
    if state.resource_source.is_some() {
        return 1; // Override: any entity with resource_source is infrastructure
    }
    match state.believed_kind {
        Some(EntityKind::Place | EntityKind::Facility) => 1,
        Some(EntityKind::Agent) if state.alive => 1,
        _ => 0, // Transient tier (includes believed_kind: None fallback)
    }
}
```

### 2. Modify eviction sort in `enforce_capacity()`

Replace the eviction sort at lines 205-210 to include tier as primary dimension:

```rust
let mut eviction_order = self
    .known_entities
    .iter()
    .map(|(entity, state)| (entity_eviction_tier(state), state.observed_tick, *entity))
    .collect::<Vec<_>>();
eviction_order.sort_unstable();
```

Eviction proceeds from the start of this sorted list (transient tier first, then oldest within tier). The paired removal of `entity_claims` at lines 212-213 is unchanged.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Claim-level eviction (that is S77BELCAPPRI-002)
- Changing entity memory capacity
- Adding new EntityKind variants
- Changing perception or observation logic

## Acceptance Criteria

### Tests That Must Pass

1. New: When entity_memory_capacity is 3 and there are 2 Places + 3 ItemLots, the 2 Places survive and 1 ItemLot survives (oldest ItemLots evicted)
2. New: An entity with `resource_source.is_some()` and `believed_kind: Some(ItemLot)` is classified as infrastructure (override)
3. New: A dead Agent (`alive == false`) is classified as transient
4. New: An entity with `believed_kind: None` is classified as transient (fallback)
5. Existing: `cargo test -p worldwake-core -- enforce_capacity`

### Invariants

1. Infrastructure-tier entities (Place, Facility, live Agent, resource_source) are evicted only after all transient-tier entities are evicted
2. Within each tier, the existing age-based ordering is preserved (oldest first)
3. Evicted entities are removed from both `known_entities` and `entity_claims`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — `enforce_capacity_preserves_infrastructure_entities` — Place/Facility survive over ItemLots
2. `crates/worldwake-core/src/belief.rs` — `enforce_capacity_resource_source_override_promotes_to_infrastructure` — resource_source override
3. `crates/worldwake-core/src/belief.rs` — `enforce_capacity_dead_agents_are_transient` — alive/dead Agent tier distinction
4. `crates/worldwake-core/src/belief.rs` — `enforce_capacity_unknown_kind_is_transient` — believed_kind: None fallback

### Commands

1. `cargo test -p worldwake-core -- enforce_capacity`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-09

Implemented tiered entity eviction in `enforce_capacity()` so transient entities are evicted before infrastructure-tier entities. The final helper classifies resource-source entities, Places, Facilities, and live Agents as infrastructure; dead Agents and `believed_kind: None` fall back to transient. Added focused proofs covering mixed-kind retention, resource-source override, dead-Agent demotion, and unknown-kind fallback.

Deviation from original plan: the ticket's sample helper inverted the tier values relative to the live ascending eviction order. The implementation corrected the helper so transient entities sort to the eviction front while preserving the ticket's intended behavior.

## Verification Result

1. `cargo test -p worldwake-core -- enforce_capacity`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
