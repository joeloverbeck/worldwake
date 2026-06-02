# S178PERFOOSPO-003: Item-decay system advances PerishableState condition

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `item_decay_system` gains condition-advancement loop, storage-context lookup, `LotOperation::Spoiled` emission, `EventTag::ItemSpoiled` emission. Perishable lots no longer archive at duration; they persist post-spoilage. The core lot lifecycle attaches `PerishableState` at perishable-lot creation and carries it across splits. `PerishableState` stores a fractional decay remainder, requiring a save-format bump.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`

## Problem

D3 replaces the archive-at-duration path for perishable food with condition advancement over time. Each item-decay tick, lots carrying `PerishableState` decay by elapsed ticks × storage-context multiplier divided by `fresh_to_spoiled_ticks`, with the division remainder carried in `PerishableState.decay_remainder`; crossing `spoiled_threshold` emits `LotOperation::Spoiled` lineage and `EventTag::ItemSpoiled` (first emission of both variants). The lot persists (no archival) so it remains a desperation affordance and a disposal candidate. Storage context is derived from live placement/possession each tick (possessed by an agent → possessed rate; inside `EntityKind::Container` → container rate; otherwise ground rate).

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `item_decay_system` at `crates/worldwake-systems/src/item_decay.rs:8-42` currently iterates ground items via `query_ground_since()`, filters by `CommodityDecayMap` membership, archives matched lots once `elapsed >= decay_ticks` via `apply_decay` (line 263-287). Storage-context distinction does not exist — only ground lots decay. Reads `world.commodity_decay()` at line 49 for the rate map; ticket 001 adds the parallel `world.commodity_perish_profiles()` accessor. `#[cfg(test)]` boundary at line 289. 19 existing `#[test]` functions across lines 290-1300 — ground-archive tests around lines 68-238 are the primary regression surface. Tests for non-perishable `Waste` archival continue to apply unchanged because Waste has no `CommodityPerishProfile` entry per ticket 001's default map (verified by spec's Non-Goal "No replacement of `CommodityDecayMap`'s Waste handling").
2. Spec D3 verified against current `archive/specs/S178-perishable-food-spoilage.md`. FND-26 (systems via state) — the system reads `PerishableState`, placement/possession, `CommodityPerishProfile`; writes `PerishableState.condition`, `last_advanced_tick`, `decay_remainder`, lineage, event. FND-28 — the archive-only path for Apple is replaced via a `world.has_component_perishable_state(lot)` skip-filter in `collect_decay_targets`, ensuring no parallel execution on perishable lots. FND-29A — `last_advanced_tick` plus `decay_remainder` make advancement replay-deterministic and cadence-independent.
3. Shared abstraction boundary: the `SystemExecutionContext` surface (`world`, `event_log`, `tick`); `world.possessor_of(lot)` as the possessed discriminator; `world.direct_container(lot)` as the container discriminator; ground/default when neither relation applies. Lot creation is centralized in `World::create_item_lot_with_provenance`, which is the correct attachment seam for perishable commodities; `World::split_lot` is the lifecycle seam that must copy perishable state rather than resetting the child to fresh.
4. Authoritative arithmetic for Apple at ground: delta = `(elapsed_ticks × storage_multiplier + decay_remainder) / 720`. With `ground=1000`, after 240 ticks condition is 667 with remainder 240; after 481 ticks condition crosses `spoiled_threshold` (333); after 720 ticks, condition reaches 0 (lot still present as a Spoiled affordance). Storage multipliers: `container=500` halves rate (480 ticks to Stale, 960 to Spoiled); `possessed=750` slows to 75% (320 ticks to Stale). Integer arithmetic per AGENTS.md Determinism invariant — no floats.

## Architecture Check

1. The condition-advancement loop is gated on `query_with::<PerishableState>` (sparse iteration), not on every lot. Non-perishable lots (Waste, Bread, Grain) are untouched by the new code path. The archive-at-duration path persists for non-perishable commodities via the unchanged `CommodityDecayMap` flow, preserving Waste decay (S82) without parallel-path risk. The skip-filter on `has_component_perishable_state` in `collect_decay_targets` makes the FND-28 single-live-path contract structural — perishable lots cannot enter the archive branch even if they remain map-listed.
2. Storage-context lookup is a pure read against live placement/possession relations (`world.possessor_of(lot)` and `world.direct_container(lot)`). No new component on places, no new tag — preserving-place context is explicitly deferred per spec Non-Goals. Three contexts (ground/container/possessed) cover the headline scenario without speculation.

## Verified Layers

1. `PerishableState.condition` advances at the correct per-tick delta for each storage context → focused unit test on the advancement helper (3 tests, one per context).
2. `LotOperation::Spoiled` appended to lot provenance when condition crosses `spoiled_threshold` → authoritative world-state assertion on `world.get_component_item_lot(lot).provenance`.
3. `EventTag::ItemSpoiled` emitted with lot identity payload → event-log delta assertion.
4. Spoiled lot persists (no archival) → authoritative world state assertion: `world.has_component_item_lot(lot)` remains `true` post-spoilage.
5. Non-perishable lots (Waste) continue archiving via the legacy path → regression assertion on existing item-decay tests.

## Landed Changes

### 1. Storage-context lookup helper

In `crates/worldwake-systems/src/item_decay.rs`, add:

```rust
enum StorageContext { Ground, Container, Possessed }

fn storage_context(world: &World, lot: EntityId) -> StorageContext {
    if world
        .possessor_of(lot)
        .is_some_and(|holder| world.entity_kind(holder) == Some(EntityKind::Agent))
    {
        return StorageContext::Possessed;
    }

    if world
        .direct_container(lot)
        .is_some_and(|container| world.entity_kind(container) == Some(EntityKind::Container))
    {
        return StorageContext::Container;
    }

    StorageContext::Ground
}
```

### 2. Condition-advancement loop

Add a sibling function to `collect_decay_targets`:

```rust
fn collect_perishable_condition_updates(world: &World, tick: Tick) -> Vec<PerishableConditionUpdate> {
    let updates: Vec<_> = world
        .query_perishable_state()
        .filter_map(|(lot, state)| {
            let commodity = world.get_component_item_lot(lot)?.commodity;
            let profile = world.commodity_perish_profiles().get(&commodity)?.clone();
            let elapsed = tick.0.saturating_sub(state.last_advanced_tick.0);
            let context = storage_context(world, lot);
            let multiplier = match context {
                StorageContext::Ground => profile.storage_rates.ground,
                StorageContext::Container => profile.storage_rates.container,
                StorageContext::Possessed => profile.storage_rates.possessed,
            };
            let numerator = u64::from(state.decay_remainder)
                .saturating_add(elapsed.saturating_mul(u64::from(multiplier.value())));
            let divisor = u64::from(profile.fresh_to_spoiled_ticks.get());
            let permille_delta = numerator / divisor;
            let decay_remainder = numerator % divisor;
            let new_value = (state.condition.value() as u64).saturating_sub(permille_delta) as u16;
            let crossed_spoiled = state.condition.value() >= profile.spoiled_threshold.value()
                && new_value < profile.spoiled_threshold.value();
            Some((lot, PerishableState {
                condition: Permille::new_unchecked(new_value),
                last_advanced_tick: tick,
                decay_remainder: if new_value == 0 { 0 } else { decay_remainder as u32 },
            }, crossed_spoiled))
        })
        .collect();
    updates
}
```

Call the perishable update collection/apply pass from `item_decay_system` before the existing `collect_decay_targets` / `apply_decay` pair. Apply updates through `WorldTxn` so component deltas, lineage, tags, and targets stay in the event record.

### 3. Lineage and event emission

When a lot crosses the spoiled threshold, append `LotOperation::Spoiled` through `WorldTxn::append_spoilage_provenance` and tag the same hidden system event with `EventTag::ItemSpoiled`, `EventTag::ItemDecay`, `EventTag::WorldMutation`, and the lot target. Emission is idempotent because the `crossed_spoiled` guard fires only when condition transitions across `spoiled_threshold`, not on every subsequent tick.

### 4. Skip perishable lots in `collect_decay_targets`

Extend the filter at line 44-54:

```rust
fn collect_decay_targets(world: &World, tick: Tick) -> Vec<EntityId> {
    world.query_ground_since()
        .filter_map(|(entity, ground_since)| {
            if world.has_component_perishable_state(entity) {
                return None; // perishable lots advance, never archive
            }
            let item_lot = world.get_component_item_lot(entity)?;
            let decay_ticks = world.commodity_decay().get(&item_lot.commodity)?;
            let elapsed = tick.0.saturating_sub(ground_since.0.0);
            (elapsed >= u64::from(decay_ticks.get())).then_some(entity)
        })
        .collect()
}
```

This is the FND-28 single-live-path enforcement — perishable lots cannot enter the archive branch regardless of `CommodityDecayMap` membership.

### 5. Initialize `PerishableState` at perishable-lot spawn

When a new item lot is spawned with a commodity that has a `CommodityPerishProfile` entry, attach `PerishableState { condition: Permille::new_unchecked(1000), last_advanced_tick: spawn_tick, decay_remainder: 0 }` inside the central lot creation helper. When an existing perishable lot is split, copy the source lot's `PerishableState` to the split child so split operations do not reset old food to fresh.

### 6. Persist fractional decay remainder

Add `PerishableState.decay_remainder` and bump `SAVE_FORMAT_VERSION` from 117 to 118. Extend component table, delta, bincode, and save/load witnesses so the fractional remainder survives serialization.

## Landed Files

- `crates/worldwake-systems/src/item_decay.rs` (modify — `storage_context` helper, `advance_perishable_conditions`, `append_spoiled_to_lineage`, `emit_item_spoiled_event`, skip-filter in `collect_decay_targets`)
- `crates/worldwake-core/src/world.rs` (modify — attach perishable state in central item-lot creation; copy perishable state on split)
- `crates/worldwake-core/src/world_txn.rs` (modify — transactional spoilage provenance helper)
- `crates/worldwake-core/src/items.rs` (modify — add `decay_remainder` to `PerishableState`)
- `crates/worldwake-core/src/delta.rs` and `crates/worldwake-core/src/component_tables.rs` (modify — update component witnesses)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 117→118 and assert remainder roundtrip)

## Out of Scope

- Eat-handler relief scaling (ticket 004).
- Belief-view accessors / perception write of `last_observed_condition` (ticket 005).
- Candidate generation (ticket 006).
- Forensic record (ticket 007).
- Preserving-place storage context (deferred per spec Non-Goals).
- Disposal/compost-into-Waste action (out of scope per S178; scenario-level only if needed by ticket 008's 1440-tick golden).
- Removal of `CommodityDecayMap.Apple` entry (left in place for back-compat with non-perishable tests; future cleanup ticket).

## Acceptance Criteria

### Acceptance Tests

1. `perishable_ground_lot_advances_condition_with_fractional_remainder` — after 240 ticks of ground exposure, an Apple lot reaches condition 667 with remainder 240.
2. `perishable_container_lot_uses_container_storage_rate` — container-stored Apple lot reaches the same condition after 480 ticks because the container multiplier is 500.
3. `perishable_possessed_lot_uses_possessed_storage_rate` — possessed Apple lot reaches the same condition after 320 ticks because the possessed multiplier is 750.
4. `perishable_lot_emits_spoilage_once_and_persists` — `LotOperation::Spoiled` appears in lot provenance at the threshold-crossing tick.
5. `perishable_lot_emits_spoilage_once_and_persists` — `EventTag::ItemSpoiled` event with lot payload is emitted at the spoilage tick.
6. `perishable_lot_emits_spoilage_once_and_persists` — the lot remains alive with `ItemLot` after spoilage; it is not archived.
7. `waste_decays_at_threshold_tick` — non-perishable Waste continues to archive through the legacy `CommodityDecayMap` path.
8. `perishable_lot_emits_spoilage_once_and_persists` — a lot that crossed `spoiled_threshold` does not re-emit `ItemSpoiled` on a later tick.
9. Existing item-decay tests pass: `cargo test -p worldwake-systems item_decay`.

### Invariants

1. Perishable lots are never archived via `collect_decay_targets` — the skip-filter excludes them (FND-28 single-live-path).
2. `PerishableState.last_advanced_tick` and `decay_remainder` are updated atomically with `condition` so replay determinism holds across variable system cadence (FND-29A).
3. `EventTag::ItemSpoiled` emission is idempotent — only the threshold-crossing tick emits, not subsequent advances.
4. Integer arithmetic only — no floats in condition advancement (AGENTS.md Determinism invariant).
5. Splitting a perishable lot preserves the source lot's condition/remainder on the child lot.

## Test Plan Result

### Landed Tests

1. `crates/worldwake-systems/src/item_decay.rs` `#[cfg(test)]` — added focused tests for all three storage contexts and spoilage event/provenance/persistence/idempotence.
2. Existing `waste_decays_at_threshold_tick` — verified it passes unchanged as the legacy non-perishable archive guard.

### Verification Commands

1. `cargo test -p worldwake-systems item_decay`
2. `cargo test --workspace`
3. `./scripts/verify.sh`

## Outcome

Implemented perishable condition advancement in `item_decay_system` before the legacy archive pass. Perishable lots now skip `collect_decay_targets`, advance condition by integer-only elapsed storage multiplier math, persist after crossing the spoiled threshold, and emit a single hidden system event tagged `ItemDecay`, `WorldMutation`, and `ItemSpoiled` on the threshold crossing. The same transaction appends `LotOperation::Spoiled` provenance with the committed event id.

The implementation also centralized perishable state creation in `World::create_item_lot_with_provenance`, copied perishable state across `World::split_lot`, added `PerishableState.decay_remainder` for cadence-independent arithmetic, and bumped `SAVE_FORMAT_VERSION` from 117 to 118 with explicit roundtrip coverage.

## Deviations

The ticket draft assumed a two-field `PerishableState` and `1000 / fresh_to_spoiled_ticks` integer baseline. That would make Apple ground spoilage traverse 1000 ticks, not the spec-pinned 720 ticks. The landed implementation stores `decay_remainder` and uses `(elapsed_ticks × storage_multiplier + remainder) / fresh_to_spoiled_ticks`, preserving deterministic integer arithmetic and the authored timeline.

The draft also referenced a generic `world.parent_of(lot)` helper. The live repo uses `world.possessor_of(lot)` and `world.direct_container(lot)` as the placement/possession seams, so the implementation uses those APIs directly.

## Verification Result

- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S178PERFOOSPO-003.md`.
- Passed `cargo test -p worldwake-systems item_decay`.
- Passed `cargo test -p worldwake-core perishable`.
- Passed `cargo test -p worldwake-sim save_format_version_is_118_after_perishable_decay_remainder`.
- Passed `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_full_nondefault_state`.
- Passed `cargo test --workspace`.
- Waived `./scripts/verify.sh` for this per-ticket closeout; the full queued spec closeout will own the pre-PR verification gate.
