# S77BELCAPPRI-002: Tiered claim eviction in `enforce_entity_claim_capacity()`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — claim eviction sort key changes in belief store
**Deps**: S77BELCAPPRI-001

## Problem

`enforce_entity_claim_capacity()` sorts claims by confidence and truncates to capacity. High-volume inventory claims from ground items (Waste, etc.) crowd out actionable claims like `ResourceAvailable` and `WorkstationPresent`. After this change, infrastructure-tier claims survive eviction ahead of item-tier claims.

## Assumption Reassessment (2026-04-09)

1. `enforce_entity_claim_capacity()` at `belief.rs:217`. Current sort key: `(Reverse(confidence), Reverse(acquired_tick), Reverse(claim_id))` at lines 241-251. Truncation at line 252. Existing tests at lines 3217, 3253, 3303, 3356.
2. `EntityBeliefAspect` at `entity_belief_claim.rs:17` has 11 variants: `Location`, `Inventory(CommodityKind)`, `Alive`, `Wounded`, `Activity`, `WorkstationPresent`, `ResourceAvailable(CommodityKind)`, `ContentionState`, `ArtifactState`, `Courage`, `Evidence`.
3. Shared boundary: the sort key in `enforce_entity_claim_capacity()` — this ticket adds a tier dimension as the primary sort key.
4. Auto-correction: the ticket's suggested example used `WorkstationTag::Bench`, but the live enum at `production.rs:10` does not define that variant. Correction applied in focused test coverage: use a live workstation variant (`WorkstationTag::Forge`). Safe because the ticket only needs any lawful `WorkstationPresent` claim to prove tier protection.

## Architecture Check

1. Tier classification uses concrete `EntityBeliefAspect` variants and `believed_kind` — no abstract scores. Aligns with P3.
2. No backward-compatibility shims. The old sort-by-confidence-only behavior is replaced entirely.

## Verification Layers

1. Infrastructure claims survive eviction when competing with item claims -> focused unit test with mixed-tier claims exceeding capacity
2. Within-tier ordering preserved (confidence -> tick -> id) -> focused unit test confirming intra-tier sort stability
3. Single-layer ticket: changes are internal to belief store capacity enforcement

## What to Change

### 1. Add `claim_eviction_tier()` helper

Add to `crates/worldwake-core/src/belief.rs`:

```rust
fn claim_eviction_tier(aspect: EntityBeliefAspect, believed_kind: Option<EntityKind>) -> u8 {
    match aspect {
        EntityBeliefAspect::ResourceAvailable(_) | EntityBeliefAspect::WorkstationPresent => 0,
        EntityBeliefAspect::Location if believed_kind == Some(EntityKind::Place) => 0,
        EntityBeliefAspect::Alive if believed_kind == Some(EntityKind::Agent) => 0,
        _ => 1,
    }
}
```

### 2. Modify sort key in `enforce_entity_claim_capacity()`

In the sort closure at line 241, look up `believed_kind` from `self.known_entities` for each entity being processed. Add `claim_eviction_tier(aspect, believed_kind)` as the primary sort dimension. Lower tier (0) sorts first and survives truncation:

```rust
claims.sort_by_key(|claim| {
    (
        claim_eviction_tier(claim.aspect, believed_kind),
        std::cmp::Reverse(effective_claim_confidence(...)),
        std::cmp::Reverse(claim.acquired_tick),
        std::cmp::Reverse(claim.claim_id),
    )
});
```

The `believed_kind` lookup happens once per entity at the start of the per-entity loop body, reading from `self.known_entities.get(&entity).and_then(|s| s.believed_kind)`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Entity-level eviction (that is S77BELCAPPRI-003)
- Changing claim generation or perception logic
- Adding new `EntityBeliefAspect` variants
- Modifying capacity profile fields

## Acceptance Criteria

### Tests That Must Pass

1. New: When capacity is 3 and there are 2 `ResourceAvailable` claims and 3 `Inventory` claims, the `ResourceAvailable` claims survive and one `Inventory` claim survives
2. New: When capacity is 2 and there are 3 `ResourceAvailable` claims, the two highest-confidence infrastructure claims survive (within-tier ordering preserved)
3. New: `WorkstationPresent` claims survive eviction when competing with `Inventory` claims
4. Existing: `cargo test -p worldwake-core -- enforce_entity_claim_capacity`

### Invariants

1. Infrastructure-tier claims (`ResourceAvailable`, `WorkstationPresent`, `Location` on Places, `Alive` on Agents) are evicted only after all item-tier claims are evicted
2. Within each tier, the existing confidence-based ordering is preserved
3. Total claim count after enforcement does not exceed `entity_claim_capacity`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — `enforce_entity_claim_capacity_preserves_infrastructure_claims` — mixed tier eviction
2. `crates/worldwake-core/src/belief.rs` — `enforce_entity_claim_capacity_respects_within_tier_ordering` — intra-tier confidence ordering
3. `crates/worldwake-core/src/belief.rs` — `enforce_entity_claim_capacity_protects_workstation_present` — workstation claim survival

### Commands

1. `cargo test -p worldwake-core -- enforce_entity_claim_capacity`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Added `claim_eviction_tier()` in `crates/worldwake-core/src/belief.rs` and made `enforce_entity_claim_capacity()` sort claims by tier before confidence/tick/id ordering.
- Preserved the existing within-tier ordering while ensuring infrastructure claims survive item-tier claims when capacity truncation occurs.
- Added focused unit coverage for mixed-tier eviction, within-tier ordering, and workstation-claim protection.

## Verification Result

- Passed `cargo test -p worldwake-core -- enforce_entity_claim_capacity`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
