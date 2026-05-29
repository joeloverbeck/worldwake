# S177: Water Source Quality, Depletion Observation, and Reliability Memory

## Summary

`ResourceSource` (`crates/worldwake-core/src/production.rs`) already carries `available_quantity`, `max_quantity`, `regeneration_ticks_per_unit`, `extraction_slots`, and `extraction_duration_ticks`. Water extraction and basin refill from a colocated water source already consume `available_quantity` (`crates/worldwake-systems/src/item_decay.rs::next_wash_basin_refill`). So depletion **state** exists. What does **not** exist:

1. **The depletion-discovery loop.** An agent who believes a well has water travels to it, finds it dry, and must update belief and use a fallback. This is canonical regression scenario D in `docs/FOUNDATIONS.md` (Rumor → Travel → Empty Source → Belief Correction → Replan), but it is not proven for water as a survival need. Today the planner has no belief-gated notion of "this source is depleted/unreliable"; it re-emits the same candidate and only fails at extraction start.
2. **Water quality.** `ResourceSource` has no quality field. Clean, muddy, and stale water are indistinguishable. Basin refill draws from *any* colocated water source regardless of quality, and there is no dirty-water-versus-travel tradeoff.

This spec adds (a) a concrete `WaterQuality` field on water `ResourceSource`s with downstream wash/thirst consequences, and (b) belief-backed **source reliability memory** so agents learn "this source was dry/dirty at tick T" and prefer fallbacks — closing canonical scenario D for water. It is the second slice of the deferred Cluster 1 material-degradation wave (`specs/IMPLEMENTATION-ORDER.md`), deferred by the 2026-05-26 second-iteration triage and now ripe after S174.

**Deliberate scope discipline (critical reassessment of the source report):** the report proposes that "drinking unsafe water can reduce thirst but cause later consequence." This spec implements only the **immediate, concrete** consequences — reduced thirst relief, raised dirtiness, basin-refill quality preference. It **defers any "unsafe water → sickness/wound" path**: a disease/illness consequence is a new carrier (`FoodSickness`/`DigestiveDistress`) that FND-5 and the report's own MUST-NOT say should not be added until it is a proven concrete consequence carrier with source, trace, recovery, and proof. Water quality here is a *utility/effectiveness* axis, not a disease vector.

## Phase

Phase 7: Consequence Carriers

## Status

📝 DRAFT — authored, awaiting activation (held adjunct wave; see `specs/IMPLEMENTATION-ORDER.md`)

## Crates

- `worldwake-core` (add `WaterQuality` to `ResourceSource` or a sibling component on water sources)
- `worldwake-sim` (event payloads for observed depletion / observed quality; reuse existing event-log substrate)
- `worldwake-systems` (Drink relief and basin refill scale with `WaterQuality`; extraction emits depletion-observed evidence on empty)
- `worldwake-ai` (source reliability memory extends the existing learned-source substrate; candidate generation discounts believed-unreliable sources and emits fallbacks; survival forensics record source-depletion/quality failures)
- `worldwake-cli` (scenario contract for `WaterQuality`; player-POV gating for source quantity/quality observation)

## Dependencies

- `archive/specs/S79-resource-source-consumption-affordances.md` — provides the `ResourceSource` extraction/consumption affordance substrate this spec extends.
- `archive/specs/S38-learned-route-source-preferences.md` — provides the learned **source-preference** substrate this spec extends with reliability memory (rather than introducing a new memory component). *Reassessment must confirm the exact `LearnedSourcePreferences` shape before ticket decomposition.*
- `archive/specs/S151-testimony-reliability-and-route-preferences.md` — provides the testimony/reliability precedent for how a believed source-reliability fact is acquired, decays, and is discounted.
- `archive/specs/S129-place-dirtiness-facility-wear.md` + `archive/specs/S176-sanitation-facility-degradation-consequences.md` — basin refill quality preference and the `WashBasinState.dirtiness_level` consequence S177 feeds (dirty water dirties the basin).
- `archive/specs/S120-survival-critical-window-forensics.md` — `SurvivalForensicExtractor` extended with source-failure records.

## Design Goals

- Water sources carry concrete **quality** (`Clean`, `Stale`, `Muddy`) as authoritative state. Drinking lower-quality water gives **less thirst relief** and **raises dirtiness**; clean water is strictly preferable when known and reachable.
- Depletion is **discovered locally**: an agent who reaches a dry source observes it (FND-14A), records a belief that it was dry at this tick, and the planner discounts it on subsequent ticks until belief decays or new evidence arrives.
- **Source reliability memory** is belief-backed and fallible: agents may hold stale "the well is full" beliefs and waste a trip, then correct. This is canonical scenario D, not omniscient correction.
- Basin refill (item-decay system) **prefers clean colocated water**; refilling from muddy water raises the basin's `dirtiness_level` (couples to S176's wash-effectiveness gate).
- Player and AI share identical source legality; the CLI surfaces only source quantity/quality the controlled agent lawfully perceives.

## Non-Goals

- **No water-borne disease / illness / sickness wound.** Explicitly deferred (see Summary). `Unsafe`/`Contaminated` quality tiers are **not** introduced in this spec; only `Clean`/`Stale`/`Muddy` as utility tiers. An illness carrier is a future spec triggered only if a concrete consequence-with-recovery is designed.
- **No abstract "water scarcity level."** The *source* is scarce (depletes), per FND-3.
- **No new global hydrology / weather / drought system.** Drought as a boundary process belongs to held `specs/S62`; this spec creates scarcity internally via depletion and quality only.
- **No new contention queue.** Extraction-slot contention is the existing `ResourceSource.extraction_slots` substrate.
- **No HTN method.** Fallback-source selection is flat GOAP candidate emission discounted by belief.
- **No backward-compatibility shim.** Sources without an authored quality default to `Clean` (behaviorally neutral); the field is added, not aliased (FND-28).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | A shared well depletes under multi-agent draw → some travel to a muddy backup → drink worse water, get dirtier, queue, or trade → scarcity behavior emerges from a finite source, not a scarcity dial |
| FND-3 (Concrete state over abstract scores) | Quality is a concrete per-source enum; reliability is belief about concrete prior observations; no `water_score` |
| FND-4 (Source/sink) | Drinking and refill draw `available_quantity`; regeneration is the existing explicit source process |
| FND-7 / FND-15 (Locality / knowledge travels) | Depletion/quality observed at the source (FND-14A); reliability beliefs travel by memory and (optionally) testimony with provenance/freshness |
| FND-14 / FND-14A / FND-14B | Remote source quantity/quality is belief-backed; co-located is same-tick observation; the planner cannot read remote `available_quantity` as authoritative |
| FND-16 (Ignorance/contradiction first-class) | Agents hold stale "well is full" beliefs; two agents can disagree about a source's reliability |
| FND-17 (Surprise from violated expectation) | Reaching a believed-full well and finding it dry is the expectation violation that updates belief |
| FND-19 (Agent symmetry) | Same extraction/drink legality and quality effects for human and AI |
| FND-21 (Intentions revisable) | A planned Drink at a depleted source fails at start; the agent replans to a fallback |
| FND-22A (Learning is concrete state) | Source reliability memory is concrete, owned per-agent, with accountable acquisition and decay — not hidden global adaptation |
| FND-26 (Systems via state) | Drink/refill read source state and write it; planner reads belief; forensics reads log; no direct calls |
| FND-28 (No backcompat) | Quality field added; no parallel old/new water path |
| FND-29 / FND-29A | "Why did this agent drink muddy water?" answerable from belief that the clean well was dry + source quality at decision tick |
| FND-31 (Validation) | Focused goldens + 1440-tick degrading-water collision scenario |

This spec realizes **Canonical Regression Scenario D** (Rumor → Travel → Empty Source → Belief Correction → Replan) for the water survival need.

## Deliverables

### D1. `WaterQuality` on water sources

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WaterQuality {
    Clean,
    Stale,
    Muddy,
}
```

Carried on water `ResourceSource`s. **Reassessment decision (pinned at reassess-spec time):** either a new optional field `quality: Option<WaterQuality>` on `ResourceSource` (defaulting to `Clean` for water commodities) or a sibling `WaterSourceQuality` component on the source entity. Preference: the field on `ResourceSource`, since quality is intrinsic to the source and every water consumer already reads `ResourceSource`. No magic numbers: quality maps to relief/dirtiness multipliers expressed in `Permille` profile parameters (D4).

### D2. Quality- and depletion-aware Drink

The Drink action (`needs_actions.rs`) currently consumes an item lot. Where water is drawn directly from a source (extraction → item lot → drink, per S79), the produced water lot carries the source's quality, and Drink relief scales by quality:

- `Clean` → full `thirst_relief_per_unit`.
- `Stale`/`Muddy` → reduced thirst relief and added dirtiness, by `Permille` factors from the consumable/quality profile (D4).

Depletion is already gated by `available_quantity`; D3 adds the *observation/belief* layer.

### D3. Depletion + quality observation → reliability memory

- On reaching a water source, a co-located agent observes `available_quantity` and `quality` (FND-14A) and records a belief with provenance + claimed tick.
- When extraction fails because the source is empty, emit `EventTag::ResourceSourceDepletedObserved` (the agent's local observation), and write/refresh the agent's **source reliability memory** (extends `LearnedSourcePreferences`, per S38): "source X dry at tick T."
- Candidate generation discounts a believed-depleted or believed-muddy source in favor of a believed-better fallback, using only belief-backed state. The discount decays with belief freshness (per S151 reliability precedent) so a long-stale "it was dry" belief eventually permits a re-check.

### D4. Quality/relief profile parameters

Quality→effect mapping lives in profile parameters (no inline constants):

- A `WaterQualityProfile` (or fields on the existing consumable/metabolism profile) giving, per quality tier, a `thirst_relief_factor: Permille` and `dirtiness_penalty: Permille`.
- Reassessment pins whether this is per-agent (tolerance varies — a hardy agent suffers less from muddy water) or a world constant table. Preference: per-agent, to honor FND-22 agent diversity and the report's "profile-driven thresholds for willingness to use unsafe affordances."

### D5. Basin refill quality preference

Extend `item_decay.rs::next_wash_basin_refill` (and `first_colocated_water_source`) to prefer the cleanest colocated water source. Refilling from `Muddy` water adds to the basin's `dirtiness_level` (couples to S176 D2's wash-effectiveness gate), making dirty-water refill a real tradeoff rather than free.

### D6. Survival forensics for source failure

Extend `SurvivalForensicExtractor` with a `SourceAcquisitionFailure` record (depleted / too-dirty / contested), analogous to S176's `DegradedSelfCareOpportunity`. Derived forensic state, never authoritative.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Water cannot run dry-and-be-discovered as a belief loop; quality cannot make one source worse than another; agents cannot learn a source is unreliable. Canonical scenario D is unproven for water.
2. **New entities/relations/records**: `WaterQuality` enum + field/component on water sources; `WaterQualityProfile` (or quality fields on existing profile); `EventTag::ResourceSourceDepletedObserved` + `ResourceSourceQualityObserved`; source-reliability entries in `LearnedSourcePreferences`; `SourceAcquisitionFailure` forensic record.
3. **Actions that mutate them**: Extraction/Drink read `available_quantity` + `quality`, draw quantity, scale relief, add dirtiness. Reaching/observing a source writes reliability beliefs. Basin refill reads quality, may raise basin dirtiness.
4. **Information production and travel**: Quality/quantity observed locally (FND-14A); reliability beliefs travel by the agent's memory and optional testimony (FND-15) with provenance/freshness; depletion-observed events are append-only.
5. **Conserved quantities**: Water `available_quantity` (already conserved, regenerating). Quality is a property, not a conserved quantity. No water created from nothing.
6. **Scarce capacities and contention**: `extraction_slots` (existing). A *clean, non-depleted* source becomes the contested affordance.
7. **Partial failures and aftermath**: Drink from muddy water → partial thirst relief + dirtiness (partial outcome). Extraction from empty source → failure + depletion-observed belief + `SourceAcquisitionFailure` record + fallback. Stale belief → wasted travel, corrected on arrival.
8. **Positive feedback loops**: (a) Many drinkers → faster depletion → more fallback travel/contention. (b) Reliability memory → all agents avoid a believed-bad source → it regenerates unused while the good source over-drains.
9. **Concrete dampeners** (physical, not numeric clamps): (a) `regeneration_ticks_per_unit` — sources refill over real time, so depletion is temporary. (b) Belief freshness decay (S151) — a stale "it was dry" belief eventually permits a re-check, so agents don't permanently abandon a recovered source. (c) Fallback sources exist with travel cost — distance dampens over-draw on any single source. (d) Muddy water is still drinkable (reduced relief), so thirst never deadlocks — the loop diverts into dirtiness aftermath, not collapse. (e) `extraction_slots` caps simultaneous draw.
10. **Agent learning**: Source reliability memory — concrete, per-agent, acquired from a local depletion/quality observation, decaying via the S151 freshness model, revisable by new observation. Abstract-but-legal agent-local summary (FND-22A).
11. **How agents can be wrong**: Believe a source is full/clean when it is dry/muddy (stale) → travel wasted → correct on arrival. Believe a recovered source is still dry (over-stale) → avoid it needlessly until belief decays.
12. **Lifecycle states**: Source: `Replenished ↔ Drawn ↔ Depleted` (by quantity + regeneration). Quality: authored, optionally degradable in a future spec (static here). Reliability belief: `Fresh → Stale → Expired` per freshness model.
13. **Temporal resolution**: Quantity/quality reads at extraction start; regeneration at the item-decay/regeneration tick; belief writes at observation tick. Concurrent draw on the last unit resolved by `extraction_slots` + existing tie-break.
14. **Boundary conditions**: Internal sources only. Cross-boundary water import (drought relief, aqueducts) is held `specs/S62` territory — explicitly out of scope.
15. **Derived views**: `SourceAcquisitionFailure` (forensic). Belief-view source quantity/quality accessors (per-actor derived). Reliability discount is a derived ranking input over belief state, not stored authority.
16. **Causal records**: `ResourceSourceDepletedObserved` / `ResourceSourceQualityObserved` events; reliability-belief acquisition records; `SourceAcquisitionFailure` in the critical window. Reconstruct "why did this agent travel to the muddy backup?"
17. **Target patterns**: Well depletes → agent observes → travels to muddy backup → drinks worse water, gets dirtier; recovered well re-checked after belief decays; two agents disagree about a source after observing it at different ticks.
18. **Save/load and replay**: New enum + field/component, profile params, two event variants, reliability-belief entries, forensic record — all replay-deterministic standard state.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `ResourceSource` (incl. `WaterQuality`) | Stored authoritative | Component on source entity |
| `WaterQualityProfile` / quality profile fields | Stored authoritative profile parameter | Per-agent (or world table — pinned at reassessment) |
| Source-reliability entries in `LearnedSourcePreferences` | Stored authoritative agent-local belief/learning state | Per-agent; fallible, decaying (FND-22A) |
| `EventTag::ResourceSourceDepletedObserved` / `…QualityObserved` | Stored event-payload | Authoritative on emission |
| `SourceAcquisitionFailure` records | Derived forensic state | View; not authoritative |
| Belief-view source quantity/quality accessors | Derived per-actor view | View; not authoritative |
| Reliability discount (ranking input) | Derived | Computed over belief state at emission |

## Planner-formalism analysis

Plain GOAP. Fallback-source selection is candidate emission discounted by belief-backed reliability — not HTN decomposition. No multi-stage method, info-gathering stage, or budget-exhaustion need that flat search cannot handle (the "discover it is dry, then replan" loop is ordinary replanning on the next tick after the start-failure, not a method). Fallback: N/A. Information reads: source quantity/quality and reliability are all belief-backed or same-tick-local. Enforced declarations only: quality field, profile params, reliability beliefs, and events all have live consumers. Proof: scenarios below.

## Belief-View Accessor Source-Class Declarations

| Accessor | Source class | Stale/unknown behavior |
|----------|--------------|------------------------|
| `water_source_available(source) -> Option<Quantity>` | FND-14A co-located; belief-backed remote | `None` if no belief and remote |
| `water_source_quality(source) -> Option<WaterQuality>` | FND-14A co-located; belief-backed remote | `None` if no belief and remote |
| `source_reliability(source) -> Option<ReliabilityBelief>` | Belief-backed (memory/testimony) with freshness | `None` if never observed/heard |

Accessors return `None` rather than reading remote authoritative state. Ownership/control of the source (whose well it is, who may draw) remains belief-gated per FND-14A — a separate axis from physical quantity/quality.

## Agent Profile Scenario Contract

If quality→effect parameters are per-agent (preferred), they live on a universal profile (`MetabolismProfile` or a new universal `WaterToleranceProfile`) with a `Default` impl, scenario-overridable via `AgentDef`. If a new component on `EntityKind::Agent` is introduced, it follows the universal-profile contract (added to `AgentDef`, `set_component_*` in `spawn_agent()`, `expect()` runtime access). `WaterQuality` on the source is authored via the place/source scenario contract. Reassessment pins the per-agent-vs-world-table choice.

## Component Registration

`WaterQuality` field on `ResourceSource` requires no new registration (existing component, field addition) — *unless* reassessment chooses the sibling-component form, in which case `WaterSourceQuality` is registered on the source entity-kind in `component_schema.rs`. Source-reliability extends the existing `LearnedSourcePreferences` component (no new registration). Any new agent profile follows the contract above.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Drink/extraction handler (`worldwake-systems`) | `ResourceSource` (quantity + quality), profile | `available_quantity`, water lot quality, `HomeostaticNeeds` (thirst, dirtiness) |
| Item-decay basin refill (`worldwake-systems`) | colocated `ResourceSource` quality/quantity | `WashBasinState` (clean units + dirtiness from muddy water), source quantity |
| Candidate emitter (`worldwake-ai`) | belief-view source quantity/quality + reliability | None (read-only emission) |
| Learned-source substrate (`worldwake-ai`) | depletion/quality observation events | source-reliability beliefs |
| Survival forensics (`worldwake-ai`) | event/trace log | `SourceAcquisitionFailure` records |

No system commands another.

## Scenario Validation (FND-31)

**Focused branch goldens:**

- **`survival-water-source-depleted.ron`** — agent believes a well is full, travels to it, finds it dry (FND-14A observation), records a reliability belief, and uses a believed-good fallback. Asserts belief update, candidate discount, no omniscient correction (the belief is wrong until arrival), deterministic replay. *Direct canonical-scenario-D proof.*
- **`survival-dirty-water-tradeoff.ron`** — clean well is depleted; only a muddy source remains; the agent drinks muddy water (reduced thirst relief + raised dirtiness) OR travels to a distant clean source depending on profile tolerance/pressure. Asserts quality-scaled relief, dirtiness penalty, and the profile-driven branch.

**1440-tick CI-owned collision scenario:**

- **`survival-degrading-water-1440.ron`** — several agents share a finite, regenerating clean source plus a muddy backup over 1440 ticks. Depletion drives fallback travel, contention (extraction slots), dirty-water drinking, and at least one critical-thirst window. Assertions prove: `available_quantity` changes and regenerates; reliability beliefs are acquired/decay; fallback selection has belief provenance (no omniscient target injection); replay equivalence.

**Illegal paths this spec must not produce:** a planner candidate for a remote source's quantity/quality with no belief carrier; instant omniscient belief correction before arrival; a global `water_scarcity` scalar; any sickness/wound from water (deferred); water appearing without a source/regeneration path.
