# S129: Place Dirtiness and Facility Wear

## Summary

Make hygiene a property of the place and the facility, not just the agent. Today, `relieve_wilderness` creates a `Waste` `ItemLot` at the place and increases the actor's `dirtiness` (modulated by `MetabolismProfile.wilderness_relief_dirtiness_penalty`). The wash action consumes water from the well's `ResourceSource` and zeros the actor's dirtiness — but the `WashBasin` carries no per-basin state, the latrine carries no fullness state, and the place itself accumulates no dirtiness from repeated use. The narrative report shows Agent C relieving in the wilderness 26 times at Fertile Fields without any place-side consequence: "the bad sign is that those Waste lots do not seem to matter much downstream." This spec adds three concrete state carriers — `PlaceDirtiness` per place, `LatrineFullness` per latrine facility, `WashBasinState` per washbasin facility — so that wilderness relief makes places dirty (which then biases sleep quality and wash demand), latrines fill up (which biases agents toward wilderness or to clean latrines), and washbasin capacity bounds wash success (folding in PR-11's partial-wash). Folds in PR-9's hygiene-topology authoring (no new place kinds; existing tags become richer) and PR-12's waste/wash event tags.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` components; `WasteCreated`, `WashFacilityUsed`, `LatrineMaintained` event tags with payload structs per S110 conventions.
- `worldwake-systems` — `relieve_wilderness` writes `PlaceDirtiness` increment; `toilet` writes `LatrineFullness` increment + `PlaceDirtiness` if latrine over capacity; `wash` reads `WashBasinState.clean_water_units` and writes the basin dirtiness increment + partial-success outcome when capacity exhausted; place dirtiness decay piggybacks on the existing item-decay maintenance pass.
- `worldwake-ai` — sleep candidate ranking reads `PlaceDirtiness` as a downward modifier on sleep quality; wash candidate ranking biases toward basins with higher `clean_water_units` and lower `dirtiness_level`; relieve candidate ranking biases toward latrines below `LatrineFullness::critical_threshold`.
- `worldwake-cli` — `WashBasinDef`, `LatrineDef` extensions for the new fields; defaults preserve existing scenario behavior.

## Dependencies

- S106 (Ground Item Decay) — **completed**. Place dirtiness decay reuses the existing maintenance-pass cadence (per-tick check, threshold-based decrement).
- S128 (Sleep Episodes and Place-Quality Recovery) — **soft, satisfied**. Sleep ranking reads `PlaceDirtiness` as a multiplicative factor on `SleepQualityProfile.recovery_modifier`; S128 is completed and archived at `archive/specs/S128-sleep-episode-place-quality.md`.
- S110 (Decision History Events) — **hard**. The three new event tags follow the S110 payload-struct conventions.
- S82 (Waste Disposal and Inventory Management) — **completed**. `Waste` as `ItemLot` plus `drop_item` already exists; place-side dirtiness is the missing per-place aggregate.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 4: "Relieve and Wash are currently functional but thin … Agent C relieved in the wilderness twenty-six times and never used a toilet because she spent most of the run at Fertile Fields, which lacks a Latrine tag. That is causally legible, but not yet rich." The narrative report confirms: Agent C dirtied 26 wilderness Waste lots at Fertile Fields, but the place itself never developed any property that would push agents toward the latrine at Riverside Camp. The depth fix the assessment asks for is "stay near food but get dirtier from wilderness relief, travel to a latrine but spend time and expose other needs" — which requires the place to carry concrete dirtiness that other agents perceive and weigh.

PR-9's topology depth ask resolves naturally if `PlaceDirtiness` accumulates: Fertile Fields becomes hygiene-poor under sustained use, biasing sleep elsewhere; Riverside Camp's three-affordance hub (water + wash + latrine) becomes the obvious recovery base; Hillside Shelter's latrine-only configuration becomes a low-contention alternative when Riverside Camp is crowded.

## Design Goals

1. `PlaceDirtiness` is per-place state, not per-agent. It accumulates from wilderness relief at the place, decays on a slow schedule (similar to evidence decay), and is observable by co-located agents through FND-14A.
2. `LatrineFullness` is per-facility state. It increments per `toilet` use; when over `critical_threshold`, the facility becomes "full" and `toilet` requires `clean_latrine` (a future maintenance action — out of scope) or `relieve_wilderness` becomes the alternative even at a latrine-tagged place.
3. `WashBasinState` is per-facility state. `clean_water_units` is consumable; `dirtiness_level` accumulates; `wash` partial success when `clean_water_units < required` (PR-11 fold-in for wash).
4. No new place kinds. PR-9 is satisfied by varying `PlaceDirtiness` accumulation rate per place tag and authoring per-place defaults — Fertile Fields' open layout accumulates faster than Hillside Shelter's enclosed shelter.
5. Sleep quality (S128) is modulated by `PlaceDirtiness`. A dirty place sleeps worse — concrete consequence chain: wilderness relief → place dirtier → sleep worse there → agent travels to clean shelter → tradeoff against hunger/thirst access.
6. Three new event tags. Per FND-30 each spec declares its causal records. PR-12's standalone "waste/wash event tag" proposal folds here, not as a separate spec.

## Non-Goals

- Disease propagation. The assessment is explicit: "No disease system required yet."
- Latrine maintenance via a `clean_latrine` action. Requires authoritative custodian/operator identity; deferred until a settlement-staff role spec exists.
- Crowding effects on sleep quality. Folded out of S128 for the same reason; if `PlaceDirtiness` proxies for "many people are using this place," the crowding ask is partially addressed without a dedicated `crowding_count` field.
- Privacy / social-judgment effects of wilderness relief. The assessment excludes social systems.
- Per-agent dirtiness tolerance / cleanliness preference. The existing `MetabolismProfile.wilderness_relief_dirtiness_penalty` already encodes per-agent variation; no new field needed.
- Composting / waste-to-fertilizer conversion. Out of scope.
- Wash basin refill via authored worker action. Refill happens through a slow per-tick natural-recovery process (clean water sourced from the same `ResourceSource` topology when adjacent — see D6).

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `PlaceDirtiness.value: Permille` is concrete state, not a derived "hygiene score." `LatrineFullness.fill: Permille` and `WashBasinState.{clean_water_units, dirtiness_level}` are concrete fields. |
| FND-5 (Carriers of Consequence) | Each new component carries downstream consequences: dirty place → bad sleep → travel decision; full latrine → wilderness-relief fallback at latrine-tagged place; empty basin → wash partial success → agent stays dirty longer. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All state is per-place or per-facility, observable only by co-located agents through perception. No global "all places are dirty" query. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | `wash` now has the additional precondition that `WashBasinState.clean_water_units > 0`; falling below that produces partial outcome instead of failing the start. `toilet` at a full latrine still succeeds (creating waste at the place) but with degraded outcome. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | `wash` partial-success when basin water insufficient: agent dirtiness reduced proportionally, basin clean water consumed, basin dirtiness incremented. `toilet` at full latrine: bladder zeroed but place dirtiness incremented. Failure is new state. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | "Dirty place → agents avoid for sleep → fewer agents → less wilderness relief → place cleans up." Dampener: place dirtiness decay over time (recovery_ticks per unit), plus the natural negative feedback that wash demand co-locates agents at clean basins. |
| FND-14 / FND-14A | Co-located agents perceive `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` directly (FND-14A — physical properties of the place/facility). Off-place propagation through `ShareBelief`. |
| FND-22 (Agent Diversity Through Concrete Variation) | Different `MetabolismProfile.wilderness_relief_dirtiness_penalty` produces different per-agent dirtiness contribution per relief. Agents already differ in `dirtiness_weight` for ranking — that variation feeds wash-vs-other-need tradeoff. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Agents who repeatedly observe Fertile Fields as dirty (perception-time `PlaceDirtiness > X`) can update their `SourceReliability` for nearby resources accordingly when S131 lands; for now, the per-perception belief is sufficient. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Relieve writes `PlaceDirtiness`; sleep tick reads it; ranking reads beliefs about it. No imperative cross-call. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `PlaceDirtiness.value` is authoritative; the per-tick decay is a maintenance-pass mutation, not a derived view. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The existing per-agent dirtiness pathway is preserved (it's correct — agents do still get dirty); the new place-side dirtiness is additive, not a replacement. No shim or compatibility layer. |
| FND-29 (Debuggability Is a Product Feature) | New event tags surface the causal chain. "Why did the agent travel to Hillside Shelter at tick 612?" becomes answerable from `PlaceDirtiness` rising at Fertile Fields plus the agent's sleep candidate ranking output. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Three new event tags land in the existing append-only event log. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers all 18 declarations. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Place/facility state is co-located perception (FND-14A). Off-place propagation: explicit `ShareBelief` for "I heard the well basin at Forest Clearing is empty." No global hygiene registry.
2. **Positive-feedback analysis.** (a) "More wilderness relief → place dirtier → agents leave for clean places → fewer relief events at the dirty place → place cleans up." Self-balancing through use-distribution. (b) "More wash use → basin dirtier → fewer wash successes → agents seek other basins → first basin recovers naturally." Self-balancing through capacity.
3. **Concrete dampeners.** (a) `PlaceDirtiness` decay rate (per-tick decrement during maintenance pass). (b) `WashBasinState` natural refill (per-tick `clean_water_units` increment up to `max_clean_water` when the basin is co-located with a `ResourceSource` of `Water` — concrete supply chain). (c) `LatrineFullness` does *not* decay automatically without a maintenance action — full latrines stay full, providing the negative feedback for the system.
4. **Stored state vs. derived read-model.** Stored: `PlaceDirtiness.value`, `LatrineFullness.fill`, `WashBasinState.{clean_water_units, dirtiness_level}`. Derived: per-tick recovery rate computed from authoritative state.

## Deliverables

### D1: `PlaceDirtiness` component

In `crates/worldwake-core/src/place_dirtiness.rs` (new module):

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaceDirtiness {
    /// Current accumulated dirtiness as Permille [0, 1000].
    /// 0 = pristine, 1000 = severely dirty.
    pub value: Permille,
    /// Per-tick decay applied during the existing item-decay
    /// maintenance pass. Authoritative per-place via component
    /// instance, not a hidden constant.
    pub decay_per_tick: Permille,
}

impl Component for PlaceDirtiness {}
```

`decay_per_tick = Permille::new_unchecked(2)` is the universal default — a fully dirty place returns to clean over ~500 ticks of disuse.

### D2: `LatrineFullness` component

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LatrineFullness {
    /// Current fill level as Permille [0, 1000].
    /// 0 = empty, 1000 = full and unusable.
    pub fill: Permille,
    /// Per-toilet-use increment.
    pub fill_per_use: Permille,
    /// Threshold beyond which the latrine is "critically full" —
    /// `toilet` still succeeds but degrades to wilderness-relief
    /// outcomes (creates Waste lot at place, increments PlaceDirtiness).
    pub critical_threshold: Permille,
}

impl Component for LatrineFullness {}
```

Defaults: `fill_per_use = 80`, `critical_threshold = 800` — a fresh latrine handles ~10 uses before degrading.

### D3: `WashBasinState` component

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WashBasinState {
    /// Available clean water for wash, in abstract units. One full
    /// wash consumes `units_per_full_wash` units; partial wash
    /// consumes proportional fraction.
    pub clean_water_units: u16,
    pub max_clean_water: u16,
    /// Per-tick refill when basin is co-located with a Water
    /// `ResourceSource` (the basin's place must hold the source).
    pub refill_per_tick: u16,
    pub units_per_full_wash: u16,
    /// Accumulated basin dirtiness from use; reduces wash effectiveness.
    pub dirtiness_level: Permille,
    pub dirtiness_per_use: Permille,
}

impl Component for WashBasinState {}
```

Defaults: `max_clean_water = 10`, `refill_per_tick = 1` (when adjacent water source), `units_per_full_wash = 2`, `dirtiness_per_use = 50`.

### D4: New `EventTag` variants

In `crates/worldwake-core/src/event_tag.rs`:

```rust
pub enum EventTag {
    // ... existing variants ...
    WasteCreated,
    WashFacilityUsed,
    LatrineMaintained, // reserved for future maintenance action
}
```

Payload structs in `crates/worldwake-core/src/decision_event_payload.rs`:

```rust
pub struct WasteCreatedPayload {
    pub creator: EntityId,
    pub place: EntityId,
    pub waste_lot: EntityId,
    pub source: WasteSource,
    pub place_dirtiness_delta: Permille,
}

pub enum WasteSource {
    WildernessRelief,
    OvercapacityLatrine { latrine: EntityId },
}

pub struct WashFacilityUsedPayload {
    pub user: EntityId,
    pub basin: EntityId,
    pub water_consumed: u16,
    pub agent_dirtiness_delta: Permille,
    pub basin_dirtiness_delta: Permille,
    pub partial: bool,
}
```

### D5: `relieve_wilderness` handler extension

In `crates/worldwake-systems/src/needs_actions.rs`, the `relieve_wilderness` commit handler:

- Existing behavior: zero bladder, create Waste `ItemLot` at place, increment agent dirtiness.
- New: read or create `PlaceDirtiness` on the actor's place, increment by `MetabolismProfile.wilderness_relief_dirtiness_penalty` (the per-agent quantity already authored). Cap at `Permille::FULL`.
- Emit `WasteCreated` with `WasteSource::WildernessRelief`.

### D6: `toilet` handler extension

In the same module, `toilet` commit handler:

- Existing behavior: zero bladder, create Waste at the latrine.
- New: read `LatrineFullness` on the latrine facility. If `fill < critical_threshold`: increment `fill` by `fill_per_use`. Emit `WasteCreated` with `WasteSource::OvercapacityLatrine` if `fill >= critical_threshold` after increment, and also increment `PlaceDirtiness` on the facility's place (the latrine has overflowed).

### D7: `wash` handler extension

In the same module, `wash` commit handler:

- Existing behavior: zero agent dirtiness, consume one unit from the well's `ResourceSource`.
- New: read `WashBasinState` on the basin. If `clean_water_units >= units_per_full_wash`: full success — consume `units_per_full_wash`, reduce agent dirtiness fully, increment `dirtiness_level` by `dirtiness_per_use`.
- If `clean_water_units < units_per_full_wash` and `clean_water_units > 0`: partial success — consume all available, reduce agent dirtiness proportionally (`available / units_per_full_wash` of the full reduction), increment `dirtiness_level` proportionally.
- If `clean_water_units == 0`: fail with `ActionError::PreconditionFailed("basin has no clean water")`. Existing well-water-consumption path is removed (basin water becomes the authoritative source); the basin's natural refill (D8) consumes from the well.
- Emit `WashFacilityUsed` with `partial` flag.

### D8: Basin natural refill maintenance

Add a `wash_basin_refill` step inside the existing item-decay maintenance pass (or a separate maintenance pass if it doesn't fit). Per-tick: for each entity with `WashBasinState`:

- If `clean_water_units < max_clean_water` and the basin's place hosts a Water `ResourceSource` with `available_quantity > 0`: increment `clean_water_units` by `min(refill_per_tick, available - clean_water_units)` and decrement the source's `available_quantity` accordingly.
- This is the concrete supply chain — basin water doesn't appear from nowhere; it draws from the same `ResourceSource` agents harvest from.

### D9: `PlaceDirtiness` decay maintenance

In the same maintenance pass: for each `PlaceDirtiness`, decrement `value` by `decay_per_tick` (saturating at zero).

### D10: AI ranking integration

In `crates/worldwake-ai/src/ranking.rs`:

- `ExploreLocation` and `Sleep` candidate ranking: when reading the candidate place, multiply the existing place-quality input by `(1000 - PlaceDirtiness.value) / 1000`. A pristine place keeps full quality; a fully-dirty place halves.
- `Wash` candidate ranking: when multiple basins are believed, prefer the one with higher `clean_water_units` and lower `dirtiness_level`. Existing ranking already considers facility presence; this adds the second-order axis.
- `Relieve` candidate ranking: prefer a latrine with `fill < critical_threshold` over wilderness; if all known latrines are critical, fall through to wilderness.

### D11: Scenario authoring

In `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct PlaceDef {
    // ... existing fields ...
    #[serde(default)]
    pub place_dirtiness: Option<PlaceDirtinessDef>,
}

pub struct WashBasinDef {
    // ... existing fields ...
    #[serde(default)]
    pub basin_state: Option<WashBasinStateDef>,
}

pub struct LatrineDef {
    // ... existing fields ...
    #[serde(default)]
    pub fullness: Option<LatrineFullnessDef>,
}
```

All fields optional — existing scenarios load with defaults. Survival-baseline rebalance (follow-up ticket): author `PlaceDirtiness.decay_per_tick` per place to differentiate (Fertile Fields recovers slowly because it has no drainage; Hillside Shelter recovers quickly).

### D12: Golden coverage

Add `crates/worldwake-ai/tests/golden_place_dirtiness.rs`:

- Three agents staying at Fertile Fields, all relieving in the wilderness — confirm `PlaceDirtiness.value` accumulates and the `WasteCreated` events log.
- Sleep ranking with `PlaceDirtiness > 500` at one place vs. `< 100` at another — confirm agent prefers the cleaner place.
- Wash partial success: basin with `clean_water_units = 1`, `units_per_full_wash = 2` — confirm partial outcome and proportional dirtiness reduction.
- Latrine over capacity: 10 uses then 11th use — confirm 11th creates Waste at the place and increments `PlaceDirtiness`.

## SystemFn Integration

No new SystemFn. `wash_basin_refill` and `place_dirtiness_decay` run inside the existing item-decay maintenance pass (`crates/worldwake-systems/src/item_decay.rs`). All three pieces share the per-tick maintenance window.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `PlaceDirtiness` | Place | Universal | `Default` (`value=0`, `decay_per_tick=2`) — every place implicitly has one |
| `LatrineFullness` | Facility (latrine-tagged) | Role-specific | `None` — only latrine facilities |
| `WashBasinState` | Facility (washbasin-tagged) | Role-specific | `None` — only washbasin facilities |

`PlaceDirtiness` is universal per FND-22 Section 5 (every place needs the field for ranking to work). `LatrineFullness` and `WashBasinState` are role-specific (only facilities of the right tag). Universal default + role-specific conditional both follow `docs/spec-drafting-rules.md` Section 5.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Item decay (S106) | `PlaceDirtiness` decay + `WashBasinState` refill share the maintenance pass | State-mediated |
| Sleep (S128) | Sleep ranking reads `PlaceDirtiness` as a quality modifier | State-mediated |
| Need projection (S126) | No direct interaction — dirtiness drives ranking, not need projection | None |
| Decision history (S110) | Three new `EventTag` variants land in the event log | State-mediated |
| Perception | Co-located agents perceive all three new components (FND-14A) | State-mediated |
| `relieve_wilderness` / `toilet` / `wash` actions | Mutate the new components per D5/D6/D7 | State-mediated |

## Profile-Driven Parameters

Per-agent: `MetabolismProfile.wilderness_relief_dirtiness_penalty` already exists; reused as the per-relief place-dirtiness contribution.

Per-place: `PlaceDirtiness.{value, decay_per_tick}` authored in scenario.

Per-facility: `LatrineFullness.{fill, fill_per_use, critical_threshold}` and `WashBasinState.{max_clean_water, refill_per_tick, units_per_full_wash, dirtiness_per_use}` authored in scenario.

No magic numbers in agent-side or system-side code; all numerics flow through the profile or scenario surface. All [0,1000] values use `Permille`.
