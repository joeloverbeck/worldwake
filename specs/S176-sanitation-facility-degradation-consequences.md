# S176: Sanitation Facility Degradation Consequences

## Summary

The sanitation carriers `WashBasinState` and `LatrineFullness` (per `archive/specs/S129-place-dirtiness-facility-wear.md`) already exist and are *partially* live, but their degradation state is **inert as a consequence**:

- `apply_wash` in `crates/worldwake-systems/src/needs_actions.rs` increments `WashBasinState.dirtiness_level` on every use (line ~1241), but `dirtiness_level` is **never read** to gate wash legality or effectiveness. A filthy basin washes exactly as well as a pristine one.
- `apply_toilet` increments `LatrineFullness.fill` and, only once `fill >= critical_threshold`, raises `PlaceDirtiness` and emits `EventTag::WasteCreated { source: OvercapacityLatrine }` (lines ~1085–1111). But the toilet action **always succeeds and fully relieves Bladder** regardless of fullness. A latrine at 100% fill still works perfectly; overflow is cosmetic dirtiness with no action-legality consequence.
- There is **no cleaning or emptying affordance**. Basin dirtiness and latrine fill rise monotonically with use and never recover except by the item-decay basin refill (which adds clean water but does not lower `dirtiness_level`, and does nothing for latrine fill).

The result: sanitation facilities degrade in their *numbers* but the world never pushes back. An agent's self-care never fails, branches, or forces a fallback because a facility became unusable. This spec wires the existing dead degradation state into **action legality, effect magnitude, and recovery labor** — the lowest-new-surface, highest-leverage slice of the deferred Cluster 1 material-degradation wave (see `specs/IMPLEMENTATION-ORDER.md`).

This spec was deferred by the 2026-05-26 second-iteration Cluster 1 triage ("the missing consequence wiring … needs S174's rest substrate as the meaningful consumer") and is now ripe: S174 (`archive/specs/S174-shelter-sleep-surfaces-safe-rest.md`) has landed, so contention over scarce *clean* self-care affordances now collides with rest scarcity.

## Phase

Phase 7: Consequence Carriers

## Status

📝 DRAFT — authored, awaiting activation (held adjunct wave; see `specs/IMPLEMENTATION-ORDER.md`)

## Crates

- `worldwake-core` (extend `WashBasinState` with an effective-dirtiness threshold field; add `clean_basin`/`empty_latrine` cleaning-duration profile fields to `MetabolismProfile`; no new component)
- `worldwake-sim` (no new event variant; reuse `EventTag::WasteCreated`; new `ActionTraceDetail` payloads for blocked/degraded self-care if not already covered by `SelfCareInterrupted`)
- `worldwake-systems` (wash effectiveness scales with `dirtiness_level` and fails above threshold; toilet precondition gated by `LatrineFullness.fill < critical_threshold`; two new maintenance actions `clean_wash_basin` and `empty_latrine`)
- `worldwake-ai` (candidate generation emits cleaning/refill goals when self-care is blocked; survival forensics record blocked/degraded self-care)
- `worldwake-cli` (scenario contract for the new threshold field and cleaning durations; player-POV gating for basin/latrine condition observation)

## Dependencies

- `archive/specs/S129-place-dirtiness-facility-wear.md` — provides `PlaceDirtiness`, `WashBasinState`, `LatrineFullness` carriers this spec turns into consequences.
- `archive/specs/S173-self-care-interruption-occupancy.md` — provides `SelfCareOccupancy` and the per-action interruption/abort discipline the new maintenance actions follow.
- `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md` — provides the rest-scarcity substrate that makes blocked self-care collide with rest contention; provides the `FailedRestOpportunity`/forensic precedent this spec mirrors for blocked self-care.
- `archive/specs/S120-survival-critical-window-forensics.md` — provides `SurvivalForensicExtractor` extended here with blocked/degraded self-care records.
- `archive/specs/S44-generalized-contention-substrate.md` + `archive/specs/S142-contention-event-inspectability.md` — provide the contention substrate that already classifies Wash/Latrine as exclusive use (per S173); cleaning actions reuse the same occupancy.
- `archive/specs/S82-waste-disposal-inventory-management.md` — provides `CommodityKind::Waste` and the waste-lot lifecycle the cleaning actions emit into.

## Design Goals

- A dirty wash basin produces **reduced wash relief**, and above a scenario-authored threshold **fails the wash precondition** entirely. Effectiveness is a function of the concrete `dirtiness_level`, never a hidden quality score.
- A full latrine **blocks the Toilet action** (`fill >= critical_threshold`), forcing the agent to empty it, queue, or fall back to Wilderness Relief — the lawful branch that already exists. Overflow dirtiness is retained as aftermath, not the only consequence.
- Recovery is concrete labor: `clean_wash_basin` and `empty_latrine` are duration-bearing, occupancy-bearing actions that consume time, reset the degradation state, and emit `Waste` / `PlaceDirtiness` aftermath. No magic "facility resets itself."
- Blocked or degraded self-care leaves **traceable evidence** in `SurvivalForensicExtractor`, so "why did this agent relieve in the wild / wash poorly / not wash at all?" is answerable from typed records.
- Player and AI obey identical facility legality. The CLI surfaces only basin/latrine condition the controlled agent lawfully perceives (co-located physical observation, FND-14A).

## Non-Goals

- **No disease, infection, odor, hygiene shame, privacy, or bathroom etiquette.** Per the report's MUST-NOT and FND-5; deferred indefinitely unless a concrete consequence carrier is later proven.
- **No new contention queue.** Cleaning actions reuse the existing S44/S173 `SelfCareOccupancy` on the basin/latrine place. A full latrine being emptied is occupied like any other self-care use.
- **No global sanitation/settlement-health score.** All state is per-facility concrete carriers (FND-3).
- **No water-source quality model.** Basin refill clean/dirty-water preference is owned by the paired `specs/S177-water-source-quality-depletion-reliability.md`; S176 consumes only the existing `WashBasinState.clean_water_units` precondition and the basin's own `dirtiness_level`.
- **No food spoilage.** Owned by `specs/S178-perishable-food-spoilage.md`.
- **No HTN method.** Cleaning/refill goals are flat GOAP prerequisite candidates emitted when self-care is blocked.
- **No backward-compatibility shim.** The Toilet action's "always succeeds" behavior is replaced by the fullness gate; goldens depending on the old behavior are updated (FND-28).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | A camp's only basin grows filthy under multi-agent use → wash relief drops → agents queue, clean it, or stay dirty → behavior emerges from concrete facility wear, not authored "facility broke" scripts |
| FND-3 (Concrete state over abstract scores) | Wash effectiveness and toilet legality derive from `WashBasinState.dirtiness_level` / `clean_water_units` and `LatrineFullness.fill`; no `sanitation_score` |
| FND-4 (Persistent identity / source-sink) | Cleaning emits `Waste` item lots with provenance; emptying a latrine moves accumulated waste into a concrete lot, not nowhere |
| FND-7 (Locality) | Facility condition is observed by co-located agents (FND-14A); remote condition is belief-backed (FND-14B); the planner cannot read remote basin/latrine state as authoritative |
| FND-8 (Preconditions / duration / cost / occupancy) | Wash gains a dirtiness precondition; Toilet gains a fullness precondition; `clean_wash_basin`/`empty_latrine` have duration, cost (time), and reuse `SelfCareOccupancy` |
| FND-10 (Aftermath) | Blocked wash/toilet is new state (forensic record + fallback), not a boolean fail; cleaning leaves `Waste` + `PlaceDirtiness` |
| FND-11 (Positive feedback) | Heavy use → dirtier/fuller facility → more fallback/cleaning → see Section H dampeners |
| FND-14A / FND-14B | Basin/latrine condition is a perceivable physical fact when co-located; belief-backed otherwise; ownership of the facility stays belief-gated |
| FND-19 (Agent symmetry) | Human and AI face the same wash/toilet gates and cleaning costs; CLI shows only lawfully perceived condition |
| FND-20 (Resource-bounded planning) | Cleaning/refill emerge as ordinary prerequisite candidates; no scripted maintenance loop |
| FND-21 (Intentions revisable) | A planned Wash at a now-too-dirty basin fails at start and the agent replans (clean, queue, or travel); planning the basin does not reserve it |
| FND-26 (Systems via state) | Wash/toilet handlers read facility state and write it; planner reads via belief view; forensics reads event/trace log; no system commands another |
| FND-28 (No backcompat) | Toilet "always succeeds" path is replaced, not aliased |
| FND-29 / FND-29A (Debuggability / causal history) | "Why did this agent relieve in the wild?" answerable from the blocked-toilet forensic record + `LatrineFullness.fill` at decision tick; `WasteCreated`/cleaning events append-only |
| FND-31 (Validation) | Focused goldens for each branch + one 1440-tick multi-agent sanitation-breakdown collision scenario (see Scenario Validation) |

## Deliverables

### D1. `WashBasinState` effective-dirtiness threshold

Extend `crates/worldwake-core/src/place_dirtiness.rs::WashBasinState` with one field:

```rust
pub struct WashBasinState {
    pub clean_water_units: u16,
    pub max_clean_water: u16,
    pub refill_per_tick: u16,
    pub units_per_full_wash: u16,
    pub dirtiness_level: Permille,
    pub dirtiness_per_use: Permille,
    /// Wash relief scales down linearly as `dirtiness_level` rises toward this
    /// threshold; at or above it the Wash precondition fails. Scenario-authored.
    pub max_effective_dirtiness: Permille,
}
```

`max_effective_dirtiness` is a `Permille` (no magic numeric constant). `Default` sets it to a value that leaves existing scenarios behaviorally unchanged until they author dirt — reassessment pins the exact default against current goldens.

### D2. Wash effectiveness gate

In `apply_wash` (`needs_actions.rs` ~line 1208), the agent-dirtiness relief (`agent_dirtiness_delta`) is scaled by basin cleanliness:

- `effective_fraction = (max_effective_dirtiness - dirtiness_level) / max_effective_dirtiness`, clamped to `[0, 1]` in `Permille`.
- Relief is multiplied by `effective_fraction`. A half-filthy basin gives half the wash benefit.

In `wash_preconditions` (~line 277), add `Precondition::TargetWashBasinNotTooDirty` (`dirtiness_level < max_effective_dirtiness`). A basin at/above threshold fails the precondition; the Wash candidate is not startable, and the agent must clean it, queue, or travel.

### D3. Latrine fullness gate

In `toilet_preconditions`, add `Precondition::PlaceLatrineNotFull` (`LatrineFullness.fill < critical_threshold`). Above the threshold the Toilet action fails to start. The existing Wilderness Relief action remains available as the lawful fallback (it has no latrine dependency), so a blocked latrine forces the agent to relieve in the wild (raising place dirtiness) or empty the latrine.

The existing overflow path in `apply_toilet` (dirtiness + `WasteCreated`) is retained for the boundary case where fill crosses the threshold on the final lawful use.

### D4. `clean_wash_basin` and `empty_latrine` maintenance actions

Two new actions registered in `needs_actions.rs`:

- `clean_wash_basin`: target = co-located `WashBasin` workstation. Duration = `MetabolismProfile.clean_basin_duration_ticks`. Reuses `SelfCareOccupancy` (exclusive). On commit, resets `WashBasinState.dirtiness_level` toward `Permille::ZERO` and consumes `clean_water_units` (cleaning uses water). Emits a `Waste` lot to the place ground and raises `PlaceDirtiness` (the grime goes somewhere — FND-4).
- `empty_latrine`: target = co-located latrine `Place`. Duration = `MetabolismProfile.empty_latrine_duration_ticks`. Reuses `SelfCareOccupancy`. On commit, resets `LatrineFullness.fill` toward `Permille::ZERO`, creates a `Waste` lot proportional to the emptied fill, and emits `EventTag::WasteCreated { source: LatrineEmptied }` (new `WasteSource` variant).

Both have explicit abort handlers (no `abort_noop`), per the S173 interruption discipline.

### D5. Candidate generation for blocked self-care

When the Wash or Toilet candidate is rejected by D2/D3 preconditions and dirtiness/bladder pressure remains, candidate generation (`worldwake-ai`) emits the corresponding maintenance goal (`clean_wash_basin` / `empty_latrine`) or falls back to Wilderness Relief, using only belief-backed / same-tick-local facility condition. No optimistic emission for fully-unknown remote facilities (mirrors S172's belief-backed Wash discipline).

### D6. Survival forensics for blocked/degraded self-care

Extend `SurvivalForensicExtractor` (`crates/worldwake-ai/src/survival_forensics.rs`) with a `DegradedSelfCareOpportunity` record (analogous to S174's `FailedRestOpportunity`): captures the facility, the degradation cause (`BasinTooDirty`, `BasinDry`, `LatrineFull`), and whether the agent fell back (wilderness relief), cleaned, queued, or did nothing. It is derived forensic state (FND-27), never authoritative.

### D7. Profile fields

Add to `MetabolismProfile` (universal agent profile, per S128): `clean_basin_duration_ticks: NonZeroU32` and `empty_latrine_duration_ticks: NonZeroU32`, both with `Default` impls and scenario-overridable via the existing `metabolism_profile` field on `AgentDef`. Per the Agent Profile Scenario Contract.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Today no self-care action can fail or degrade because a facility is dirty/full; there is no recovery labor. The world cannot produce "the only basin is filthy, so everyone is dirty and someone finally cleans it," nor "the latrine is full, so people relieve in the wild and the camp degrades."
2. **New entities/relations/records**: `WashBasinState.max_effective_dirtiness` field; `MetabolismProfile.clean_basin_duration_ticks` / `empty_latrine_duration_ticks`; `clean_wash_basin` + `empty_latrine` action defs; `WasteSource::LatrineEmptied` variant; `DegradedSelfCareOpportunity` forensic record; new `Precondition` variants `TargetWashBasinNotTooDirty`, `PlaceLatrineNotFull`. No new ECS component.
3. **Actions that mutate them**: `apply_wash` reads `dirtiness_level` (effectiveness), writes it (per-use increment); `clean_wash_basin` resets it. `apply_toilet` reads/writes `LatrineFullness.fill`; `empty_latrine` resets it. Both maintenance actions emit `Waste` and `PlaceDirtiness`.
4. **Information production and travel**: Co-located agents observe basin/latrine condition (FND-14A). Remote condition is belief-backed (FND-14B); stale beliefs cause candidates that fail at start. Cleaning/emptying emit `EventTag::WasteCreated` into the append-only log.
5. **Conserved quantities**: Wash consumes `clean_water_units` (already conserved). Emptying a latrine transforms accumulated fill into a `Waste` lot (source/sink explicit). No quantity created from nothing.
6. **Scarce capacities and contention**: A *clean* basin / *non-full* latrine becomes the scarce affordance. Contention via existing S44/S173 `SelfCareOccupancy` — no new queue. Cleaning occupies the facility exclusively.
7. **Partial failures and aftermath**: Wash at a dirty-but-usable basin → reduced relief (partial outcome). Wash rejected (basin too dirty/dry) → no episode, `DegradedSelfCareOpportunity` recorded, agent cleans/queues/travels. Toilet rejected (latrine full) → Wilderness Relief fallback (raises place dirtiness) or `empty_latrine`. Cleaning interrupted → partial; explicit abort handler releases occupancy.
8. **Positive feedback loops**: (a) More use → dirtier basin / fuller latrine → degraded or blocked self-care. (b) Blocked latrine → more Wilderness Relief → rising `PlaceDirtiness` → more ambient dirtiness pressure.
9. **Concrete dampeners** (physical, not numeric clamps): (a) Cleaning/emptying labor exists and lowers the state — an agent (or another) can always restore the facility through a real action. (b) `clean_water_units` and basin refill (item-decay system, bounded by colocated water source quantity) cap how fast a basin can be both used and cleaned — water is finite. (c) `PlaceDirtiness.decay_per_tick` (existing, item-decay system) reverses ambient dirtiness over time. (d) Wilderness Relief is always available, so bladder pressure never deadlocks — it diverts the loop into place-dirtiness aftermath rather than agent collapse. (e) Cleaning consumes the cleaner's time (occupancy), competing with their own self-care — social pushback / labor scarcity dampens over-cleaning.
10. **Agent learning**: None new. Agents replan from current observation each tick. (A learned "this basin is usually filthy" preference could fold into the existing `LearnedSourcePreferences` substrate in a future spec if scenarios prove it necessary; not introduced here.)
11. **How agents can be wrong**: Believe a basin is clean / latrine empty when stale → precondition rejects at start → replan. Believe a remote facility is usable → arrive and find it blocked → `DegradedSelfCareOpportunity` recorded.
12. **Lifecycle states**: `WashBasinState`: `Clean ↔ Dirty ↔ TooDirty(blocked)` by `dirtiness_level` vs `max_effective_dirtiness`; orthogonal `Wet ↔ Dry` by `clean_water_units`. `LatrineFullness`: `Usable ↔ Full(blocked)` by `fill` vs `critical_threshold`. All transitions via use / cleaning / refill / decay — no winking.
13. **Temporal resolution**: All reads/writes at action start/commit/abort tick boundaries; basin refill and dirtiness decay at the item-decay system tick. Concurrent same-tick wash attempts on the last usable basin resolved by existing S44 tie-break.
14. **Boundary conditions**: N/A — facilities are local place-graph topology. (Water *supply* to basins is local via the colocated source; cross-boundary water is out of scope here and S177.)
15. **Derived views**: `DegradedSelfCareOpportunity` (forensic, derived). Belief-view accessors for basin dirtiness / latrine fill are per-actor derived views over authoritative state (see source-class table).
16. **Causal records**: `EventTag::WasteCreated` (existing + new `LatrineEmptied` source) on overflow and emptying; cleaning action-trace details; `DegradedSelfCareOpportunity` in the active critical window. Together they reconstruct why self-care degraded.
17. **Target patterns**: Filthy shared basin → reduced wash → eventual clean; dry basin → wait for refill or travel; full latrine → wilderness relief raises place dirtiness → someone empties it; multi-agent camp where sanitation degrades over a long run and recovers through labor.
18. **Save/load and replay**: One new `Permille` field, two `NonZeroU32` profile fields, two action defs, new precondition/forensic/`WasteSource` variants — all standard ECS/profile/trace state, replay-deterministic. No new authoritative event variant.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `WashBasinState` (incl. new `max_effective_dirtiness`) | Stored authoritative | Component on Place |
| `LatrineFullness` | Stored authoritative | Component on Place |
| `PlaceDirtiness` | Stored authoritative | Component on Place |
| `MetabolismProfile.clean_basin_duration_ticks` / `empty_latrine_duration_ticks` | Stored authoritative profile parameter | Per-agent profile |
| `WasteSource::LatrineEmptied` | Stored event-payload classification | Authoritative on event emission |
| `DegradedSelfCareOpportunity` records | Derived forensic state | View over event/trace log; not authoritative |
| Belief-view basin-dirtiness / latrine-fill accessors | Derived per-actor view | View; not authoritative |

## Planner-formalism analysis

Plain GOAP. Cleaning/refill are ordinary prerequisite candidates emitted when the primary self-care candidate is blocked. No HTN method: no multi-stage decomposition, information-gathering stage, role-specific strategy, budget exhaustion, or utility thrash that flat affordance search cannot handle. Fallback: N/A (no method). Information reads: all facility-condition inputs are belief-backed or same-tick-local. Enforced declarations only: every new field/precondition has a live consumer (effectiveness scaler, precondition gate, candidate emitter, forensic recorder). Proof surface: scenarios below.

## Belief-View Accessor Source-Class Declarations

| Accessor | Source class | Stale/unknown behavior |
|----------|--------------|------------------------|
| `wash_basin_dirtiness(place) -> Option<Permille>` | FND-14A co-located; belief-backed remote | `None` if no belief and remote |
| `wash_basin_clean_units(place) -> Option<u16>` | FND-14A co-located; belief-backed remote | `None` if no belief and remote |
| `latrine_fill(place) -> Option<Permille>` | FND-14A co-located; belief-backed remote | `None` if no belief and remote |

Accessors return `None` rather than reading remote authoritative state. Who *owns* or *controls* the facility is not exposed here — that remains belief-gated per FND-14A's social-fact carve-out.

## Agent Profile Scenario Contract

`MetabolismProfile` is a universal agent profile (registered on `EntityKind::Agent` per S128) with a `Default` impl. The two new duration fields get defaults and are scenario-overridable via the existing `metabolism_profile: Option<MetabolismProfile>` field on `AgentDef`. No new component on `EntityKind::Agent`. `max_effective_dirtiness` is authored on the place's `WashBasinState` via the existing place scenario contract.

## Component Registration

No new components. `WashBasinState`, `LatrineFullness`, `PlaceDirtiness` registrations in `crates/worldwake-core/src/component_schema.rs` are unchanged (field addition only). The two new actions register through the standard `register_def` path in `needs_actions.rs::register_needs_actions`.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Wash handler (`worldwake-systems`) | `WashBasinState` (dirtiness + clean units), `MetabolismProfile` | `WashBasinState.dirtiness_level`, `HomeostaticNeeds.dirtiness` |
| Toilet handler (`worldwake-systems`) | `LatrineFullness`, `PlaceDirtiness` | `LatrineFullness.fill`, `PlaceDirtiness`, `Waste` lot |
| Cleaning handlers (`worldwake-systems`) | `WashBasinState`/`LatrineFullness`, `SelfCareOccupancy`, profile | facility state reset, `Waste` lot, `PlaceDirtiness` |
| Candidate emitter (`worldwake-ai`) | Belief-view facility condition | None (read-only emission) |
| Survival forensics (`worldwake-ai`) | event/trace log, `CriticalWindowFrame` | `DegradedSelfCareOpportunity` records |
| Item-decay system (`worldwake-systems`) | `PlaceDirtiness`, `WashBasinState`, colocated water source | dirtiness decay, basin clean-water refill |

No system commands another; all via authoritative state and the event/trace log.

## Scenario Validation (FND-31)

**Focused branch goldens:**

- **`survival-basin-dirty-dirty.ron`** — repeated washes raise basin `dirtiness_level`; relief drops proportionally; at threshold the Wash precondition fails; the agent runs `clean_wash_basin`; subsequent wash recovers full relief. Asserts effectiveness scaling, precondition rejection, cleaning aftermath (Waste lot), and deterministic replay.
- **`survival-latrine-full.ron`** — repeated toilet uses raise `fill` to `critical_threshold`; the Toilet precondition fails; the agent falls back to Wilderness Relief (place dirtiness rises) OR runs `empty_latrine`; asserts the branch chosen, `WasteCreated` provenance, and the `DegradedSelfCareOpportunity` forensic record.

**1440-tick CI-owned collision scenario (registered in `docs/scenario-roadmap.md`, run only via `.github/workflows`):**

- **`survival-sanitation-breakdown-1440.ron`** — multiple agents share one basin and one latrine over 1440 ticks under ordinary need pressure. Sanitation degrades; agents queue (S44 occupancy), clean, empty, and fall back to wilderness relief; place dirtiness rises and decays. Assertions prove: facility-state arithmetic (dirtiness/fill cross thresholds and recover via labor), no omniscient remote-facility reads (belief barrier), `DegradedSelfCareOpportunity` causal records, and replay equivalence.

**Illegal paths this spec must not produce:** wash relief unaffected by `dirtiness_level`; toilet succeeding at/above `critical_threshold`; facility state resetting without a cleaning/decay action; a planner candidate for a remote facility's condition with no belief carrier; any `sanitation_score`.
