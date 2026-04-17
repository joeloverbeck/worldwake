# S106: Ground Item Decay

**Status**: COMPLETED

## Summary

Add time-based decay for items resting on the ground. Currently, items created by actions (Waste from toilet/relieve, dropped items from `drop_item`) persist indefinitely on the ground with no removal mechanism. S82 (Waste Disposal) explicitly deferred item decay. The simulation has `evidence_decay.rs` for `SceneEvidence` entries but no analogous system for physical item entities. This violates FND-11: agent population creates a positive feedback loop — more agents produce more waste, more waste pollutes perception and beliefs — with no physical dampener on the world-state side.

The fix: a new `ItemDecay` SystemFn checks ground items each tick and archives those whose ground time exceeds a per-commodity-kind decay threshold. Decay is transformation (archive with event log entry), not silent deletion, preserving FND-4 traceability.

## Phase

Core infrastructure (world dynamics — item lifecycle)

## Crates

- `worldwake-core` (new `GroundSince` component, `CommodityDecayEntry` type, `EventTag::ItemDecay`)
- `worldwake-sim` (new `ItemDecay` SystemId)
- `worldwake-systems` (new `item_decay.rs` SystemFn)
- `worldwake-cli` (scenario decay profile support)
- `worldwake-ai` (golden tests)

## Dependencies

None — new system with no dependencies on other pending specs.

## Problem Statement

### Evidence

Observer run on a multi-agent variant of `scenarios/survival-baseline.ron` (seed 104004, 1440 ticks). Note: the current `survival-baseline.ron` has 1 agent; the data below is illustrative of the accumulation problem at scale:

| Location | Ground Waste Items | Source | Growth Rate |
|----------|-------------------|--------|-------------|
| Fertile Fields | 55 | relieve_wilderness (3 agents, ~22 each) | ~1 per 26 ticks |
| Forest Clearing | 14 | relieve_wilderness | ~1 per 103 ticks |
| Riverside Camp | 1 | relieve_wilderness | negligible |

After 1440 ticks, 70 Waste items exist on the ground with no removal pathway. Extrapolating to multi-day runs: 2880 ticks → ~140 items, 4320 ticks → ~210 items. The growth is linear in simulation length and multiplicative in agent count.

S82 added `drop_item` (agents can shed carried items) and `FreeCarryCapacity` (agents pursue dropping when inventory is 80%+ full), but stated as a **non-goal** (S82 lines 37-40):

> "Waste decay, composting, or environmental cleanup systems — deferred"

### Architectural Violations

- **FND-11**: Item accumulation is a positive feedback loop (more agents → more waste → more entity count → slower processing) with no physical dampener. The only "dampener" is the time cost of the actions that produce waste, which does not limit the accumulation rate relative to consumption.
- **FND-5**: Waste items are not carriers of consequence in the current system — they exist as inert entities that clutter perception and beliefs without producing downstream effects. Decay transforms them into absence-events, which are carriers (an agent expecting items may discover they've decayed, creating violated expectations per FND-17).

## Design

### CommodityDecayEntry

A per-commodity-kind decay configuration. Stored as a `BTreeMap<CommodityKind, NonZeroU32>` in the scenario definition, where the value is the number of ticks an item of that commodity kind can remain on the ground before being archived.

```rust
/// Ticks-to-decay for a commodity kind on the ground.
/// Items not in this map do not decay.
pub type CommodityDecayMap = BTreeMap<CommodityKind, NonZeroU32>;
```

**Default decay values** (scenario-configurable):

| Commodity | Decay Ticks | Rationale |
|-----------|-------------|-----------|
| Waste | 200 | Biological waste decomposes relatively quickly |
| Apple | 720 | Food rots in ~half a simulated day |
| Water | None | Water evaporation is not modeled in the current system |
| All other commodities | None | Durable goods do not decay by default |

Scenarios override these values in the `ScenarioDef`. If no decay map is provided, the default above applies (Waste: 200, Apple: 720).

### GroundSince Component

New ECS component tracking when an item was placed on the ground:

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GroundSince(pub Tick);
```

**Lifecycle:**
- **Tracked on loose ground items only**: eligible kinds are `ItemLot` and `UniqueItem`, with `effective_place` present, no direct container, no possessor, and not in transit.
- **Set/reset** when an item enters that loose-ground state through any tick-aware path: explicit `WorldTxn::set_ground_location`, `remove_from_container` from a grounded container, `clear_possessor` when the item already has a place, and archive-preparation drop/spill resolutions.
- **Cleared** when the item leaves loose-ground state by being possessed, put into a container, or moved into transit.
- **Not set** for items created directly in agent inventory or containers — only loose ground items have `GroundSince`.

Items spawned on the ground during scenario initialization get `GroundSince(Tick(0))`.

### ItemDecay SystemFn

New system following the `evidence_decay.rs` pattern:

```rust
pub fn item_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext { world, event_log, tick, .. } = ctx;

    let decay_map: &CommodityDecayMap = /* read from world config */;
    let mut to_archive = Vec::new();

    for (entity, ground_since) in world.query_ground_since() {
        let Some(item_lot) = world.get_component_item_lot(entity) else { continue };
        let Some(&decay_ticks) = decay_map.get(&item_lot.commodity) else { continue };

        let elapsed = tick.0.saturating_sub(ground_since.0 .0);
        if elapsed >= u64::from(decay_ticks.get()) {
            to_archive.push(entity);
        }
    }

    for entity in to_archive {
        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::SystemTick(tick),
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::ItemDecay)
            .add_tag(EventTag::WorldMutation);

        // archive_entity may fail if the entity has archive dependencies;
        // skip and log rather than panic.
        if let Err(e) = txn.archive_entity(entity) {
            continue;
        }

        let _ = txn.commit(event_log);
    }

    Ok(())
}
```

The API follows the existing `evidence_decay_system` pattern (evidence_decay.rs) for `SystemExecutionContext` destructuring, `WorldTxn` creation, tag registration, and commit.

### System Ordering

New `SystemId::ItemDecay` inserted after `EvidenceDecay` in the canonical order:

```
Needs → Production → Trade → Combat → BanditCamp → Contention → Politics
→ Perception → ExpectationCheck → EvidenceDecay → ItemDecay → Patrol → Compaction
```

Note: `ArtifactLifecycle` runs in the separate `pre_action()` pass, not in the canonical per-tick order.

**Ordering rationale:**
- After `Perception`: same-tick observers can still see the item on its last tick before decay
- After `EvidenceDecay`: groups cleanup systems together
- After `ExpectationCheck`: decayed items are detected by agents on the **next tick** when they re-observe the location and notice the absence via `noticed_missing_subjects` in `collect_direct_local_observation_batch` (perception.rs:494-508)
- Before `Compaction`: archived entities are included in checkpoint state

### Scenario Configuration

The `CommodityDecayMap` is stored as a world-level configuration, not per-agent. It is loaded from the scenario RON file:

```ron
(
    // ... existing fields ...
    commodity_decay: {
        Waste: 200,
        Apple: 720,
    },
)
```

If `commodity_decay` is absent from the RON file, the default map is applied (Waste: 200, Apple: 720).

## Non-Goals

- Decay for carried or stored items (inventory spoilage — deferred)
- Multi-stage decomposition chains (Waste → Compost → Nutrients — deferred; Waste simply archives)
- Environmental modifiers on decay rate (roofed/unroofed, temperature — deferred)
- Scavenger/cleanup agent actions (agents actively cleaning waste — deferred)
- Container decay or facility degradation (structural decay — different system)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Decay rates interact with production rates and agent behavior to produce emergent item-count equilibrium |
| FND-2 (No Ungrounded Triggers) | Decay is a deterministic function of concrete elapsed time and commodity properties — not a drama lever |
| FND-3 (Concrete State) | `GroundSince` is stored authoritative state with accountable origin (set when placed on ground). Elapsed time is derived from `current_tick - ground_since` |
| FND-4 (Persistent Identity) | Items are archived, not deleted. Archive records when and why. `archive_entity` preserves entity metadata with `archived_at` set |
| FND-5 (Carriers of Consequence) | Decay creates downstream consequences: agents holding stale beliefs about the item discover its absence (FND-17 violated expectations) |
| FND-6 (World Runs Without Observers) | Decay runs regardless of whether agents are present. Waste at an unvisited location still decays |
| FND-8 (Preconditions/Duration/Cost) | Decay has explicit preconditions (item on ground, commodity in decay map, elapsed ≥ threshold) and produces explicit aftermath (archived entity, event log entry) |
| FND-9 (Scheduling) | Decay runs at a declared position in the canonical system order. No scheduling ambiguity |
| FND-10 (Outcomes Leave Aftermath) | The item is gone from the active world, the event log records what happened, agents with stale beliefs may discover the absence |
| FND-11 (Physical Dampener) | Decay is the physical dampener for item accumulation. Production creates items; decay removes them after a concrete time period determined by commodity properties |
| FND-12 (Performance Compression) | Bounded entity count prevents linear performance degradation with simulation length |
| FND-26 (Systems Through State) | ItemDecay reads `GroundSince`, `ItemLot`, and the decay map; writes archive state and event log. No cross-system calls |
| FND-28 (No Backward Compat) | New system. Existing scenarios gain decay behavior through default decay map |
| FND-30 (Causal Hooks) | All 18 items addressed through Design, Section H, and Testing sections. `GroundSince` and `CommodityDecayMap` both derive `Serialize`/`Deserialize` and survive save/load/replay without changing world meaning |

## FND-01 Section H Analyses

### 1. Information-Path Analysis

Item decay does not create new information paths. The archived entity is no longer perceivable. Agents who previously observed the item retain a stale belief — the belief will decay via S101's activation-based system (no re-observation to refresh it). Agents returning to the location may notice the item's absence via the existing missing-entity detection in `collect_direct_local_observation_batch` (lines 477-491 of `perception.rs`), which checks believed-here entities against currently observed entities.

### 2. Positive-Feedback Analysis

**Existing loop broken:** More agents → more waste → more entities at location → more perception pollution → less effective agents. Decay breaks this loop: waste created at rate R is archived at rate 1/D (where D = decay ticks), producing a steady-state entity count of approximately R * D.

**New loop introduced?** No. Decay rate is a static per-commodity parameter. It does not increase with entity count or agent behavior.

### 3. Concrete Dampeners

The decay system itself is the dampener: each commodity kind has a concrete time-to-decay grounded in the physical properties of the commodity (waste decomposes, food rots). The dampener is a world process (time-based decomposition), not a numeric cap.

**Equilibrium estimate:** With 3 agents producing ~1 waste per 26 ticks each and Waste decay at 200 ticks, steady-state waste count ≈ 3 * (200/26) ≈ 23 items. This is a 58% reduction from the observed 55 items at 1440 ticks (which had no decay).

### 4. Stored State vs. Derived Read-Model

| Item | Classification | Location |
|------|---------------|----------|
| `GroundSince(Tick)` | Stored state (ECS component) | Per-entity component |
| `CommodityDecayMap` | Stored state (world configuration) | World config (loaded from scenario) |
| Elapsed ground time | Derived (computed per tick) | `item_decay_system` |
| Archive decision | Derived (elapsed ≥ threshold) | `item_decay_system` |

## SystemFn Integration

**New SystemId:** `ItemDecay`

**Registration:** Added to `define_system_ids!` macro in `crates/worldwake-sim/src/system_manifest.rs`, after `EvidenceDecay` and before `Patrol` in the canonical per-tick order.

**Dispatch:** Registered in `dispatch_table()` in `crates/worldwake-systems/src/lib.rs` mapping `SystemId::ItemDecay` to `item_decay_system`.

**Signature:** `fn item_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>` — follows the standard SystemFn pattern used by `evidence_decay_system` (evidence_decay.rs:7).

## Component Registration

### GroundSince

- **Kind:** Runtime-generated state (set by `set_ground_location`, cleared by pickup/container)
- **Registration:** New component in `ComponentSchema` and `ComponentTables`
- **Entity kinds:** `EntityKind::ItemLot` and `EntityKind::UniqueItem` (any entity that can be placed on the ground)
- **Scenario-definable:** Not directly — `GroundSince` is set automatically when items are placed on the ground during scenario initialization. Items spawned on the ground get `GroundSince(Tick(0))`
- **Exempt from AgentDef contract** — this is a runtime-generated component on items, not an agent behavioral component

### CommodityDecayMap (world configuration)

- **Kind:** World-level configuration (not per-entity)
- **Scenario-definable:** Yes — `commodity_decay` field in `ScenarioDef`
- **Storage:** `World` resource or dedicated component on a singleton config entity — implementation follows existing world configuration patterns

## Cross-System Interactions

- **Needs system → ItemDecay**: Needs creates Waste via toilet/relieve actions. ItemDecay archives it after decay ticks. Interaction is mediated through item entity state (FND-26 compliant).
- **Perception → ItemDecay**: Perception runs before ItemDecay, so same-tick observers can still see items on their last tick. After decay, the item is archived and no longer observable.
- **Belief system**: No direct interaction. Stale beliefs about decayed items decay naturally via S101 activation-based retention (no re-observation to refresh).
- **Conservation**: Total items created minus items archived equals live item count at any tick. The `archive_entity` call preserves the entity's metadata — it is not deleted from the world, just marked as archived and excluded from active queries.

## Testing Strategy

1. **Unit test in `item_decay.rs`**: Create 3 ground items (Waste at tick 10, Apple at tick 10, Sword at tick 10). Set Waste decay to 50, Apple decay to 100, Sword has no entry. Run decay system at tick 60: Waste is archived, Apple and Sword survive. Run at tick 110: Apple is archived, Sword survives. Run at tick 200: Sword still survives.

2. **Unit test for GroundSince lifecycle**: Create item, put in container (no GroundSince), remove to ground (GroundSince set to current tick). Pick up (GroundSince removed). Drop again (GroundSince set to new tick). Verify ground time resets on each transition into loose-ground state.

3. **Golden E2E test**: Scenario with 2 agents producing Waste via `relieve_wilderness` for 400 ticks with Waste decay at 200. Assert that ground Waste entity count reaches a steady state (never exceeds ~production_rate * decay_ticks) rather than growing unboundedly. Assert archived Waste count grows.

4. **Event log test**: Verify decayed items produce events with `EventTag::ItemDecay` and `EventTag::WorldMutation` tags.

5. **Conservation test**: At every checkpoint tick, verify using `verify_live_lot_conservation` and `verify_authoritative_conservation` (conservation.rs) that `items_created - items_archived == live_item_count`.

6. **Regression**: All existing golden tests must pass. Default decay values (Waste: 200, Apple: 720) must not interfere with existing test scenarios (which typically run < 200 ticks or have pre-placed items that should not decay during the test window).

## Outcome

Completed on 2026-04-17.

- Landed the full S106 substrate and runtime path across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-cli`: `GroundSince`, persisted `CommodityDecayMap`, `EventTag::ItemDecay`, `SystemId::ItemDecay`, live `item_decay_system`, and scenario/default decay configuration.
- Corrected the original GroundSince lifecycle assumption during implementation so loose-ground state is synchronized from the full relation-derived predicate, not only `set_ground_location`.
- Added the end-to-end proof layer in `worldwake-ai` with Scenario 342 in `golden_item_decay.rs`, plus regenerated golden inventory/detail docs for the new scenario coverage.

## Deviations

- The drafted golden proof surface originally assumed a latrine-oriented setup and a direct archived-commodity total. The landed golden instead keeps agents at an outdoor place so repeated `relieve_wilderness` is the lawful action path, and proves conservation through `WildernessRelief` and `ItemDecay` event counts against live authoritative Waste totals.
- The original ticket decomposition split the work across 001–004; the archived ticket chain now carries the detailed per-slice reassessment and verification history, while this spec records only the final delivered contract.

## Verification Results

- Passed `cargo test -p worldwake-core ground_since`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-systems item_decay`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai golden_waste_decay_reaches_steady_state`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
