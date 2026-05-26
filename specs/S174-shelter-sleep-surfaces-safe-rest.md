# S174: Shelter, Sleep Surfaces, and Safe-Rest Consequence Carrier

## Summary

At draft intake, sleep is the weakest major self-care family. `SleepEpisode` (per `archive/specs/S128-sleep-episode-place-quality.md`) carries duration and partial recovery; `SleepQualityProfile` modulates recovery by place; `WakeReason` discriminates IntendedDuration / TargetRecovery / ProjectedNeedBreach / ScheduledCommitment / LocalDisturbance (`crates/worldwake-core/src/decision_event_payload.rs`); and `SelfCareUseKind::Sleep` was added by `archive/specs/S173-self-care-interruption-occupancy.md` for trace-detail completeness — it is currently constructed at `crates/worldwake-sim/src/tick_step.rs::abort_trace_detail_for_instance` for sleep aborts, and this spec replaces that use with the new structured `SleepInterrupted` variant. But sleep currently has no rest-site occupancy: multiple agents can intend to sleep in the same shelter without contention, the planner treats sleep as `FeasibilityStrategy::AlwaysLikely` in `crates/worldwake-ai/src/goal_schema.rs::DECL_SLEEP`, `WakeReason::LocalDisturbance` is a single coarse variant with no structured cause, and "why did this sleep fail or recover poorly?" cannot be answered with typed evidence. This spec introduces rest-site identity, multi-occupant rest capacity, a two-path Sleep goal schema (known rest site vs. rough sleep fallback), structured wake-reason causes, and a failed-rest forensic record — the consequence carriers that turn fatigue recovery into a situated survival decision.

S173 explicitly deferred this work in its Non-Goals: "No shelter or sleep-surface scarcity model. Sleep contention remains place-level capacity (or unimplemented) until a future spec proves sleep-surface scarcity matters." This spec is that future spec.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (`RestCapacity` + `RestOccupancy` components on Place; `WakeReason` extension with `LocalDisturbance { cause }`; `SleepFailureCause` enum; profile field for rough-sleep recovery floor)
- `worldwake-sim` (action-trace `ActionTraceDetail::SleepInterrupted { cause }`; `EventTag::SleepEpisodeEnded` payload extension)
- `worldwake-systems` (sleep action handler writes/releases `RestOccupancy`; place-level capacity precondition; sleep-quality lookup integrates rough-sleep floor)
- `worldwake-ai` (Sleep goal schema split into `KnownRestSite` belief-backed branch + `RoughSleep` AlwaysLikely fallback; rest-site candidate generator; failed-rest forensic record in `SurvivalForensicExtractor`)
- `worldwake-cli` (scenario contract for `RestCapacity`, rough-sleep recovery floor, and rest-site discovery; player-POV gating for occupancy)

## Dependencies

- `archive/specs/S128-sleep-episode-place-quality.md` — provides `SleepEpisode`, `SleepQualityProfile`, `ShelterTag`, `GroundComfortTag`, `SleepRecoveryModifier`, and the `WakeReason` enum that this spec extends.
- `archive/specs/S173-self-care-interruption-occupancy.md` — provides the `SelfCareOccupancy` precedent (single-occupant pattern this spec mirrors with multi-occupant `RestOccupancy`), the per-action interruption-contract discipline, and `ActionTraceDetail::SelfCareInterrupted`.
- `archive/specs/S172-wash-discovery-budget-closure.md` — provides the budget-exhaustion / decision-trace branch pinning pattern this spec applies to the Sleep goal schema.
- `archive/specs/S44-generalized-contention-substrate.md` — provides `ContentionQueue`/`ContentionPolicy` reused for queueing on full rest sites (no parallel queue).
- `archive/specs/S120-survival-critical-window-forensics.md` — provides `SurvivalForensicExtractor` and `CriticalWindowFrame` that this spec extends with failed-rest-opportunity records.
- `archive/specs/S163-cli-player-pov-boundary.md` — provides the player-POV gating pattern used for `RestOccupancy` CLI assertion.

## Design Goals

- Rest-site identity and capacity become concrete world state. A "two-bedroll shelter" is not a planner assumption; it is `RestCapacity = 2` on the place.
- Sleep candidate enumeration splits into two lawful branches: `KnownRestSite` (belief-backed, higher recovery, possibly contested) and `RoughSleep` (always-available fallback, lower recovery, more interruptible). Both branches pass through the same `SleepEpisode` carrier.
- `WakeReason::LocalDisturbance` is replaced with `LocalDisturbance { cause: SleepFailureCause }` so wake events answer "why did this sleep end?" without trace bloat.
- A new failed-rest-opportunity record in `SurvivalForensicExtractor` captures the case where the planner considered a rest site but could not take it (occupied, dirty, exposed, contested) — enabling the forensic chain S175 needs for repeated-failed-rest deprivation collapse.
- No hidden `safe_to_sleep` boolean, no global night-danger roll, no abstract safety score, no hotel/bed ownership. Safety is decomposed into observable concrete inputs (rest capacity, occupancy, place dirtiness, shelter tag, remembered local disturbance, co-located hostile presence).
- Player and AI agents obey identical rest-site legality. The CLI surfaces only rest-site state the controlled agent lawfully perceives.

## Non-Goals

- No environmental exposure model. Cold/heat/wetness exposure is registered as a P1 follow-up in `specs/IMPLEMENTATION-ORDER.md` and explicitly deferred until S174 lands.
- No camp/fire creation actions. A "make camp" affordance is a future-spec trigger if scenarios reveal that scenario-authored placement of rest-capable places is insufficient.
- No reuse of the existing single-occupant `SelfCareOccupancy` for rest sites. Rest occupancy requires multi-occupant capacity (a shelter with 3 bedrolls hosts 3 simultaneous sleepers). A separate `RestOccupancy` component avoids breaking S173's single-occupant contract for Wash/Latrine. `SelfCareUseKind::Sleep` (introduced by S173 for trace-detail completeness) remains in the enum but does not carry occupancy.
- No predator / night-danger ecology. `specs/S61` (held) is the proper home; this spec only consumes co-located hostile presence already produced by ordinary combat/perception systems as a wake cause.
- No food spoilage / water-quality / latrine-blocking degradation consequences. Registered as a deferred wave in `specs/IMPLEMENTATION-ORDER.md`.
- No rest-site memory beyond what the existing `LearnedRoutePreferences` / `LearnedSourcePreferences` substrate already provides. Per-place rest-outcome learning is folded into the existing `S38-learned-route-source-preferences` carriers if any extension is later proven necessary; this spec does not introduce a new memory component.
- No HTN method for sleep. Both branches (KnownRestSite + RoughSleep) are flat GOAP candidate emission. The two-path split lives in the goal schema and candidate enumerator, not in method decomposition.
- No fatigue collapse / `DeathCause` consequence path. That work belongs to `S175-fatigue-collapse-and-failed-rest-traceability.md` (paired spec, this iteration); S174 supplies the failed-rest opportunity records S175 consumes, but adds no new wound or death surface.
- No backward-compatibility shim around `FeasibilityStrategy::AlwaysLikely` for the existing Sleep goal. The strategy is replaced; goldens that depend on the old planner behavior get updated in this spec, per FND-28.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Two tired agents → one occupies a single-slot shelter → other waits, walks to a worse rest site, or sleeps rough → recovery and interruption emerge from ordinary place state, not authored "sleep failed" scripts |
| FND-3 (Concrete state over abstract scores) | Rest capacity, occupancy, shelter tag, place dirtiness, and remembered disturbance are concrete carriers; safety is the composition of those carriers at candidate emission time, never a `safety_score` |
| FND-4 (Persistent identity) | `RestOccupancy.occupants` carries `EntityId` set; each occupant's sleep is paired to the place via existing `SleepEpisode.place` |
| FND-7 (Locality of information) | Remote rest-site occupancy is belief-backed (FND-14B); co-located rest-site occupancy is FND-14A; the planner cannot read remote `RestOccupancy` as authoritative |
| FND-8 (Preconditions / duration / cost / occupancy / contention) | Sleep at a known rest site requires `RestCapacity > current occupant count`; contended sites resolve through the existing S44 queue, not planner intent |
| FND-9 (Scheduling) | Occupancy writes/releases happen at well-defined ticks (action start, commit, abort, actor death, place departure) |
| FND-10 (Aftermath) | `SleepEpisode` partial recovery + structured `WakeReason::LocalDisturbance { cause }` + `ActionTraceDetail::SleepInterrupted { cause }` together expose why recovery was poor or interrupted |
| FND-11 (Positive feedback) | Repeated rest failure → rising fatigue → more critical sleep need is the candidate loop; physical dampeners enumerated in the FND-01 Section H analysis (place capacity grows static, alternate rough-sleep always available, paired S175 collapse path closes the loop) |
| FND-14 / FND-14A (Belief vs. world / same-tick local) | Rest-site capacity is public place-graph topology (`RestCapacity`); rest-site occupancy is FND-14A when co-located, belief-backed otherwise — same split S173 pinned for `SelfCareOccupancy` |
| FND-14B (Planner-visible inputs) | KnownRestSite candidates pass belief-backed source-class checks; RoughSleep candidates use only self state + same-tick local observation of current place |
| FND-19 (Agent symmetry) | Human and AI use identical rest-site legality, same capacity gates, same wake-cause taxonomy |
| FND-20 (Resource-bounded planning) | Two-branch Sleep schema is flat GOAP candidate generation; no HTN method registered |
| FND-21 (Intentions revisable) | A planned Sleep at a known rest site does not silently reserve the place; the actor must arrive and start the action to take a slot |
| FND-26 (Systems via state) | Sleep action handler reads `RestCapacity`/`RestOccupancy` from state and writes occupancy; planner reads via belief view; no system commands another |
| FND-28 (No backcompat in live authority) | `WakeReason::LocalDisturbance` is replaced with the structured variant; `FeasibilityStrategy::AlwaysLikely` is replaced on the Sleep schema; no parallel "old vs new" sleep planning path |
| FND-29 (Debuggability) | "Why did this agent sleep rough instead of in the shelter?" answerable from belief-view trace + occupancy state at decision tick + structured wake cause |
| FND-29A (Causal history) | `EventTag::SleepEpisodeEnded` payload carries structured cause; action-trace detail enriches; both append-only |
| FND-31 (Validation) | Five scenarios cover rest-site contention, rough-sleep fallback, structured-cause interruption, deterministic replay, and player-POV symmetry |

## Deliverables

### D1. `RestCapacity` and `RestOccupancy` components on `EntityKind::Place`

```rust
/// Maximum simultaneous sleepers a Place can host as a "known rest site."
/// A Place without `RestCapacity` is not a known rest site — only Rough Sleep
/// is available there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestCapacity(pub NonZeroU32);

impl Component for RestCapacity {}

/// Authoritative occupancy state. Multi-occupant: a shelter with capacity 2
/// can carry two `EntityId`s simultaneously. Empty when no agent is sleeping
/// here at this rest site.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestOccupancy {
    pub occupants: BTreeSet<EntityId>,
}

impl Component for RestOccupancy {}
```

Both components live in `crates/worldwake-core/src/rest_site.rs` (new module).

Component registration in `crates/worldwake-core/src/component_schema.rs` (precedent: `SleepQualityProfile` registration on `EntityKind::Place`):

| Entity kind filter | Component | Lifecycle |
|--------------------|-----------|-----------|
| `\|kind\| kind == EntityKind::Place` | `RestCapacity` | Optional; scenario-authored on `PlaceDef`. Absence means the place is not a known rest site. |
| `\|kind\| kind == EntityKind::Place` | `RestOccupancy` | Runtime-managed; written/updated at sleep action start; cleared on commit/abort/abandon/actor-death/place-departure. Initialized empty on first occupancy. |

`BTreeSet<EntityId>` (not `HashSet`) is required for determinism per the Critical Invariants in `CLAUDE.md`.

**Capacity exhaustion is the precondition gate, not planner intent.** Sleep action start checks `RestOccupancy.occupants.len() < RestCapacity.0.get()` for `KnownRestSite` candidates. A full site fails to start the action; the planner replans next tick.

### D2. Sleep action handler writes / releases `RestOccupancy`

The current sleep action handler `start_sleep_episode` in `crates/worldwake-systems/src/needs_actions.rs` (which returns `Result<Option<ActionState>, ActionError>` per the standard action-handler contract) is extended to:

- On action start, if the target place has `RestCapacity` and the action is enumerated as a `KnownRestSite` candidate (see D4): insert the actor into `RestOccupancy.occupants`. If `RestCapacity` is absent OR the candidate was `RoughSleep`, no `RestOccupancy` write occurs.
- On commit (`end_sleep_episode` natural wake), abort (`abort_sleep_episode` interruption), or actor death/incapacitation: remove the actor from `RestOccupancy.occupants` if present.
- On actor place-departure mid-sleep (e.g., forced movement by combat knockback): the existing `SleepEpisode` cleanup path also removes the actor from `RestOccupancy`.

When the KnownRestSite precondition fails because the rest site is full, `start_sleep_episode` returns `Err(ActionError)` through the standard action-handler precondition-failure path. The action framework's generic precondition-rejection handling (existing `agent_tick.rs::handle_plan_failure`) treats this as a planner-replan signal; the planner next tick reads current occupancy and may emit a `RoughSleep` candidate, a different `KnownRestSite` candidate, or queue via the existing S44 `PlannerOpKind::QueueForFacilityUse` operator. No new error variant is introduced — rest-site-full uses the same `ActionError` contract as every other precondition rejection.

The S44 contention substrate is reused for queueing on full rest sites. A new `PromotableContentionKind::RestSite` variant is added to `crates/worldwake-systems/src/facility_queue.rs::PromotableContentionKind` (precedent: `SelfCareWash` and `SelfCareLatrine` from S173), and `promotable_contention_kind` recognizes `(ActionDomain::Needs, "sleep")` when the target place carries `RestCapacity`. Per-place `ContentionPolicy` applies; no per-kind policy routing. The exhaustive match on `PromotableContentionKind` in `contention_target_matches_kind` (`crates/worldwake-systems/src/facility_queue.rs`) gains a new arm for `RestSite` that matches when the target is a Place carrying `RestCapacity`.

`RoughSleep` candidates are not classified for contention (rough sleep is location-flexible; the candidate's effective place is the actor's current place, and the place need not carry `RestCapacity`).

### D3. `WakeReason::LocalDisturbance` restructuring and `SleepFailureCause` enum

The existing `WakeReason` enum in `crates/worldwake-core/src/decision_event_payload.rs` is extended in place — the `LocalDisturbance` variant gains a structured cause payload (FND-28: replace, do not duplicate):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WakeReason {
    IntendedDuration,
    TargetRecovery,
    ProjectedNeedBreach {
        need: HomeostaticNeedId,
        projected_breach_tick: Tick,
    },
    ScheduledCommitment,
    LocalDisturbance { cause: SleepFailureCause },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SleepFailureCause {
    /// Hostile actor entered same place mid-sleep.
    HostileProximity,
    /// Another actor took the rest slot (only possible in degenerate races; the
    /// occupancy precondition normally prevents this).
    RestSiteContended,
    /// Place dirtiness or other place-state change invalidated the surface.
    SurfaceInvalidated,
    /// Actor was wounded/incapacitated during sleep through ordinary processes.
    ActorIncapacitated,
    /// Generic disturbance — preserved as a fallback for sources that do not
    /// yet classify their cause. New disturbance sources should map to a
    /// specific cause; `Generic` is a transitional bucket.
    Generic,
}
```

**`WakeCondition::LocalDisturbance` is left bare** in `crates/worldwake-core/src/sleep_episode.rs`. The asymmetry between `WakeReason` (outcome carrying a cause) and `WakeCondition` (trigger predicate without a cause) is intentional: `WakeCondition::LocalDisturbance` is currently a soft trigger configuration pushed onto sleep-episode condition lists by `crates/worldwake-systems/src/sleep_synthesis.rs` (4 push sites) and is matched by `sleep_wake_reason()` in `needs_actions.rs` to `None` — meaning natural-wake `LocalDisturbance` does not currently produce a `WakeReason` at all. The only path that constructs `WakeReason::LocalDisturbance` is the forced-abort path (`abort_sleep_episode` → `end_sleep_episode` with `forced_reason: Some(WakeReason::LocalDisturbance { cause })`), where the cause is known by the caller. Treating `WakeReason` and `WakeCondition` as semantically distinct avoids manufacturing a synthetic cause at the `sleep_synthesis.rs` push sites (the cause arises at abort time, not at sleep-start time). Aligns with FND-28: one structured cause surface, not two.

`abort_sleep_episode` is updated to thread a `SleepFailureCause` value through to `end_sleep_episode`. The single `WakeReason::LocalDisturbance` construction site at `crates/worldwake-systems/src/needs_actions.rs::abort_sleep_episode` is updated to accept and forward the cause. No `WakeReason::LocalDisturbance` value remains constructible without an explicit cause.

### D4. Two-path Sleep goal schema

**Prerequisite: `FeasibilityStrategy::CandidateBacked` variant.** The current `FeasibilityStrategy` enum in `crates/worldwake-ai/src/goal_schema.rs` (variants: `OwnedCommodityCheck`, `EvidencePlaceLocal`, `AlwaysLikely`, `CommodityPresenceCheck`, `ColocationOrDead`, `NoOpinion`, `SellCheck`, `CargoDestinationCheck`, `CorpseBurialCheck`, `PlaceMatch`) does not include a "feasibility = any lawful candidate exists" strategy. This spec adds `CandidateBacked` as a new variant with that semantic: the goal is feasible iff the dispatch loop produces at least one candidate from any registered emitter (no separate pre-flight feasibility check). The variant is consumed by the Sleep two-path schema below; no other goal currently needs it, but it is generally reusable for any goal whose feasibility is "anything to do" rather than belief-checked. Update points: enum variant addition in `goal_schema.rs`, `Default` or match-arm sites (verify per `New Enum Variant on Cross-Crate Enum` audit — `FeasibilityStrategy` is consumed by the planner dispatch path; enumerate exhaustive match sites within `worldwake-ai`).

With the variant added, `DECL_SLEEP` (the `GoalSchema` static in `crates/worldwake-ai/src/goal_schema.rs`) is rewritten. The current `FeasibilityStrategy::AlwaysLikely` is removed. The Sleep goal declares two op-relevance branches and a two-stage candidate enumerator:

```rust
const SLEEP_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Sleep,                 // existing
    PlannerOpKind::QueueForFacilityUse,   // existing; new for rest-site queueing
];

pub static DECL_SLEEP: GoalSchema = GoalSchema {
    // ... existing fields ...
    feasibility_strategy: FeasibilityStrategy::CandidateBacked,
    relevant_ops: SLEEP_OPS,
    // ...
};
```

The Sleep candidate emitter is restructured. The current single-pass emitter `emit_sleep_goal` in `crates/worldwake-ai/src/candidate_generation.rs` is replaced by a two-pass function `sleep_rest_opportunities` that enumerates:

1. **`KnownRestSite` pass (belief-backed)**: For each rest-site Place in the actor's belief view (places carrying `RestCapacity`, observed directly by co-location or remembered via belief), emit a Sleep candidate with `target_place = <known rest site>` only if the belief view's `rest_site_occupant_count(<place>)` is less than capacity (FND-14A for co-located, belief-backed for remote per FND-14B).
2. **`RoughSleep` pass (current-place fallback)**: Emit a single Sleep candidate with `target_place = <actor's current effective place>`. This candidate carries a flag indicating rough sleep. No `RestCapacity` is required; no `RestOccupancy` is consulted (rough sleep has no per-place exclusivity).

If pass 1 produces zero candidates (no known rest sites believed available) and pass 2 fires, the actor will rough-sleep at their current place. Per FND-21, the planner does not silently reserve a rest slot — the actor must arrive at the rest site and pass D2's start-time precondition.

`FeasibilityStrategy::CandidateBacked` (added by this spec) determines feasibility by whether any lawful candidate is produced. Sleep is no longer `AlwaysLikely`; it is "lawful when at least one of {KnownRestSite, RoughSleep} produces a candidate."

`RoughSleep` recovery is gated by a per-agent profile field `rough_sleep_recovery_floor: Permille` (added to `MetabolismProfile` in `crates/worldwake-core/src/needs.rs`). The sleep action handler applies this floor as a hard ceiling on rough-sleep `SleepRecoveryModifier` regardless of the place's `SleepQualityProfile`. Default value `Permille::new(300)` (≈ 0.3x), scenario-overridable.

### D5. Belief-view accessor `rest_site_occupant_count` and `rest_site_capacity`

The Sleep candidate emitter reads rest-site occupancy through the existing belief-view pattern (precedent: `FacilityBeliefView::self_care_occupant` from S173 `S173SELCARINT-006`).

| Input | Source class | Stale/unknown behavior |
|-------|--------------|------------------------|
| Own rest-site capacity (places the actor is co-located with) | Same-tick local physical observation (FND-14A) | Read directly from `RestCapacity` on the place |
| Remote rest-site capacity | Public topology (FND-14B "public structural substrate") | `RestCapacity` is scenario-authored topology and does not change at runtime; agents may know remote capacity through ordinary topology beliefs |
| Own rest-site occupancy (co-located) | FND-14A | Read directly from `RestOccupancy` on the place |
| Remote rest-site occupancy | Belief-backed (FND-14B) | If no belief about occupancy, candidate proceeds optimistically; precondition at action start rejects if occupied. This mirrors S173's wash-basin behavior: emit candidate, fail at start, replan. |

The `rest_site_occupant_count` and `rest_site_capacity` accessors return `None` for places with no `RestCapacity` registered — those are not known rest sites; the actor can only rough-sleep there.

**Belief-view surface (deliverable bullets).** The three new accessors are integrated into the existing belief-view trait pipeline (precedent: `FacilityBeliefView::self_care_occupant` from S173, defined at `crates/worldwake-sim/src/belief_view.rs`):

- Add `rest_site_capacity(place_id) -> Option<NonZeroU32>`, `rest_site_occupant_count(place_id) -> Option<u32>`, and `is_co_located_with_rest_site(place_id) -> bool` as methods on the `FacilityBeliefView` trait alongside the existing `self_care_occupant`.
- Provide backing implementations on `RuntimeBeliefView` (the aggregator trait at `crates/worldwake-sim/src/belief_view.rs` that wires authoritative-state reads under the FND-14A/14B source-class discipline). The implementations follow `self_care_occupant`'s pattern: read authoritative `RestCapacity` / `RestOccupancy` when co-located, return belief-backed/topology values when remote, return `None` when no lawful source exists.
- Update the blanket `impl<T> GoalBeliefView for T where T: ... + FacilityBeliefView + ...` (at the bottom of `belief_view.rs`) to forward the new methods. No new trait is introduced; the additions are method-level extensions of `FacilityBeliefView` flowing through the existing aggregation pattern.

### D6. `EventTag::SleepEpisodeEnded` payload extension

The existing `SleepEpisodeEndedPayload` (`crates/worldwake-core/src/decision_event_payload.rs:53`) carries the wake reason. The structured cause from D3 flows through the existing field — no new event-tag variant. This is FND-28 alignment: enrich the existing record, do not parallel it.

### D7. `ActionTraceDetail::SleepInterrupted` variant

A new variant is added to `ActionTraceDetail` in `crates/worldwake-sim/src/action_trace.rs`:

```rust
SleepInterrupted {
    place: EntityId,
    cause: SleepFailureCause,
    accumulated_recovery: Permille,
    was_rough_sleep: bool,
},
```

This variant is populated by `tick_step.rs::abort_trace_detail_for_instance` when sleep aborts mid-episode. The existing `ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind::Sleep, basin: None }` from S173 remains valid for trace-sink consumers that do not care about the structured cause; `SleepInterrupted` is the structured surface. (Implementation choice: the abort helper emits `SleepInterrupted` for sleep actions and `SelfCareInterrupted` for the other five families. They are not mutually exclusive at the consumer level but the abort helper picks the most specific variant.)

### D8. `CriticalWindowFrame::failed_rest_opportunities` extension

`SurvivalForensicExtractor` (`crates/worldwake-ai/src/survival_forensics.rs:157`) is extended to capture failed-rest opportunities. A new field on `CriticalWindowFrame` (`survival_forensics.rs:19-40`):

```rust
pub failed_rest_opportunities: Vec<FailedRestOpportunity>,
```

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedRestOpportunity {
    pub tick: Tick,
    pub place: EntityId,
    pub kind: FailedRestKind,
    pub was_rough: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailedRestKind {
    /// Sleep started but was interrupted mid-episode.
    Interrupted { cause: SleepFailureCause },
    /// Sleep candidate was emitted but precondition rejected at start
    /// (rest site became full between candidate emission and arrival).
    PreconditionRejected,
    /// Sleep candidate was emitted but the actor was preempted by a
    /// higher-priority need before reaching the rest site.
    PreemptedByHigherNeed { need: HomeostaticNeedId },
}
```

A record is appended to the current critical window's `failed_rest_opportunities` when: (a) sleep aborts mid-episode, (b) sleep action start fails the rest-site precondition, or (c) an active fatigue critical window observes the actor abandon a Sleep intention for another need.

This field is the consumer surface S175 reads to prove "fatigue collapse follows N failed-rest opportunities."

### D9. Agent profile field: `rough_sleep_recovery_floor`

Add to `MetabolismProfile` in `crates/worldwake-core/src/needs.rs`:

```rust
pub rough_sleep_recovery_floor: Permille,
```

Default: `Permille::new(300)` (≈ 0.3x). Scenario-overridable via the existing `metabolism_profile: Option<MetabolismProfile>` field on `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` (the profile is set directly on `AgentDef`; no `MetabolismProfileDef` wrapper exists). The sleep handler caps rough-sleep recovery rate (per-tick `Permille` added to `fatigue` reduction) at this floor; the place's `SleepQualityProfile.recovery_modifier` is ignored when the candidate was `RoughSleep`. Known-rest-site sleep continues to use the existing `SleepQualityProfile` modifier path unchanged.

### D10. Scenario contract: `RestCapacity` on `PlaceDef`

Add `rest_capacity: Option<u32>` to `PlaceDef` in `crates/worldwake-cli/src/scenario/types.rs`. `Some(n)` writes `RestCapacity(NonZeroU32::new(n)?)` on the place; `None` omits the component. Scenario authoring example:

```ron
PlaceDef(
    name: "shelter_north",
    sleep_quality: Some(SleepQualityProfileDef(
        shelter: Roofed,
        ground_comfort: Soft,
        recovery_modifier_permille: 1100,
    )),
    rest_capacity: Some(2),
    // ...
),
```

### D11. Player-POV CLI assertion for rest occupancy

The CLI must not display `RestOccupancy.occupants` for a place the controlled agent has no lawful observation of. Assertion follows the S163 gating pattern and is added to Scenario D below.

## Authoritative-to-AI Impact Analysis

D2 modifies action preconditions (rest-site capacity gate). D4 modifies candidate emission and goal schema. D8 extends forensics. Per CLAUDE.md's 7-point Authoritative-to-AI Impact Rule:

1. `get_affordances` — **flag**: D4's two-path emitter is the new affordance discovery surface; verify the affordance enumerator surface reads `RestCapacity`/`RestOccupancy` via D5's source-classified belief view, not authoritative world state on behalf of remote actors.
2. `generate_candidates` — **flag**: `sleep_rest_opportunities` is a new candidate emitter replacing the implicit `AlwaysLikely` Sleep enumeration. The KnownRestSite pass filters on belief-view rest-site capacity/occupancy; the RoughSleep pass emits a single current-place candidate.
3. `search_plan` — **flag**: `SLEEP_OPS` gains `QueueForFacilityUse`. Verify the planner can compose travel + sleep + queue chains for KnownRestSite candidates.
4. `BestEffort` action start — **flag**: D2's `RestSiteFull` start failure must propagate to a clean replan. The existing failure-attribution code in `agent_tick.rs::handle_plan_failure` should cover this via generic precondition rejection; spec validation confirms at ticket time.
5. `handle_plan_failure` — **flag**: replan after rest-site precondition rejection. The actor's next tick reads current `RestOccupancy` via FND-14A (co-located) or belief; may emit `RoughSleep` candidate.
6. Payload revalidation — **flag**: sleep actions use `ActionPayload::None`, so payload revalidation passes through `requested_affordance_matches`. The rest-site precondition is a precondition check, not a payload-override check; verify the precondition path is exercised for None-payload actions.
7. Golden tests — **flag**: Scenarios A–E below cover known-rest-site contention, rough-sleep fallback, structured-cause interruption, failed-rest forensic record, and player-POV symmetry. All must pass.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Without rest-site identity, capacity, and occupancy, sleep is not contested — multiple agents can "sleep" in the same shelter without conflict. Without structured wake causes, the forensic question "why did this agent sleep rough instead of in the shelter?" is unanswerable from typed evidence. Without the failed-rest-opportunity record, S175's fatigue-collapse proof has no causal chain to expose.

2. **New entities/relations/records**:
   - `RestCapacity` component on `EntityKind::Place` (scenario-authored topology).
   - `RestOccupancy` component on `EntityKind::Place` (runtime-managed multi-occupant claim).
   - `SleepFailureCause` enum (5 variants).
   - `WakeReason::LocalDisturbance { cause }` restructured variant; `WakeCondition::LocalDisturbance` remains a bare trigger predicate.
   - `ActionTraceDetail::SleepInterrupted { place, cause, accumulated_recovery, was_rough_sleep }` variant.
   - `FailedRestOpportunity` + `FailedRestKind` types in `survival_forensics.rs`.
   - `CriticalWindowFrame.failed_rest_opportunities: Vec<FailedRestOpportunity>` field.
   - `PromotableContentionKind::RestSite` variant in `worldwake-systems::facility_queue`.
   - `MetabolismProfile.rough_sleep_recovery_floor: Permille` field.
   - No new `EventTag` variant. `EventTag::SleepEpisodeEnded` carries the structured cause via its existing payload (FND-28).

3. **Actions that mutate them**:
   - Sleep action start writes `RestOccupancy.occupants` (KnownRestSite path only); commit and abort remove the actor.
   - Actor death, incapacitation, or place-departure mid-sleep triggers `RestOccupancy` cleanup via the existing `SleepEpisode` end path.
   - `tick_step.rs::abort_trace_detail_for_instance` populates `ActionTraceDetail::SleepInterrupted`.
   - `survival_forensics.rs` writes `FailedRestOpportunity` entries during active critical windows.

4. **Information production and travel**:
   - Co-located agents observe `RestCapacity` and `RestOccupancy` via FND-14A.
   - Remote rest-site occupancy is belief-backed (FND-14B); agents may carry stale beliefs and emit candidates that fail at action start.
   - `RestCapacity` is public topology; agents may know remote capacity through ordinary topology beliefs.
   - `EventTag::SleepEpisodeEnded` records travel through the authoritative event log; `ActionTraceDetail::SleepInterrupted` enriches the action-trace sink for forensics and goldens.

5. **Conserved quantities**: No new conserved resource. `RestOccupancy.occupants` is a non-conserved presence claim.

6. **Scarce capacities and contention**: `RestCapacity` defines per-place sleep-slot scarcity. Contention via S44 `ContentionQueue` classified by `PromotableContentionKind::RestSite`. Rough sleep is not scarce (any non-transit place supports it). Per-place `ContentionPolicy` applies uniformly.

7. **Partial failures and aftermath**: Five lawful Sleep abort/failure shapes:
   - Sleep candidate emitted but precondition rejected at start (rest site full on arrival): no episode created; no `RestOccupancy` write; `FailedRestOpportunity { kind: PreconditionRejected }` recorded.
   - Sleep started at KnownRestSite, interrupted by structured cause: `SleepEpisode` ends with partial recovery; `RestOccupancy` releases; `WakeReason::LocalDisturbance { cause }` + `ActionTraceDetail::SleepInterrupted` fire.
   - Sleep started as RoughSleep, interrupted: same as above but `was_rough_sleep: true`, no `RestOccupancy` cleanup needed.
   - Sleep completed normally at low recovery rate (rough sleep): `SleepEpisode` ends with capped recovery via `rough_sleep_recovery_floor`.
   - Actor preempted by higher-priority need before reaching rest site: `FailedRestOpportunity { kind: PreemptedByHigherNeed }` recorded; ordinary replan.

8. **Positive feedback loops**:
   - Repeated rest interruption → rising fatigue → next sleep attempt is more urgent (higher pressure) but the world state that interrupted the previous attempt may still apply. The dampener is point 9.
   - Contention queue on the only good rest site → multiple agents queued → some leave to rough-sleep → queue dampens.

9. **Physical dampeners**:
   - `rough_sleep_recovery_floor` ensures rough sleep is always partially restorative — the actor never deadlocks unable to recover any fatigue.
   - `RestCapacity` is finite and scenario-authored; runtime cannot grow it.
   - S175 (paired spec) introduces the exhaustion-collapse path: after `exhaustion_collapse_ticks` of fatigue critical exposure, the loop ends via deprivation wound and eventual death.
   - `SleepEpisode` partial recovery preserved across interruptions: each partial sleep reduces fatigue, so repeated short sleeps cumulatively help.

10. **Agent learning**: None added by this spec. The existing `LearnedRoutePreferences` + `LearnedSourcePreferences` substrate (per `archive/specs/S38-learned-route-source-preferences.md`) covers route-level preferences; rest-outcome learning could fold into that substrate in a future spec if scenarios prove it necessary. P1.3-style rest-memory blocker (per the prior triage's deferred follow-up) is NOT introduced here; agents replan from current observation each tick.

11. **How agents can be wrong**:
    - Believe a rest site is free when it is occupied (stale belief). D2's start-time precondition rejects; agent replans (RoughSleep or alternate KnownRestSite).
    - Believe a rest site exists at a remote place when it has been destroyed/changed. Topology beliefs may decay; out of scope (existing belief decay applies).
    - Believe rough sleep is safe when a hostile is en route. Standard interruption; `SleepFailureCause::HostileProximity` fires.

12. **Lifecycle states**:
    - `RestCapacity`: static (scenario-authored), no runtime transitions.
    - `RestOccupancy`: `Empty` ↔ `PartiallyOccupied` ↔ `Full`. Transitions on each insert/remove; component is initialized on first occupancy and may be cleared back to empty when last occupant releases (or retained as empty — implementation choice, both are equivalent under the precondition `len() < capacity`).
    - `SleepEpisode` lifecycle unchanged from S128.

13. **Temporal resolution**: All occupancy writes/removes at action-start, action-commit, action-abort, and incapacitation tick boundaries. Concurrent same-tick attempts on the last free rest slot are resolved by the existing S44 contention tie-break.

14. **Boundary conditions**: Not applicable — rest sites are local place-graph topology.

15. **Derived views**: None new. `RestCapacity` and `RestOccupancy` are authoritative. The belief-view accessors `rest_site_capacity` and `rest_site_occupant_count` (D5) are derived per-actor views over authoritative state. `ActionTraceDetail::SleepInterrupted` is derived trace-sink payload.

16. **Causal records**:
    - `RestOccupancy` writes/removals appear in the event log (component lifecycle).
    - `EventTag::SleepEpisodeEnded` (existing) carries the structured wake cause via D6's payload extension.
    - `ActionTraceDetail::SleepInterrupted` enriches the action-trace sink.
    - `EventTag::ContentionResolved` / `EventTag::QueueGrantPromoted` (existing S142 substrate) fire on rest-site queue grants.
    - `FailedRestOpportunity` records in the active critical window expose the causal chain leading to fatigue collapse.

17. **Target patterns**:
    - Two tired agents, one capacity-1 shelter → one occupies → other waits or rough-sleeps.
    - Capacity-2 shelter with three tired agents → two occupy → third queues, rough-sleeps, or travels.
    - Sleep interrupted by hostile co-location → `SleepFailureCause::HostileProximity` fires; partial recovery preserved; agent flees or fights.
    - Rough sleep produces capped fatigue recovery; agent who repeatedly rough-sleeps accumulates fatigue critical exposure.
    - Sleep candidate emitted at distance; on arrival, rest site is full; `FailedRestOpportunity::PreconditionRejected` recorded; agent replans.

18. **Save/load and replay**: `RestCapacity`, `RestOccupancy`, restructured `WakeReason`, new `ActionTraceDetail` variant, new `FailedRestOpportunity` records, new `MetabolismProfile` field are all standard ECS state or trace payload. All are replay-deterministic. `BTreeSet<EntityId>` enforces deterministic occupant iteration. No new authoritative event variant means no new save-format consideration beyond the components themselves.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `RestCapacity` | Stored authoritative state | Component on Place, scenario-authored |
| `RestOccupancy` | Stored authoritative state | Component on Place, runtime-managed |
| `WakeReason::LocalDisturbance { cause }` (restructured) | Stored authoritative event-payload state | Carried in `EventTag::SleepEpisodeEnded` |
| `SleepFailureCause` enum value | Stored when carried by `WakeReason`; derived when carried by trace | Authoritative only on event-log emission |
| `ActionTraceDetail::SleepInterrupted` | Derived trace-sink payload | View; not authoritative |
| `FailedRestOpportunity` records in `CriticalWindowFrame` | Derived forensic state | View over event log + action-trace state; not authoritative |
| `PromotableContentionKind::RestSite` | Stored classification (crate-private enum variant) | Authoritative within `worldwake-systems` |
| `MetabolismProfile.rough_sleep_recovery_floor` | Stored authoritative profile parameter | Per-agent profile state |
| Belief-view `rest_site_occupant_count` / `rest_site_capacity` | Derived (per-actor source-classified view) | View; not authoritative |

## Planner-formalism analysis

Sleep candidate generation remains plain GOAP across both branches. No HTN method is registered.

- **Reusable pursuit pattern**: The two-path split (KnownRestSite + RoughSleep fallback) is candidate enumeration, not HTN decomposition. Each branch produces a single Sleep candidate; the planner chooses among them by ordinary utility ranking.
- **Why flat GOAP is sufficient**: There is no multi-stage decomposition, no information-gathering stage, no role-specific strategy, no repeated planner-budget exhaustion, no utility thrash between equivalent branches, and no method-specific failure attribution beyond what `FailedRestOpportunity` records expose at the forensic layer. HTN would over-formalize a two-candidate branch.
- **Fallback policy**: N/A (no method registered).
- **Information reads**: All sleep-candidate inputs (capacity, occupancy, place quality, current effective place, fatigue level, fatigue critical exposure, scheduled commitments) are belief-backed or same-tick local observations per D5.
- **Enforced declarations only**: All new schema fields have live consumers (precondition gate, candidate emitter, trace populator, forensic recorder).
- **Proof surface**: Scenarios A–E below.

## Belief-View Accessor Source-Class Declarations

| Accessor | Source class | Stale/unknown behavior |
|----------|--------------|------------------------|
| `rest_site_capacity(place_id) -> Option<NonZeroU32>` | Public topology (FND-14B) | Returns `None` for places without `RestCapacity` |
| `rest_site_occupant_count(place_id) -> Option<u32>` | FND-14A when co-located; belief-backed when remote | Returns `None` if no belief and remote; returns count if co-located or belief carries it |
| `is_co_located_with_rest_site(place_id) -> bool` | Self + same-tick local observation | Always known |

The accessors return `None` rather than reading remote authoritative state. Sleep candidate generation must tolerate `None` returns gracefully — the KnownRestSite pass skips candidates whose capacity is unknown OR whose occupancy belief is unknown-and-remote (no optimistic emission for fully-unknown remote sites; this mirrors S172's belief-backed Wash discipline).

Social/relational facts about the rest site (who owns the shelter, who has access rights) are NOT exposed by these accessors. Those remain belief-backed per FND-14A's social-fact carve-out. Capacity and occupant count are perceivable physical facts; ownership of the shelter is not.

## Agent Profile Scenario Contract

`MetabolismProfile.rough_sleep_recovery_floor` is a new field on an existing universal profile (`MetabolismProfile` is registered on `EntityKind::Agent` per `archive/specs/S128`). The profile is universal (every agent has it) and has a `Default` impl. The new field gets a default value (`Permille::new(300)`) and is scenario-overridable via the existing `metabolism_profile: Option<MetabolismProfile>` field on `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs`).

No new component is added to `EntityKind::Agent`; `RestCapacity` and `RestOccupancy` are on `EntityKind::Place`. The agent profile change is field-only.

## Component Registration

Both components are registered in `crates/worldwake-core/src/component_schema.rs`:

```rust
register_place_component::<RestCapacity>(...)
register_place_component::<RestOccupancy>(...)
```

`RestCapacity` is scenario-authored (precedent: `SleepQualityProfile` registration on `EntityKind::Place`). `RestOccupancy` is runtime-managed (precedent: `SelfCareOccupancy` registration on `EntityKind::Place` from S173).

`PromotableContentionKind::RestSite` is added crate-private in `crates/worldwake-systems/src/facility_queue.rs::PromotableContentionKind`. No core-resident type references it.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Sleep action handler (`worldwake-systems`) | `RestCapacity`, `RestOccupancy`, `SleepQualityProfile`, `MetabolismProfile` | `RestOccupancy`, `SleepEpisode`, `HomeostaticNeeds.fatigue` |
| Sleep candidate emitter (`worldwake-ai`) | Belief-view rest-site capacity / occupant count | None (read-only emission) |
| Action-trace tick-step (`worldwake-sim`) | `SleepEpisode`, `SleepFailureCause` | `ActionTraceDetail::SleepInterrupted` |
| Survival forensics (`worldwake-ai`) | `SleepEpisode`, `EventTag::SleepEpisodeEnded` payload, action-trace events, `CriticalWindowFrame` | `FailedRestOpportunity` records |
| Contention substrate (`worldwake-systems::facility_queue`) | `RestCapacity`, `RestOccupancy` | Queue/grant state via existing S44 path |

No system commands another. All interaction is via authoritative state and the event/trace log.

## Scenario Validation

Five scenarios prove the consequence carriers. All scenarios run under deterministic ChaCha8Rng seed and assert replay equivalence.

### Scenario A — Rest-Site Contention (`survival-safe-rest.ron`)

Topology: two places. `shelter_north` has `SleepQualityProfile { shelter: Roofed, recovery_modifier: 1100 permille }` and `RestCapacity(1)`. `open_camp` has no `SleepQualityProfile` (rough-sleep only).

Agents: two tired agents (`fatigue` near critical), both co-located at the shelter at tick 0.

Assertions:
1. Both agents emit Sleep candidates targeting `shelter_north` via FND-14A direct observation.
2. One agent's action starts and writes `RestOccupancy.occupants = {agent_a}`.
3. The other agent's action start fails the rest-site precondition; planner replans next tick.
4. The losing agent either emits a `RoughSleep` candidate at `shelter_north` (which is allowed even though the place has `RestCapacity`, because rough sleep is location-flexible and reuses the current place without consuming a rest slot) OR travels to `open_camp` and rough-sleeps.
5. Recovery modifier comparison: the shelter occupant accumulates recovery at ≈ 1.1x; the rough-sleeping agent accumulates at the capped `rough_sleep_recovery_floor` (≈ 0.3x).
6. `RestOccupancy` releases on commit; `EventTag::SleepEpisodeEnded` carries `WakeReason::TargetRecovery`.
7. `FailedRestOpportunity::PreconditionRejected` is recorded in the losing agent's active critical window.

### Scenario B — Multi-Slot Rest-Site Contention (`survival-sleep-contention.ron`)

Topology: one place. `barracks` has `RestCapacity(2)` and roofed `SleepQualityProfile`.

Agents: three tired agents co-located.

Assertions:
1. Two agents occupy `barracks` (`RestOccupancy.occupants.len() == 2`).
2. Third agent fails rest-site precondition; either queues via S44 or rough-sleeps at the same place (rough-sleep falls back to current place, ignoring `RestCapacity`).
3. Queue grant promotion fires when one occupant releases; the third agent transitions to KnownRestSite occupancy.
4. No stuck idle window under elevated fatigue (no agent fails to make progress).
5. Deterministic replay.

### Scenario C — Structured-Cause Interruption (`survival-rest-interrupted-by-danger.ron`)

Topology: `shelter` with `RestCapacity(1)`. Adjacent `outpost` hosts a hostile agent.

Agents: one tired agent at `shelter`; hostile agent travels toward `shelter` mid-sleep.

Assertions:
1. Tired agent starts Sleep at `shelter`; `RestOccupancy.occupants = {tired_agent}`.
2. Hostile arrives; sleep aborts mid-episode.
3. `WakeReason::LocalDisturbance { cause: SleepFailureCause::HostileProximity }` fires in `EventTag::SleepEpisodeEnded`.
4. `ActionTraceDetail::SleepInterrupted { cause: HostileProximity, was_rough_sleep: false, accumulated_recovery: <partial> }` populates.
5. `RestOccupancy` releases.
6. Partial recovery preserved in `HomeostaticNeeds.fatigue`.
7. Agent's next tick emits a different goal (flee/fight) by ordinary replan.

### Scenario D — Player-POV Symmetry (`survival-rest-cli.ron`)

Topology: two places; controlled agent at place A, `RestOccupancy` exists at place B with a known sleeper.

Assertions (CLI surface, per S163 gating pattern):
1. The controlled agent's player-facing display shows `RestOccupancy` at place A (co-located) accurately.
2. The controlled agent's player-facing display does NOT surface `RestOccupancy.occupants` at remote place B unless the agent has a lawful belief about it (rumor, prior co-located observation, etc.).
3. Same belief gating applies to `RestCapacity` lookups for display: capacity is public topology, occupancy is not.

### Scenario E — Repeated Failed Rest Feeds S175 (`survival-failed-rest-cascade.ron`)

Topology: `shelter` with `RestCapacity(1)` perpetually occupied by a non-cooperating agent (e.g., a sleeping invalid actor). Adjacent `open_field` supports only rough sleep.

Agents: one tired agent that must repeatedly attempt and fail rest at `shelter`, then fall back to rough sleep at `open_field`.

Assertions:
1. Over N cycles, agent accumulates ≥ N `FailedRestOpportunity` records in the active critical window (PreconditionRejected at shelter, then Interrupted or capped at open_field).
2. `HomeostaticNeeds.fatigue` enters critical exposure; `DeprivationExposure.fatigue_critical_ticks` accumulates.
3. The scenario is the feed for S175's exhaustion-collapse golden — S174 proves the carrier exists; S175 proves the collapse.
4. No hidden rescue refills fatigue; recovery comes only from concrete rough-sleep recovery (capped at floor).

## Open Questions

1. Should `RoughSleep` candidates be emittable at places that DO carry `RestCapacity` (i.e., rough-sleeping on the shelter floor even when the bed slot is taken)? D4 currently allows this; it makes the fallback always available. Alternative: forbid rough sleep at a place with `RestCapacity`, forcing the agent to travel. The current design preserves the report's "always-legal rough sleep" stance; if scenario play surfaces a reason to forbid same-place rough sleep, restrict it then.
2. Should `FailedRestOpportunity::PreemptedByHigherNeed` be recorded eagerly (every time the actor abandons a sleep intention) or only during active critical windows? Current design ties it to the active critical window (matching the existing `SurvivalForensicExtractor` window-scoped semantics). If S175 needs records outside critical windows, the extractor's scope expands; that decision is taken at S175 ticket time.
3. Should the `Generic` `SleepFailureCause` variant be removed before merge once all current call sites map to specific causes? If yes, `WakeReason::LocalDisturbance` becomes a proper exhaustive structured cause. Current spec keeps `Generic` as a transitional bucket; the spec marks this as a hardening question for the implementation tickets.
