# S177: Water Source Quality and Quality-Belief Memory

## Summary

The depletion half of canonical regression scenario D is **already proven for water**. Today:

- Perception writes `last_observed_capacity` + `last_observed_capacity_tick` into the agent's `SourceReliability` on co-located observation (`crates/worldwake-systems/src/perception.rs:174` via `ReliabilityRecord::observe_capacity`).
- Candidate ranking discounts believed-depleted sources via `apply_source_reliability_discount` (`crates/worldwake-ai/src/ranking.rs:534`) and `source_composite_rank` (`crates/worldwake-ai/src/source_composite.rs`), whose `capacity_factor_permille` floors the rank of a freshly-observed empty source.
- Failed extraction at start writes belief via `record_failed_source_attempt` (`crates/worldwake-systems/src/production_actions.rs:1228`) and emits `EventTag::SourceExpectationFailure` with `SourceExpectationFailurePayload` (`crates/worldwake-ai/src/agent_tick/mod.rs:1051-1052`) for source attribution.
- Existing water-specific goldens prove the loop: `golden_local_depleted_source_regenerates_without_spurious_failure_memory` (`crates/worldwake-ai/tests/scenarios/ai_decisions.rs:455`), `empty_but_fresh_observation_demotes_depleted_source` (`crates/worldwake-ai/src/source_composite.rs:533`), and the source-failure consumption proven in `crates/worldwake-ai/tests/scenarios/survival_preferences.rs:110`.
- The S38/S151 substrate is the live `SourceReliability` component (`crates/worldwake-core/src/experience.rs:173-210`), keyed by `SourceKey { entity, commodity }` and valued by `ReliabilityRecord { successful_acquisitions, failed_attempts, last_attempt_tick, last_observed_capacity, last_observed_capacity_tick, provenance_events, … }`, with `enforce_limits` decay over `PreferenceProfile.memory_retention_ticks`.

What is **missing** is the **quality axis**:

1. **Water quality.** `ResourceSource` (`crates/worldwake-core/src/production.rs:75-83`) has no quality field. Clean, stale, and muddy water are indistinguishable. Basin refill draws from *any* colocated water source regardless of quality (`crates/worldwake-systems/src/item_decay.rs::first_colocated_water_source`). Drink relief is the commodity-intrinsic `CommodityConsumableProfile.thirst_relief_per_unit` (`crates/worldwake-core/src/items.rs:139`) with no per-quality scaling. There is no dirty-water-versus-travel tradeoff.
2. **Quality-belief memory.** Even if water sources had quality, agents would have no concrete per-agent learned record of "this source was muddy at tick T" feeding ranking — `ReliabilityRecord` records capacity and success/failure ratios but not quality.
3. **Quality consequences.** Drink does not raise dirtiness when the lot is muddy; basin refill does not prefer cleanest water; `ItemLot` (`crates/worldwake-core/src/items.rs:317`) does not carry the source's quality at extraction time.

This spec adds (a) a concrete `WaterQuality` field on water `ResourceSource`s with downstream wash/thirst consequences, (b) item-lot quality propagation and quality-belief memory atop the existing `SourceReliability` substrate, and (c) a quality-aware emergent target: when a clean source is depleted and only a muddy backup is available, the planner chooses to drink muddy water (accepting reduced relief + raised dirtiness) versus traveling further to clean water, with the choice mediated by per-agent tolerance.

**Deliberate scope discipline (critical reassessment of the source report):** the report proposes that "drinking unsafe water can reduce thirst but cause later consequence." This spec implements only the **immediate, concrete** consequences — reduced thirst relief, raised dirtiness, basin-refill quality preference. It **defers any "unsafe water → sickness/wound" path**: a disease/illness consequence is a new carrier (`FoodSickness`/`DigestiveDistress`) that FND-5 and the report's own MUST-NOT say should not be added until it is a proven concrete consequence carrier with source, trace, recovery, and proof. Water quality here is a *utility/effectiveness* axis, not a disease vector.

## Phase

Phase 7: Consequence Carriers

## Status

🚧 IN PROGRESS — implementation started with `archive/tickets/S177WATSRCQUA-001.md` on 2026-05-31.

## Crates

- `worldwake-core` (`WaterQuality` enum + field on `ResourceSource`; `quality` field on `ItemLot`; quality-observation extensions to `ReliabilityRecord`; new universal `WaterToleranceProfile` component; new `EventTag::ResourceSourceQualityObserved` variant)
- `worldwake-sim` (belief-view accessor for source quality; widening of `SourceExpectationFailurePayload` is **not** required — existing event variant is reused for depletion, the new variant is for quality)
- `worldwake-systems` (Drink relief and basin refill scale with `WaterQuality`; extraction commits the source quality onto the produced lot; perception writes `last_observed_quality` into `SourceReliability` on co-located source observation; dirty-water refill raises basin dirtiness)
- `worldwake-ai` (source-rank composite reads per-source quality belief and `WaterToleranceProfile` to discount muddy/stale; survival forensics record `SourceAcquisitionFailure`)
- `worldwake-cli` (scenario contract for `WaterQuality` on `ResourceSourceDef`; scenario contract for `WaterToleranceProfile` on `AgentDef`; player-POV gating for source quality observation)

## Dependencies

- `archive/specs/S79-resource-source-consumption-affordances.md` — provides the `ResourceSource` extraction/consumption affordance substrate this spec extends; established the water-extraction-to-item-lot path.
- `archive/specs/S38-learned-route-source-preferences.md` — landed `SourceReliability` (`crates/worldwake-core/src/experience.rs:173`), which this spec extends with quality observation fields rather than introducing a new component.
- `archive/specs/S151-testimony-reliability-and-route-preferences.md` — landed `TestimonyReliability` and refined `RoutePreference`; informs the freshness/decay design here but is **not** the literal precedent for source-reliability decay (which is `SourceReliability::enforce_limits` over `PreferenceProfile.memory_retention_ticks`).
- `archive/specs/S129-place-dirtiness-facility-wear.md` + `archive/specs/S176-sanitation-facility-degradation-consequences.md` — provide `WashBasinState.dirtiness_level` (`crates/worldwake-core/src/place_dirtiness.rs:50`) and the wash-effectiveness gate this spec couples to (dirty water dirties the basin; the basin's wash effectiveness then degrades per S176).
- `archive/specs/S120-survival-critical-window-forensics.md` — provides `SurvivalForensicExtractor` and the `DegradedSelfCareOpportunity` record in `crates/worldwake-ai/src/survival_forensics.rs`, cited as the precedent for `SourceAcquisitionFailure`.

## Design Goals

- Water sources carry concrete **quality** (`Clean`, `Stale`, `Muddy`) as authoritative state. Drinking lower-quality water gives **less thirst relief** and **raises dirtiness**; clean water is strictly preferable when known and reachable.
- Quality is **observed locally** (FND-14A) when an agent is co-located with the source, and stored as a concrete observation field on the agent's `SourceReliability` record alongside the existing `last_observed_capacity` (no parallel reliability store).
- **Per-agent tolerance** to lower-quality water lives on a new universal `WaterToleranceProfile` component — a hardy agent's relief factor for `Muddy` is higher than a fragile agent's, so two agents in the same situation can lawfully diverge (FND-22).
- Basin refill (item-decay system) **prefers clean colocated water**; refilling from muddy water raises the basin's `dirtiness_level` (couples to S176's wash-effectiveness gate).
- Player and AI share identical source legality; the CLI surfaces only source quality the controlled agent lawfully perceives.
- The **emergent target** is the scarcity ↔ quality tradeoff: a shared clean well depletes under multi-agent draw, the believed-good fallback turns out to be muddy on arrival, and different agents make different drink-vs-travel-further choices based on `WaterToleranceProfile` diversity.

## Non-Goals

- **No water-borne disease / illness / sickness wound.** Explicitly deferred (see Summary). `Unsafe`/`Contaminated` quality tiers are **not** introduced in this spec; only `Clean`/`Stale`/`Muddy` as utility tiers. An illness carrier is a future spec triggered only if a concrete consequence-with-recovery is designed.
- **No abstract "water scarcity level."** The *source* is scarce (depletes), per FND-3.
- **No new global hydrology / weather / drought system.** Drought as a boundary process belongs to held `specs/S62`; this spec creates scarcity internally via depletion and quality only.
- **No new contention queue.** Extraction-slot contention is the existing `ResourceSource.extraction_slots` substrate (`ResourceExtractionQueues`).
- **No HTN method.** Fallback-source selection is flat GOAP candidate emission discounted by belief.
- **No new "depletion belief" memory or `EventTag::ResourceSourceDepletedObserved` variant.** The depletion half is already implemented via `SourceReliability.last_observed_capacity` + perception + ranking discount + `EventTag::SourceExpectationFailure`. Adding a parallel surface would violate FND-28. The new event tag introduced here is **`ResourceSourceQualityObserved`** for the quality axis only.
- **No generalization of `ItemLot.quality` to non-water commodities.** The field is `Option<WaterQuality>`; food spoilage (S178) is a separate, future commodity-quality story and is not required to share this surface. Per YAGNI.
- **No backward-compatibility shim.** Sources without an authored quality default to `Clean` via `#[serde(default)]`; the field is added, not aliased (FND-28). Same for `ItemLot.quality` (`None` for non-water commodities).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | A shared well depletes under multi-agent draw → some travel to a muddy backup → drink worse water (per their `WaterToleranceProfile`), get dirtier, queue, or trade → scarcity ↔ quality tradeoff emerges from a finite source + per-agent tolerance, not a scarcity dial |
| FND-3 (Concrete state over abstract scores) | Quality is a concrete per-source enum; per-agent tolerance is a concrete profile; reliability is concrete observation history; no `water_score` |
| FND-4 (Source/sink) | Drinking and refill draw `available_quantity`; regeneration is the existing explicit source process; quality is a property, not a conserved quantity |
| FND-7 / FND-15 (Locality / knowledge travels) | Quality observed at the source (FND-14A); quality belief is per-agent, written by perception, decaying via `SourceReliability::enforce_limits` |
| FND-14 / FND-14A / FND-14B | Remote source quality is belief-backed; co-located is same-tick observation; the planner cannot read remote `ResourceSource.quality` as authoritative |
| FND-16 (Ignorance/contradiction first-class) | Agents hold stale "well is clean" beliefs; two agents can disagree about a source's quality after observing it at different ticks |
| FND-17 (Surprise from violated expectation) | Reaching a believed-clean well and finding it muddy is the expectation violation that updates belief; emits `EventTag::ResourceSourceQualityObserved` |
| FND-19 (Agent symmetry) | Same extraction/drink legality and quality effects for human and AI; CLI gating exposes only what the controlled agent lawfully perceives |
| FND-21 (Intentions revisable) | A planned Drink at a now-believed-muddy source can be replaced by travel-to-clean-source on the next tick after the quality observation lands |
| FND-22 (Agent diversity) | `WaterToleranceProfile` is per-agent — hardy vs. fragile agents lawfully diverge on the same situation |
| FND-22A (Learning is concrete state) | Quality observation memory is concrete, owned per-agent on `SourceReliability`, with accountable acquisition (`observe_quality`) and decay (`enforce_limits`) — not hidden global adaptation |
| FND-26 (Systems via state) | Drink/refill read source state and write it; planner reads belief; forensics reads log; no direct calls |
| FND-28 (No backcompat) | `WaterQuality` field added; no parallel old/new water path; `ItemLot.quality` is the new authoritative path. Depletion belief surface is **reused** (`SourceReliability` + `SourceExpectationFailure`); no parallel surface is introduced |
| FND-29 / FND-29A | "Why did this agent drink muddy water?" answerable from: per-agent `WaterToleranceProfile` + quality belief on `SourceReliability` + clean-well depletion history + decision trace |
| FND-31 (Validation) | Focused goldens + 1440-tick quality-tradeoff collision scenario |

This spec **extends** the proven canonical regression scenario D infrastructure (already realized for water at the depletion level) with the quality dimension: rumor of clean water → travel → muddy on arrival → belief correction → drink-or-travel-further replan.

## Deliverables

### D1. `WaterQuality` enum, `ResourceSource.quality`, `ResourceSourceDef.quality`

```rust
// crates/worldwake-core/src/production.rs (new sibling type)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WaterQuality {
    Clean,
    Stale,
    Muddy,
}
```

`Option<WaterQuality>` is compatible with `ResourceSource`'s existing derive set (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`).

Add the field to `ResourceSource`:

```rust
pub struct ResourceSource {
    pub commodity: CommodityKind,
    pub available_quantity: Quantity,
    pub max_quantity: Quantity,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub last_regeneration_tick: Option<Tick>,
    pub extraction_slots: NonZeroU8,
    pub extraction_duration_ticks: NonZeroU32,
    #[serde(default)]
    pub quality: Option<WaterQuality>, // None on non-water commodities; Some(Clean) default for water
}
```

`ResourceSource` is registered on `EntityKind::Facility || EntityKind::Place` at `crates/worldwake-core/src/component_schema.rs:1787`; no registration change needed.

Extend `ResourceSourceDef` (`crates/worldwake-cli/src/scenario/types.rs:820-831`) with `quality: Option<WaterQuality>` (defaulting to `Clean` for water commodities, `None` otherwise) and propagate through the `set_component_resource_source` construction in `crates/worldwake-cli/src/scenario/mod.rs:496-507`. Use `#[serde(default)]` so existing RON scenarios deserialize unchanged.

### D2. Quality-aware Drink (relief scaling + dirtiness penalty)

`commit_drink` (`crates/worldwake-systems/src/needs_actions.rs:1112-1121`) delegates to the consumable-effect path that reads `CommodityConsumableProfile.thirst_relief_per_unit` from the item lot's commodity (`crates/worldwake-core/src/items.rs:139`). After D3 lands, the consumed lot carries `Option<WaterQuality>`. Drink commit:

- Looks up the actor's `WaterToleranceProfile` (D5).
- For `Some(quality)` on the lot: multiplies the commodity-intrinsic `thirst_relief_per_unit` by `tolerance.thirst_relief_factor(quality)` (a `Permille`); raises `dirtiness` by `tolerance.dirtiness_penalty(quality)` (a `Permille`).
- For `None` (non-water commodity): preserves existing behavior (no quality scaling, no extra dirtiness).
- `Clean` yields `thirst_relief_factor = 1000` and `dirtiness_penalty = 0` by default — behaviorally neutral.

Depletion is already gated by `available_quantity`; no precondition change is needed (quality is a utility axis, not a gate). The Authoritative-to-AI Impact Analysis below enumerates the impact.

### D3. `ItemLot.quality` propagation at extraction commit

Extend `ItemLot` (`crates/worldwake-core/src/items.rs:317`) with `#[serde(default)] pub quality: Option<WaterQuality>` (None for non-water commodities). `Option<WaterQuality>` is compatible with `ItemLot`'s existing derives (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`).

At extraction commit (`apply_harvest_resource` in `crates/worldwake-systems/src/production_actions.rs:956-1067`), read the source's `quality` and write it onto the produced `ItemLot`. The water lot then carries its origin source's quality through the inventory until Drink consumes it (D2).

Per FND-28, the field is added — no parallel "untyped water lot" path is preserved.

### D4. Quality observation on `SourceReliability` + `EventTag::ResourceSourceQualityObserved`

Extend `ReliabilityRecord` (`crates/worldwake-core/src/experience.rs:79-98`) with quality observation fields, mirroring the existing capacity-observation pattern:

```rust
pub struct ReliabilityRecord {
    // existing fields …
    pub last_observed_capacity: u16,
    pub last_observed_capacity_tick: Tick,
    // new:
    #[serde(default)]
    pub last_observed_quality: Option<WaterQuality>,
    #[serde(default)]
    pub last_observed_quality_tick: Tick,
}

impl ReliabilityRecord {
    pub fn observe_quality(&mut self, quality: WaterQuality, tick: Tick) {
        self.last_observed_quality = Some(quality);
        self.last_observed_quality_tick = tick;
    }
}
```

Perception (`crates/worldwake-systems/src/perception.rs:174`, where `observe_capacity` is already called) extends the same co-located-observation path with `observe_quality` when the source is a water source with a `Some(quality)`. The write is gated on co-location (FND-14A); remote sources retain their stale quality belief until the agent observes them.

Add `EventTag::ResourceSourceQualityObserved` to `crates/worldwake-core/src/event_tag.rs` and emit it at the same observation site, carrying source attribution analogous to `SourceExpectationFailurePayload`.

`SourceReliability::enforce_limits` (already at `experience.rs:180-203`) decays both the capacity and quality observations together because they share the same parent `ReliabilityRecord` keyed by `SourceKey` and pruned by `last_attempt_tick` over `PreferenceProfile.memory_retention_ticks`.

Candidate ranking (the existing `source_composite_rank` in `crates/worldwake-ai/src/source_composite.rs`) gains a new factor for quality belief, combined with the existing `trust_factor`, `wait_factor`, and `capacity_factor` via `compose_factors`. The new factor reads `last_observed_quality` + freshness + the agent's `WaterToleranceProfile` (D5) to discount muddy/stale sources by the agent's per-quality tolerance. The discount decays with belief freshness so a long-stale "it was muddy" belief eventually permits a re-check (same model as the existing capacity-observation freshness).

### D5. `WaterToleranceProfile` universal per-agent component

New universal profile component in `worldwake-core` (precedent: 35 existing profile components, including `MetabolismProfile`, `PerceptionProfile`, `CognitiveProfile`, `PreferenceProfile`, `MemoryCapacityProfile`):

```rust
// crates/worldwake-core/src/water_tolerance_profile.rs (new module)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaterToleranceProfile {
    /// Per-quality relief multiplier applied to `CommodityConsumableProfile.thirst_relief_per_unit`.
    /// Clean = 1000‰ (neutral); Stale, Muddy < 1000‰.
    pub thirst_relief_factor: BTreeMap<WaterQuality, Permille>,
    /// Per-quality dirtiness penalty added to `HomeostaticNeeds::dirtiness` on Drink commit.
    /// Clean = 0‰; Stale, Muddy > 0‰.
    pub dirtiness_penalty: BTreeMap<WaterQuality, Permille>,
}

impl Component for WaterToleranceProfile {}

impl Default for WaterToleranceProfile {
    fn default() -> Self {
        // Default tolerance: Clean is neutral; Stale halves relief and adds a small dirtiness
        // penalty; Muddy further reduces relief and adds a larger penalty. Hardy/fragile
        // agents override via scenario authoring.
        Self {
            thirst_relief_factor: BTreeMap::from([
                (WaterQuality::Clean, Permille::new(1000).unwrap()),
                (WaterQuality::Stale, Permille::new(700).unwrap()),
                (WaterQuality::Muddy, Permille::new(450).unwrap()),
            ]),
            dirtiness_penalty: BTreeMap::from([
                (WaterQuality::Clean, Permille::new(0).unwrap()),
                (WaterQuality::Stale, Permille::new(80).unwrap()),
                (WaterQuality::Muddy, Permille::new(200).unwrap()),
            ]),
        }
    }
}
```

Universal-profile contract per `docs/spec-drafting-rules.md` Section 5:
- Register on `EntityKind::Agent` in `crates/worldwake-core/src/component_schema.rs` with generated `set_component_water_tolerance_profile` / `get_component_water_tolerance_profile` accessors.
- Add `water_tolerance_profile: Option<WaterToleranceProfile>` to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`.
- In `spawn_agent()` (`crates/worldwake-cli/src/scenario/mod.rs`), apply `let tolerance = agent_def.water_tolerance_profile.unwrap_or_default(); txn.set_component_water_tolerance_profile(agent_id, tolerance)?;` (universal pattern, always applied with default).
- Runtime access on known agents uses `expect()` per the universal contract.

Add a `GoalBeliefView` accessor `water_tolerance_profile(agent: EntityId) -> Option<WaterToleranceProfile>` in `crates/worldwake-sim/src/belief_view.rs` so the ranking layer can read the actor's tolerance (the agent is the planner's self-authoritative scope per FND-14B). Provide the `RuntimeBeliefView` backing impl in `crates/worldwake-sim/src/per_agent_belief_view.rs`, following the existing per-impl pattern (no `impl_goal_belief_view!` macro is used in the codebase; individual impl blocks are the convention).

### D6. Basin refill quality preference

Implemented by `archive/tickets/S177WATSRCQUA-006.md`: `crates/worldwake-systems/src/item_decay.rs::first_colocated_water_source` now **prefers the cleanest** colocated water source by iterating all colocated water sources at the place and returning the one whose `quality` is best (`Clean < Stale < Muddy` per `WaterQuality::Ord`), tie-breaking deterministically by entity id (BTreeMap iteration order).

`next_wash_basin_refill` now adds to the basin's `dirtiness_level` when transferring water from a `Some(Muddy)` or `Some(Stale)` source. The per-quality increment is owned by `WashBasinState.dirty_water_refill_penalty: BTreeMap<WaterQuality, Permille>` with a `#[serde(default)]` default; this stays on the basin to honor FND-26 state cohesion — the basin owns its dirtying mechanics, not the source.

This couples to S176 D2's wash-effectiveness gate: muddy-water refill raises basin dirtiness, which then reduces wash effectiveness via the `max_effective_dirtiness` boundary.

### D7. Survival forensics: `SourceAcquisitionFailure` record

Extend `SurvivalForensicExtractor` in `crates/worldwake-ai/src/survival_forensics.rs` with a `SourceAcquisitionFailure` derived forensic record, modeled on the existing `DegradedSelfCareOpportunity` record:

```rust
pub struct SourceAcquisitionFailure {
    pub tick: Tick,
    pub source: EntityId,
    pub cause: SourceFailureCause,
    pub outcome: SourceFailureOutcome,
}

pub enum SourceFailureCause {
    Depleted,           // available_quantity == 0 at start
    QualityRejected,    // tolerance discount drove the candidate below ranking floor
}

pub enum SourceFailureOutcome {
    DrankAnyway,        // accepted muddy/stale water
    TraveledToFallback, // selected a believed-better fallback
    GaveUp,             // no fallback met threshold
}
```

Implemented by `archive/tickets/S177WATSRCQUA-007.md`: this is a **derived view** populated from the event log and action-trace events. Causes come from existing `EventTag::SourceExpectationFailure` (depletion) and new `EventTag::ResourceSourceQualityObserved` (quality) emissions; outcomes come from same-window action trace response (`drink`, `travel`, or no response). It is not authoritative state. The extractor receives the append-only event log as a read-only input during `observe()` and stores records on `CriticalWindowFrame.source_acquisition_failures`.

Note: the original spec proposed a "contested" cause for queue contention; that is dropped here because (a) `ResourceExtractionQueues` queue waits are already surfaced as `wait_factor_permille` in the source-rank composite and as `observe_wait` on `ReliabilityRecord`, and (b) widening the forensic record's cause set into contention duplicates the existing queue substrate. Re-add only if a future spec proves a missing forensic surface for queue-contention starvation specifically.

### D8. CLI player-POV gating for source quality

Implemented by `archive/tickets/S177WATSRCQUA-008.md`: before that ticket, the observer surfaced only `available_quantity > 0` boolean presence in its local survival summary and anomaly-support helper. The landed player-POV gating follows these rules:

- When the controlled agent is co-located with a water source, surface its `quality` (FND-14A direct read of authoritative state is lawful because perception would deliver the same fact same-tick).
- When the controlled agent is *not* co-located, surface only what the agent's `SourceReliability.last_observed_quality` records, with freshness annotation (e.g., "Muddy (observed 200 ticks ago)").
- Apply identical legality to AI agents — agent symmetry per FND-19.

The CLI did not invent a new `GoalBeliefView` accessor. The observer-local helper keeps the same legal source split explicitly: co-located reads use authoritative state under FND-14A, and remote reads use the controlled agent's `SourceReliability.last_observed_quality` record.

## Authoritative-to-AI Impact Analysis

Per CLAUDE.md's Authoritative-to-AI Impact Rule, this spec adds an authoritative field on `ResourceSource` (`quality`) read by the AI ranking composite, and modifies basin refill source-selection. The 7-point checklist:

1. **`get_affordances`** — pass. Quality is a property of an already-affordable source. No new precondition gates affordance enumeration.
2. **`generate_candidates`** — pass. Candidate emission already reads source via the existing `resource_source(entity)` accessor and routes through `AcquireCommodity { commodity: Water, … }` (no new `GoalKind`). The quality field flows through transparently after D1.
3. **`search_plan`** — pass. No new search-control change. Quality affects ranking via the source-rank composite (D4), not search.
4. **`BestEffort` action start** — flag for ticket review: basin refill (`next_wash_basin_refill`) gains a quality-preference selection (D6); verify no new revalidation path is required. Drink and Extraction action starts are unchanged (quality is a utility consequence, not a precondition).
5. **`handle_plan_failure`** — pass. No new failure class is added; quality is a utility axis, not a gate. Existing `SourceExpectationFailure` covers the depletion failure case; quality-rejected candidates simply rank below the floor.
6. **Payload revalidation** — pass. No new payload-synthesized action. Quality is read at commit time through the existing `ItemLot.quality` field (D3) which is populated at extraction commit and remains static through the inventory.
7. **Golden tests** — flag: new goldens (Scenario Validation below) must run alongside the existing `survival_preferences.rs`, `quantity_aware_acquisition.rs`, `ai_decisions.rs::golden_local_depleted_source_regenerates_without_spurious_failure_memory`, `source_composite.rs::empty_but_fresh_observation_demotes_depleted_source`, and `survival-basin-competition-1440.ron` to confirm no regressions on the depletion half.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Water quality cannot make one source worse than another; agents cannot drink muddy water at a cost; basin refill cannot prefer clean water; the scarcity ↔ quality emergent tradeoff is absent. (Depletion-discovery is already proven; this spec does NOT re-introduce it.)
2. **New entities/relations/records**: `WaterQuality` enum; `quality: Option<WaterQuality>` field on `ResourceSource`; `quality: Option<WaterQuality>` field on `ItemLot`; `last_observed_quality` + `last_observed_quality_tick` fields on `ReliabilityRecord`; new universal `WaterToleranceProfile` component; new `EventTag::ResourceSourceQualityObserved` variant + payload; new `SourceAcquisitionFailure` derived forensic record + cause/outcome enums; new `dirty_water_refill_penalty` map on `WashBasinState`.
3. **Actions that mutate them**: Extraction commit (`apply_harvest_resource`) reads source `quality`, writes it onto the produced `ItemLot`. Drink commit reads `ItemLot.quality` + `WaterToleranceProfile`, scales relief, adds dirtiness. Perception (`observe_capacity` site) writes `observe_quality` into the agent's `SourceReliability` on co-located observation; emits `EventTag::ResourceSourceQualityObserved`. Basin refill (`next_wash_basin_refill` / `first_colocated_water_source`) reads source `quality`, transfers water, optionally raises `WashBasinState.dirtiness_level`.
4. **Information production and travel**: Quality is observed locally (FND-14A). Quality belief travels by the agent's memory (`SourceReliability`) with freshness, decaying via `SourceReliability::enforce_limits` over `PreferenceProfile.memory_retention_ticks`. `ResourceSourceQualityObserved` events are append-only.
5. **Conserved quantities**: Water `available_quantity` (already conserved, regenerating; unchanged by this spec). Quality is a *property*, not a conserved quantity. No water created from nothing.
6. **Scarce capacities and contention**: `extraction_slots` (existing, unchanged). A *clean, non-depleted* source becomes the contested affordance — but contention is already mediated by `ResourceExtractionQueues`.
7. **Partial failures and aftermath**: Drink from muddy water → partial thirst relief + raised dirtiness (partial outcome, not a failure). Reaching a believed-clean source and finding it muddy → quality-observation belief update + ranking re-evaluation on next tick + optional `SourceAcquisitionFailure` (QualityRejected) record. Stale clean-belief → arrival surprise, no failure path.
8. **Positive feedback loops**: (a) Many drinkers → faster depletion of the clean source → more fallback travel to muddy → more agents accept the muddy tradeoff. (b) Quality belief → all agents avoid a believed-muddy source → it goes undrunk while the clean source over-drains.
9. **Concrete dampeners** (physical, not numeric clamps): (a) `regeneration_ticks_per_unit` — sources refill over real time. (b) Quality observation freshness decay via `SourceReliability::enforce_limits` — a stale "it was muddy" belief eventually permits a re-check. (c) Fallback sources exist with travel cost — distance dampens over-draw on any single source. (d) Muddy water is still drinkable (reduced relief by tolerance factor), so thirst never deadlocks — the loop diverts into dirtiness aftermath, not collapse. (e) `extraction_slots` caps simultaneous draw. (f) Per-agent `WaterToleranceProfile` diversity — some agents drink muddy when others would travel further, so the system never collapses to a single coordinated choice (FND-22 → FND-11 dampener).
10. **Agent learning**: Quality observation memory is concrete, per-agent, on `SourceReliability.last_observed_quality` — acquired by perception at co-located observation, decaying via `SourceReliability::enforce_limits` over `PreferenceProfile.memory_retention_ticks` (the actual project decay mechanism — *not* the S151 testimony observation-counter model), revisable by new observation. Abstract-but-legal agent-local summary (FND-22A).
11. **How agents can be wrong**: Believe a source is clean when it is muddy (stale) → travel wasted → correct on arrival. Believe a recovered source is still muddy (over-stale) → avoid it needlessly until belief decays. Different agents disagree about a source's quality after observing it at different ticks.
12. **Lifecycle states**: Source quality: authored, optionally degradable in a future spec (static here). Quality belief: `Fresh → Stale → Expired` per the same freshness mechanism as capacity belief. `ItemLot.quality`: set at extraction commit, static for the lot's lifetime.
13. **Temporal resolution**: Quality reads at extraction start; quality write to lot at extraction commit; basin refill quality check at item-decay tick; quality belief writes at observation tick. Concurrent observation of the same source by multiple agents is each agent's per-agent `SourceReliability` write — no cross-agent contention.
14. **Boundary conditions**: Internal sources only. Cross-boundary water import (drought relief, aqueducts) is held `specs/S62` territory — explicitly out of scope.
15. **Derived views**: `SourceAcquisitionFailure` (forensic). Quality composite-rank factor (derived per-decision). The quality-belief read on `ResourceReliability.last_observed_quality` is per-actor derived from the agent's stored belief; not stored authority outside the agent's component.
16. **Causal records**: `EventTag::ResourceSourceQualityObserved` events (new). Existing `EventTag::SourceExpectationFailure` for depletion (reused, not duplicated). `SourceAcquisitionFailure` records in the critical window. Reconstruct "why did this agent drink muddy water?" from: per-agent `WaterToleranceProfile` + clean-well depletion observation history + muddy-source quality observation + decision trace.
17. **Target patterns**: Well depletes (already proven) → agent observes empty (already proven) → agent travels to backup → backup observed muddy on arrival (new) → drink muddy or travel further per tolerance (new). Recovered source re-checked after belief decays. Two agents disagree about a source's quality after observing it at different ticks.
18. **Save/load and replay**: New enum + field on `ResourceSource` + field on `ItemLot` + two fields on `ReliabilityRecord` + new `WaterToleranceProfile` component + one event variant + payload + new forensic record + new field on `WashBasinState` — all replay-deterministic standard state, all using `#[serde(default)]` for save/RON compatibility.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `ResourceSource.quality` | Stored authoritative | Field on existing component on source entity |
| `ItemLot.quality` | Stored authoritative | Field on existing component on lot entity |
| `ReliabilityRecord.last_observed_quality{,_tick}` | Stored authoritative agent-local belief | Per-agent; fallible, decaying (FND-22A) |
| `WaterToleranceProfile` | Stored authoritative profile parameter | Per-agent; universal component |
| `WashBasinState.dirty_water_refill_penalty` | Stored authoritative scenario parameter | Per-basin |
| `EventTag::ResourceSourceQualityObserved` (and its payload) | Stored event-payload | Authoritative on emission |
| `SourceAcquisitionFailure` records | Derived forensic state | View; not authoritative |
| Quality belief-view accessor reads | Derived per-actor view | View; not authoritative |
| Quality composite-rank factor | Derived | Computed over belief state at emission |

## Planner-formalism analysis

Plain GOAP. Fallback-source selection is candidate emission discounted by the source-rank composite (existing) extended with a quality factor (D4) — not HTN decomposition. No multi-stage method, info-gathering stage, or budget-exhaustion need that flat search cannot handle. The "arrive at muddy, decide drink-or-travel-further" loop is ordinary replanning on the next tick after the quality observation lands, not a method. Fallback: N/A. Information reads: source quality and per-agent tolerance are belief-backed or actor-self per FND-14B. Enforced declarations only: quality field, tolerance profile, reliability quality fields, and event variant all have live consumers (D2, D4, D6, D7, D8). Proof: scenarios below.

## Belief-View Accessor Source-Class Declarations

| Accessor | Source class | Stale/unknown behavior |
|----------|--------------|------------------------|
| `resource_source(entity) -> Option<ResourceSource>` (existing) | FND-14A co-located; belief-backed remote | Quality flows through this accessor after D1; `None` on remote without belief |
| `water_tolerance_profile(agent) -> Option<WaterToleranceProfile>` (new) | Self (actor's own profile) | Universal-profile: `expect()` on known agents per the universal contract |
| `source_reliability_record(agent, source_key) -> Option<ReliabilityRecord>` (new, sugar over the existing `source_reliability(agent)`) | Self (actor's own learned state) | `None` if source has not been observed |

The spec does **not** introduce a separate `water_source_available` or `water_source_quality` accessor — the existing `resource_source(entity)` accessor already returns the full struct including the new `quality` field after D1 lands. The spec does **not** introduce a source-scoped `source_reliability(source)` because it would collide with the existing agent-scoped `source_reliability(agent)` at `belief_view.rs:730`; reads of per-source quality belief go through the agent's `SourceReliability.sources` map keyed by `SourceKey { entity, commodity }`.

Ownership/control of the source (whose well it is, who may draw) remains belief-gated per FND-14A — a separate axis from physical quality.

## Agent Profile Scenario Contract

`WaterToleranceProfile` is a **universal** component on `EntityKind::Agent` and follows the universal-profile contract:

1. Defined in `worldwake-core` with `Default` impl (D5).
2. Registered in `crates/worldwake-core/src/component_schema.rs` with kind filter `|kind| kind == EntityKind::Agent`.
3. Added to `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs`) as `water_tolerance_profile: Option<WaterToleranceProfile>` with `#[serde(default)]`.
4. Applied in `spawn_agent()` (`crates/worldwake-cli/src/scenario/mod.rs`) via `unwrap_or_default()` — always inserted.
5. Runtime access on known agents uses `expect()` (universal-profile contract).

`WaterQuality` on the source is authored via `ResourceSourceDef.quality` (D1). `WashBasinState.dirty_water_refill_penalty` is authored via the existing place/basin scenario contract (D6).

## Component Registration

| Component | Crate | Registration Kind Filter | Source |
|-----------|-------|---------------------------|--------|
| `ResourceSource` (extended with `quality`) | core | `EntityKind::Facility \|\| EntityKind::Place` | Existing registration; field addition only |
| `ItemLot` (extended with `quality`) | core | `EntityKind::ItemLot` | Existing registration; field addition only |
| `WashBasinState` (extended with `dirty_water_refill_penalty`) | core | (existing basin filter) | Existing registration; field addition only |
| `ReliabilityRecord` fields (on `SourceReliability`) | core | `EntityKind::Agent` (via parent component) | Existing registration; field additions only |
| `WaterToleranceProfile` | core | `EntityKind::Agent` | **NEW** — register in `component_schema.rs` with universal-profile insert/get accessors |

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Drink/extraction handler (`worldwake-systems`) | `ResourceSource` (quantity + quality), `ItemLot.quality`, `WaterToleranceProfile`, `CommodityConsumableProfile` | `available_quantity`, water lot `quality`, `HomeostaticNeeds` (thirst, dirtiness) |
| Item-decay basin refill (`worldwake-systems`) | colocated `ResourceSource` quality/quantity, `WashBasinState.dirty_water_refill_penalty` | `WashBasinState` (clean units + dirtiness from muddy water), source quantity |
| Perception (`worldwake-systems`) | colocated `ResourceSource.quality` | `ReliabilityRecord.last_observed_quality{,_tick}`, `EventTag::ResourceSourceQualityObserved` emission |
| Source-rank composite (`worldwake-ai`) | belief-view source quality + reliability + `WaterToleranceProfile` | None (read-only emission) |
| Survival forensics (`worldwake-ai`) | event/trace log | `SourceAcquisitionFailure` records |
| Observer/CLI (`worldwake-cli`) | belief-view source quality, `SourceReliability.last_observed_quality` for controlled agent | None |

No system commands another.

## Scenario Validation (FND-31)

**Focused branch goldens (implemented by `archive/tickets/S177WATSRCQUA-009.md`, in addition to the existing depletion-discovery goldens which must continue to pass):**

- **`survival-water-quality-on-arrival.ron`** — agent has a seeded recent belief that a remote well is clean, travels to it, finds it `Muddy` on arrival (FND-14A observation), records a quality belief, and emits `EventTag::ResourceSourceQualityObserved` without pre-arrival omniscient correction. The focused golden also covers deterministic replay. *Direct quality-axis proof for canonical scenario D extended.*
- **`survival-dirty-water-tolerance-tradeoff.ron`** — two agents share identical source beliefs and world state but differ in `WaterToleranceProfile`: a hardy neutral-tolerance agent selects the local Muddy source, while a fragile agent selects the clean fallback through the quality-aware `SourceComposite` decision-trace boundary. This proves FND-22 diversity at the strongest stable planner-facing layer.
- **`survival-muddy-basin-refill.ron`** — only colocated water source is muddy; basin refill draws from it, raises `WashBasinState.dirtiness_level`, and the subsequent wash effectiveness degrades per S176. Asserts basin-side quality coupling.

**1440-tick CI-owned collision scenario:**

- **`survival-quality-degrading-1440.ron`** — several agents share a finite, regenerating clean source, a muddy backup, and a farther clean fallback over 1440 ticks. Depletion pressure drives fallback travel (proven path), the muddy backup is observed through co-located perception, agents with different `WaterToleranceProfile` make different drink-location choices, basin dirtiness rises from muddy-water refill, and at least one critical-thirst window forms. The golden seeds source/location entity beliefs but strips water quality from those snapshots, so fallback selection is belief-backed while the Muddy fact still enters through `ResourceSourceQualityObserved` rather than omniscient quality memory. Assertions prove scenario completion, quality belief acquisition, quality-tolerance choice diversity, muddy-refill basin dirtiness, critical-window forensics, depletion-only baseline preservation, and replay equivalence.

**Existing goldens that must continue to pass (regression coverage for the proven depletion half):**

- `crates/worldwake-ai/tests/scenarios/ai_decisions.rs::golden_local_depleted_source_regenerates_without_spurious_failure_memory`
- `crates/worldwake-ai/src/source_composite.rs::empty_but_fresh_observation_demotes_depleted_source` (unit test)
- `crates/worldwake-ai/tests/scenarios/survival_preferences.rs` (consumes `EventTag::SourceExpectationFailure`)
- `crates/worldwake-ai/tests/scenarios/quantity_aware_acquisition.rs` (mid-second-action depletion surface)
- `scenarios/survival-basin-competition-1440.ron`

**Illegal paths this spec must not produce:** a planner candidate for a remote source's quality with no belief carrier; instant omniscient quality-belief correction before arrival; a global `water_scarcity` scalar; any sickness/wound from water (deferred); water appearing without a source/regeneration path; a parallel "depletion belief" surface coexisting with `SourceReliability.last_observed_capacity` (FND-28).
