# S178: Perishable Food Spoilage and Lot Condition

## Summary

Before this spec began implementation, `LotOperation::Spoiled` existed in the item-lot lineage enum (`crates/worldwake-core/src/items.rs:230-256`) but was **never constructed anywhere** — it was a schema variant with no emitter. Ticket `archive/tickets/S178PERFOOSPO-003.md` lands the first emitter in `item_decay_system`. The original unused variant was a strong, deliberate hint left by `archive/specs/S82-waste-disposal-inventory-management.md` / the item-lot lineage design: spoilage was always intended to be richer than the prior behavior. Before S178:

- `CommodityDecayMap` (`crates/worldwake-core/src/items.rs:337-346`) mapped a commodity to a single decay duration (`Apple → nz(720)` ticks, `Waste → nz(200)`), and `item_decay_system` (`crates/worldwake-systems/src/item_decay.rs`) **archived ground item lots** once that duration elapsed.
- Decay was therefore binary and location-limited: a lot existed at full value until it was removed (archived) — there was no per-lot freshness, no reduced relief from aging food, and stored/cached food (not on the ground) did not perish at all.
- `ItemLot` (`crates/worldwake-core/src/items.rs:315-325`) carries `commodity`, `quantity`, `provenance`, and the S177-added `quality: Option<WaterQuality>` field. There is no condition/freshness field for food perishability.

The pre-S178 result was food as a binary timer (fresh until it vanished), exactly the "stored resources disappear as invisible timer failure" anti-pattern the source report identifies. This spec adds **per-lot `PerishableState`** as a separate ECS component on item lots so perishable food degrades through `Fresh → Stale → Spoiled` with concrete consequences — reduced relief from stale food, a spoiled-but-still-existing lot (ticket 003 now emits `LotOperation::Spoiled`), and a profile-driven desperation choice to eat spoiled food. It is the third slice of the deferred Cluster 1 material-degradation wave (`specs/IMPLEMENTATION-ORDER.md`).

**Critical reassessment of the source report:** the report floats a later `FoodSickness`/`DigestiveDistress` consequence for eating spoiled food. This spec **defers that disease consequence** (FND-5 / the report's own MUST-NOT — no disease ecology until it is a proven concrete carrier with recovery and proof). Spoilage here changes *affordance value and lineage*, not health. Spoiled food gives minimal relief; it does not (yet) wound.

## Phase

Phase 7: Consequence Carriers

## Status

📝 DRAFT — authored, awaiting activation (held adjunct wave; see `specs/IMPLEMENTATION-ORDER.md`)

## Crates

- `worldwake-core` (`PerishableState` component on item lots; per-commodity `CommodityPerishProfile` + `StorageRateMultipliers`; new `EventTag::ItemSpoiled` variant added to enum + `ALL` array; first emission of `LotOperation::Spoiled`)
- `worldwake-sim` (belief-view accessors `lot_condition`, `lot_freshness_band`, and `commodity_perish_profile` on `GoalBeliefView`; per-agent perception of co-located lot condition under FND-14A via `PerAgentBeliefView`)
- `worldwake-systems` (item-decay system advances `PerishableState` over time by storage context, instead of only archiving; Eat relief scales by condition)
- `worldwake-ai` (candidate generation prefers fresher lots; eats spoiled food only under profile-permitted desperation; survival forensics record spoiled-cache discovery)
- `worldwake-cli` (scenario contract for `CommodityPerishProfile` and `MetabolismProfile.spoiled_food_hunger_threshold`; player-POV gating for lot condition observation)

## Dependencies

- `archive/specs/S82-waste-disposal-inventory-management.md` — provides `ItemLot`, `CommodityKind::Waste`, the lineage `LotOperation` enum (including the historically unused `Spoiled` variant that ticket `archive/tickets/S178PERFOOSPO-003.md` now emits), and the `CommodityDecayMap` substrate this spec extends.
- `archive/specs/S79-resource-source-consumption-affordances.md` — provides the consumable-profile / Eat-Drink action substrate this spec scales by condition.
- `archive/specs/S129-place-dirtiness-facility-wear.md` — precedent for a per-entity degradation component advanced by the item-decay system.
- `archive/specs/S120-survival-critical-window-forensics.md` — `SurvivalForensicExtractor` extended with spoiled-cache-discovery records.
- `archive/specs/S177-water-source-quality-depletion-reliability.md` — precedent for a perception axis on item lots (`ItemLot.quality: Option<WaterQuality>`), for condition-scaled relief in a consume action (Drink), and for the belief-vs-observed mismatch surface (`SourceAcquisitionFailure`, `ReliabilityRecord.last_observed_quality`) this spec mirrors for food spoilage.

## Design Goals

- Perishable food lots carry concrete `PerishableState` (a `Permille` condition + derived `Fresh`/`Stale`/`Spoiled` band by per-commodity thresholds). Condition degrades over time, faster or slower by **storage context** (exposed on the ground vs. inside a container vs. directly possessed by an agent).
- Eating **fresh** food gives full relief; **stale** gives reduced relief (scaled by condition); **spoiled** gives minimal relief and is normally avoided.
- A spoiled lot **still exists** — it transforms (emits `LotOperation::Spoiled`, preserving provenance/derivation lineage per FND-4) rather than vanishing. It remains an affordance (desperation food) or a candidate for disposal/composting into `Waste`.
- Eating spoiled food is a **lawful, profile-gated desperation choice**: a starving agent with high risk tolerance may eat spoiled food for minimal relief; a well-fed or cautious agent will not. No script — emergent from hunger pressure × profile.
- Stored/cached food perishes too (not just ground lots), so a remembered cache can spoil before the agent arrives — feeding the belief/fallback loop (couples to S177's reliability theme and canonical scenario D).
- Player/AI symmetric; the CLI surfaces only lot condition the controlled agent lawfully perceives.

## Non-Goals

- **No food-borne sickness, disease, digestive distress, or wound from spoiled food.** Deferred (see Summary; FND-5). Spoilage is a value/lineage axis only.
- **No cooking, preservation crafting, recipes, or nutrition model.** A "preserve food" or "cook" action is a future-spec trigger if scenarios prove storage context alone is insufficient.
- **No preserving-place tag or place component.** Storage context in this spec covers only three concrete cases: ground/default, container (via `world.direct_container(lot)` with `EntityKind::Container`), and possessed (via `world.possessor_of(lot)` with an agent holder). Preserving places (cellars, larders, cold storage) are deferred to a future Cold Storage spec that lands when scenarios prove ground/container/possessed differentiation is insufficient (YAGNI per FND-28). Until then, food in a container slows but does not stop spoilage.
- **No perishability for `Grain` or `Bread`.** This spec's default `CommodityPerishProfile` covers `Apple` only. `Grain` and `Bread` (and any later food commodities) remain non-perishable in this spec. A sibling spec extends the profile map once `survival-food-spoilage-cache-1440` proves the pattern works for `Apple`.
- **No new global food-supply score.** Per-lot concrete state only (FND-3).
- **No replacement of `CommodityDecayMap`'s Waste handling.** Waste continues to decay/archive as today; this spec governs *perishable food* lots specifically.
- **No HTN method.** Fresher-lot preference and desperation-eat are flat GOAP candidate ranking/gating.
- **No backward-compatibility shim.** The "archive at decay duration" path for perishable food is replaced by condition advancement → spoilage → optional disposal; goldens depending on the old archive-only behavior are updated (FND-28). Non-perishable commodities are unaffected.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | A remembered apple cache spoils before the hungry agent arrives → eat spoiled for scraps, seek a fallback, or trade → drama emerges from time + storage, not an authored "food rots now" trigger |
| FND-2 (No ungrounded triggers) | Storage-context rate multipliers and condition thresholds are world-authored profile values, not bare numeric dials; defaults pinned in D2 are concrete and per-commodity |
| FND-3 (Concrete state over abstract scores) | Condition is a per-lot `Permille`; bands are concrete thresholds; no `freshness_score` aggregate |
| FND-4 (Persistent identity / derivation lineage) | A spoiling lot keeps its identity; spoilage emits `LotOperation::Spoiled` preserving provenance — "this spoiled lot came from that harvest" |
| FND-5 (Carriers of consequence) | Spoilage propagates: reduced relief, desperation affordance, disposal-into-Waste, belief invalidation of a cache — real downstream effects, not decorative realism |
| FND-7 / FND-14A / FND-14B | Lot condition observed when co-located/possessed; remote cache condition is belief-backed via D8 accessors; the planner cannot read a remote lot's freshness as authoritative |
| FND-10 (Aftermath) | Spoilage is granular: full → reduced → minimal relief, then a still-present spoiled lot; not a boolean disappearance |
| FND-11 (Positive feedback) | Hoarding more food than can be eaten before spoiling → waste; see Section H dampeners |
| FND-13 (Boundary processes) | Off-map perishable inflow arrives carrying `PerishableState` from `specs/S62-boundary-processes-remote-shocks.md`'s boundary substrate when that spec lands; until then, scenarios spawn fresh lots authoritatively |
| FND-16 / FND-17 (Ignorance / violated expectation) | Believing a cache is fresh and finding it spoiled is the expectation violation that updates belief |
| FND-19 (Agent symmetry) | Same condition-scaled relief and desperation gating for human and AI |
| FND-22 (Agent diversity) | Risk tolerance / hunger threshold for eating spoiled food is a per-agent profile parameter — agents differ |
| FND-26 (Systems via state) | Item-decay system advances condition; Eat reads it; planner reads belief view; no direct calls |
| FND-28 (No backcompat) | Perishable archive-only path replaced by condition advancement; no parallel path |
| FND-29 / FND-29A | "Why did this agent eat spoiled food?" answerable from hunger pressure + lot condition + profile threshold at decision tick; `LotOperation::Spoiled` append-only in lineage |
| FND-31 (Validation) | Focused goldens + 1440-tick food-spoilage-cache collision scenario |

## Deliverables

### D1. `PerishableState` component on item lots

```rust
/// Per-lot freshness for perishable commodities. `condition` is Permille:
/// 1000 = fully fresh, 0 = fully spoiled. The Fresh/Stale/Spoiled band is
/// derived from `condition` against the commodity's perish profile thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerishableState {
    pub condition: Permille,
    pub last_advanced_tick: Tick,
    /// Fractional decay numerator carried between item-decay ticks.
    ///
    /// The denominator is the commodity's `fresh_to_spoiled_ticks`; carrying
    /// this remainder keeps integer-only advancement cadence-independent.
    pub decay_remainder: u32,
}

impl Component for PerishableState {}
```

Attached to item lots of perishable commodities at creation (harvest/production/spawn). Non-perishable commodities carry no `PerishableState` at all — sparse-component fit, and the item-decay system's iteration is `query_with(PerishableState)`, not "every lot".

**Construction at lot creation**: `PerishableState { condition: Permille::new_unchecked(1000), last_advanced_tick: creation_tick, decay_remainder: 0 }`. No `Default` impl — the component is always constructed with the explicit creation tick so advancement is deterministic from tick zero of the lot's lifetime.

**Why a separate component, not a field on `ItemLot`**: `ItemLot.quality: Option<WaterQuality>` (added by S177) is an immutable perception axis set at extraction time. `PerishableState.condition` is mutable system-driven state advanced each item-decay tick; the item-decay system iterates only perishable lots (sparse population); and lots without `PerishableState` simply do not perish. This mirrors the `GroundSince` precedent (per-lot mutable tick state as a separate component) rather than the `quality` precedent (per-lot immutable enum on the lot struct).

### D2. Per-commodity perish profile + storage-rate multipliers

A `CommodityPerishProfile` (parallel to `CommodityDecayMap`, world-authored): per perishable commodity, the ticks-to-traverse the full condition range, the `Permille` thresholds dividing `Fresh`/`Stale`/`Spoiled`, and storage-context rate multipliers.

```rust
pub struct CommodityPerishProfile {
    pub fresh_to_spoiled_ticks: NonZeroU32,
    pub stale_threshold: Permille,   // condition below this → Stale
    pub spoiled_threshold: Permille, // condition below this → Spoiled
    pub storage_rates: StorageRateMultipliers,
}

pub struct StorageRateMultipliers {
    pub ground: Permille,    // baseline spoil rate
    pub container: Permille, // rate when the lot is inside a Container entity
    pub possessed: Permille, // rate when directly possessed by an agent
}
```

`StorageRateMultipliers` values are `Permille` multipliers applied to the baseline spoil rate (1000 = full rate, 500 = half rate, 0 = no decay). The system reads the storage context per tick (see D3) and applies the corresponding multiplier.

**Default `CommodityPerishProfile` map (pinned by reassessment):**

```rust
pub fn default_commodity_perish_profile_map() -> BTreeMap<CommodityKind, CommodityPerishProfile> {
    BTreeMap::from([
        (CommodityKind::Apple, CommodityPerishProfile {
            fresh_to_spoiled_ticks: nz(720), // reuses existing decay-map signal
            stale_threshold: Permille::new_unchecked(667),   // Fresh: 1000→667 (≈240 ticks ground)
            spoiled_threshold: Permille::new_unchecked(333), // Stale: 667→333; Spoiled: 333→0
            storage_rates: StorageRateMultipliers {
                ground: Permille::new_unchecked(1000),    // baseline
                container: Permille::new_unchecked(500),  // half rate (doubles shelf life)
                possessed: Permille::new_unchecked(750),  // slightly preserved by carry
            },
        }),
    ])
}
```

`Grain` and `Bread` are intentionally absent (see Non-Goals). The `CommodityDecayMap.Apple` entry continues to exist for compatibility with non-perishable behavior, but for any commodity with a `CommodityPerishProfile` entry the item-decay system's behavior is governed by D3, not by the old `archive_at_duration` path.

### D3. Item-decay system advances condition (instead of archive-only for food)

Extend `item_decay_system` (`crates/worldwake-systems/src/item_decay.rs`): for lots with `PerishableState`, advance `condition` down by the per-commodity elapsed interval × storage-context multiplier divided by `fresh_to_spoiled_ticks`, carrying the division remainder in `decay_remainder`. This preserves deterministic integer arithmetic without quantizing Apple ground spoilage from 720 ticks to 1000 ticks. `last_advanced_tick` tracks the elapsed interval, and perishable lots advance instead of waiting to archive at a single duration.

**Storage-context determination per tick:**

1. **Possessed**: `world.possessor_of(lot)` is an agent → `storage_rates.possessed`.
2. **Container**: `world.direct_container(lot)` has `EntityKind::Container` → `storage_rates.container`.
3. **Ground**: neither possession nor direct-container storage applies → `storage_rates.ground`.

If none of the three matches, the spoil rate defaults to ground baseline (defensive — should not occur for well-spawned lots). No preserving-place option exists in this spec (deferred per Non-Goals).

When `condition` crosses `spoiled_threshold`, the system:
- appends `LotOperation::Spoiled` to the lot's provenance (finally using the variant),
- emits `EventTag::ItemSpoiled` (new variant added to `EventTag` enum at `crates/worldwake-core/src/event_tag.rs:7-57` and to the canonical `ALL` array at `event_tag.rs:69-118` — no exhaustive match sites in the workspace beyond the enum's `ALL` constant, so no migration cost),
- leaves the lot in place (the spoiled lot **persists** — it is not archived on spoilage).

Archival/disposal is a separate downstream choice (eat, compost into `Waste`, or leave as evidence).

### D4. Condition-scaled Eat

Ticket `archive/tickets/S178PERFOOSPO-004.md` lands condition-scaled hunger relief in the Eat path (`crates/worldwake-systems/src/needs_actions.rs`). Hunger relief scales by the lot's `condition` band, following the precedent set by S177 for `WaterToleranceProfile.thirst_relief_factor(quality)` in the Drink path:

- `Fresh` → full `hunger_relief_per_unit`.
- `Stale` → reduced relief (scaled linearly by `condition` between `stale_threshold` and `spoiled_threshold`).
- `Spoiled` → minimal relief (current implementation pins a `Permille(150)` floor; per-commodity tuning is future scope if scenarios warrant it).

The Eat handler reads `PerishableState` directly from the lot at action commit — this is a same-tick co-located read (Eat preconditions require possession or co-location), so authoritative-component access is lawful per FND-14A.

The Eat precondition still **allows** eating spoiled food (it is not blocked) — the choice is governed at candidate generation (D5), not by precondition rejection. This keeps spoiled food a lawful desperation affordance.

### D5. Candidate generation: prefer fresh, desperation-gate spoiled

Candidate generation (`crates/worldwake-ai/src/candidate_generation.rs`, `emit_need_driven_candidates` plus its self-consume evidence helpers) ranks lot-backed self-consume candidates by **believed** lot condition (fresher preferred) and **suppresses spoiled-food food candidates unless** hunger pressure reaches a per-agent profile threshold (`MetabolismProfile.spoiled_food_hunger_threshold` from D7) — the desperation gate.

All reads are belief-view-mediated: candidate generation calls `GoalBeliefView::lot_condition` and `GoalBeliefView::lot_freshness_band` (D8), never directly reads `PerishableState` from authoritative world state. A remembered cache believed fresh may be emitted; on arrival, if observed spoiled, the agent re-ranks (eat spoiled if desperate, else seek fallback).

### D6. Survival forensics for spoiled-cache discovery

Extend `SurvivalForensicExtractor` (`crates/worldwake-ai/src/survival_forensics.rs:278`) with a `SpoiledFoodDiscovery` record:

```rust
pub struct SpoiledFoodDiscovery {
    pub tick: Tick,
    pub lot: EntityId,
    pub believed_condition: Permille,
    pub observed_condition: Permille,
    pub outcome: SpoiledFoodOutcome, // AteAnyway | TraveledToFallback | GaveUp
}
```

The agent reached a believed-edible food lot and found it spoiled. Derived forensic state, never authoritative. Added to `LocalSurvivalStateSummary.spoiled_food_discoveries: Vec<SpoiledFoodDiscovery>` following the `source_acquisition_failures` precedent at `survival_forensics.rs:55`. Feeds the same belief-correction story as S177's `SourceAcquisitionFailure`.

### D7. Profile field

Add `spoiled_food_hunger_threshold: Permille` to the universal `MetabolismProfile` (`crates/worldwake-core/src/needs.rs:140-199`), with a `Default` value extending the existing `Default for MetabolismProfile` impl (lines 283-309) and a scenario override via `AgentDef.metabolism_profile: Option<MetabolismProfile>` (`crates/worldwake-cli/src/scenario/types.rs:645`).

Agents differ in willingness to eat spoiled food (FND-22). No new component, no new `AgentDef` field, no new `spawn_agent` call — the existing universal-profile path (`crates/worldwake-cli/src/scenario/mod.rs:980-981`, `unwrap_or_default()`) carries the new field automatically.

### D8. Belief-view accessors for lot condition

Per the "New Component Read by AI Crate" pattern, expose lot condition to the AI crate through `GoalBeliefView`, not through direct authoritative reads.

**New trait methods on `GoalBeliefView` (`crates/worldwake-sim/src/belief_view.rs`):**

```rust
fn lot_condition(&self, lot: EntityId) -> Option<Permille>;
fn commodity_perish_profile(&self, commodity: CommodityKind) -> Option<CommodityPerishProfile>;
fn lot_freshness_band(&self, lot: EntityId) -> Option<Freshness>;
```

`Freshness` is a derived enum (`Fresh | Stale | Spoiled`) computed by the accessor from `lot_condition` and the commodity's perish profile thresholds. It is not authoritative state — the `Permille` is.

**`RuntimeBeliefView` impl**: routes the accessor through `PerAgentBeliefView` so co-located/possessed reads return the authoritative `PerishableState.condition`, and remote reads consult the agent's belief store.

**`PerAgentBeliefView` integration**: follow the `observed_item_lot_quantity` precedent (`crates/worldwake-sim/src/per_agent_belief_view.rs:525-538`) — when `has_authoritative_local_visibility(lot)` returns true, return `world.get_component_perishable_state(lot).map(|s| s.condition)`; otherwise return the belief-store entry's `last_observed_condition` (added as a new field on the existing lot-belief record, following S177's `ReliabilityRecord.last_observed_quality` precedent).

**`impl_goal_belief_view!` macro forwarding**: extend the macro to forward both new methods to the chosen `BeliefView` so trait blanket impls compile across consumer crates.

Remote-belief reads return `None` rather than reading authoritative remote `PerishableState` — staleness is the point. Beliefs about a remote cache's freshness go stale as the cache spoils unobserved, and that staleness drives the FND-17 expectation-mismatch story when the agent arrives.

## FND-01 Section H — Causal Hooks Declaration

1. **Original missing downstream consequence**: Before S178 landed its first slices, food was a binary timer; it could not age into reduced relief, could not become spoiled-but-edible, and stored caches never perished. A remembered cache could not disappoint. `LotOperation::Spoiled` had no emitter. Tickets `archive/tickets/S178PERFOOSPO-003.md` and `archive/tickets/S178PERFOOSPO-004.md` now cover the decay emitter and condition-scaled Eat relief; belief/cache/candidate/forensic consequences remain in later tickets.
2. **New entities/relations/records**: `PerishableState` component on item lots; `CommodityPerishProfile` + `StorageRateMultipliers`; new `EventTag::ItemSpoiled` variant; first emission of `LotOperation::Spoiled`; `MetabolismProfile.spoiled_food_hunger_threshold`; `SpoiledFoodDiscovery` forensic record; new belief-store field `last_observed_condition` on lot-belief entries.
3. **Actions that mutate them**: Item-decay system advances `condition` and emits `Spoiled` lineage + event. Eat reads `condition`, scales relief. Harvest/production/spawn initialize `PerishableState` at full condition. Disposal/compost transforms a spoiled lot into `Waste` (explicit sink).
4. **Information production and travel**: Lot condition observed when co-located/possessed (FND-14A) via D8 accessors; remote/cache condition belief-backed (FND-14B) — beliefs go stale as the cache spoils unobserved. `ItemSpoiled` append-only in event log.
5. **Conserved quantities**: Spoilage transforms a lot (condition change + optional later compost into `Waste`); quantity is conserved through the transformation — food does not vanish into nothing (FND-4).
6. **Scarce capacities and contention**: *Fresh* food becomes the scarce affordance; existing item-lot ownership/possession governs access. No new queue.
7. **Partial failures and aftermath**: Eat stale → partial relief. Eat spoiled (desperate) → minimal relief. Cache spoils before arrival → `SpoiledFoodDiscovery` + fallback. Spoiled lot left in place → evidence / disposal candidate.
8. **Positive feedback loops**: (a) Over-hoarding → more food than eatable before spoiling → waste, reducing effective supply. (b) Everyone prefers the one fresh source → it depletes while older stock spoils unused.
9. **Concrete dampeners** (physical, not numeric clamps): (a) Spoilage itself is the dampener on hoarding — stockpiles degrade, so hoarding has a real cost (the report's key Don't-Starve lesson). (b) Storage context: containers slow spoilage roughly 2× compared to ground (per D2's `container: 500` multiplier), giving a concrete (scarce, authored) mitigation rather than an infinite-shelf-life cheat. Possession slows it modestly (`750`) — carrying food preserves it somewhat versus leaving it on the ground. (c) Spoiled food still gives minimal relief, so hunger never deadlocks purely from spoilage — it diverts into reduced-relief / fallback, not instant collapse. (d) Eating draws down stock, so fresh stock is consumed before it ages when supply is tight — consumption competes with spoilage.
10. **Agent learning**: None new beyond belief invalidation of a spoiled cache (handled by ordinary belief update — `last_observed_condition` is updated on perception). Per-agent `spoiled_food_hunger_threshold` is a static profile parameter (diversity), not learned state.
11. **How agents can be wrong**: Believe a cache is fresh when it has spoiled (stale belief) → travel wasted, corrected on arrival. Believe own stored food is fresh longer than it is → discover spoilage when next eating.
12. **Lifecycle states**: Lot condition: `Fresh → Stale → Spoiled` (monotonic under decay; no un-spoiling). Lot existence: `Present(condition) → Spoiled(present) → Disposed/Composted(Waste)` or `Consumed`. Visibility/edibility/value are distinct axes (a spoiled lot is visible and edible but low-value).
13. **Temporal resolution**: Condition advanced at the item-decay system tick by `(elapsed_ticks × storage_multiplier + decay_remainder) / fresh_to_spoiled_ticks`, with the modulus stored back into `decay_remainder`. Eat reads condition at action commit. `last_advanced_tick` plus `decay_remainder` make advancement deterministic and idempotent across variable system cadence and across storage-context transitions (carry → drop on ground → pick up again is handled by re-reading possession/container state at each advancement).
14. **Boundary conditions**: Imported food via boundary processes is held by `specs/S62-boundary-processes-remote-shocks.md`; off-map food arrives already carrying a `PerishableState` (initialized at the boundary materialization tick) if perishable.
15. **Derived views**: `Fresh`/`Stale`/`Spoiled` band (`Freshness` enum) is derived from `condition` (the `Permille` is authoritative). `SpoiledFoodDiscovery` forensic (derived). `GoalBeliefView::lot_condition` / `lot_freshness_band` accessors (per-actor derived view, D8).
16. **Causal records**: `LotOperation::Spoiled` in lot provenance; `EventTag::ItemSpoiled` (new variant — added to enum + `ALL` array); `SpoiledFoodDiscovery` in the critical window. Reconstruct "why did this agent eat spoiled food / travel past the spoiled cache?"
17. **Target patterns**: Apple harvested fresh → ages to stale (reduced relief) → spoils (minimal relief, still present); hungry agent eats spoiled only when desperate; cache spoils before a believed-fresh trip; food in a container lasts longer than food on the ground.
18. **Save/load and replay**: New component (`condition` + `last_advanced_tick` + `decay_remainder` makes advancement replay-deterministic), perish profile + storage-rate multipliers, one event-tag variant, first use of an existing lineage variant, one profile field, one forensic record, one belief-store field — all standard replay-deterministic state. `BTreeMap` keys throughout the profile map preserve iteration determinism (AGENTS.md invariant).

## Authoritative-to-AI Impact Analysis

D5 modifies candidate emission (suppresses spoiled-Eat candidates by hunger threshold), triggering AGENTS.md's Authoritative-to-AI Impact Rule. The 7-point checklist:

1. **`get_affordances`** — PASS. Eat affordance still produced for any reachable food lot; precondition still allows spoiled food per D4 (the gate is at candidate emission, not at affordance generation).
2. **`generate_candidates`** — **flag** for implementation. D5 introduces condition-ranked emission and the desperation gate. All reads must route through `GoalBeliefView::lot_condition` / `lot_freshness_band` (D8) — never directly read authoritative `PerishableState` from world state for a remote lot. Verify under golden trace that no candidate-generation call site reads `world.get_component_perishable_state` directly.
3. **`search_plan`** — PASS. Terminal "hunger relieved" semantics unchanged; no new terminal ordering.
4. **`BestEffort` action start** — PASS. Eat still starts on a spoiled food lot when the candidate is lawfully selected (desperate agent). The lot is co-located/possessed at action commit, so authoritative read at start is FND-14A-compliant.
5. **`handle_plan_failure`** — PASS. No new failure modes from Eat itself; existing replan paths cover "cache spoiled, observed on arrival, replan".
6. **Payload revalidation** — **flag** for implementation. Eat payload is affordance-derived (the target lot). Verify that `with_payload_override_validator` is not required for Eat after D5 lands — the candidate's target lot is selected by belief; on arrival the authoritative lot is the same entity (identity preserved per FND-4), so revalidation should pass even when condition differs from belief. If a spoiled-lot Eat fails revalidation for any reason, classify under existing discrepancy paths rather than introducing a new variant.
7. **Golden tests** — PASS. Scenario Validation section provides focused goldens (`survival-food-spoilage-lifecycle`, `survival-food-spoilage-cache`) and the 1440-tick CI-owned collision scenario (`survival-food-spoilage-cache-1440`).

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `PerishableState` (condition + last_advanced_tick + decay_remainder) | Stored authoritative | Component on item lot |
| `CommodityPerishProfile` + `StorageRateMultipliers` | Stored authoritative | World-authored profile (parallel to `CommodityDecayMap`) |
| `LotOperation::Spoiled` lineage entry | Stored authoritative | Lot provenance (append) |
| `EventTag::ItemSpoiled` | Stored event-payload | Authoritative on emission; added to `EventTag` enum + `ALL` array |
| `MetabolismProfile.spoiled_food_hunger_threshold` | Stored authoritative profile parameter | Per-agent |
| Lot-belief `last_observed_condition` field | Stored authoritative | Per-agent belief store (S177 `last_observed_quality` precedent) |
| `Fresh`/`Stale`/`Spoiled` band (`Freshness` enum) | Derived | Computed from `condition` vs profile thresholds |
| `SpoiledFoodDiscovery` records | Derived forensic state | View; not authoritative |
| `GoalBeliefView::lot_condition` / `lot_freshness_band` | Derived per-actor view | View; not authoritative — splits authoritative co-located reads from belief-backed remote reads |

## Planner-formalism analysis

Plain GOAP. Fresher-lot preference is candidate ranking; desperation-eat is a candidate gate keyed on hunger pressure × profile threshold. No HTN method: no multi-stage decomposition, info-gathering stage, or budget-exhaustion need beyond ordinary ranking. Fallback: N/A. Information reads: lot condition is belief-view-mediated via D8 (co-located/possessed authoritative; remote belief-backed); hunger is self state. Enforced declarations only: `PerishableState`, perish profile + storage multipliers, threshold, lineage variant, event tag, belief-store field, and the two belief-view accessors all have live consumers (decay advancer, relief scaler, candidate ranker/gate, forensic recorder). Proof: scenarios below.

## Belief-View Accessor Source-Class Declarations

| Accessor | Source class | Stale/unknown behavior |
|----------|--------------|------------------------|
| `lot_condition(lot) -> Option<Permille>` | FND-14A co-located; direct possession; belief-backed remote via `last_observed_condition` | `None` if no belief entry and not co-located/possessed |
| `lot_freshness_band(lot) -> Option<Freshness>` | Derived from `lot_condition` against the commodity's `CommodityPerishProfile` thresholds | `None` when condition unknown |

Accessors return `None` rather than reading a remote lot's authoritative condition. Who *owns* the lot stays belief-gated per FND-14A (a co-located agent sees the apple's freshness, not who it belongs to).

## Agent Profile Scenario Contract

`MetabolismProfile.spoiled_food_hunger_threshold` is a new field on an existing universal profile with a `Default` extended in `crates/worldwake-core/src/needs.rs:283-309`, scenario-overridable via the existing `AgentDef.metabolism_profile: Option<MetabolismProfile>` field at `crates/worldwake-cli/src/scenario/types.rs:645`. No new component on `EntityKind::Agent`. `PerishableState` is runtime-generated on item lots (initialized at lot creation per D1) and is exercised via scenarios that author perishable commodities; the `CommodityPerishProfile` is world-authored via a scenario contract parallel to how `CommodityDecayMap` is currently authored (an `Option<BTreeMap<CommodityKind, CommodityPerishProfile>>` on `ScenarioDef`, defaulting to `default_commodity_perish_profile_map()` if absent).

## Component Registration

`PerishableState` is registered on the item-lot entity kind in `crates/worldwake-core/src/component_schema.rs` with the kind filter `|kind| kind == EntityKind::ItemLot` (precedent: existing item-lot components at `component_schema.rs:2420-2443`). It is runtime-initialized at lot creation for perishable commodities, so it is exempt from the universal-agent-profile contract (it is lot state, not agent configuration), but its presence is driven by the world-authored `CommodityPerishProfile` (which commodities perish) and runtime storage-context tracking.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Item-decay system (`worldwake-systems`) | `PerishableState`, `world.possessor_of(lot)`, `world.direct_container(lot)`, `CommodityPerishProfile` | `PerishableState.condition`, `PerishableState.last_advanced_tick`, `PerishableState.decay_remainder`, `LotOperation::Spoiled` lineage, `EventTag::ItemSpoiled` |
| Eat handler (`worldwake-systems`) | `PerishableState` (co-located/possessed), consumable profile | `HomeostaticNeeds.hunger`, lot quantity |
| Belief view (`worldwake-sim`) | `PerishableState` (FND-14A co-located/possessed), agent belief store `last_observed_condition` | None (read-only accessor) |
| Perception (`worldwake-systems`) | `PerishableState` (co-located/possessed) | belief store `last_observed_condition` |
| Candidate emitter (`worldwake-ai`) | `GoalBeliefView::lot_condition`, `lot_freshness_band`, hunger, `MetabolismProfile.spoiled_food_hunger_threshold` | None (read-only emission) |
| Survival forensics (`worldwake-ai`) | event/trace log, belief vs observed condition | `SpoiledFoodDiscovery` records |

No system commands another.

## Scenario Validation (FND-31)

**Focused branch goldens:**

- **`survival-food-spoilage-lifecycle.ron`** — a perishable lot ages Fresh → Stale → Spoiled; relief scales down at each band; `LotOperation::Spoiled` and `EventTag::ItemSpoiled` fire; the spoiled lot persists (not archived). Asserts condition arithmetic across the three storage contexts (ground, container, possessed), lineage emission, relief scaling, deterministic replay.
- **`survival-food-spoilage-cache.ron`** — agent remembers a fresh apple cache; the cache spoils unobserved before arrival; on arrival the agent observes spoilage (`SpoiledFoodDiscovery`) and either eats spoiled (if hunger > `spoiled_food_hunger_threshold`) or seeks a fallback. Asserts belief invalidation, the profile-gated desperation branch, the `GoalBeliefView::lot_condition` co-located-vs-belief split, and no omniscient correction.

**1440-tick CI-owned collision scenario:**

- **`survival-food-spoilage-cache-1440.ron`** — multiple agents draw from perishable stock over 1440 ticks under hunger pressure; food ages, spoils, is eaten fresh/stale/spoiled, hoarded-and-wasted, or composted. Assertions prove: exact per-lot condition lineage (`Spoiled` provenance), spoilage-as-hoarding-dampener (over-acquired stock degrades), desperation-eat only above threshold, container vs. ground vs. possessed rate differentiation, and replay equivalence.

**Illegal paths this spec must not produce:** food relief unaffected by condition; a spoiled lot vanishing instead of transforming; eating spoiled food producing a wound/sickness (deferred); a planner candidate for a remote lot's freshness with no belief carrier; a global `food_freshness` aggregate; candidate-generation reading `world.get_component_perishable_state` directly for a remote lot (must route through `GoalBeliefView`).
