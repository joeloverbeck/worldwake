# S176: Sanitation Facility Degradation Consequences

## Summary

The sanitation carriers `WashBasinState` and `LatrineFullness` (per `archive/specs/S129-place-dirtiness-facility-wear.md`) already exist and are *partially* live, but their degradation state is **inert as a consequence**:

- `apply_wash` in `crates/worldwake-systems/src/needs_actions.rs` (~line 1208) increments `WashBasinState.dirtiness_level` on every use (~line 1241), but `dirtiness_level` is **never read** to gate wash legality or effectiveness. A filthy basin washes exactly as well as a pristine one.
- `apply_toilet` (~lines 1071–1123) increments `LatrineFullness.fill` and, only once `fill >= critical_threshold`, raises `PlaceDirtiness` and emits `EventTag::WasteCreated { source: OvercapacityLatrine }` (~lines 1089–1110). But the toilet action **always succeeds and fully relieves Bladder** regardless of fullness. A latrine at 100% fill still works perfectly; overflow is cosmetic dirtiness with no action-legality consequence.
- There is **no cleaning or emptying affordance**. Basin dirtiness and latrine fill rise monotonically with use and never recover except by the item-decay basin refill (which adds clean water but does not lower `dirtiness_level`, and does nothing for latrine fill).

The result: sanitation facilities degrade in their *numbers* but the world never pushes back. An agent's self-care never fails, branches, or forces a fallback because a facility became unusable. This spec wires the existing dead degradation state into **action legality, effect magnitude, and recovery labor** — the lowest-new-surface, highest-leverage slice of the deferred Cluster 1 material-degradation wave (see `specs/IMPLEMENTATION-ORDER.md`).

This spec was deferred by the 2026-05-26 second-iteration Cluster 1 triage ("the missing consequence wiring … needs S174's rest substrate as the meaningful consumer") and is now ripe: S174 (`archive/specs/S174-shelter-sleep-surfaces-safe-rest.md`) has landed, so contention over scarce *clean* self-care affordances now collides with rest scarcity.

The new recovery labor — `clean_wash_basin` and `empty_latrine` — is modeled as **plain-GOAP prerequisite operations** the existing `GoalKind::Wash` / `GoalKind::Relieve` plan search inserts when a degradation precondition is blocked, mirroring the existing `QueueForFacilityUse` op (`crates/worldwake-ai/src/planner_ops.rs`, inserted mid-plan at `crates/worldwake-ai/src/goal_model.rs`). **No new `GoalKind`** is introduced; the consequence wiring stays inside the self-care goals that already exist.

## Phase

Phase 7: Consequence Carriers

## Status

✅ COMPLETED — implemented via tickets S176SANFACDEG-001..008 (2026-05-29).

## Crates

- `worldwake-core` — extend `WashBasinState` (`crates/worldwake-core/src/place_dirtiness.rs`) with an effective-dirtiness threshold field; add `clean_basin`/`empty_latrine` cleaning-duration profile fields to `MetabolismProfile` (`crates/worldwake-core/src/needs.rs`); add `WasteSource::LatrineEmptied` variant (`crates/worldwake-core/src/decision_event_payload.rs`); add `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}` variants (`crates/worldwake-core/src/self_care_occupancy.rs`). No new ECS component.
- `worldwake-sim` — add `Precondition::{TargetWashBasinNotTooDirty, PlaceLatrineNotFull}` variants to `crates/worldwake-sim/src/action_semantics.rs` and their evaluation arms in `crates/worldwake-sim/src/action_validation.rs` and `crates/worldwake-sim/src/affordance_query.rs`; add `MetabolismDurationKind::{CleanBasin, EmptyLatrine}` variants and their resolver arms in `action_semantics.rs` and `crates/worldwake-sim/src/belief_view.rs`; reuse `EventTag::WasteCreated` (no new event variant); reuse `ActionTraceDetail::SelfCareInterrupted` for cleaning interruption.
- `worldwake-systems` — wash effectiveness scales with `dirtiness_level` and fails above threshold; toilet precondition gated by `LatrineFullness.fill < critical_threshold`; two new maintenance actions `clean_wash_basin` and `empty_latrine` with explicit abort handlers (`crates/worldwake-systems/src/needs_actions.rs`).
- `worldwake-ai` — new `PlannerOpKind::{CleanWashBasin, EmptyLatrine}`, `classify_action_def` arms (`crates/worldwake-ai/src/planner_ops.rs`), and inclusion of the new ops in the `GoalKind::Wash` / `GoalKind::Relieve` plan search (`crates/worldwake-ai/src/goal_model.rs`); candidate generation reads facility condition via the **existing** belief-view accessors; survival forensics record blocked/degraded self-care (`crates/worldwake-ai/src/survival_forensics.rs`).
- `worldwake-cli` — add the `max_effective_dirtiness` field to `WashBasinStateDef` (`crates/worldwake-cli/src/scenario/types.rs`) and the cleaning durations to the metabolism contract; player-POV gating for basin/latrine condition observation in the observer (`crates/worldwake-cli/src/bin/observer.rs`).

## Dependencies

- `archive/specs/S129-place-dirtiness-facility-wear.md` — provides `PlaceDirtiness`, `WashBasinState`, `LatrineFullness` carriers this spec turns into consequences.
- `archive/specs/S173-self-care-interruption-occupancy.md` — provides `SelfCareOccupancy` and the per-action interruption/abort discipline the new maintenance actions follow.
- `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md` — provides the rest-scarcity substrate that makes blocked self-care collide with rest contention; provides the `FailedRestOpportunity`/forensic precedent this spec mirrors for blocked self-care (`crates/worldwake-ai/src/survival_forensics.rs`).
- `archive/specs/S120-survival-critical-window-forensics.md` — provides `SurvivalForensicExtractor` and `CriticalWindowFrame`, extended here with blocked/degraded self-care records.
- `archive/specs/S44-generalized-contention-substrate.md` + `archive/specs/S142-contention-event-inspectability.md` — provide the contention substrate that already classifies Wash/Latrine as exclusive use (per S173); cleaning actions reuse the same occupancy.
- `archive/specs/S82-waste-disposal-inventory-management.md` — provides `CommodityKind::Waste` (`crates/worldwake-core/src/items.rs`) and the waste-lot lifecycle the cleaning actions emit into.
- `archive/specs/S128-sleep-episode-place-quality.md` — establishes the universal `MetabolismProfile` agent-profile contract the new duration fields follow.
- `archive/specs/S172-wash-discovery-budget-closure.md` — establishes the belief-backed Wash candidate discipline D7 mirrors.

## Design Goals

- A dirty wash basin produces **reduced wash relief**, and above a scenario-authored threshold **fails the wash precondition** entirely. Effectiveness is a function of the concrete `dirtiness_level`, never a hidden quality score.
- A full latrine **blocks the Toilet action** (`fill >= critical_threshold`), forcing the agent to empty it, queue, or fall back to Wilderness Relief — the lawful branch that already exists. Overflow dirtiness is retained as aftermath, not the only consequence.
- Recovery is concrete labor: `clean_wash_basin` and `empty_latrine` are duration-bearing, occupancy-bearing actions that consume time, reset the degradation state, and emit `Waste` / `PlaceDirtiness` aftermath. No magic "facility resets itself."
- Recovery labor is **discovered by the planner as an ordinary prerequisite**, not emitted as a standalone goal: the cleaning ops are inserted into the existing `Wash` / `Relieve` plan search when the degradation precondition blocks the primary self-care action (FND-20).
- Blocked or degraded self-care leaves **traceable evidence** in `SurvivalForensicExtractor`, so "why did this agent relieve in the wild / wash poorly / not wash at all?" is answerable from typed records.
- Player and AI obey identical facility legality. The CLI surfaces only basin/latrine condition the controlled agent lawfully perceives (co-located physical observation, FND-14A).

## Non-Goals

- **No disease, infection, odor, hygiene shame, privacy, or bathroom etiquette.** Per the report's MUST-NOT and FND-5; deferred indefinitely unless a concrete consequence carrier is later proven.
- **No new `GoalKind`.** Cleaning/emptying are plain-GOAP prerequisite operations within the existing `Wash` / `Relieve` goals (mirroring `QueueForFacilityUse`), not new goal kinds. This deliberately avoids the full `GoalDispatchKey` / `GoalDispatchDeclaration` / `GoalKindPlannerExt` integration surface.
- **No new contention queue.** Cleaning actions reuse the existing S44/S173 `SelfCareOccupancy` on the basin/latrine place. A full latrine being emptied is occupied like any other self-care use.
- **No new belief-view accessors.** D7 reads facility condition through the **existing** `facility_wash_basin_state` / `wash_basin_state` / `latrine_fullness` accessors on `GoalBeliefView`.
- **No global sanitation/settlement-health score.** All state is per-facility concrete carriers (FND-3).
- **No water-source quality model.** Basin refill clean/dirty-water preference is owned by the paired `archive/specs/S177-water-source-quality-depletion-reliability.md`; S176 consumes only the existing `WashBasinState.clean_water_units` precondition and the basin's own `dirtiness_level`.
- **No food spoilage.** Owned by `specs/S178-perishable-food-spoilage.md`.
- **No HTN method.** Cleaning/refill ops are flat GOAP prerequisite operations.
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
| FND-20 (Resource-bounded planning) | Cleaning/refill are ordinary prerequisite ops inserted into the existing self-care plan search; no scripted maintenance loop, no new goal kind |
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

`WashBasinState` is registered on `EntityKind::Facility` (it is the WashBasin workstation), not on `EntityKind::Place` — see `crates/worldwake-core/src/component_schema.rs` (~line 2087). The field addition does not change the registration.

The scenario-contract wrapper `WashBasinStateDef` (`crates/worldwake-cli/src/scenario/types.rs`, ~lines 544–567) must gain the matching field with a serde default (the struct already carries `#[serde(default)]`), so existing scenarios — none of which author basin state explicitly; all rely on `unwrap_or_default()` at `crates/worldwake-cli/src/scenario/mod.rs` (~line 414) — continue to deserialize unchanged. See D10.

### D2. Wash effectiveness gate

In `apply_wash` (`needs_actions.rs` ~line 1208), the agent-dirtiness relief (`agent_dirtiness_delta`, computed at ~lines 1229–1233 and applied via `needs.dirtiness.saturating_sub(...)` at ~line 1254) is scaled by basin cleanliness:

- `effective_fraction = (max_effective_dirtiness - dirtiness_level) / max_effective_dirtiness`, clamped to `[0, 1]` in `Permille`.
- Relief is multiplied by `effective_fraction`. A half-filthy basin gives half the wash benefit.

In `wash_preconditions` (`needs_actions.rs` ~line 277 — this function exists and is wired at the `wash` registration, ~line 102), add the new `Precondition::TargetWashBasinNotTooDirty { target_index: 0 }` (defined in D4), checking `dirtiness_level < max_effective_dirtiness` on the basin facility (target 0, mirroring the existing `Precondition::TargetHasWashBasinClean { target_index: 0, min }`). A basin at/above threshold fails the precondition; the Wash candidate is not startable, and the planner must insert a cleaning prerequisite (D6), queue, or the agent travels.

### D3. Latrine fullness gate

There is **no `toilet_preconditions` function**. The `toilet` action is registered inline at `needs_actions.rs` ~lines 88–97 with `preconditions: vec![Precondition::ActorAlive]`, `targets: vec![TargetSpec::ActorPlace]` (target 0 = the actor's latrine place, ~line 174), and `actor_constraints: [ActorAlive, ActorAtPlaceTag(PlaceTag::Latrine)]` (~lines 161–163).

Add the new `Precondition::PlaceLatrineNotFull { target_index: 0 }` (defined in D4) to that inline precondition vec (~line 92), checking `LatrineFullness.fill < critical_threshold` on the ActorPlace target. Above the threshold the Toilet action fails to start. The existing `relieve_wilderness` action remains available as the lawful fallback (it has its own `OUTDOOR_RELIEF_TAGS` actor constraint, no latrine dependency, `needs_actions.rs` ~lines 111–144), so a blocked latrine forces the agent to relieve in the wild (raising place dirtiness) or empty the latrine (D5).

The existing overflow path in `apply_toilet` (dirtiness + `WasteCreated { source: OvercapacityLatrine }`, ~lines 1089–1110) is retained for the boundary case where fill crosses the threshold on the final lawful use.

### D4. `Precondition` enum extension and evaluation sites

Add two variants to `Precondition` (`crates/worldwake-sim/src/action_semantics.rs`, ~line 47), following the shape of the existing `TargetHasWashBasinClean { target_index: u8, min: u16 }`:

```rust
TargetWashBasinNotTooDirty { target_index: u8 },
PlaceLatrineNotFull { target_index: u8 },
```

`Precondition` is matched exhaustively at two runtime evaluation sites; both need a new arm for each variant:

- `crates/worldwake-sim/src/action_validation.rs` (~line 110) — the start/commit validation path. `TargetWashBasinNotTooDirty` reads the target facility's `WashBasinState` and requires `dirtiness_level < max_effective_dirtiness`; `PlaceLatrineNotFull` reads the target place's `LatrineFullness` and requires `fill < critical_threshold`.
- `crates/worldwake-sim/src/affordance_query.rs` (~lines 336, 477) — the affordance-enumeration path that gates candidate startability. Mirror the validation semantics so `get_affordances` correctly suppresses a blocked Wash/Toilet candidate.

[Pattern: New Enum Variant on Cross-Crate Enum] — the variants are payload-free `{ target_index: u8 }` and satisfy the enum's existing derives; the two sites above are the only genuinely-exhaustive matches.

### D5. `clean_wash_basin` and `empty_latrine` maintenance actions

Two new actions registered through the standard `register_def` / `defs.register` path in `needs_actions.rs::register_needs_actions`:

- `clean_wash_basin`: target = co-located `WashBasin` facility (`TargetSpec::EntityAtActorPlace { kind: Facility }`, as `wash`). Duration = `DurationExpr::ActorMetabolism { kind: MetabolismDurationKind::CleanBasin }` (new kind, D9). Reuses `SelfCareOccupancy` (exclusive) with the new `SelfCareUseKind::CleanWashBasin`. On commit, resets `WashBasinState.dirtiness_level` toward `Permille::ZERO` and consumes `clean_water_units` (cleaning uses water). Emits a `Waste` lot to the place ground and raises `PlaceDirtiness` (the grime goes somewhere — FND-4).
- `empty_latrine`: target = co-located latrine place (`TargetSpec::ActorPlace`, as `toilet`). Duration = `DurationExpr::ActorMetabolism { kind: MetabolismDurationKind::EmptyLatrine }` (new kind, D9). Reuses `SelfCareOccupancy` with the new `SelfCareUseKind::EmptyLatrine`. On commit, resets `LatrineFullness.fill` toward `Permille::ZERO`, creates a `Waste` lot proportional to the emptied fill, and emits `EventTag::WasteCreated { source: WasteSource::LatrineEmptied }`.

Supporting cross-crate enum extensions this deliverable carries:

- `WasteSource::LatrineEmptied` (`crates/worldwake-core/src/decision_event_payload.rs`, ~line 73) — new variant alongside `WildernessRelief`/`OvercapacityLatrine`. Current use sites are construction-only (`decision_event_payload.rs`, `crates/worldwake-sim/src/save_load.rs`, `crates/worldwake-cli/src/bin/observer.rs`); no exhaustive `match` requires a new arm, but the variant must be added before its construction site compiles.
- `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}` (`crates/worldwake-core/src/self_care_occupancy.rs`, ~lines 17–24) — required because `SelfCareOccupancy.use_kind` and `ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind, .. }` (`crates/worldwake-sim/src/action_trace.rs`, ~line 66) are typed by it. ~24 non-test match sites on `SelfCareUseKind` must gain arms for the new variants.

Both maintenance actions have explicit abort handlers (no `abort_noop`), per the S173 interruption discipline; cleaning interrupted before commit reuses `ActionTraceDetail::SelfCareInterrupted` and releases occupancy.

### D6. Planner-op integration for cleaning prerequisites

So the planner can insert cleaning as a prerequisite of a blocked self-care goal (the Q1 prerequisite-op model, mirroring `QueueForFacilityUse`):

- Add `PlannerOpKind::{CleanWashBasin, EmptyLatrine}` to `crates/worldwake-ai/src/planner_ops.rs` (~line 14).
- Add `classify_action_def` arms mapping `(ActionDomain::Needs, "clean_wash_basin")` and `(ActionDomain::Needs, "empty_latrine")` to the new ops (`planner_ops.rs` ~lines 87–141).
- Include the new ops in the `GoalKind::Wash` / `GoalKind::Relieve` plan search in `crates/worldwake-ai/src/goal_model.rs`: the relevant-op lists (~lines 618, 667, 1602, 1649), `apply_planner_step` (~line 1249 for `Relieve`, ~line 1253 for `Wash`), and the mid-plan-op handling (the `QueueForFacilityUse` precedent at ~line 1143). The cleaning op advances the goal by establishing the `TargetWashBasinNotTooDirty` / `PlaceLatrineNotFull` precondition, so the search treats it the same way it treats `QueueForFacilityUse`: a prerequisite step that unblocks the terminal self-care op.

No new `GoalKind`, `GoalDispatchKey`, or `GoalKindPlannerExt` surface is added — the ops live entirely within the existing Wash/Relieve goals.

### D7. Candidate generation for blocked self-care

The existing `emit_wash_goal` (`crates/worldwake-ai/src/candidate_generation.rs` ~line 4654) and `emit_relieve_goal` (~line 4586) already emit belief-backed `Wash`/`Relieve` candidates and a wilderness-relief fallback. They are unchanged in their goal-emission shape: the cleaning prerequisite is discovered by the plan search (D6), not by emitting a new goal.

Candidate generation and the plan search read facility condition through the **existing** `GoalBeliefView` accessors — no new accessors are added (this revises the original draft, which proposed `wash_basin_dirtiness` / `wash_basin_clean_units` / `latrine_fill`):

- `facility_wash_basin_state(entity) -> Option<WashBasinState>` (`crates/worldwake-sim/src/belief_view.rs` ~line 495) — `Option`, returns `None` when no belief and remote.
- `wash_basin_state(agent, basin) -> WashBasinState` (~line 561) and `latrine_fullness(agent, place) -> LatrineFullness` (~line 557) — these return a default rather than `Option`; the planner integration must treat a defaulted/absent read as "condition unknown" and must not synthesize a cleaning prerequisite for a fully-unknown remote facility (mirrors S172's belief-backed Wash discipline). Where the existing default-return shape masks the unknown case, prefer the `Option`-returning `facility_wash_basin_state` for the gating read.

No optimistic emission for fully-unknown remote facilities. Stale "basin clean / latrine empty" beliefs cause a candidate that fails at start and replans.

### D8. Survival forensics for blocked/degraded self-care

Extend `SurvivalForensicExtractor` (`crates/worldwake-ai/src/survival_forensics.rs`) with a `DegradedSelfCareOpportunity` record, analogous to the existing `FailedRestOpportunity` (`survival_forensics.rs` ~lines 54–67) and recorded on `CriticalWindowFrame` alongside `failed_rest_opportunities` (~line 50, `#[serde(default)]`):

```rust
pub struct DegradedSelfCareOpportunity {
    pub tick: Tick,
    pub facility: EntityId,
    pub cause: DegradedSelfCareCause, // BasinTooDirty | BasinDry | LatrineFull
    pub outcome: DegradedSelfCareOutcome, // WildernessRelief | Cleaned | Queued | DidNothing
}
```

Add a `degraded_self_care_opportunities: Vec<DegradedSelfCareOpportunity>` field (`#[serde(default)]`) to `CriticalWindowFrame`, mirroring the rest precedent. It is derived forensic state (FND-27), never authoritative.

### D9. Profile fields and duration kinds

Add to `MetabolismProfile` (`crates/worldwake-core/src/needs.rs`, universal agent profile per S128, alongside the existing `wash_ticks` / `toilet_ticks`): `clean_basin_duration_ticks: NonZeroU32` and `empty_latrine_duration_ticks: NonZeroU32`, both with `Default` values and scenario-overridable via the existing `metabolism_profile: Option<MetabolismProfile>` field on `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs` ~line 636, applied via `unwrap_or_default()` at `crates/worldwake-cli/src/scenario/mod.rs` ~line 978).

The duration is selected through `MetabolismDurationKind` (`crates/worldwake-sim/src/action_semantics.rs` ~line 104, currently `Toilet`/`Wash`). Add `MetabolismDurationKind::{CleanBasin, EmptyLatrine}` and resolver arms at **both** sites that map a kind to a profile field:

- `action_semantics.rs` ~lines 231–232 (`CleanBasin => profile.clean_basin_duration_ticks.get()`, `EmptyLatrine => profile.empty_latrine_duration_ticks.get()`).
- `crates/worldwake-sim/src/belief_view.rs` ~lines 2742–2743 (the planner-facing duration estimate — same mapping).

### D10. CLI scenario contract and observer condition display

- **Scenario contract**: add `max_effective_dirtiness` to `WashBasinStateDef` (`crates/worldwake-cli/src/scenario/types.rs` ~lines 544–567) with a serde default and the matching `From<WashBasinStateDef> for WashBasinState` mapping; expose the two new metabolism durations through the existing metabolism contract (no new wrapper — `MetabolismProfile` is authored whole via `AgentDef.metabolism_profile`).
- **Observer display**: the observer currently shows basin *presence* only (`crates/worldwake-cli/src/bin/observer.rs` ~line 2101: `wash={}`), with no condition and no latrine display. Extend the player-POV place summary to surface basin `dirtiness_level` / `clean_water_units` and latrine `fill` **only for the controlled agent's co-located facilities** (FND-14A) — read through the same belief-view accessors as D7, never from remote authoritative state.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Today no self-care action can fail or degrade because a facility is dirty/full; there is no recovery labor. The world cannot produce "the only basin is filthy, so everyone is dirty and someone finally cleans it," nor "the latrine is full, so people relieve in the wild and the camp degrades."
2. **New entities/relations/records**: `WashBasinState.max_effective_dirtiness` field; `MetabolismProfile.clean_basin_duration_ticks` / `empty_latrine_duration_ticks`; `clean_wash_basin` + `empty_latrine` action defs; `WasteSource::LatrineEmptied` variant; `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}` variants; `MetabolismDurationKind::{CleanBasin, EmptyLatrine}` variants; `PlannerOpKind::{CleanWashBasin, EmptyLatrine}` variants; `DegradedSelfCareOpportunity` forensic record; `Precondition` variants `TargetWashBasinNotTooDirty`, `PlaceLatrineNotFull`. No new ECS component, no new `GoalKind`, no new `EventTag`.
3. **Actions that mutate them**: `apply_wash` reads `dirtiness_level` (effectiveness), writes it (per-use increment); `clean_wash_basin` resets it. `apply_toilet` reads/writes `LatrineFullness.fill`; `empty_latrine` resets it. Both maintenance actions emit `Waste` and `PlaceDirtiness`.
4. **Information production and travel**: Co-located agents observe basin/latrine condition (FND-14A) through the existing belief-view accessors. Remote condition is belief-backed (FND-14B); stale beliefs cause candidates that fail at start. Cleaning/emptying emit `EventTag::WasteCreated` into the append-only log.
5. **Conserved quantities**: Wash consumes `clean_water_units` (already conserved). Emptying a latrine transforms accumulated fill into a `Waste` lot (source/sink explicit). No quantity created from nothing.
6. **Scarce capacities and contention**: A *clean* basin / *non-full* latrine becomes the scarce affordance. Contention via existing S44/S173 `SelfCareOccupancy` — no new queue. Cleaning occupies the facility exclusively (new `SelfCareUseKind` variants).
7. **Partial failures and aftermath**: Wash at a dirty-but-usable basin → reduced relief (partial outcome). Wash rejected (basin too dirty/dry) → no episode, `DegradedSelfCareOpportunity` recorded, planner inserts cleaning prerequisite or agent queues/travels. Toilet rejected (latrine full) → Wilderness Relief fallback (raises place dirtiness) or `empty_latrine`. Cleaning interrupted → partial; explicit abort handler releases occupancy.
8. **Positive feedback loops**: (a) More use → dirtier basin / fuller latrine → degraded or blocked self-care. (b) Blocked latrine → more Wilderness Relief → rising `PlaceDirtiness` → more ambient dirtiness pressure.
9. **Concrete dampeners** (physical, not numeric clamps): (a) Cleaning/emptying labor exists and lowers the state — an agent (or another) can always restore the facility through a real action. (b) `clean_water_units` and basin refill (item-decay system, bounded by colocated water source quantity) cap how fast a basin can be both used and cleaned — water is finite. (c) `PlaceDirtiness.decay_per_tick` (existing, item-decay system) reverses ambient dirtiness over time. (d) Wilderness Relief is always available, so bladder pressure never deadlocks — it diverts the loop into place-dirtiness aftermath rather than agent collapse. (e) Cleaning consumes the cleaner's time (occupancy), competing with their own self-care — labor scarcity dampens over-cleaning.
10. **Agent learning**: None new. Agents replan from current observation each tick. (A learned "this basin is usually filthy" preference could fold into a future learned-preference substrate if scenarios prove it necessary; not introduced here, and no such substrate exists today.)
11. **How agents can be wrong**: Believe a basin is clean / latrine empty when stale → precondition rejects at start → replan. Believe a remote facility is usable → arrive and find it blocked → `DegradedSelfCareOpportunity` recorded.
12. **Lifecycle states**: `WashBasinState`: `Clean ↔ Dirty ↔ TooDirty(blocked)` by `dirtiness_level` vs `max_effective_dirtiness`; orthogonal `Wet ↔ Dry` by `clean_water_units`. `LatrineFullness`: `Usable ↔ Full(blocked)` by `fill` vs `critical_threshold`. All transitions via use / cleaning / refill / decay — no winking.
13. **Temporal resolution**: All reads/writes at action start/commit/abort tick boundaries; basin refill and dirtiness decay at the item-decay system tick. Concurrent same-tick wash attempts on the last usable basin resolved by existing S44 tie-break.
14. **Boundary conditions**: N/A — facilities are local place-graph topology. (Water *supply* to basins is local via the colocated source; cross-boundary water is out of scope here and S177.)
15. **Derived views**: `DegradedSelfCareOpportunity` (forensic, derived). Belief-view accessors for basin dirtiness / latrine fill are the existing per-actor derived views over authoritative state (see source-class table).
16. **Causal records**: `EventTag::WasteCreated` (existing + new `LatrineEmptied` source) on overflow and emptying; cleaning action-trace details (`SelfCareInterrupted` on abort); `DegradedSelfCareOpportunity` in the active critical window. Together they reconstruct why self-care degraded.
17. **Target patterns**: Filthy shared basin → reduced wash → eventual clean; dry basin → wait for refill or travel; full latrine → wilderness relief raises place dirtiness → someone empties it; multi-agent camp where sanitation degrades over a long run and recovers through labor.
18. **Save/load and replay**: One new `Permille` field, two `NonZeroU32` profile fields, two action defs, new precondition/forensic/`WasteSource`/`SelfCareUseKind`/`MetabolismDurationKind`/`PlannerOpKind` variants — all standard ECS/profile/trace/planner state, replay-deterministic. No new authoritative event variant.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `WashBasinState` (incl. new `max_effective_dirtiness`) | Stored authoritative | Component on **Facility** (WashBasin workstation) |
| `LatrineFullness` | Stored authoritative | Component on Place |
| `PlaceDirtiness` | Stored authoritative | Component on Place |
| `MetabolismProfile.clean_basin_duration_ticks` / `empty_latrine_duration_ticks` | Stored authoritative profile parameter | Per-agent profile |
| `WasteSource::LatrineEmptied` | Stored event-payload classification | Authoritative on event emission |
| `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}` | Stored occupancy classification | On `SelfCareOccupancy` component |
| `PlannerOpKind::{CleanWashBasin, EmptyLatrine}` | Planner-internal op classification | Derived from action def via `classify_action_def`; not authoritative |
| `DegradedSelfCareOpportunity` records | Derived forensic state | View over event/trace log; not authoritative |
| Existing belief-view basin/latrine accessors | Derived per-actor view | View; not authoritative |

## Planner-formalism analysis

Plain GOAP. `clean_wash_basin` / `empty_latrine` are ordinary prerequisite operations (`PlannerOpKind::{CleanWashBasin, EmptyLatrine}`) the existing `GoalKind::Wash` / `GoalKind::Relieve` plan search inserts when the degradation precondition (`TargetWashBasinNotTooDirty` / `PlaceLatrineNotFull`) blocks the terminal self-care op — exactly as `QueueForFacilityUse` is inserted when a facility is occupied. No HTN method: no multi-stage decomposition, information-gathering stage, role-specific strategy, budget exhaustion, or utility thrash that flat affordance search cannot handle. No new `GoalKind` and therefore no `GoalDispatchKey` / `GoalDispatchDeclaration` / `GoalKindPlannerExt` surface. Information reads: all facility-condition inputs come through the existing belief-backed / same-tick-local accessors. Enforced declarations only: every new field/precondition/op has a live consumer (effectiveness scaler, precondition gate, plan-step inserter, forensic recorder). Proof surface: scenarios below.

## Belief-View Accessor Source-Class Declarations

No new accessors. D7 reads facility condition through the existing `GoalBeliefView` methods:

| Accessor (existing) | Source class | Stale/unknown behavior |
|---------------------|--------------|------------------------|
| `facility_wash_basin_state(entity) -> Option<WashBasinState>` (`belief_view.rs:495`) | FND-14A co-located; belief-backed remote | `None` if no belief and remote — preferred for the gating read |
| `wash_basin_state(agent, basin) -> WashBasinState` (`belief_view.rs:561`) | FND-14A co-located; belief-backed remote | returns default when absent; planner must treat default as "unknown" and not synthesize a remote cleaning prerequisite |
| `latrine_fullness(agent, place) -> LatrineFullness` (`belief_view.rs:557`) | FND-14A co-located; belief-backed remote | returns default when absent; same unknown-handling discipline |

These accessors do not read remote authoritative state. Who *owns* or *controls* the facility is not exposed here — that remains belief-gated per FND-14A's social-fact carve-out.

## Agent Profile Scenario Contract

`MetabolismProfile` is a universal agent profile (registered on `EntityKind::Agent`, per S128) with a `Default` impl. The two new duration fields get defaults and are scenario-overridable via the existing `metabolism_profile: Option<MetabolismProfile>` field on `AgentDef`, applied with `unwrap_or_default()`. No new component on `EntityKind::Agent`. `max_effective_dirtiness` is authored on the facility's `WashBasinState` via the existing `WashBasinStateDef` place/facility scenario contract (D10).

## Component Registration

No new components. `WashBasinState` (on `EntityKind::Facility`), `LatrineFullness`, `PlaceDirtiness` (on `EntityKind::Place`) registrations in `crates/worldwake-core/src/component_schema.rs` are unchanged (field addition only). The two new actions register through the standard `register_def` path in `needs_actions.rs::register_needs_actions`.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Wash handler (`worldwake-systems`) | `WashBasinState` (dirtiness + clean units), `MetabolismProfile` | `WashBasinState.dirtiness_level`, `HomeostaticNeeds.dirtiness` |
| Toilet handler (`worldwake-systems`) | `LatrineFullness`, `PlaceDirtiness` | `LatrineFullness.fill`, `PlaceDirtiness`, `Waste` lot |
| Cleaning handlers (`worldwake-systems`) | `WashBasinState`/`LatrineFullness`, `SelfCareOccupancy`, profile | facility state reset, `Waste` lot, `PlaceDirtiness` |
| Precondition evaluation (`worldwake-sim`) | `WashBasinState`, `LatrineFullness` (via target) | None (read-only gate) |
| Planner search (`worldwake-ai`) | belief-view facility condition, action defs via `classify_action_def` | None (inserts plan steps) |
| Survival forensics (`worldwake-ai`) | event/trace log, `CriticalWindowFrame` | `DegradedSelfCareOpportunity` records |
| Item-decay system (`worldwake-systems`) | `PlaceDirtiness`, `WashBasinState`, colocated water source | dirtiness decay, basin clean-water refill |

No system commands another; all via authoritative state and the event/trace log.

## Authoritative-to-AI Impact Analysis

This spec modifies action preconditions (D2 wash, D3 toilet) and the self-care plan search (D6/D7), so the full CLAUDE.md trace applies:

1. `get_affordances` — D4 adds evaluation arms in `affordance_query.rs` for the two new precondition variants, so a too-dirty basin / full latrine candidate is correctly suppressed at affordance time.
2. `generate_candidates` — D7 leaves `emit_wash_goal` / `emit_relieve_goal` emission shape unchanged; cleaning is not a candidate goal. Belief-backed reads gate emission.
3. `search_plan` — D6 includes `PlannerOpKind::{CleanWashBasin, EmptyLatrine}` in the Wash/Relieve relevant-op lists and `apply_planner_step`, so the search inserts a cleaning prerequisite when the terminal op is blocked (mirroring `QueueForFacilityUse`). Terminal ordering: cleaning precedes the self-care op.
4. `BestEffort` action start — the new precondition rejections at start are handled by `action_validation.rs` (D4); a rejected Wash/Toilet does not commit.
5. `handle_plan_failure` — after a too-dirty/too-full rejection the agent replans, reaching the cleaning prerequisite, the queue, or the wilderness-relief fallback. `DegradedSelfCareOpportunity` is recorded (D8).
6. Payload revalidation — `clean_wash_basin` / `empty_latrine` use affordance-derived targets (co-located facility/place); if planner-synthesized payloads are introduced, register `with_payload_override_validator` so `plan_revalidation.rs` revalidation succeeds.
7. Golden tests — the two focused goldens + the 1440-tick scenario (below) must pass; goldens depending on the old "toilet always succeeds" behavior are updated (FND-28).

## Scenario Validation (FND-31)

**Focused branch goldens:**

- **`survival-basin-dirty-dirty.ron`** — repeated washes raise basin `dirtiness_level`; relief drops proportionally; at threshold the Wash precondition fails; the planner inserts `clean_wash_basin`; subsequent wash recovers full relief. Asserts effectiveness scaling, precondition rejection, cleaning aftermath (Waste lot), and deterministic replay.
- **`survival-latrine-full.ron`** — repeated toilet uses raise `fill` to `critical_threshold`; the Toilet precondition fails; the agent falls back to Wilderness Relief (place dirtiness rises) OR runs `empty_latrine`; asserts the branch chosen, `WasteCreated` provenance (`OvercapacityLatrine` on overflow, `LatrineEmptied` on emptying), and the `DegradedSelfCareOpportunity` forensic record.

**1440-tick CI-owned collision scenario (registered in `docs/scenario-roadmap.md`, run only via `.github/workflows`):**

- **`survival-sanitation-breakdown-1440.ron`** — multiple agents share one basin and one latrine over 1440 ticks under ordinary need pressure. Sanitation degrades; agents queue (S44 occupancy), clean, empty, and fall back to wilderness relief; place dirtiness rises and decays. Assertions prove: facility-state arithmetic (dirtiness/fill cross thresholds and recover via labor), no omniscient remote-facility reads (belief barrier), `DegradedSelfCareOpportunity` causal records, and replay equivalence.

**Illegal paths this spec must not produce:** wash relief unaffected by `dirtiness_level`; toilet succeeding at/above `critical_threshold`; facility state resetting without a cleaning/decay action; a planner cleaning prerequisite synthesized for a remote facility with no belief carrier; a new `GoalKind` for cleaning; any `sanitation_score`.

## Outcome

**Completion date**: 2026-05-29 (tickets S176SANFACDEG-001..008, all archived under `archive/tickets/`).

**What landed**:
- D1/D9/D10 carriers: `WashBasinState.max_effective_dirtiness` (`Permille`, default full-scale) + `WashBasinStateDef` contract; `MetabolismProfile.clean_basin_duration_ticks` / `empty_latrine_duration_ticks` (serde-defaulted, mirroring `rough_sleep_recovery_floor` — not added to `MetabolismProfile::new()`); two `SAVE_FORMAT_VERSION` bumps (108→110); observer place summary surfaces co-located basin dirtiness/clean-units + latrine fill.
- D2/D3/D4 gates: `Precondition::{TargetWashBasinNotTooDirty, PlaceLatrineNotFull}` with evaluation in `action_validation.rs` + `affordance_query.rs`; `apply_wash` relief scales with basin dirtiness; toilet blocked at/above latrine `critical_threshold`. Added `FacilityBeliefView::latrine_fullness` for affordance gating (absent component / unknown-remote = optimistically usable; only a known-full co-located latrine is gated, with authoritative commit re-check).
- D5: `clean_wash_basin` + `empty_latrine` duration/occupancy-bearing actions with explicit abort handlers, via new `EffectStep::{CleanWashBasin, EmptyLatrine}` + `EffectSink` methods; `WasteSource::LatrineEmptied`; `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}`.
- D6/D7: `PlannerOpKind::{CleanWashBasin, EmptyLatrine}` in the `WASH_OPS`/`RELIEVE_OPS` relevant-op lists; the planner models facility condition (snapshot `latrine_fullness`, `PlanningState` overrides) and the hypothetical effect sink simulates the cleaning reset so the GOAP search finds `[clean, wash]` / `[empty_latrine, toilet]`. Synthesized self-care candidates now respect the degradation gates (a synthesized wash can no longer bypass a too-dirty basin). No new `GoalKind`/`GoalDispatchKey`.
- D8: `DegradedSelfCareOpportunity` forensic record on `CriticalWindowFrame`, derived from action-trace recovery/fallback commits + degradation-gated start failures; added `latrine_present` to `LocalSurvivalStateSummary`.
- Validation: focused goldens `survival-basin-dirty-dirty` / `survival-latrine-full` (default lane) + the `#[ignore]` 1440-tick `survival-sanitation-breakdown-1440` collision golden (registered in `docs/scenario-roadmap.md` §5.20 and the `golden-survival` CI matrix).

**Notable deviations** are documented per-ticket in the archived tickets — chiefly: `PlannerOpKind` classification landed in 004 (registry-consistency invariant); the planner facility-state model + synthesized-candidate gate were engine capability gaps fixed in 005; the 1440 scenario isolates sanitation (food/water zeroed) to stay survivable.
