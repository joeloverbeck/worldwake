# S173: Self-Care Interruption Contracts and Facility Occupancy

## Summary

At draft intake, only `sleep` had a durable interruption contract: `SleepEpisode` carried accumulated recovery, `abort_sleep_episode` ended the episode with `WakeReason::LocalDisturbance`, and the episode preserved partial progress across replans. `eat`, `drink`, `toilet`, `relieve_wilderness`, and `wash` all registered `abort_noop` (`crates/worldwake-systems/src/needs_actions.rs`) and had no occupancy state — interrupting any of them left no state, no structured trace beyond the engine-level `EventTag::ActionAborted` record, and (for Wash and Toilet) no facility release because there was no facility reservation to release. Tickets through `archive/tickets/S173SELCARINT-004.md` have since landed `SelfCareOccupancy`, Wash/Toilet occupancy release, and Wash/Toilet trace detail; `archive/tickets/S173SELCARINT-005.md` added the remaining atomic-action and Sleep trace discriminator mapping; `archive/tickets/S173SELCARINT-006.md` landed the occupancy-aware emitter filter; `archive/tickets/S173SELCARINT-007.md` added the standard golden proof for Scenarios A, B, and C; `archive/tickets/S173SELCARINT-008.md` added the player-POV symmetry proof for Scenario D. The remaining spec family proves repeated-interruption deprivation collapse.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (`SelfCareOccupancy` component, `SelfCareUseKind` enum)
- `worldwake-sim` (abort trace-detail mapping, new `ActionTraceDetail::SelfCareInterrupted` variant, decision-trace surface)
- `worldwake-systems` (per-family abort handlers replacing `abort_noop`, occupancy mutation in `wash` and `toilet`, `PromotableContentionKind` extension)
- `worldwake-ai` (candidate emission filter on occupancy; revalidation respects occupancy; failure attribution for contended/disconfirmed basins)
- `worldwake-cli` (scenario contract authoring for interruption and collapse scenarios)

## Dependencies

- `archive/specs/S172-wash-discovery-budget-closure.md` — landed first; this spec assumes Wash budget accounting is correct before adding occupancy contention
- `archive/specs/S44-generalized-contention-substrate.md` — provides `ContentionQueue`/`ContentionPolicy`
- `archive/specs/S142-contention-event-inspectability.md` — provides facility-queue promotion and `EventTag::ContentionResolved`/`QueueGrantPromoted`
- `archive/specs/S128-sleep-episode-place-quality.md` — provides the precedent: `SleepEpisode` durable abort contract that this spec mirrors for Wash and Toilet
- `archive/specs/S129-place-dirtiness-facility-wear.md` — provides `WashBasinState` (registered on `EntityKind::Facility`) and `LatrineFullness` / `PlaceTag::Latrine`
- `archive/specs/S81-golden-gaps-simulation-remediation.md` — provides the deprivation-death proof that the repeated-interruption collapse scenario extends
- `archive/specs/S163-cli-player-pov-boundary.md` — provides the player-POV gating pattern Scenario D extends

## Design Goals

- Every self-care action declares its interruption contract: start state, tick effects, commit effects, abort cleanup, recovery-visible facts, trace surface.
- Mechanically exclusive facilities (`WashBasin`-tagged `Facility`, `Latrine`-tagged `Place`) are reserved on action start and released on commit, abort, or actor incapacitation.
- Eat, Drink, and Wilderness-Relief remain atomic (no partial state) — but their abort path is explicitly mapped to the typed `ActionTraceDetail::SelfCareInterrupted` payload so "interrupted before commit" is distinguishable from "never attempted" in the action-trace sink. The authoritative causal hook remains the existing `EventTag::ActionAborted` record, already fired by the engine for every aborted action; the spec adds structured payload, not a new causal surface.
- Sleep retains its existing `SleepEpisode` contract unchanged; this spec layers the same typed trace detail above it so all six families share the same inspection shape.
- Repeated interruption can lawfully end in deprivation collapse — proven end-to-end by a scenario that composes repeated Wash interruption with the existing hunger-deprivation wound/death substrate.
- No new abstract score, no hidden rescue, no scenario-specific target injection, no planner intent-as-lock.

## Non-Goals

- No partial-progress state for Eat, Drink, Toilet, or Wilderness-Relief. They remain atomic; the spec explicitly forbids inventing partial bodily-progress math without a durable state carrier (FND-3).
- No `WashSessionProgress` duration-based partial-Wash carrier. Commit-time partial relief when clean water is insufficient (already present) is preserved.
- No new `EventTag` variant. The authoritative causal record for an interrupted self-care action is the already-firing `EventTag::ActionAborted`; the new information surfaced by this spec — which use kind, which basin/latrine — lives in `ActionTraceDetail::SelfCareInterrupted` (the typed trace-sink payload), keeping the event log free of redundant variants (FND-28).
- No relocation of `PromotableContentionKind` to `worldwake-core`. It remains crate-private to `worldwake-systems`; no core-resident type references it.
- No per-kind `ContentionPolicy` routing. The existing per-facility `ContentionPolicy` (`grant_hold_ticks`, `auto_promote`, `max_waiters`) applies uniformly to self-care facilities as to other exclusive workstations. Open Question #1 from the original draft is resolved by this commitment.
- No social etiquette, privacy, bathroom politics, disease ecology, odor, or social shame system.
- No shelter or sleep-surface scarcity model. Sleep contention remains place-level capacity (or unimplemented) until a future spec proves sleep-surface scarcity matters.
- No queue-jumping policy, patience-threshold negotiation, or social-rank arbitration. First-come first-served via the existing `S44` contention substrate.
- No recovery-memory blocker (avoid recently-failed basin). Agents replan from observation each tick (P1.3 in source report, deferred).
- No backward-compatibility shim around `abort_noop`. Replaced where applicable; removed from the five self-care call sites in `needs_actions.rs`. The `abort_noop` symbol itself remains in scope as the registered abort for non-self-care actions that legitimately have nothing to clean up.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Two dirty agents → one occupies basin → other waits or replans through ordinary world processes; Scenario E uses controlled local cancellations only to stress the already-landed interruption/occupancy carrier while death still emerges from the needs and wound systems |
| FND-3 (Concrete state) | `SelfCareOccupancy` is authoritative world state with stable identity; not a score |
| FND-4 (Persistent identity) | Occupancy carries occupant `EntityId`, started-tick, use kind, and the `GoalKey` that drove start; release/abandon are explicit transitions |
| FND-8 (Action preconditions, occupancy) | Directly satisfies: every self-care action declares preconditions, duration, occupancy, interruption, contention |
| FND-9 (Scheduling) | Occupy/release/abandon all occur at well-defined tick boundaries (action start, commit, abort, actor death) |
| FND-10 (Aftermath) | Interrupted self-care leaves traceable state (`EventTag::ActionAborted` causal record + typed `ActionTraceDetail::SelfCareInterrupted` payload) and released occupancy, never silent reset |
| FND-11 (Positive feedback) | Repeated interruption → rising hunger deprivation → starvation wounds is itself the dampener (wounds reduce capacity; eventually death stops the loop) |
| FND-19 (Agent symmetry) | Human and AI agents share identical interruption, occupancy, and contention semantics |
| FND-21 (Intentions revisable) | Losing actor does not silently reserve the basin; explicit occupancy or queue grant is required |
| FND-26 (Systems via state) | Action handlers read/write `SelfCareOccupancy`; planner reads it via belief or co-located observation; no system commands another |
| FND-28 (No backcompat) | `abort_noop` is removed from the five self-care call sites; no parallel `EventTag` variant introduced for an event already covered by `EventTag::ActionAborted`; no parallel `ContentionKind` introduced in core for a discriminator already covered by the crate-private `PromotableContentionKind` |
| FND-29 (Debuggability) | "Why didn't this agent wash?" is answerable from `SelfCareOccupancy` history + `EventTag::ActionAborted` records filtered by action name + `ActionTraceDetail::SelfCareInterrupted` typed payload + decision trace |
| FND-29A (Causal history) | Occupy/release/abandon writes and `EventTag::ActionAborted` records are append-only and survive replay; the trace detail enriches but does not replace the authoritative event |
| FND-31 (Validation) | Five scenarios cover atomic-abort, durable-occupancy-release, contested basin, repeated-interruption collapse, and player-POV symmetry |

## Deliverables

### D1. `SelfCareOccupancy` component and `SelfCareUseKind` enum

```rust
/// Authoritative world state. Attached to the facility entity (a `WashBasin`-tagged
/// `Facility`, or a `Latrine`-tagged `Place`) while a self-care action is mid-flight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfCareOccupancy {
    pub occupant: EntityId,
    pub use_kind: SelfCareUseKind,
    pub started_tick: Tick,
    /// The `GoalKey` that the occupant was pursuing when occupancy was written.
    /// Records why the occupancy started; consumed by decision-trace queries.
    pub goal_key: GoalKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfCareUseKind {
    Wash,
    LatrineRelief,
    // Sleep surfaces remain place-level for now; if scarce sleep surfaces are
    // introduced later they extend this enum.
}
```

`SelfCareOccupancy` is defined in `crates/worldwake-core/src/` (new module, e.g. `self_care_occupancy.rs`). The struct lives in core because `component_schema.rs` references component types via `crate::TypeName` and cannot register types defined in higher crates.

Component registration in `crates/worldwake-core/src/component_schema.rs` (precedent: `ResourceSource` at line 1785, `ResourceExtractionQueues` at 1835):

| Entity kind filter | Component | Lifecycle |
|--------------------|-----------|-----------|
| `\|kind\| kind == EntityKind::Facility \|\| kind == EntityKind::Place` | `SelfCareOccupancy` | Absent by default; written at action start on the targeted `Facility` (Wash) or `Place` (LatrineRelief); removed on commit/abort/abandon |

The component is runtime-managed, not scenario-authored — it mirrors `SleepEpisode`'s lifecycle on `EntityKind::Agent` (`component_schema.rs:2169-2191`). No `PlaceDef` or `FacilityDef` field is required, and no Agent-profile change applies (the component lives on the facility, not the agent). See Component Registration section.

Implementation note: although the component is registered on both `EntityKind::Facility` and `EntityKind::Place`, the Wash action handler will only ever write it on a `Facility` carrying `WorkstationTag::WashBasin`, and the Toilet action handler will only ever write it on a `Place` carrying `PlaceTag::Latrine`. The dual-kind filter exists because the same component carries occupancy semantics for both targets; no `WashBasin` `EntityKind` variant exists (`WashBasin` is a `WorkstationTag` at `crates/worldwake-core/src/production.rs:15`).

### D2. Per-action-family interruption contracts

The contract table below is the authoritative source for action-handler implementation. Every self-care action handler must match this table; deviations require a spec amendment.

| Action | Goal | Start state written | Tick effects | Commit effects | Abort cleanup | Recovery-visible facts | Trace surface |
|--------|------|---------------------|--------------|----------------|---------------|------------------------|---------------|
| `eat` | (commodity-consumption flow) | None | None | Consume 1 unit, reduce hunger, apply bladder fill | None (no state was written) | Item still controlled? Yes → may retry. Item gone → replan toward acquisition. | `ActionTraceDetail::SelfCareInterrupted { kind: Eat, basin: None }` (assumes D2 note option (i); see below) |
| `drink` | (commodity-consumption flow) | None | None | Consume 1 unit, reduce thirst, apply bladder fill | None | Item still controlled? Yes → may retry. Item gone → replan. | `ActionTraceDetail::SelfCareInterrupted { kind: Drink, basin: None }` (assumes D2 note option (i); see below) |
| `sleep` | `GoalKind::Sleep` | `SleepEpisode` (existing) | Tick accumulates `Permille` recovery | Commit ends episode with accumulated recovery | `end_sleep_episode(..., WakeReason::LocalDisturbance, ...)` (existing) | `SleepEpisode` removed; partial recovery preserved in `HomeostaticNeeds::fatigue` | Existing `EventTag::SleepEpisodeEnded`; new trace detail enriches the abort/interrupt path |
| `toilet` | `GoalKind::Relieve` | `SelfCareOccupancy { use_kind: LatrineRelief, … }` on the latrine-tagged `Place` | None | Clear bladder, create Waste, update `LatrineFullness`, increase place dirtiness, **remove occupancy** | **Remove occupancy** | Latrine still available? Yes → may retry. Latrine occupied → wait or replan to alternate. | `ActionTraceDetail::SelfCareInterrupted { kind: LatrineRelief, basin: Some(<latrine place id>) }` |
| `relieve_wilderness` | `GoalKind::Relieve` | None | None | Clear bladder, create Waste/evidence, increase actor dirtiness, increase place dirtiness | None | Same place still valid? Replan freely. | `ActionTraceDetail::SelfCareInterrupted { kind: WildernessRelief, basin: None }` — extends `SelfCareUseKind` (see note) |
| `wash` | `GoalKind::Wash` | `SelfCareOccupancy { use_kind: Wash, … }` on the `WashBasin`-tagged `Facility` | None | Reduce dirtiness, consume `clean_water_units`, dirty basin (existing); partial relief when `clean_water_units < min` (existing); **remove occupancy** | **Remove occupancy** | Basin available + clean water? Retry. Basin occupied by another? Wait or replan. Basin dry → replan to alternate or acquisition. | `ActionTraceDetail::SelfCareInterrupted { kind: Wash, basin: Some(<basin entity>) }` |

**SelfCareUseKind discriminator scope**: The table references `SelfCareUseKind` variants for trace detail on `eat`, `drink`, `sleep`, and `relieve_wilderness` even though those actions write no occupancy. The trace detail surfaces interruption attribution; it does not imply occupancy. The minimum enum required by D1 is `{ Wash, LatrineRelief }` (the only variants that carry occupancy). For trace-detail completeness, the implementation may either (i) extend `SelfCareUseKind` with non-occupancy variants `Eat`, `Drink`, `Sleep`, `WildernessRelief` — at the cost of a slightly broader enum than its occupancy role implies, but with the benefit of a single discriminator across the trace surface — or (ii) define a sibling `SelfCareTraceKind` enum carrying the six families for trace purposes only. The implementation chooses (i) at ticket time unless the broader enum produces semantic confusion at registration; the spec marks this as a trace-detail design call, not a contract change.

`abort_noop` is removed from the call sites for `eat`, `drink`, `toilet`, `wash`, and `relieve_wilderness` in `needs_actions.rs` (lines 27, 33, 45, 51, 57); replaced with two new handlers:

- `abort_release_self_care_occupancy` for `toilet` and `wash` — removes `SelfCareOccupancy` from the facility/place.
- `abort_emit_self_care_interrupted` for `eat`, `drink`, and `relieve_wilderness` — no state effect; the named handler keeps the registration surface explicit.

The action-trace detail is populated at the `tick_step.rs::abort_trace_detail_for_instance` emission boundary, because `ActionExecutionContext` does not carry an `ActionTraceSink`. That trace-side helper maps `eat`, `drink`, `sleep`, `toilet`, `wash`, and `relieve_wilderness` to `ActionTraceDetail::SelfCareInterrupted` alongside the existing engine-level `EventTag::ActionAborted` emission in `tick_action.rs` / `interrupt_abort.rs` — it enriches, not replaces, the authoritative causal record.

### D3. `PromotableContentionKind` extension

`crates/worldwake-systems/src/facility_queue.rs::promotable_contention_kind` (line 463) is extended to classify self-care actions by `(ActionDomain, action_name)`, matching the existing pattern (`(ActionDomain::Corpse, "loot" | "bury") => Corpse`, `(ActionDomain::Care, "heal") => Care`). Self-care actions use `ActionPayload::None` and `ActionDomain::Needs` (`needs_actions.rs:109, 152`), so the new match arms are:

```rust
(ActionDomain::Needs, "wash") => Some(PromotableContentionKind::SelfCareWash),
(ActionDomain::Needs, "toilet") => Some(PromotableContentionKind::SelfCareLatrine),
```

The `PromotableContentionKind` enum (`facility_queue.rs:29`) gains two new variants:

```rust
enum PromotableContentionKind {
    FacilityExclusive(WorkstationTag),
    Corpse,
    Care,
    SelfCareWash,       // new
    SelfCareLatrine,    // new
}
```

The enum remains crate-private (not `pub`) to `worldwake-systems`; no core-resident type references it. The `Tag` mirror pattern from `worldwake-validation-patterns.md` does not apply.

`exclusive_facility_workstation_tag` (`facility_queue_actions.rs:152-172`) matches only on `ActionPayload::Harvest` and `ActionPayload::Craft`. It does not match on `wash`/`toilet` (which use `ActionPayload::None`), so the D3 implementation added explicit `ActionDomain::Needs` name-based classification rather than flowing through the existing `FacilityExclusive(WashBasin)` auto-promotion path; both `SelfCareWash` and `SelfCareLatrine` were genuine net-new variants.

`relieve_wilderness` is NOT classified — wilderness relief is location-flexible and does not require occupancy. If a future spec introduces specific scarce wilderness-relief affordances, that classification is added then.

`sleep` is NOT classified at the facility-queue layer — sleep already has `SleepEpisode` as its durable carrier, and sleep-surface scarcity is a separate future spec.

`ContentionPolicy` selection is unchanged: the same per-facility policy applies to self-care facilities as to other exclusive workstations (resolves Non-Goal on per-kind policy routing).

### D4. Reservation requirements on `wash` and `toilet` action handlers

The `reservation_requirements: Vec::new()` on the `wash` and `toilet` action registrations in `crates/worldwake-systems/src/needs_actions.rs` (via the shared `register_def` helper, line 170 default) is replaced with a single-entry reservation requirement: the target facility must be reservable (no current `SelfCareOccupancy`) for the action to start. On start, `SelfCareOccupancy` is written; on commit (in `commit_wash` and `commit_toilet`) or abort (via D2's new handlers), it is removed.

If the contention substrate (`S44` `ContentionQueue` at `crates/worldwake-core/src/contention.rs`) is used, agents that lose the race join the queue with the existing grant/expiry semantics. The spec reuses S44; it does not introduce a parallel queue.

### D5. Belief-source classification for occupancy

The Wash and Relieve candidate paths read facility occupancy state subject to FND-14A and FND-14B:

| Input | Source class | Stale/unknown behavior |
|-------|--------------|------------------------|
| Own basin occupancy (actor is the occupant) | Self | Always known |
| Co-located basin occupancy | Same-tick local physical observation (FND-14A) | Read from world state when actor is at the basin's place |
| Remote basin occupancy | Belief-backed (FND-14B) | If no belief, no candidate (no plan composed assuming the basin is free); revalidation at action start drops the action without a plan if the basin is occupied on arrival |

Ticket `S173SELCARINT-006` corrected the original draft assumption that no new read surface was required. `facility_wash_basin_state` / `wash_basin_state` remain the water-state accessors; self-care occupancy is exposed to candidate generation through `GoalBeliefView` / `FacilityBeliefView::self_care_occupant`. That accessor reads current `SelfCareOccupancy` only for self/current-place/co-located targets and reads remote occupancy only from an existing belief-backed contention grant carrier (`BelievedContentionState::grant_holder`). This keeps the FND-14A/14B split central instead of letting candidate generation query remote authoritative occupancy.

### D6. Candidate-emitter occupancy filter

`crates/worldwake-ai/src/candidate_generation.rs::wash_access_opportunities` filters wash candidates on `facility_wash_basin_state(*workstation).is_some_and(|state| state.clean_water_units > 0)` and, as of `S173SELCARINT-006`, drops candidates when `self_care_occupant(*workstation)` reports an occupant other than the actor. The occupancy check is conditioned on the FND-14B source-class table in D5: if the actor is the occupant or co-located with the facility, the accessor can read current `SelfCareOccupancy` from world state (FND-14A); if remote, it gates only on belief-backed contention grant state.

The symmetric filter applies to the Relieve candidate path's latrine-target enumeration (the emitter site for `GoalKind::Relieve` candidates over latrine-tagged `Place` entities; precise function name to be confirmed at ticket time — the wash and relieve emitters both live in `candidate_generation.rs` and follow the same shape).

Candidates whose target facility is known to be occupied by another actor are filtered out. If revalidation at action start later finds the basin occupied (because state changed between candidate emission and action start), the action fails to start; the planner replans next tick from current state. Queue-join is achievable via the existing `PlannerOpKind::QueueForFacilityUse` operator (`planner_ops.rs:23`), already reachable in `WASH_OPS` and `RELIEVE_OPS` (`goal_schema.rs:100-101`).

### D7. `ActionTraceDetail::SelfCareInterrupted` variant

A new variant is added to the `ActionTraceDetail` enum in `crates/worldwake-sim/src/action_trace.rs:32-63`:

```rust
SelfCareInterrupted {
    kind: SelfCareUseKind,
    basin: Option<EntityId>,
},
```

This variant carries the structured "which use kind, which facility/place" payload that the existing `ActionTraceKind::Aborted { instance_id, reason: String }` (line 65-86) does not type. It is populated by `tick_step.rs::abort_trace_detail_for_instance` when `step_tick()` emits an abort `ActionTraceEvent`.

Note: `ActionTraceDetail::from_payload` (line 740+) currently returns `None` for `ActionPayload::None`, which is what all six self-care actions register. The abort trace helper therefore populates `detail` directly rather than via `from_payload`. No change to `from_payload` is required.

No new `EventTag` variant. The authoritative causal record for an interrupted self-care action remains `EventTag::ActionAborted`, which already fires from `tick_action.rs:96, 188, 220, 265` and `interrupt_abort.rs:147` for every aborted action. This deliverable adds structured payload, not a new causal surface (FND-28, FND-29A).

### D8. Repeated-interruption deprivation-collapse trace

The spec proves end-to-end that repeated self-care interruption can lawfully end in deprivation collapse. Live reassessment for `S173SELCARINT-009` narrowed the death axis: the current deprivation system creates wounds for hunger and thirst only; bladder critical exposure causes an accident, and dirtiness/fatigue exposure does not currently create wounds. The proof shape:

1. Agent has critical dirtiness and rising hunger.
2. Agent autonomously attempts Wash and is interrupted before commit by repeated local cancellation in the golden harness.
3. Abort cleanup releases occupancy, emits `EventTag::ActionAborted` (engine-level, already authoritative), and populates `ActionTraceDetail::SelfCareInterrupted` (structured detail).
4. The agent retries Wash while hunger continues to rise under the normal needs system.
5. After enough cycles, `DeprivationExposure::hunger_critical_ticks` crosses `MetabolismProfile::starvation_tolerance_ticks`; starvation wounds accumulate; wound load eventually exceeds capacity; `DeathCause::NeedDeprivation { need: Hunger }` fires with `EventTag::Death`.
6. The event log + action trace + authoritative world state expose every step: each interruption, each release, the accumulating exposure, starvation wound creation, and the eventual death.

This is implemented by Scenario E below; no new mechanism is required, only a scenario that composes existing carriers.

### D9. Player POV CLI assertions for occupancy

The CLI must not display `SelfCareOccupancy` for a basin the controlled agent has no lawful observation of. The assertion is added to the same Scenario D location as in S172 (extended scope) and follows the S163 (`archive/specs/S163-cli-player-pov-boundary.md`) gating pattern.

## Authoritative-to-AI Impact Analysis

D4 modifies action preconditions (`reservation_requirements`); D6 modifies candidate emission. Per CLAUDE.md's 7-point Authoritative-to-AI Impact Rule:

1. `get_affordances` — **flag**: the occupancy gate is a new criterion in affordance discovery for Wash/Relieve; verify the affordance enumerator surface (`affordance_query.rs`) reads `SelfCareOccupancy` via the FND-14A/14B-split source class from D5, not authoritative world state on behalf of remote actors.
2. `generate_candidates` — **flag**: `wash_access_opportunities` and the relieve-candidate emitter gain occupancy filters (D6) alongside the existing `clean_water_units > 0` filter.
3. `search_plan` — N/A: no plan-search semantics change. `WASH_OPS`/`RELIEVE_OPS` op-relevance lists are unchanged; queue-join via `PlannerOpKind::QueueForFacilityUse` is already reachable.
4. `BestEffort` action start — **flag**: the new `reservation_requirements` entry on `wash` and `toilet` (D4) is checked at start; when revalidation rejects, the start fails gracefully and the planner replans next tick.
5. `handle_plan_failure` — **flag**: replan path after revalidation rejects an occupied facility/latrine. The existing failure-attribution code in `agent_tick.rs::handle_plan_failure` should already cover this via generic precondition rejection; spec validation confirms it does at ticket time.
6. Payload revalidation — **flag**: self-care actions use `ActionPayload::None`, so payload revalidation passes through `requested_affordance_matches` rather than a payload-override validator. The occupancy check is a precondition check, not a payload-override check; verify at ticket time that the precondition path is exercised for None-payload actions.
7. Golden tests — **flag**: Scenarios A–E (Scenario Validation section below) cover atomic-abort, contested basin, interrupted-release, player POV symmetry, and repeated-interruption collapse. All must pass.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Without per-action interruption contracts and basin occupancy, two dirty agents can "use" the same basin simultaneously without world conflict, an interrupted Wash leaves no structured trace beyond the generic `EventTag::ActionAborted` record, and "why didn't this agent wash?" cannot be answered with typed evidence. The collision-proof loop (basin → contention → wait/replan → interrupted → release → retry → eventual relief or lawful failure) is impossible to demonstrate.

2. **New entities/relations/records**:
   - `SelfCareOccupancy` component on `EntityKind::Facility` (Wash) and `EntityKind::Place` (latrine).
   - `SelfCareUseKind` enum (minimally `Wash`, `LatrineRelief`; trace-detail completeness may add `Eat`, `Drink`, `Sleep`, `WildernessRelief` — see D2 note).
   - `ActionTraceDetail::SelfCareInterrupted { kind, basin }` variant (structured trace payload).
   - `PromotableContentionKind::SelfCareWash` and `::SelfCareLatrine` (extension of the crate-private `PromotableContentionKind` enum in `worldwake-systems`; not promoted to core).
   - No new `EventTag` variant. The authoritative causal record is the already-firing `EventTag::ActionAborted`.

3. **Actions that mutate them**:
   - `wash` action start writes `SelfCareOccupancy`; commit and abort remove it.
   - `toilet` action start writes `SelfCareOccupancy`; commit and abort remove it.
   - Actor death or place departure mid-action triggers abandon cleanup (S44 substrate already handles this via grant expiry; spec reuses).
   - `tick_step.rs::abort_trace_detail_for_instance` populates `ActionTraceEvent.detail` with the new typed variant at the trace emission boundary.

4. **Information production and travel**:
   - Co-located agents observe basin occupancy via FND-14A same-tick local observation.
   - Remote occupancy must be belief-backed (FND-14B; via perception, reports, witness testimony).
   - `EventTag::ActionAborted` records travel through the authoritative event log; `ActionTraceDetail::SelfCareInterrupted` enriches the action-trace sink for observers and goldens; both surface through the agent's own decision-trace for replanning.

5. **Conserved quantities**: No new conserved resource. `WashBasinState::clean_water_units` continues to be conserved through Wash commit. `SelfCareOccupancy` is a non-conserved presence claim (no quantity).

6. **Scarce capacities and contention**: `SelfCareOccupancy` is the carrier of exclusive use. One occupant per `WashBasin`-tagged `Facility` or `Latrine`-tagged `Place` at a time. Contention via `S44` `ContentionQueue` with grant/expiry, classified by the new `PromotableContentionKind::SelfCareWash` / `SelfCareLatrine` variants. Wilderness-relief is not scarce.

7. **Partial failures and aftermath**: Five lawful abort/failure shapes:
   - Atomic abort with no state change (Eat/Drink/Wilderness-Relief): `EventTag::ActionAborted` fires (existing); action trace maps the abort to `ActionTraceDetail::SelfCareInterrupted` for typed inspection.
   - Atomic abort with occupancy release (Toilet/Wash): `EventTag::ActionAborted` fires (existing); abort handler removes `SelfCareOccupancy`, and action trace maps the abort to `ActionTraceDetail::SelfCareInterrupted`.
   - Durable abort with partial-progress preserved (Sleep): existing `SleepEpisode` aftermath + `EventTag::SleepEpisodeEnded`; action trace maps the abort to `ActionTraceDetail::SelfCareInterrupted`.
   - Action start blocked by contention: queue join via S44; no occupancy written; no abort fires.
   - Repeated interruption → deprivation collapse: hunger deprivation wounds accumulate; eventual death (S81 substrate).

8. **Positive feedback loops**:
   - Interruption → replan → interrupted again is the candidate loop. The dampener is point 9.
   - Contention → wait → other agents arrive → longer queue is bounded by FCFS grant expiry and by agents replanning to alternate facilities or absorbing the deprivation.

9. **Physical dampeners**:
   - Deprivation wounds (S17/S81): accumulating hunger or thirst critical exposure produces concrete wound state that reduces capacity and eventually kills the agent. The interruption→retry loop is bounded by the agent's mortality. Live reassessment for `S173SELCARINT-009` confirmed that dirtiness/fatigue exposure does not currently produce deprivation wounds, and bladder exposure produces an accident rather than death.
   - `WashBasinState::clean_water_units` depletion: contested basins also run dry, ending contention by removing the affordance.
   - Travel cost: alternate basin is non-free; agents may absorb dirtiness rather than walk far.
   - Sleep recovery preserved across interruptions: each partial sleep does reduce fatigue, so repeated short sleeps cumulatively help.

10. **Agent learning**: None added by this spec. An agent that keeps trying the same blocked basin will keep getting precondition rejections at action start; their next-tick candidate evaluation reads current state and may choose differently based on the updated belief. P1.3 recovery memory remains deferred.

11. **How agents can be wrong**:
    - Believe basin is free when it is occupied (stale belief). Revalidation at action start rejects; agent replans.
    - Believe basin has clean water when it is dry. Existing `wash_preconditions` rejection path applies; no new failure variant introduced.
    - Believe wilderness relief is safe when a predator is en route. Standard interruption; abort cleanup populates `ActionTraceDetail::SelfCareInterrupted { kind: WildernessRelief, basin: None }`.

12. **Lifecycle states**:
    - `SelfCareOccupancy`: `Reserved` (action start) → `Released` (commit) | `AbandonedOnAbort` (abort) | `AbandonedOnIncapacitation` (actor death or place departure).
    - All transitions are explicit world processes — no decay timer, no silent cleanup.

13. **Temporal resolution**: Occupy/release/abandon happen at the action-start, action-commit, action-abort, and incapacitation tick boundaries. Concurrent same-tick attempts on a free basin are resolved by the existing S44 contention tie-break.

14. **Boundary conditions**: Not applicable — self-care is local.

15. **Derived views**: None new. `SelfCareOccupancy` is authoritative. Planner snapshots may read it co-located (FND-14A) or via belief, but the snapshot is not authoritative. `ActionTraceDetail::SelfCareInterrupted` is derived trace-sink payload, not authoritative state.

16. **Causal records**:
    - `SelfCareOccupancy` writes/removals appear in the event log (component lifecycle).
    - `EventTag::ActionAborted` (existing) records every aborted self-care action; action name and actor are already carried in the event metadata.
    - `ActionTraceDetail::SelfCareInterrupted` enriches the action-trace sink for observer/golden consumption.
    - `EventTag::ContentionResolved` (existing S142 substrate) fires on grant/expiry.
    - `EventTag::QueueGrantPromoted` (existing) fires on queue promotion.
    - The `goal_key` field on `SelfCareOccupancy` records the `GoalKey` that drove start, supporting "why did this agent start washing here" trace queries.
    - Repeated-interruption deprivation collapse is traceable end-to-end via existing event log.

17. **Target patterns**:
   - Two dirty agents, one basin → one occupies → other waits → first commits → second occupies.
   - Wash interrupted by hostile presence → occupancy released → agent replans to alternate.
   - Repeated Sleep interruption preserves accumulated recovery; eventually agent rests fully.
   - Repeated Toilet interruption → bladder accident → place dirtiness rises → wilderness relief substitution.
   - Repeated Wash interruption + rising dirtiness → deprivation wound (existing severity ladder) → eventual collapse.

18. **Save/load and replay**: `SelfCareOccupancy` is standard ECS state; `ActionTraceDetail` is standard action-trace payload. Both are replay-deterministic. No new authoritative event variant means no new save-format consideration beyond the component itself.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `SelfCareOccupancy` | Stored authoritative state | Component on facility/place entity |
| `SelfCareUseKind` | Stored value (inside `SelfCareOccupancy` and on trace detail) | Authoritative when carried by `SelfCareOccupancy`; derived when carried by trace |
| `EventTag::ActionAborted` records (existing) | Stored authoritative event-log entry | Append-only history |
| `ActionTraceDetail::SelfCareInterrupted` | Derived trace-sink payload | View; not authoritative |
| `PromotableContentionKind::SelfCareWash` / `SelfCareLatrine` | Stored classification (crate-private enum variant) | Authoritative within `worldwake-systems`; not in core |
| Planner-visible basin-free hint | Derived (read of `SelfCareOccupancy` presence via belief or co-location) | View; not authoritative |

## Planner-formalism analysis

Wash and Relieve candidate emission remains plain GOAP. No HTN method is registered. Contention-aware planning is achieved through:

1. Candidate emission filters out facilities/places whose occupancy is known (via belief or co-location) to be held by another actor (D6).
2. If revalidation at action start finds the target held, the action fails to start and the planner replans next tick from current state.
3. Queue-join via S44 `QueueForFacilityUse` op is already supported (`PlannerOpKind::QueueForFacilityUse` exists at `planner_ops.rs:23`). The op is already reachable in `WASH_OPS` and `RELIEVE_OPS` (`goal_schema.rs:100-101`). No op-classification change is required.

No method-required goal is introduced. No new HTN schema contract.

## Systemic-validation analysis (FND-31)

| Check | Negative case | Mechanism |
|-------|---------------|-----------|
| No simultaneous use | Two `wash` actions commit at the same basin at the same tick | Reservation requirement (D4); second action start fails the new precondition check; assertion in Scenario B |
| No silent rescue | Interrupted Wash silently re-runs from start with state restored | Abort cleanup explicitly removes occupancy; replan must go through normal candidate emission; assertion in Scenario C |
| No planner-intent lock | Agent A "plans to use" basin → agent B cannot use it | Spec forbids: intent is not entitlement (FND-21). Only `SelfCareOccupancy` blocks; written only at action start |
| Replay determinism | `SelfCareOccupancy` write/remove order differs across replays | Standard ECS replay invariants; assertion in long-run scenario replay-equivalence test |
| No remote-truth leak | Agent A reads remote basin's `SelfCareOccupancy` directly | Belief-source classification table (D5); assertion in Scenario D player-POV test |
| Collapse traceability | Death from repeated interruption without traceable cause | `DeathCause::NeedDeprivation` + per-action `EventTag::ActionAborted` records filtered by actor and action name + `ActionTraceDetail::SelfCareInterrupted` payloads in trace; Scenario E |
| No backward-compat | `abort_noop` still registered for `eat`/`drink`/`toilet`/`wash`/`relieve_wilderness` after this lands | Compile-time check: the self-care `abort_noop` call sites in `needs_actions.rs` are replaced by the new abort handlers |
| No parallel causal surface | New `EventTag::SelfCareInterrupted` variant added in addition to existing `ActionAborted` | Spec forbids: D7 explicitly reuses `EventTag::ActionAborted` for the causal record; only the typed trace detail is new |

## SystemFn Integration

No new SystemFn is introduced. The spec modifies:

- Action handlers for `wash`, `toilet`, `eat`, `drink`, `relieve_wilderness` in `crates/worldwake-systems/src/needs_actions.rs`.
- `promotable_contention_kind` and `PromotableContentionKind` in `crates/worldwake-systems/src/facility_queue.rs`.
- `ActionTraceDetail` enum in `crates/worldwake-sim/src/action_trace.rs` (new variant only).
- Candidate emitters in `crates/worldwake-ai/src/candidate_generation.rs` (`wash_access_opportunities` and the relieve emitter).
- Component registration in `crates/worldwake-core/src/component_schema.rs`.

Ordering against other systems is unchanged. `SelfCareOccupancy` writes/removes happen synchronously inside action start/commit/abort, which already run at well-defined tick boundaries.

## Component Registration

| Component | EntityKind filter | Classification | Default |
|-----------|------------------|----------------|---------|
| `SelfCareOccupancy` | `\|kind\| kind == EntityKind::Facility \|\| kind == EntityKind::Place` | Role-specific, runtime-managed | Absent — written only during action lifetime, removed on commit/abort/abandon |

The component is runtime-managed (no scenario authoring), mirroring `SleepEpisode`'s lifecycle on `EntityKind::Agent`. No `AgentDef`, `PlaceDef`, or `FacilityDef` field is required. The Agent Profile Scenario Contract in `docs/spec-drafting-rules.md` §5 does not apply because no Agent-side component is added.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| `archive/specs/S172-wash-discovery-budget-closure.md` | Wash candidate emission must filter on known occupancy (D6) | State-mediated read |
| S44/S142 (contention substrate) | `SelfCareOccupancy` + `PromotableContentionKind::SelfCareWash`/`SelfCareLatrine` participate in existing queue/grant flow | State-mediated |
| S128 (Sleep episode) | Sleep retains its existing contract; `ActionTraceDetail::SelfCareInterrupted` payload is layered for uniform inspection | Trace-only, no behavior change |
| S129 (WashBasinState, LatrineFullness, PlaceTag::Latrine) | Basin water/dirtiness state continues to gate `wash_preconditions`; occupancy is a parallel gate | State-mediated |
| S81 (Deprivation death) | Repeated interruption + rising deprivation → existing death pathway | State-mediated; no new mechanism |
| `archive/specs/S163-cli-player-pov-boundary.md` | UI surfaces `SelfCareOccupancy` only for co-located or belief-known facilities/places | State-mediated via belief view |
| Action engine (existing) | New abort handler variants registered; existing `EventTag::ActionAborted` emission unchanged; `tick_step` maps abort traces to typed self-care detail | State-mediated |

## Profile-Driven Parameters

The spec adds no new `MetabolismProfile` field. Existing fields used:

- `MetabolismProfile::wash_ticks: NonZeroU32` (`crates/worldwake-core/src/needs.rs:166`) — per-agent Wash duration.
- `MetabolismProfile::toilet_ticks: NonZeroU32` (`needs.rs:164`) — per-agent Toilet duration.
- `MetabolismProfile::hunger_rate: Permille` (`needs.rs`) — per-agent hunger accumulation rate used by Scenario E's collapse proof.
- `MetabolismProfile::starvation_tolerance_ticks: NonZeroU32` (`needs.rs`) — ticks at critical hunger before starvation wounds worsen.
- `DeprivationExposure::hunger_critical_ticks: u32` (`needs.rs`) — cumulative ticks at critical hunger; drives starvation-wound emergence per S17/S81.
- S44 `ContentionPolicy` per facility entity (`crates/worldwake-core/src/contention.rs:52-56`: `grant_hold_ticks`, `auto_promote`, `max_waiters`) — scenario-configurable, applied uniformly to self-care facilities as to other exclusive workstations.

No hardcoded constants. No new `Permille` field. Scenario authors configure all interruption-relevant pressures (hostile presence frequency, deprivation thresholds, basin clean-water capacity) via existing profiles and scenario `.ron` parameters.

## Scenario Validation

### Scenario A — Per-family abort emits `ActionTraceDetail::SelfCareInterrupted`

One agent attempts each of the five action families. An external interruption (planner replan to higher-priority goal, or local disturbance) aborts each before commit.

Assertions:
- `EventTag::ActionAborted` fires for each action (engine-level, already authoritative).
- `ActionTraceEvent.detail = Some(ActionTraceDetail::SelfCareInterrupted { kind, basin })` for each — `kind` discriminates Eat/Drink/Sleep/LatrineRelief/WildernessRelief/Wash, `basin` is `Some(id)` for Toilet+Wash and `None` for the others.
- Eat and Drink: no item consumed; possession unchanged.
- Sleep: `SleepEpisode` ends with `WakeReason::LocalDisturbance`; `accumulated_recovery` preserved as a `HomeostaticNeeds::fatigue` reduction.
- Toilet and Wash: `SelfCareOccupancy` written at start, removed at abort.
- Wilderness-Relief: no `SelfCareOccupancy` written; no Waste created.

### Scenario B — Contested basin, one occupant, other waits or replans

Two dirty agents start at the same `WashBasin`-tagged `Facility` with one `clean_water_units` budget. Agent A's Wash action starts and writes `SelfCareOccupancy`. Agent B attempts Wash same tick.

Assertions:
- Only Agent A's action commits.
- Agent B's candidate is filtered (D6) or revalidation rejects — assertion that no second `Wash` action commits on the same basin in the same tick.
- Agent B either joins the contention queue (S44) or replans (alternate basin if known; wait-for-deprivation if not). Both paths are lawful.
- `EventTag::ContentionResolved` fires (S142 substrate).

### Scenario C — Interrupted Wash releases basin and recovers

Agent A starts Wash; before commit, a hostile predator (or higher-priority self-care) interrupts. Agent B is queued or present.

Assertions:
- Agent A's abort fires `EventTag::ActionAborted` (engine-level) and populates `ActionTraceDetail::SelfCareInterrupted { kind: Wash, basin: Some(<id>) }`.
- `SelfCareOccupancy` is removed from the basin.
- Agent B (if queued) receives a grant within the configured grant-expiry window OR Agent A's next-tick replan re-attempts the basin if it is still free.
- No leftover occupancy at end of run.

### Scenario D — Player POV symmetry for occupancy

Landed by `archive/tickets/S173SELCARINT-008.md`: `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent` extends the S172 Scenario D player-POV surface with `SelfCareOccupancy`. The controlled agent remains at a place without a co-located basin and without belief about the remote basin; the remote basin carries authoritative occupancy, but `FacilityBeliefView::self_care_occupant` returns `None` for the controlled agent. The same test also asserts the co-located occupant can see the occupancy via the FND-14A path.

### Scenario E — Repeated interruption → lawful deprivation collapse

CI-only ignored golden. One AI-controlled agent has critical dirtiness and rising hunger. The agent repeatedly selects a local Wash action; the harness cancels those Wash actions before commit while hunger continues to rise under the normal needs system. Live reassessment selected hunger because the current deprivation system creates wounds only for hunger and thirst; dirtiness/fatigue critical exposure does not currently produce wounds, and bladder critical exposure produces an accident rather than death.

Assertions:
- At least three Wash interruptions occur before death.
- `EventTag::ActionAborted` and `ActionTraceDetail::SelfCareInterrupted` expose every interrupted Wash.
- `SelfCareOccupancy` is released between retries.
- `DeprivationExposure::hunger_critical_ticks` climbs across the run and starvation wounds appear.
- Eventually `DeathCause::NeedDeprivation { need: Hunger }` + `EventTag::Death` fires for the agent.
- No actions start after death.
- Replay companion is deterministic.

## Risks and Open Questions

1. **Sleep-surface scarcity remains place-level.** The spec explicitly defers sleep-surface identity. If a future scenario reveals two agents trying to sleep at the same scarce surface and the place-capacity model is insufficient, a follow-up spec adds `SleepSurface` and extends `SelfCareUseKind`.
2. **`WashSessionProgress` deferred.** Duration-based partial Wash is interesting (Wash for 5 of 10 ticks, get partial dirtiness reduction) but invisible without a durable carrier. Deferred unless a scenario proves it matters.
3. **`SelfCareUseKind` discriminator scope (D2 note).** Whether to extend `SelfCareUseKind` with `Eat`/`Drink`/`Sleep`/`WildernessRelief` for trace-detail completeness vs. introduce a sibling `SelfCareTraceKind` enum is an implementation-time call; the spec describes the trade-off in D2 and leaves the resolution to ticket-time.
4. **Scenario E run length.** The landed ignored golden bounds the run to 160 ticks by authoring a short `starvation_tolerance_ticks` value. It remains in the ignored golden-survival lane because it is a composite end-to-end scenario.
5. **Deprivation-wound threshold field location.** Live reassessment for `S173SELCARINT-009` resolved this: starvation wounds use `MetabolismProfile::starvation_tolerance_ticks`; dehydration wounds use `dehydration_tolerance_ticks`; dirtiness/fatigue do not currently create deprivation wounds.
6. **`abort_noop` call sites elsewhere.** The spec replaces `abort_noop` for the five self-care actions in `needs_actions.rs:27, 33, 45, 51, 57`. The `start_gate.rs` `abort_noop` (line 399) is inside `#[cfg(test)]` (line 308 boundary) and is unaffected. The `abort_noop` symbol itself is preserved as the registered abort for any non-self-care action that legitimately has nothing to clean up.

## Out of Scope (Tracked Elsewhere)

- Wash budget closure and discovery — `archive/specs/S172-wash-discovery-budget-closure.md`.
- Recovery-memory blockers (avoid retrying a recently-failed basin) — deferred (P1.3 in source report).
- Sleep-surface scarcity / `SleepSlot` — deferred unless a scenario proves it matters.
- `WashSessionProgress` duration-partial state — deferred.
- Self-care patience profiles (when to abandon a queue) — deferred (P1.2 in source report; existing S44 grant-expiry suffices for first pass).
- Disease, sanitation economy, etiquette, privacy, social shame — deferred (P2 in source report).
- Adjacent-cluster redesign (pursuit, obligation, trade, theft, justice, combat as interruption sources) — out of scope; this spec uses them only as pressure sources, not as redesign targets.
