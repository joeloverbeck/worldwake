# S129: Place Dirtiness and Facility Wear

## Summary

Make hygiene a property of the place and the facility, not just the agent. Today, `relieve_wilderness` creates a `Waste` `ItemLot` at the place and increases the actor's `dirtiness` (modulated by `MetabolismProfile.wilderness_relief_dirtiness_penalty`). The wash action consumes water from the well's `ResourceSource` and zeros the actor's dirtiness — but the `WashBasin` facility carries no per-basin state, latrine-tagged places carry no fullness state, and the place itself accumulates no dirtiness from repeated use. The narrative report shows Agent C relieving in the wilderness 26 times at Fertile Fields without any place-side consequence: "the bad sign is that those Waste lots do not seem to matter much downstream." This spec adds three concrete state carriers — `PlaceDirtiness` per place, `LatrineFullness` per latrine-tagged place, `WashBasinState` per washbasin facility — so that wilderness relief makes places dirty (which then biases sleep quality and wash demand), latrines fill up (which biases agents toward wilderness or to clean latrines), and washbasin capacity bounds wash success (folding in PR-11's partial-wash). Folds in PR-9's hygiene-topology authoring (no new place kinds; existing tags become richer) and PR-12's waste/wash event tags.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` components; `WasteCreated`, `WashFacilityUsed` event tags with payload structs per S110 conventions.
- `worldwake-sim` — new `Precondition::TargetHasWashBasinClean` variant in `action_semantics.rs` with matching arms in `action_validation.rs` and `affordance_query.rs`; new `place_dirtiness`, `latrine_fullness`, `wash_basin_state` accessors on `ProfileBeliefView` in `belief_view.rs` (auto-forwarded to `GoalBeliefView` via the existing blanket impl).
- `worldwake-systems` — `relieve_wilderness` writes `PlaceDirtiness` increment; `toilet` writes `LatrineFullness` increment + `PlaceDirtiness` if latrine over capacity; `wash` reads `WashBasinState.clean_water_units` and writes the basin dirtiness increment + partial-success outcome when capacity exhausted; place dirtiness decay and basin natural refill piggyback on the existing item-decay maintenance pass.
- `worldwake-ai` — `emit_wash_goal` splits per-basin (one candidate per `WashBasin` facility, anchored on `OpportunityAnchor::Facility(basin_id)`); `emit_relieve_goal` splits per-place-latrine (one candidate per latrine-tagged place reachable, anchored on `OpportunityAnchor::Place(place_id)`, plus a wilderness fallback). Sleep candidate ranking reads `PlaceDirtiness` as a downward modifier on sleep quality; wash candidate ranking biases toward basins with higher `clean_water_units` and lower `dirtiness_level`; relieve candidate ranking biases toward latrines below `LatrineFullness::critical_threshold`.
- `worldwake-cli` — `PlaceDirtinessDef`, `LatrineFullnessDef`, `WashBasinStateDef` wrapper structs in `scenario/types.rs`; spawn-time integration in `scenario/mod.rs`; defaults preserve existing scenario behavior.

## Dependencies

- S106 (Ground Item Decay) — **completed**, archived at `archive/specs/S106-ground-item-decay.md`. Place dirtiness decay and basin natural refill reuse the existing `item_decay_system` maintenance-pass cadence.
- S128 (Sleep Episodes and Place-Quality Recovery) — **completed**, archived at `archive/specs/S128-sleep-episode-place-quality.md`. Sleep ranking reads `PlaceDirtiness` as a multiplicative factor applied alongside `SleepQualityProfile.recovery_modifier`.
- S110 (Decision History Events) — **completed**, archived at `archive/specs/S110-decision-history-events.md`. The two new event tags follow the S110 payload-struct conventions in `decision_event_payload.rs`.
- S82 (Waste Disposal and Inventory Management) — **completed**, archived at `archive/specs/S82-waste-disposal-inventory-management.md`. `Waste` as `CommodityKind` plus `create_item_lot` already exists; place-side dirtiness is the missing per-place aggregate.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 4: "Relieve and Wash are currently functional but thin … Agent C relieved in the wilderness twenty-six times and never used a toilet because she spent most of the run at Fertile Fields, which lacks a Latrine tag. That is causally legible, but not yet rich." The narrative report confirms: Agent C dirtied 26 wilderness Waste lots at Fertile Fields, but the place itself never developed any property that would push agents toward the latrine at Riverside Camp. The depth fix the assessment asks for is "stay near food but get dirtier from wilderness relief, travel to a latrine but spend time and expose other needs" — which requires the place to carry concrete dirtiness that other agents perceive and weigh.

PR-9's topology depth ask resolves naturally if `PlaceDirtiness` accumulates: Fertile Fields becomes hygiene-poor under sustained use, biasing sleep elsewhere; Riverside Camp's three-affordance hub (water + wash + latrine) becomes the obvious recovery base; Hillside Shelter's latrine-only configuration becomes a low-contention alternative when Riverside Camp is crowded.

## Design Goals

1. `PlaceDirtiness` is per-place state, not per-agent. It accumulates from wilderness relief (and from latrine overflow) at the place, decays on a slow schedule, and is observable by co-located agents through FND-14A.
2. `LatrineFullness` is per-place state on latrine-tagged places (`PlaceTag::Latrine`). It increments per `toilet` use; when over `critical_threshold`, the latrine becomes "critically full" and `toilet` still succeeds but degrades to wilderness-relief outcomes (Waste lot at place + place dirtiness increment).
3. `WashBasinState` is per-facility state on `WorkstationTag::WashBasin` facilities. `clean_water_units` is consumable; `dirtiness_level` accumulates; `wash` partial-success when `clean_water_units < units_per_full_wash` (PR-11 fold-in).
4. No new place kinds and no new workstation tags. PR-9 is satisfied by varying `PlaceDirtiness.decay_per_tick` and `dirtiness_per_use` per place — Fertile Fields' open layout accumulates faster than Hillside Shelter's enclosed shelter.
5. Sleep quality (S128) is modulated by `PlaceDirtiness`. A dirty place sleeps worse — concrete consequence chain: wilderness relief → place dirtier → sleep worse there → agent travels to clean shelter → tradeoff against hunger/thirst access.
6. Two new event tags. Per FND-30 each spec declares its causal records. PR-12's standalone "waste/wash event tag" proposal folds here, not as a separate spec. A future settlement-staff `clean_latrine` action (out of scope) will introduce its own event tag at that time, not preemptively here.

## Non-Goals

- Disease propagation. The assessment is explicit: "No disease system required yet."
- Latrine maintenance via a `clean_latrine` action. Requires authoritative custodian/operator identity; deferred until a settlement-staff role spec exists. **The corresponding `LatrineMaintained` event tag is therefore deferred too** — adding an unused tag would violate FND-28 (no premature abstraction).
- Crowding effects on sleep quality. Folded out of S128 for the same reason; if `PlaceDirtiness` proxies for "many people are using this place," the crowding ask is partially addressed without a dedicated `crowding_count` field.
- Privacy / social-judgment effects of wilderness relief. The assessment excludes social systems.
- Per-agent dirtiness tolerance / cleanliness preference. The existing `MetabolismProfile.wilderness_relief_dirtiness_penalty` already encodes per-agent variation for the *agent's* dirtiness; place dirtiness uses its own per-place authoring (D1).
- Composting / waste-to-fertilizer conversion. Out of scope.
- Wash basin refill via authored worker action. Refill happens through a slow per-tick natural-recovery process (clean water sourced from the same `ResourceSource` topology when adjacent — see D8).
- Adding a `WorkstationTag::Latrine`. Latrines remain `PlaceTag::Latrine` per the existing `toilet` precondition `ActorAtPlaceTag(Latrine)`.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `PlaceDirtiness.value: Permille` is concrete state, not a derived "hygiene score." `LatrineFullness.fill: Permille` and `WashBasinState.{clean_water_units, dirtiness_level}` are concrete fields. |
| FND-4 (Persistent Identity, Object Permanence, and Explicit Transfer) | D8's basin natural refill is an explicit source/sink path: water flows from the place's `ResourceSource.available_quantity` to `WashBasinState.clean_water_units` (decrement source, increment basin), then is consumed by `wash`. The transfer is a per-tick world process, not a hidden spawn. Existing wash water consumption today already decrements `ResourceSource.available_quantity` without producing a Waste/output lot, so the source-to-sink shape is consistent. |
| FND-5 (Carriers of Consequence) | Each new component carries downstream consequences: dirty place → bad sleep → travel decision; full latrine → wilderness-relief fallback at latrine-tagged place; empty basin → wash partial success → agent stays dirty longer. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All state is per-place or per-facility, observable only by co-located agents through perception. No global "all places are dirty" query. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | `wash` gains a new `Precondition::TargetHasWashBasinClean { target_index, min }` so affordance generation can rule out empty basins early; the partial-vs-full branch lives in commit. `toilet` at a critically full latrine still succeeds (creating waste at the place) but with degraded outcome. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | `wash` partial-success when basin water insufficient: agent dirtiness reduced proportionally, basin clean water consumed, basin dirtiness incremented. `toilet` at full latrine: bladder zeroed but place dirtiness incremented and Waste lot created at place. Failure is new state. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | "Dirty place → agents avoid for sleep → fewer agents → less wilderness relief → place cleans up." Dampener: place dirtiness decay over time (per-place `decay_per_tick`), plus the natural negative feedback that wash demand co-locates agents at clean basins. |
| FND-12 (Performance May Compress Computation, Never Causality) | D7's removal of direct well-to-actor consumption in favor of D8's per-tick well-to-basin refill changes *when* wells deplete (per tick proportional to basin demand, not per wash event). This is a visible retiming agents will observe through `ResourceSource.available_quantity` changes in beliefs; the change is declared world law, not invisible compression. |
| FND-14 / FND-14A | Co-located agents perceive `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` directly (FND-14A — physical properties of the place/facility). Off-place propagation through `ShareBelief`. |
| FND-22 (Agent Diversity Through Concrete Variation) | Agents already differ in `UtilityProfile.dirtiness_weight` and `MetabolismProfile.wilderness_relief_dirtiness_penalty`. Both feed wash-vs-other-need tradeoffs and per-agent dirtiness rate without authoring per-place behavior. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Agents who repeatedly observe Fertile Fields as dirty (perception-time `PlaceDirtiness > X`) can update their `SourceReliability` for nearby resources accordingly when S131 lands; for now, the per-perception belief is sufficient. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Relieve writes `PlaceDirtiness`; sleep tick reads it; ranking reads beliefs about it. No imperative cross-call. The maintenance pass `item_decay_system` reads `ResourceSource` in `worldwake-core` (already imported); FND-26 system-decoupling is preserved. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `PlaceDirtiness.value` is authoritative; the per-tick decay is a maintenance-pass mutation, not a derived view. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The existing per-agent dirtiness pathway is preserved (it's correct — agents do still get dirty); the new place-side dirtiness is additive, not a replacement. The current wash-via-well water consumption is *removed* (not shimmed) when D7 lands; basin water becomes the authoritative consumed quantity. The deferred `LatrineMaintained` event tag is *not* introduced preemptively — no dead variants. |
| FND-29 (Debuggability Is a Product Feature) | New event tags surface the causal chain. "Why did the agent travel to Hillside Shelter at tick 612?" becomes answerable from `PlaceDirtiness` rising at Fertile Fields plus the agent's sleep candidate ranking output. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Two new event tags (`WasteCreated`, `WashFacilityUsed`) land in the existing append-only event log. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers the four mandated subsections (Information-path, Positive-feedback, Concrete dampeners, Stored-vs-derived). |
| FND-31 (Validation and Falsification Are First-Class) | D12 declares target patterns each golden test proves, plus adversarial sweeps and artifacts the system must never produce. |

## FND-01 Section H — Causal Hooks Declaration

This section covers the four subsections mandated by `docs/spec-drafting-rules.md`.

1. **Information-path analysis.** Place/facility state is co-located perception (FND-14A). Off-place propagation: explicit `ShareBelief` for "I heard the well basin at Forest Clearing is empty." No global hygiene registry. AI ranking reads via new `ProfileBeliefView` accessors (`place_dirtiness`, `latrine_fullness`, `wash_basin_state`), backed by `RuntimeBeliefView` and forwarded through the existing `GoalBeliefView` blanket impl.
2. **Positive-feedback analysis.** (a) "More wilderness relief → place dirtier → agents leave for clean places → fewer relief events at the dirty place → place cleans up." Self-balancing through use-distribution. (b) "More wash use → basin dirtier → fewer wash successes → agents seek other basins → first basin recovers naturally." Self-balancing through capacity.
3. **Concrete dampeners.** (a) `PlaceDirtiness` decay rate (per-tick decrement during maintenance pass). (b) `WashBasinState` natural refill (per-tick `clean_water_units` increment up to `max_clean_water` when the basin is co-located with a `ResourceSource` of `Water` — concrete supply chain). (c) `LatrineFullness` does *not* decay automatically without a maintenance action — full latrines stay full, providing the negative feedback for the system.
4. **Stored state vs. derived read-model.** Stored: `PlaceDirtiness.{value, decay_per_tick, dirtiness_per_use}`, `LatrineFullness.{fill, fill_per_use, critical_threshold}`, `WashBasinState.{clean_water_units, max_clean_water, refill_per_tick, units_per_full_wash, dirtiness_level, dirtiness_per_use}`. Derived: per-tick decay/refill maintenance computations from authoritative state.

## Authoritative-to-AI Impact Analysis

D7 modifies the `wash` action's precondition surface (new `TargetHasWashBasinClean`; removal of the existing `TargetHasResourceSource` for Water on target index 1). Per CLAUDE.md "Authoritative-to-AI Impact Rule", the seven-point checklist for the implementing ticket:

1. **`get_affordances`** (`crates/worldwake-sim/src/affordance_query.rs`) — must add a match arm for `Precondition::TargetHasWashBasinClean`, reading `WashBasinState.clean_water_units` against the requested minimum. Wash candidates emitted only when the basin holds at least one unit.
2. **`generate_candidates`** (`crates/worldwake-ai/src/candidate_generation.rs`) — `emit_wash_goal` is rewritten per D10 to emit per-basin candidates with `OpportunityAnchor::Facility(basin_id)`. Each candidate must reflect the basin's current `clean_water_units` and `dirtiness_level` so ranking can score them.
3. **`search_plan`** — terminal-ordering and barrier logic for the wash op must respect partial-success outcomes: a wash that reduces dirtiness only partially should still close the goal if the agent's residual dirtiness is below the goal's `is_satisfied` threshold, and otherwise leave the goal active for a follow-up wash.
4. **`BestEffort` action start** (`tick_step.rs`) — when the basin's `clean_water_units` drops between affordance discovery and action start (e.g., another agent washed first), the start path must degrade gracefully: either fail with `ActionError::PreconditionFailed` (driving replan) or proceed to partial-success commit, depending on the new precondition's gating semantics. The chosen behavior is fail-then-replan to preserve the candidate-emission gate as authoritative.
5. **`handle_plan_failure`** (`agent_tick.rs`) — when the new precondition rejects, the agent replans; the wash candidate is rebuilt against the now-current basin state, and a different basin (per the per-basin candidate split) may be chosen.
6. **Payload revalidation** (`plan_revalidation.rs`) — wash uses affordance-derived target payloads (basin entity id, place id), not synthesized payloads. No `with_payload_override_validator` hook is required.
7. **Golden tests** — D12 covers all five preceding paths (basin-empty rejection, partial-success commit, basin-state ranking, latrine-overflow Waste creation, place-dirtiness sleep penalty).

The implementing ticket must run `cargo test -p worldwake-ai` before claiming completion; any pre-existing wash-flow golden that asserts `TargetHasResourceSource { commodity: Water, ... }` on `wash` must be updated to assert the new basin-side precondition.

## Deliverables

### D1: `PlaceDirtiness` component

In `crates/worldwake-core/src/place_dirtiness.rs` (new module):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaceDirtiness {
    /// Current accumulated dirtiness as Permille [0, 1000].
    /// 0 = pristine, 1000 = severely dirty.
    pub value: Permille,
    /// Per-tick decay applied during the existing item-decay
    /// maintenance pass. Authoritative per-place via component
    /// instance, not a hidden constant.
    pub decay_per_tick: Permille,
    /// Per-use dirtiness contribution from a wilderness-relief or
    /// latrine-overflow event at this place. Authored per-place so
    /// open layouts (e.g., Fertile Fields) accumulate faster than
    /// enclosed shelters (e.g., Hillside Shelter).
    pub dirtiness_per_use: Permille,
}

impl Default for PlaceDirtiness {
    fn default() -> Self {
        Self {
            value: Permille::ZERO,
            decay_per_tick: Permille::new_unchecked(2),
            dirtiness_per_use: Permille::new_unchecked(80),
        }
    }
}

impl Component for PlaceDirtiness {}
```

Component schema registration with kind filter `|kind| kind == EntityKind::Place,` (matching the `SleepQualityProfile` precedent at `component_schema.rs:1660`). `decay_per_tick = Permille::new_unchecked(2)` and `dirtiness_per_use = Permille::new_unchecked(80)` are universal defaults — a fully dirty place returns to clean over ~500 ticks of disuse, and ~12 wilderness reliefs saturate the value.

### D2: `LatrineFullness` component (place-keyed)

In `crates/worldwake-core/src/place_dirtiness.rs` (same module):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LatrineFullness {
    /// Current fill level as Permille [0, 1000].
    /// 0 = empty, 1000 = full.
    pub fill: Permille,
    /// Per-toilet-use increment.
    pub fill_per_use: Permille,
    /// Threshold beyond which the latrine is "critically full" —
    /// `toilet` still succeeds but degrades to wilderness-relief
    /// outcomes (creates Waste lot at place, increments PlaceDirtiness).
    pub critical_threshold: Permille,
}

impl Default for LatrineFullness {
    fn default() -> Self {
        Self {
            fill: Permille::ZERO,
            fill_per_use: Permille::new_unchecked(80),
            critical_threshold: Permille::new_unchecked(800),
        }
    }
}

impl Component for LatrineFullness {}
```

Component schema registration with kind filter `|kind| kind == EntityKind::Place,` — role-specific by `PlaceTag::Latrine`, mirroring the `BanditCamp` / `SceneEvidence` place-only precedent. Defaults: `fill_per_use = 80`, `critical_threshold = 800` — a fresh latrine handles ~10 uses before degrading. The `toilet` action already gates on `Precondition::ActorAtPlaceTag(Latrine)`, so handlers reach the component via `effective_place(actor)` without a new TargetSpec on the action.

### D3: `WashBasinState` component (facility-keyed)

In `crates/worldwake-core/src/place_dirtiness.rs` (same module):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WashBasinState {
    /// Available clean water for wash, in abstract units. One full
    /// wash consumes `units_per_full_wash` units; partial wash
    /// consumes proportional fraction.
    pub clean_water_units: u16,
    pub max_clean_water: u16,
    /// Per-tick refill when basin is co-located with a Water
    /// `ResourceSource` (the basin's place must host the source).
    pub refill_per_tick: u16,
    pub units_per_full_wash: u16,
    /// Accumulated basin dirtiness from use; reduces wash effectiveness.
    pub dirtiness_level: Permille,
    pub dirtiness_per_use: Permille,
}

impl Default for WashBasinState {
    fn default() -> Self {
        Self {
            clean_water_units: 10,
            max_clean_water: 10,
            refill_per_tick: 1,
            units_per_full_wash: 2,
            dirtiness_level: Permille::ZERO,
            dirtiness_per_use: Permille::new_unchecked(50),
        }
    }
}

impl Component for WashBasinState {}
```

Component schema registration with kind filter `|kind| kind == EntityKind::Facility,` — role-specific by `WorkstationTag::WashBasin`. Defaults: `max_clean_water = 10`, `refill_per_tick = 1` (when adjacent water source), `units_per_full_wash = 2`, `dirtiness_per_use = 50`.

### D4: New `EventTag` variants and payloads

In `crates/worldwake-core/src/event_tag.rs`:

```rust
pub enum EventTag {
    // ... existing variants ...
    WasteCreated,
    WashFacilityUsed,
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
    OvercapacityLatrine,
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

Both payload structs are added as variants on the `DecisionEventPayload` enum following the `SleepEpisodeStartedPayload` precedent at `decision_event_payload.rs:38–46`. `LatrineMaintained` is **not** added — it is deferred until a `clean_latrine` action lands (see Non-Goals).

### D5: `relieve_wilderness` handler extension

In `crates/worldwake-systems/src/needs_actions.rs`, the `relieve_wilderness` commit handler (currently at lines 648–691):

- Existing behavior preserved: zero bladder, create Waste `ItemLot` at place via `txn.create_item_lot(CommodityKind::Waste, Quantity(1))`, set ground location, emit `DisturbanceMarker` evidence, increment agent dirtiness via `needs.dirtiness.saturating_add(profile.wilderness_relief_dirtiness_penalty)`.
- New: read or create `PlaceDirtiness` on `txn.effective_place(instance.actor)?`. Increment `value` using `saturating_add(place_dirtiness.dirtiness_per_use)` (mirroring the agent-side dirtiness pattern; no manual cap needed since `saturating_add` clamps at 1000).
- Emit `WasteCreated` decision event with `WasteSource::WildernessRelief` and the actual `place_dirtiness_delta`.

### D6: `toilet` handler extension

In the same module, `toilet` commit handler (currently at lines 617–646):

- Existing behavior preserved: zero bladder, create Waste lot at the actor's place.
- New: read `LatrineFullness` from `effective_place(actor)` (no new TargetSpec on the action; the existing `ActorAtPlaceTag(Latrine)` precondition guarantees the place carries the component when the registration kind filter binds it).
- Branch on `fill < critical_threshold`:
  - If under threshold: `fill = fill.saturating_add(fill_per_use)`. Emit `WasteCreated` with `WasteSource::OvercapacityLatrine` only if the new fill crossed `critical_threshold` on this tick (so the first overflow event records the transition).
  - If at or over threshold: still zero bladder, but additionally increment `PlaceDirtiness.value` by `dirtiness_per_use` and emit `WasteCreated` with `WasteSource::OvercapacityLatrine` (the latrine is overflowing).

### D7: `wash` handler refactor

In the same module, `wash` commit handler (currently at lines 694–734) and the action's precondition list (currently at lines 225–250):

- **Precondition change**: Remove `Precondition::TargetHasResourceSource { target_index: 1, commodity: Water, min_available: 1 }`. Add `Precondition::TargetHasWashBasinClean { target_index: 0, min: 1 }` — this is a new variant on the `Precondition` enum in `worldwake-sim/src/action_semantics.rs:47`, with corresponding arms in `action_validation.rs` and `affordance_query.rs`. This enables affordance generation to rule out empty basins early (Authoritative-to-AI checklist point 1).
- **TargetSpec change**: `wash` now needs only a single target, the basin. The well is no longer a direct target of the action — water flows from the well to the basin per-tick via D8 instead.
- **Commit logic**:
  - Read `WashBasinState` from the basin (`instance.targets[0]`).
  - If `clean_water_units >= units_per_full_wash`: full success — consume `units_per_full_wash`, set agent dirtiness to `pm(0)`, increment `dirtiness_level` by `dirtiness_per_use`.
  - If `0 < clean_water_units < units_per_full_wash`: partial success — consume all available, reduce agent dirtiness proportionally (`(available * agent_dirtiness) / units_per_full_wash`), increment `dirtiness_level` proportionally.
  - The `clean_water_units == 0` case is unreachable at commit because the new precondition rules it out at affordance time; if reached anyway (race after affordance check), fail with `ActionError::PreconditionFailed(format!("basin {basin_id} has no clean water"))`. (`PreconditionFailed` carries `String`, not `&'static str` — see `action_handler.rs:296`.)
- Emit `WashFacilityUsed` with `partial` flag.

### D8: Basin natural refill maintenance

Add a `wash_basin_refill` step inside `crates/worldwake-systems/src/item_decay.rs` (the existing maintenance pass). Per-tick: for each entity with `WashBasinState`:

- Resolve the basin's place via `effective_place(basin)`. If the place hosts a `ResourceSource` with `commodity == CommodityKind::Water` and `available_quantity > 0`, transfer up to `min(refill_per_tick, max_clean_water - clean_water_units, available_quantity)` units: increment `clean_water_units`, decrement `ResourceSource.available_quantity` accordingly.
- This is the concrete supply chain — basin water doesn't appear from nowhere; it draws from the same `ResourceSource` agents harvest from. The well's `regeneration_ticks_per_unit` continues to refill the source as today.
- Causal note: under this design wells deplete proportional to total basin demand across the place graph (per-tick refill) rather than per individual wash event. The retiming is visible to agents through belief observation of `ResourceSource.available_quantity` and is consistent with FND-12 — the world law is declared, not invisibly compressed.

### D9: `PlaceDirtiness` decay maintenance

In the same maintenance pass: for each entity with `PlaceDirtiness`, decrement `value` by `decay_per_tick` (saturating at zero via `saturating_sub`).

### D10: AI candidate emission and ranking integration

**Candidate emission** (`crates/worldwake-ai/src/candidate_generation.rs`):

- `emit_wash_goal` (currently at lines 3313–3390) — split per-basin: enumerate all `WorkstationTag::WashBasin` facilities reachable through `wash_access_opportunities()`, emit one candidate per basin with `OpportunityAnchor::Facility(basin_id)`. Each candidate carries the basin's place id for travel cost reasoning. Replaces the current single-place-anchored candidate.
- `emit_relieve_goal` (currently at lines 3282–3309) — split per-place-latrine + wilderness fallback: enumerate all reachable `PlaceTag::Latrine` places, emit one candidate per latrine-tagged place with `OpportunityAnchor::Place(place_id)`; additionally emit a wilderness candidate with `OpportunityAnchor::None` (the existing form) so the ranking can prefer wilderness when all known latrines are critically full. Replaces the current single un-anchored candidate.

**Belief-view accessors** (`crates/worldwake-sim/src/belief_view.rs`):

- Add three new methods on `ProfileBeliefView` (the trait at line 758 hosting `place_sleep_quality_profile` at line 770):
  - `fn place_dirtiness(&self, agent: EntityId, place: EntityId) -> PlaceDirtiness`
  - `fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness`
  - `fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState`
- `RuntimeBeliefView` impl forwards each accessor to authoritative world state through `WorldTxn::get_component_*` (FND-14A — co-located perception of physical place/facility properties).
- The existing blanket `impl<T> GoalBeliefView for T where T: … + ProfileBeliefView` at `belief_view.rs:1359` automatically forwards the new methods to `GoalBeliefView` consumers; no separate macro is required.

**Ranking integration** (`crates/worldwake-ai/src/ranking.rs`):

- `Sleep` candidate ranking: extend the existing `recovery_modifier` integration at lines 1649–1665. After applying `recovery_modifier`, multiply by `(1000 - place_dirtiness.value) / 1000` (saturating arithmetic to avoid overflow). A pristine place keeps full quality; a fully-dirty place halves.
- `ExploreLocation` candidate ranking: when scoring a candidate place, apply the same `(1000 - place_dirtiness.value) / 1000` multiplier on the existing place-quality input.
- `Wash` candidate ranking: each per-basin candidate scored by `clean_water_units` (positive contribution) and `dirtiness_level` (negative contribution). Existing ranking already considers facility presence; this adds the second-order axes.
- `Relieve` candidate ranking: each per-place-latrine candidate scored against its `latrine_fullness.fill`; latrines below `critical_threshold` rank above the wilderness fallback. If all known latrines are at or over `critical_threshold`, the wilderness candidate ranks highest by default.

### D11: Scenario authoring

In `crates/worldwake-cli/src/scenario/types.rs`, add three new wrapper types and extend the existing defs:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaceDirtinessDef {
    #[serde(default)]
    pub value: Option<Permille>,
    #[serde(default)]
    pub decay_per_tick: Option<Permille>,
    #[serde(default)]
    pub dirtiness_per_use: Option<Permille>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatrineFullnessDef {
    #[serde(default)]
    pub fill: Option<Permille>,
    #[serde(default)]
    pub fill_per_use: Option<Permille>,
    #[serde(default)]
    pub critical_threshold: Option<Permille>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WashBasinStateDef {
    #[serde(default)]
    pub clean_water_units: Option<u16>,
    #[serde(default)]
    pub max_clean_water: Option<u16>,
    #[serde(default)]
    pub refill_per_tick: Option<u16>,
    #[serde(default)]
    pub units_per_full_wash: Option<u16>,
    #[serde(default)]
    pub dirtiness_level: Option<Permille>,
    #[serde(default)]
    pub dirtiness_per_use: Option<Permille>,
}

pub struct PlaceDef {
    // ... existing fields ...
    #[serde(default)]
    pub place_dirtiness: Option<PlaceDirtinessDef>,
    #[serde(default)]
    pub latrine_fullness: Option<LatrineFullnessDef>, // ignored unless tags include PlaceTag::Latrine
}

pub struct FacilityDef {
    // ... existing fields ...
    #[serde(default)]
    pub wash_basin_state: Option<WashBasinStateDef>, // ignored unless workstation == WashBasin
}
```

Each `*Def` provides an `into_profile()` (or `From<*Def>`) returning the runtime component with field-level fallbacks to component `Default` values. In `crates/worldwake-cli/src/scenario/mod.rs`:

- `spawn_place` always calls `txn.set_component_place_dirtiness(place_id, place_def.place_dirtiness.map(Into::into).unwrap_or_default())` (universal-on-Place pattern, mirroring `set_component_sleep_quality_profile` at `mod.rs:282–290`).
- `spawn_place` conditionally sets `LatrineFullness` only when the place's tags include `PlaceTag::Latrine`: `if place_def.tags.contains(&PlaceTag::Latrine) { txn.set_component_latrine_fullness(place_id, place_def.latrine_fullness.map(Into::into).unwrap_or_default())?; }`.
- The facility spawn loop conditionally sets `WashBasinState` only when `facility_def.workstation == WorkstationTag::WashBasin`.

Survival-baseline rebalance (follow-up ticket): author per-place `PlaceDirtiness.{decay_per_tick, dirtiness_per_use}` to differentiate (Fertile Fields recovers slowly; Hillside Shelter recovers quickly).

### D12: Golden coverage

Add `crates/worldwake-ai/tests/golden_place_dirtiness.rs`:

**Target patterns** (FND-31 falsification frame):

1. **Place dirtiness accumulation**: Three agents staying at Fertile Fields, all relieving in the wilderness. Assert `PlaceDirtiness.value` accumulates monotonically (relief_count × dirtiness_per_use, saturating at 1000), and `WasteCreated` event tag appears N times in the event log. **Must never produce**: `value` decreasing during the relief phase, or `WasteCreated` events without corresponding Waste `ItemLot` entities (conservation regression).
2. **Sleep ranking under dirtiness**: Two candidate places — one with `PlaceDirtiness.value > 500`, one with `value < 100` — both with identical `SleepQualityProfile`. Assert agent sleep candidate ranking selects the cleaner place. **Must never produce**: agent picks the dirtier place when other recoveries are equal.
3. **Wash partial success**: Basin authored with `clean_water_units = 1`, `units_per_full_wash = 2`, agent dirtiness = 1000. Assert wash commits as partial-success: agent dirtiness reduced to 500 (proportional 1/2), basin `clean_water_units = 0`, `WashFacilityUsed.partial == true`. **Must never produce**: full success when water insufficient, or basin going negative.
4. **Latrine overcapacity**: Latrine-tagged place authored with `fill_per_use = 100`, `critical_threshold = 800`. Run nine `toilet` actions (fill = 900 — over critical), then a tenth. Assert the tenth invocation creates a Waste lot at the place and increments `PlaceDirtiness`. **Must never produce**: `LatrineFullness.fill` decreasing without a maintenance action, or overcapacity not creating Waste.
5. **Basin natural refill from co-located source**: Basin at place hosting a Water `ResourceSource`. Run wash actions until `clean_water_units = 0`, then advance ticks with no wash. Assert `clean_water_units` recovers per `refill_per_tick` until reaching `max_clean_water`, and `ResourceSource.available_quantity` decrements correspondingly (conservation of water transfers). **Must never produce**: basin refilling without consuming source quantity, or refilling above `max_clean_water`.
6. **Auth-to-AI replan on basin emptiness**: Two basins at one place; one drained mid-plan. Assert wash candidate emission ranks the non-empty basin first; assert that if the chosen basin is drained between affordance and start, the agent replans onto the second basin (Authoritative-to-AI checklist points 4–5). **Must never produce**: agent attempting wash at a known-empty basin without `PreconditionFailed` and replan.

Adversarial sweeps the architecture must support (not necessarily exercised in S129's goldens, but reachable):

- `decay_per_tick = 0` with continuous wilderness relief — `PlaceDirtiness.value` should saturate at 1000 and stay there.
- `refill_per_tick = 0` with continuous wash — basin should plateau at `clean_water_units = 0`.
- `critical_threshold = 0` — every `toilet` use is overcapacity from the first action.

## SystemFn Integration

No new SystemFn. `wash_basin_refill` and `place_dirtiness_decay` run inside the existing `item_decay_system` maintenance pass (`crates/worldwake-systems/src/item_decay.rs`). All three pieces share the per-tick maintenance window. The system is already imported with `worldwake_core::ResourceSource` access (item_decay.rs:3), so D8's source read does not introduce a new crate dependency.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `PlaceDirtiness` | Place | Universal | `Default` (`value=0`, `decay_per_tick=2`, `dirtiness_per_use=80`) — every place implicitly has one. Universal-on-Place precedent set by S128's `SleepQualityProfile`. |
| `LatrineFullness` | Place | Role-specific (only places with `PlaceTag::Latrine`) | `Default` applied conditionally at scenario spawn |
| `WashBasinState` | Facility | Role-specific (only facilities with `WorkstationTag::WashBasin`) | `Default` applied conditionally at scenario spawn |

`PlaceDirtiness` is universal because ranking needs to read it for every reachable place; absent components would force `Option<&PlaceDirtiness>` plumbing in `ProfileBeliefView`. `LatrineFullness` and `WashBasinState` are role-specific (only entities of the right tag), following `docs/spec-drafting-rules.md` Section 5 conventions adapted for Place/Facility entities.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Item decay (S106) | `place_dirtiness_decay` and `wash_basin_refill` run inside `item_decay_system` | State-mediated |
| Sleep (S128) | Sleep ranking reads `PlaceDirtiness` as a quality modifier alongside `SleepQualityProfile.recovery_modifier` | State-mediated |
| Need projection (S126) | No direct interaction — dirtiness drives ranking, not need projection | None |
| Decision history (S110) | Two new `EventTag` variants (`WasteCreated`, `WashFacilityUsed`) land in the event log | State-mediated |
| Perception | Co-located agents perceive all three new components (FND-14A) | State-mediated |
| `relieve_wilderness` / `toilet` / `wash` actions in `worldwake-systems` | Mutate the new components per D5/D6/D7 | State-mediated |
| AI candidate emission and ranking in `worldwake-ai` | Reads new components via `ProfileBeliefView` accessors defined in `worldwake-sim` | State-mediated |
| Action precondition surface in `worldwake-sim` | New `Precondition::TargetHasWashBasinClean` variant evaluated by `affordance_query.rs` and `action_validation.rs` | State-mediated |

## Profile-Driven Parameters

Per-agent: `MetabolismProfile.wilderness_relief_dirtiness_penalty` (already exists, used for *agent-side* dirtiness only); `UtilityProfile.dirtiness_weight` (already exists, used for ranking weights).

Per-place: `PlaceDirtiness.{value, decay_per_tick, dirtiness_per_use}` and `LatrineFullness.{fill, fill_per_use, critical_threshold}` authored in scenario.

Per-facility: `WashBasinState.{clean_water_units, max_clean_water, refill_per_tick, units_per_full_wash, dirtiness_level, dirtiness_per_use}` authored in scenario.

No magic numbers in agent-side or system-side code; all numerics flow through component fields populated from scenario authoring with documented defaults. All [0,1000] values use `Permille`.
