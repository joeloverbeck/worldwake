# S178PERFOOSPO-003: Item-decay system advances PerishableState condition

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `item_decay_system` gains condition-advancement loop, storage-context lookup, `LotOperation::Spoiled` emission, `EventTag::ItemSpoiled` emission. Perishable lots no longer archive at duration; they persist post-spoilage. Spawn sites for perishable commodities attach `PerishableState` at creation.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`

## Problem

D3 replaces the archive-at-duration path for perishable food with condition advancement over time. Each item-decay tick, lots carrying `PerishableState` decay by baseline-rate × storage-context multiplier; crossing `spoiled_threshold` emits `LotOperation::Spoiled` lineage and `EventTag::ItemSpoiled` (first emission of both variants). The lot persists (no archival) so it remains a desperation affordance and a disposal candidate. Storage context is derived from the lot's parent entity each tick (possessed → agent parent; container → `EntityKind::Container` parent; ground → `GroundSince` component).

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `item_decay_system` at `crates/worldwake-systems/src/item_decay.rs:8-42` currently iterates ground items via `query_ground_since()`, filters by `CommodityDecayMap` membership, archives matched lots once `elapsed >= decay_ticks` via `apply_decay` (line 263-287). Storage-context distinction does not exist — only ground lots decay. Reads `world.commodity_decay()` at line 49 for the rate map; ticket 001 adds the parallel `world.commodity_perish_profiles()` accessor. `#[cfg(test)]` boundary at line 289. 19 existing `#[test]` functions across lines 290-1300 — ground-archive tests around lines 68-238 are the primary regression surface. Tests for non-perishable `Waste` archival continue to apply unchanged because Waste has no `CommodityPerishProfile` entry per ticket 001's default map (verified by spec's Non-Goal "No replacement of `CommodityDecayMap`'s Waste handling").
2. Spec D3 verified against current `specs/S178-perishable-food-spoilage.md`. FND-26 (systems via state) — the system reads `PerishableState`, lot parent, `CommodityPerishProfile`; writes `PerishableState.condition`, lineage, event. FND-28 — the archive-only path for Apple is replaced via a `world.has_component_perishable_state(lot)` skip-filter in `collect_decay_targets`, ensuring no parallel execution on perishable lots. FND-29A — `last_advanced_tick` makes advancement replay-deterministic.
3. Shared abstraction boundary: the `SystemExecutionContext` surface (`world`, `event_log`, `tick`); the `GroundSince` component as the storage-context discriminator for ground; the parent-entity lookup (`world.parent_of(lot)` / `world.entity_kind(parent)`) as the discriminator for possessed/container. Lot-spawn sites for perishable commodities are an additional integration point — `set_component_item_lot` workspace-wide grep enumerates them at implementation time.
4. Authoritative arithmetic for Apple at ground: baseline rate = `1000 / 720 ≈ 1.39 permille/tick`. After 240 ticks ground exposure, condition crosses `stale_threshold` (667). After 480 ticks, crosses `spoiled_threshold` (333). After 720 ticks, condition reaches 0 (lot still present as a Spoiled affordance). Storage multipliers: `container=500` halves rate (480 ticks to Stale, 960 to Spoiled); `possessed=750` slows to 75% (320 ticks to Stale). Integer arithmetic per AGENTS.md Determinism invariant — no floats.

## Architecture Check

1. The condition-advancement loop is gated on `query_with::<PerishableState>` (sparse iteration), not on every lot. Non-perishable lots (Waste, Bread, Grain) are untouched by the new code path. The archive-at-duration path persists for non-perishable commodities via the unchanged `CommodityDecayMap` flow, preserving Waste decay (S82) without parallel-path risk. The skip-filter on `has_component_perishable_state` in `collect_decay_targets` makes the FND-28 single-live-path contract structural — perishable lots cannot enter the archive branch even if they remain map-listed.
2. Storage-context lookup is a pure read against the lot's parent entity. No new component on places, no new tag — preserving-place context is explicitly deferred per spec Non-Goals. Three contexts (ground/container/possessed) cover the headline scenario without speculation.

## Verification Layers

1. `PerishableState.condition` advances at the correct per-tick delta for each storage context → focused unit test on the advancement helper (3 tests, one per context).
2. `LotOperation::Spoiled` appended to lot provenance when condition crosses `spoiled_threshold` → authoritative world-state assertion on `world.get_component_item_lot(lot).provenance`.
3. `EventTag::ItemSpoiled` emitted with lot identity payload → event-log delta assertion.
4. Spoiled lot persists (no archival) → authoritative world state assertion: `world.has_component_item_lot(lot)` remains `true` post-spoilage.
5. Non-perishable lots (Waste) continue archiving via the legacy path → regression assertion on existing item-decay tests.

## What to Change

### 1. Storage-context lookup helper

In `crates/worldwake-systems/src/item_decay.rs`, add:

```rust
enum StorageContext { Ground, Container, Possessed }

fn storage_context(world: &World, lot: EntityId) -> StorageContext {
    if let Some(parent) = world.parent_of(lot) {
        match world.entity_kind(parent) {
            Some(EntityKind::Agent) => return StorageContext::Possessed,
            Some(EntityKind::Container) => return StorageContext::Container,
            _ => {}
        }
    }
    StorageContext::Ground
}
```

### 2. Condition-advancement loop

Add a sibling function to `collect_decay_targets`:

```rust
fn advance_perishable_conditions(world: &mut World, event_log: &mut EventLog, tick: Tick) {
    let updates: Vec<_> = world
        .iter_perishable_states()
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
            let baseline = 1000u64 / u64::from(profile.fresh_to_spoiled_ticks.get());
            let permille_delta = elapsed
                .saturating_mul(baseline)
                .saturating_mul(u64::from(multiplier.value()))
                / 1000;
            let new_value = (state.condition.value() as u64).saturating_sub(permille_delta) as u16;
            let crossed_spoiled = state.condition.value() >= profile.spoiled_threshold.value()
                && new_value < profile.spoiled_threshold.value();
            Some((lot, PerishableState {
                condition: Permille::new_unchecked(new_value),
                last_advanced_tick: tick,
            }, crossed_spoiled))
        })
        .collect();

    for (lot, new_state, crossed) in updates {
        world.set_component_perishable_state(lot, new_state);
        if crossed {
            append_spoiled_to_lineage(world, lot, tick);
            emit_item_spoiled_event(event_log, tick, lot);
        }
    }
}
```

Call `advance_perishable_conditions` from `item_decay_system` before the existing `collect_decay_targets` / `apply_decay` pair.

### 3. Lineage and event emission

`append_spoiled_to_lineage(world, lot, tick)` pushes `ProvenanceEntry { operation: LotOperation::Spoiled, tick, source: None }` onto the lot's `provenance` vec. `emit_item_spoiled_event(event_log, tick, lot)` writes a transactional event tagged `EventTag::ItemSpoiled` with the lot's `EntityId` as payload. Both helpers are file-private to `item_decay.rs`. Emission is idempotent because the `crossed_spoiled` guard fires only when condition transitions across `spoiled_threshold`, not on every subsequent tick.

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

When a new item lot is spawned with a commodity that has a `CommodityPerishProfile` entry, attach `PerishableState { condition: Permille::new_unchecked(1000), last_advanced_tick: spawn_tick }`. Spawn sites are discovered via `rg 'set_component_item_lot' crates/` during implementation. Likely sites: harvest action commits, production action commits, scenario lot spawn.

## Files to Touch

- `crates/worldwake-systems/src/item_decay.rs` (modify — `storage_context` helper, `advance_perishable_conditions`, `append_spoiled_to_lineage`, `emit_item_spoiled_event`, skip-filter in `collect_decay_targets`)
- Likely: `crates/worldwake-systems/src/needs_actions.rs` (modify — if harvest commits ItemLot here, attach `PerishableState` for perishable commodities)
- Likely: `crates/worldwake-systems/src/production_actions.rs` (modify — if production commits ItemLot, attach `PerishableState`)
- Likely: `crates/worldwake-cli/src/scenario/mod.rs` (modify — if scenarios spawn ItemLot directly, attach `PerishableState`)
- To be confirmed: lot-spawn sites enumerated via `rg 'set_component_item_lot' crates/` at implementation time

## Out of Scope

- Eat-handler relief scaling (ticket 004).
- Belief-view accessors / perception write of `last_observed_condition` (ticket 005).
- Candidate generation (ticket 006).
- Forensic record (ticket 007).
- Preserving-place storage context (deferred per spec Non-Goals).
- Disposal/compost-into-Waste action (out of scope per S178; scenario-level only if needed by ticket 008's 1440-tick golden).
- Removal of `CommodityDecayMap.Apple` entry (left in place for back-compat with non-perishable tests; future cleanup ticket).

## Acceptance Criteria

### Tests That Must Pass

1. `perishable_condition_advances_at_ground_baseline_rate` — after N ticks of ground exposure, an Apple lot's condition decreased by exactly `N × baseline_rate × ground_multiplier / 1000`.
2. `perishable_condition_advances_slower_in_container` — container-stored Apple lot decreases at half the ground rate (per `storage_rates.container=500`).
3. `perishable_condition_advances_at_intermediate_rate_when_possessed` — possessed lot at 75% rate (per `storage_rates.possessed=750`).
4. `perishable_lot_emits_spoiled_lineage_at_threshold` — `LotOperation::Spoiled` appears in lot provenance at the spoilage-crossing tick.
5. `perishable_lot_emits_item_spoiled_event` — `EventTag::ItemSpoiled` event with lot payload at the spoilage tick.
6. `perishable_lot_persists_after_spoilage` — `world.has_component_item_lot(lot)` is `true` after spoilage; lot is not archived.
7. `waste_lot_still_archives_via_legacy_path` — non-perishable Waste continues to archive at `CommodityDecayMap.Waste=200` ticks (regression guard against the skip-filter).
8. `item_spoiled_emission_is_idempotent_across_subsequent_ticks` — a lot that crossed `spoiled_threshold` does not re-emit `ItemSpoiled` on later ticks.
9. Existing item-decay tests pass: `cargo test -p worldwake-systems item_decay`.

### Invariants

1. Perishable lots are never archived via `collect_decay_targets` — the skip-filter excludes them (FND-28 single-live-path).
2. `PerishableState.last_advanced_tick` is updated atomically with `condition` so replay determinism holds across variable system cadence (FND-29A).
3. `EventTag::ItemSpoiled` emission is idempotent — only the threshold-crossing tick emits, not subsequent advances.
4. Integer arithmetic only — no floats in condition advancement (AGENTS.md Determinism invariant).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/item_decay.rs` `#[cfg(test)]` — add 8 new tests (3 storage contexts × condition advance + lineage + event + persistence + idempotence + waste regression).
2. Existing `waste_archives_at_duration` (or equivalently named) — verify it passes unchanged.

### Commands

1. `cargo test -p worldwake-systems item_decay`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
