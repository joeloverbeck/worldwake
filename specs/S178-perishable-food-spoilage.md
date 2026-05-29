# S178: Perishable Food Spoilage and Lot Condition

## Summary

`LotOperation::Spoiled` exists in the item-lot lineage enum (`crates/worldwake-core/src/items.rs`) but is **never constructed anywhere** — it is a schema variant with no emitter. This is a strong, deliberate hint left by `archive/specs/S82-waste-disposal-inventory-management.md` / the item-lot lineage design: spoilage was always intended to be richer than the current behavior. Today:

- `CommodityDecayMap` (`items.rs`) maps a commodity to a single decay duration (Apple → 720 ticks, Waste → 200), and `item_decay_system` (`crates/worldwake-systems/src/item_decay.rs`) **archives ground item lots** once that duration elapses.
- Decay is therefore binary and location-limited: a lot exists at full value until it is removed (archived) — there is no per-lot freshness, no reduced relief from aging food, and stored/cached food (not on the ground) does not perish at all.
- `ItemLot` carries only `commodity`, `quantity`, `provenance`. There is no condition/freshness field.

The result: food is a binary timer (fresh until it vanishes), which is exactly the "stored resources disappear as invisible timer failure" anti-pattern the source report identifies. This spec adds **per-lot `PerishableState`** so perishable food degrades through `Fresh → Stale → Spoiled` with concrete consequences — reduced relief from stale food, a spoiled-but-still-existing lot (finally emitting `LotOperation::Spoiled`), and a profile-driven desperation choice to eat spoiled food. It is the third slice of the deferred Cluster 1 material-degradation wave (`specs/IMPLEMENTATION-ORDER.md`).

**Critical reassessment of the source report:** the report floats a later `FoodSickness`/`DigestiveDistress` consequence for eating spoiled food. This spec **defers that disease consequence** (FND-5 / the report's own MUST-NOT — no disease ecology until it is a proven concrete carrier with recovery and proof). Spoilage here changes *affordance value and lineage*, not health. Spoiled food gives minimal relief; it does not (yet) wound.

## Phase

Phase 7: Consequence Carriers

## Status

📝 DRAFT — authored, awaiting activation (held adjunct wave; see `specs/IMPLEMENTATION-ORDER.md`)

## Crates

- `worldwake-core` (`PerishableState` component on item lots; per-commodity perish profile; emit `LotOperation::Spoiled`)
- `worldwake-sim` (event payloads for condition change / spoilage; reuse lineage substrate)
- `worldwake-systems` (item-decay system advances `PerishableState` over time by storage context, instead of only archiving; Eat relief scales by condition)
- `worldwake-ai` (candidate generation prefers fresher lots; eats spoiled food only under profile-permitted desperation; survival forensics record spoiled-cache discovery)
- `worldwake-cli` (scenario contract for `PerishableState` and perish profile; player-POV gating for lot condition observation)

## Dependencies

- `archive/specs/S82-waste-disposal-inventory-management.md` — provides `ItemLot`, `CommodityKind::Waste`, the lineage `LotOperation` enum (incl. the unused `Spoiled` variant), and the `CommodityDecayMap` substrate this spec extends.
- `archive/specs/S79-resource-source-consumption-affordances.md` — provides the consumable-profile / Eat-Drink action substrate this spec scales by condition.
- `archive/specs/S129-place-dirtiness-facility-wear.md` — precedent for a per-entity degradation component advanced by the item-decay system.
- `archive/specs/S120-survival-critical-window-forensics.md` — `SurvivalForensicExtractor` extended with spoiled-cache-discovery records.

## Design Goals

- Perishable food lots carry concrete `PerishableState` (a `Permille` condition + derived `Fresh`/`Stale`/`Spoiled` band by per-commodity thresholds). Condition degrades over time, faster or slower by **storage context** (exposed on the ground vs. in a container vs. a cold/preserving place).
- Eating **fresh** food gives full relief; **stale** gives reduced relief (scaled by condition); **spoiled** gives minimal relief and is normally avoided.
- A spoiled lot **still exists** — it transforms (emits `LotOperation::Spoiled`, preserving provenance/derivation lineage per FND-4) rather than vanishing. It remains an affordance (desperation food) or a candidate for disposal/composting into `Waste`.
- Eating spoiled food is a **lawful, profile-gated desperation choice**: a starving agent with high risk tolerance may eat spoiled food for minimal relief; a well-fed or cautious agent will not. No script — emergent from hunger pressure × profile.
- Stored/cached food perishes too (not just ground lots), so a remembered cache can spoil before the agent arrives — feeding the belief/fallback loop (couples to S177's reliability theme and canonical scenario D).
- Player/AI symmetric; the CLI surfaces only lot condition the controlled agent lawfully perceives.

## Non-Goals

- **No food-borne sickness, disease, digestive distress, or wound from spoiled food.** Deferred (see Summary; FND-5). Spoilage is a value/lineage axis only.
- **No cooking, preservation crafting, recipes, or nutrition model.** A "preserve food" or "cook" action is a future-spec trigger if scenarios prove storage context alone is insufficient. Storage context here is *place/container property*, not a crafting action.
- **No new global food-supply score.** Per-lot concrete state only (FND-3).
- **No replacement of `CommodityDecayMap`'s Waste handling.** Waste continues to decay/archive as today; this spec governs *perishable food* lots specifically.
- **No HTN method.** Fresher-lot preference and desperation-eat are flat GOAP candidate ranking/gating.
- **No backward-compatibility shim.** The "archive at decay duration" path for perishable food is replaced by condition advancement → spoilage → optional disposal; goldens depending on the old archive-only behavior are updated (FND-28). Non-perishable commodities are unaffected.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | A remembered apple cache spoils before the hungry agent arrives → eat spoiled for scraps, seek a fallback, or trade → drama emerges from time + storage, not an authored "food rots now" trigger |
| FND-3 (Concrete state over abstract scores) | Condition is a per-lot `Permille`; bands are concrete thresholds; no `freshness_score` aggregate |
| FND-4 (Persistent identity / derivation lineage) | A spoiling lot keeps its identity; spoilage emits `LotOperation::Spoiled` preserving provenance — "this spoiled lot came from that harvest" |
| FND-5 (Carriers of consequence) | Spoilage propagates: reduced relief, desperation affordance, disposal-into-Waste, belief invalidation of a cache — real downstream effects, not decorative realism |
| FND-7 / FND-14A / FND-14B | Lot condition observed when co-located/possessed; remote cache condition is belief-backed; the planner cannot read a remote lot's freshness as authoritative |
| FND-10 (Aftermath) | Spoilage is granular: full → reduced → minimal relief, then a still-present spoiled lot; not a boolean disappearance |
| FND-11 (Positive feedback) | Hoarding more food than can be eaten before spoiling → waste; see Section H dampeners |
| FND-16 / FND-17 (Ignorance / violated expectation) | Believing a cache is fresh and finding it spoiled is the expectation violation that updates belief |
| FND-19 (Agent symmetry) | Same condition-scaled relief and desperation gating for human and AI |
| FND-22 (Agent diversity) | Risk tolerance / hunger threshold for eating spoiled food is a per-agent profile parameter — agents differ |
| FND-26 (Systems via state) | Item-decay system advances condition; Eat reads it; planner reads belief; no direct calls |
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
}

impl Component for PerishableState {}
```

Attached to item lots of perishable commodities at creation (harvest/production/spawn). Non-perishable commodities carry no `PerishableState`.

### D2. Per-commodity perish profile

A `CommodityPerishProfile` (parallel to `CommodityDecayMap`, world-authored): per perishable commodity, the ticks-to-traverse the full condition range and the `Permille` thresholds dividing `Fresh`/`Stale`/`Spoiled`, plus storage-context multipliers:

```rust
pub struct CommodityPerishProfile {
    pub fresh_to_spoiled_ticks: NonZeroU32,
    pub stale_threshold: Permille,   // condition below this → Stale
    pub spoiled_threshold: Permille, // condition below this → Spoiled
}
// Storage-context multipliers (ground / container / preserving place)
// expressed as Permille rate factors — no inline magic numbers.
```

Default map covers `Apple` (reusing the existing 720-tick signal as the ground baseline). Reassessment pins exact defaults against current item-decay goldens.

### D3. Item-decay system advances condition (instead of archive-only for food)

Extend `item_decay_system` (`item_decay.rs`): for lots with `PerishableState`, advance `condition` down by the per-commodity rate × storage-context multiplier each elapsed interval, instead of waiting to archive at a single duration. When `condition` crosses `spoiled_threshold`, emit `LotOperation::Spoiled` into the lot's provenance (finally using the variant) and emit `EventTag::ItemSpoiled`. The spoiled lot **persists** (it is not archived on spoilage); archival/disposal is a separate downstream choice (eat, compost into `Waste`, or leave as evidence). Storage context is read from the lot's location (ground place vs. container vs. a place tagged preserving — reassessment pins the storage-context source).

### D4. Condition-scaled Eat

In the Eat path (`needs_actions.rs`), hunger relief scales by the lot's `condition` band:

- `Fresh` → full `hunger_relief_per_unit`.
- `Stale` → reduced relief (scaled linearly by `condition` between thresholds).
- `Spoiled` → minimal relief (a small `Permille` floor from the profile).

The Eat precondition still **allows** eating spoiled food (it is not blocked) — the choice is governed at candidate generation (D5), not by precondition rejection. This keeps spoiled food a lawful desperation affordance.

### D5. Candidate generation: prefer fresh, desperation-gate spoiled

Candidate generation (`worldwake-ai`) ranks Eat candidates by believed lot condition (fresher preferred) and **suppresses spoiled-food Eat candidates unless** hunger pressure exceeds a per-agent profile threshold (`spoiled_food_hunger_threshold`) — the desperation gate. All reads are belief-backed / same-tick-local lot condition. A remembered cache believed fresh may be emitted; on arrival, if observed spoiled, the agent re-ranks (eat spoiled if desperate, else seek fallback).

### D6. Survival forensics for spoiled-cache discovery

Extend `SurvivalForensicExtractor` with a `SpoiledFoodDiscovery` record: the agent reached a believed-edible food lot and found it spoiled. Derived forensic state, never authoritative. Feeds the same belief-correction story as S177's `SourceAcquisitionFailure`.

### D7. Profile field

Add `spoiled_food_hunger_threshold: Permille` to a universal agent profile (`MetabolismProfile`), with a `Default` and scenario override via `AgentDef`. Agents differ in willingness to eat spoiled food (FND-22).

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Food is a binary timer; it cannot age into reduced relief, cannot become spoiled-but-edible, and stored caches never perish. A remembered cache cannot disappoint. `LotOperation::Spoiled` has no emitter.
2. **New entities/relations/records**: `PerishableState` component on item lots; `CommodityPerishProfile` + storage-context multipliers; `EventTag::ItemSpoiled`; first emission of `LotOperation::Spoiled`; `MetabolismProfile.spoiled_food_hunger_threshold`; `SpoiledFoodDiscovery` forensic record.
3. **Actions that mutate them**: Item-decay system advances `condition`, emits `Spoiled` lineage + event. Eat reads `condition`, scales relief. Harvest/production/spawn initialize `PerishableState`. Disposal/compost transforms a spoiled lot into `Waste` (explicit sink).
4. **Information production and travel**: Lot condition observed when co-located/possessed (FND-14A); remote/cache condition belief-backed (FND-14B) — beliefs go stale as the cache spoils unobserved. `ItemSpoiled` append-only.
5. **Conserved quantities**: Spoilage transforms a lot (condition change + optional later compost into `Waste`); quantity is conserved through the transformation — food does not vanish into nothing (FND-4).
6. **Scarce capacities and contention**: *Fresh* food becomes the scarce affordance; existing item-lot ownership/possession governs access. No new queue.
7. **Partial failures and aftermath**: Eat stale → partial relief. Eat spoiled (desperate) → minimal relief. Cache spoils before arrival → `SpoiledFoodDiscovery` + fallback. Spoiled lot left in place → evidence / disposal candidate.
8. **Positive feedback loops**: (a) Over-hoarding → more food than eatable before spoiling → waste, reducing effective supply. (b) Everyone prefers the one fresh source → it depletes while older stock spoils unused.
9. **Concrete dampeners** (physical, not numeric clamps): (a) Spoilage itself is the dampener on hoarding — stockpiles degrade, so hoarding has a real cost (the report's key Don't-Starve lesson). (b) Storage context: a preserving place slows spoilage, giving a concrete (scarce, authored) mitigation rather than an infinite-shelf-life cheat. (c) Spoiled food still gives minimal relief, so hunger never deadlocks purely from spoilage — it diverts into reduced-relief / fallback, not instant collapse. (d) Eating draws down stock, so fresh stock is consumed before it ages when supply is tight — consumption competes with spoilage.
10. **Agent learning**: None new beyond belief invalidation of a spoiled cache (handled by ordinary belief update). Per-agent `spoiled_food_hunger_threshold` is a static profile parameter (diversity), not learned state.
11. **How agents can be wrong**: Believe a cache is fresh when it has spoiled (stale belief) → travel wasted, corrected on arrival. Believe own stored food is fresh longer than it is → discover spoilage when next eating.
12. **Lifecycle states**: Lot condition: `Fresh → Stale → Spoiled` (monotonic under decay; no un-spoiling). Lot existence: `Present(condition) → Spoiled(present) → Disposed/Composted(Waste)` or `Consumed`. Visibility/edibility/value are distinct axes (a spoiled lot is visible and edible but low-value).
13. **Temporal resolution**: Condition advanced at the item-decay system tick by elapsed-interval × rate. Eat reads condition at action commit. `last_advanced_tick` makes advancement deterministic and idempotent across variable system cadence.
14. **Boundary conditions**: N/A — lots are local. (Imported food via boundary processes is held `specs/S62`; off-map food arrives already carrying a `PerishableState` if perishable.)
15. **Derived views**: `Fresh`/`Stale`/`Spoiled` band is derived from `condition` (the `Permille` is authoritative). `SpoiledFoodDiscovery` forensic (derived). Belief-view lot-condition accessor (per-actor derived).
16. **Causal records**: `LotOperation::Spoiled` in lot provenance; `EventTag::ItemSpoiled`; `SpoiledFoodDiscovery` in the critical window. Reconstruct "why did this agent eat spoiled food / travel past the spoiled cache?"
17. **Target patterns**: Apple harvested fresh → ages to stale (reduced relief) → spoils (minimal relief, still present); hungry agent eats spoiled only when desperate; cache spoils before a believed-fresh trip; preserving place keeps a lot fresh longer than the ground.
18. **Save/load and replay**: New component (`condition` + `last_advanced_tick` makes advancement replay-deterministic), perish profile, one event variant, first use of an existing lineage variant, one profile field, one forensic record — all standard replay-deterministic state.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `PerishableState` (condition + last_advanced_tick) | Stored authoritative | Component on item lot |
| `CommodityPerishProfile` + storage multipliers | Stored authoritative | World-authored profile (parallel to `CommodityDecayMap`) |
| `LotOperation::Spoiled` lineage entry | Stored authoritative | Lot provenance (append) |
| `EventTag::ItemSpoiled` | Stored event-payload | Authoritative on emission |
| `MetabolismProfile.spoiled_food_hunger_threshold` | Stored authoritative profile parameter | Per-agent |
| `Fresh`/`Stale`/`Spoiled` band | Derived | Computed from `condition` vs profile thresholds |
| `SpoiledFoodDiscovery` records | Derived forensic state | View; not authoritative |
| Belief-view lot-condition accessor | Derived per-actor view | View; not authoritative |

## Planner-formalism analysis

Plain GOAP. Fresher-lot preference is candidate ranking; desperation-eat is a candidate gate keyed on hunger pressure × profile threshold. No HTN method: no multi-stage decomposition, info-gathering stage, or budget-exhaustion need beyond ordinary ranking. Fallback: N/A. Information reads: lot condition is belief-backed or same-tick-local/possessed; hunger is self state. Enforced declarations only: `PerishableState`, perish profile, threshold, lineage variant, and event all have live consumers (decay advancer, relief scaler, candidate ranker/gate, forensic recorder). Proof: scenarios below.

## Belief-View Accessor Source-Class Declarations

| Accessor | Source class | Stale/unknown behavior |
|----------|--------------|------------------------|
| `lot_condition(lot) -> Option<Permille>` | FND-14A co-located; direct possession; belief-backed remote | `None` if no belief and not co-located/possessed |
| `lot_freshness_band(lot) -> Option<Freshness>` | Derived from `lot_condition` | `None` when condition unknown |

Accessors return `None` rather than reading a remote lot's authoritative condition. Who *owns* the lot stays belief-gated per FND-14A (a co-located agent sees the apple's freshness, not who it belongs to).

## Agent Profile Scenario Contract

`MetabolismProfile.spoiled_food_hunger_threshold` is a new field on an existing universal profile with a `Default`, scenario-overridable via `AgentDef`. No new component on `EntityKind::Agent`. `PerishableState` is runtime-generated on item lots (initialized at lot creation, like other lot state) and is exercised via scenarios that author perishable commodities and storage contexts; the `CommodityPerishProfile` is world-authored via the scenario's commodity-profile contract (parallel to how `CommodityDecayMap` is authored).

## Component Registration

`PerishableState` is registered on the item-lot entity kind in `crates/worldwake-core/src/component_schema.rs` (precedent: existing item-lot components). It is runtime-initialized at lot creation for perishable commodities, so it is exempt from the universal-agent-profile contract (it is lot state, not agent configuration), but its presence is driven by the world-authored `CommodityPerishProfile` (which commodities perish) and scenario-authored storage contexts.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Item-decay system (`worldwake-systems`) | `PerishableState`, lot location/storage context, `CommodityPerishProfile` | `PerishableState.condition`, `LotOperation::Spoiled`, `ItemSpoiled` event |
| Eat handler (`worldwake-systems`) | `PerishableState`, consumable profile | `HomeostaticNeeds.hunger`, lot quantity |
| Candidate emitter (`worldwake-ai`) | belief-view lot condition, hunger, profile threshold | None (read-only emission) |
| Survival forensics (`worldwake-ai`) | event/trace log, belief vs observed condition | `SpoiledFoodDiscovery` records |

No system commands another.

## Scenario Validation (FND-31)

**Focused branch goldens:**

- **`survival-food-spoilage-lifecycle.ron`** — a perishable lot ages Fresh → Stale → Spoiled; relief scales down at each band; `LotOperation::Spoiled` and `EventTag::ItemSpoiled` fire; the spoiled lot persists (not archived). Asserts condition arithmetic, lineage emission, relief scaling, deterministic replay.
- **`survival-food-spoilage-cache.ron`** — agent remembers a fresh apple cache; the cache spoils unobserved before arrival; on arrival the agent observes spoilage (`SpoiledFoodDiscovery`) and either eats spoiled (if hunger > profile threshold) or seeks a fallback. Asserts belief invalidation, the profile-gated desperation branch, and no omniscient correction.

**1440-tick CI-owned collision scenario:**

- **`survival-food-spoilage-cache-1440.ron`** — multiple agents draw from perishable stock over 1440 ticks under hunger pressure; food ages, spoils, is eaten fresh/stale/spoiled, hoarded-and-wasted, or composted. Assertions prove: exact per-lot condition lineage (`Spoiled` provenance), spoilage-as-hoarding-dampener (over-acquired stock degrades), desperation-eat only above threshold, and replay equivalence.

**Illegal paths this spec must not produce:** food relief unaffected by condition; a spoiled lot vanishing instead of transforming; eating spoiled food producing a wound/sickness (deferred); a planner candidate for a remote lot's freshness with no belief carrier; a global `food_freshness` aggregate; `LotOperation::Spoiled` still unused.
