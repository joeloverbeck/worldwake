# S173: Self-Care Interruption Contracts and Facility Occupancy

## Summary

Today only `sleep` has a durable interruption contract: `SleepEpisode` carries accumulated recovery, `abort_sleep_episode` ends the episode with `WakeReason::LocalDisturbance`, and the episode preserves partial progress across replans. `eat`, `drink`, `toilet`, `relieve_wilderness`, and `wash` all register `abort_noop` and have no occupancy state — interrupting any of them leaves no state, no trace beyond the engine-level `ActionAborted` event, and (for Wash and Toilet) no facility release because there is no facility reservation to release. The contention substrate `S44`/`S142` exists, and `WashBasinState` (`S129`) is per-facility, but `promotable_contention_kind` recognizes only Harvest/Craft/Corpse/Care exclusivity — needs actions are absent. As a result, two dirty agents at the same basin cannot lawfully contend for it; nothing in the world stops them from "using" it simultaneously because neither one ever reserved it. This spec defines an explicit interruption contract per self-care action family, extends `promotable_contention_kind` to classify Wash and Toilet as exclusive use, adds the minimum `SelfCareOccupancy` carrier required to release on abort, and proves the loop end-to-end: contested basin → one occupant → other waits or replans → interrupted occupant releases on abort → repeated interruption can lawfully escalate to deprivation collapse.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (`SelfCareOccupancy` component, `SelfCareUseKind` enum, action-trace payload extensions)
- `worldwake-sim` (interrupt-abort handler registration, decision-trace surface)
- `worldwake-systems` (per-family abort handlers replacing `abort_noop`, occupancy mutation in `wash` and `toilet`, `promotable_contention_kind` classification)
- `worldwake-ai` (revalidation respects occupancy; failure attribution for contended/disconfirmed basins)
- `worldwake-cli` (scenario contract authoring for interruption and collapse scenarios)

## Dependencies

- `archive/specs/S172-wash-discovery-budget-closure.md` — landed first; this spec assumes Wash budget accounting is correct before adding occupancy contention
- `archive/specs/S44-generalized-contention-substrate.md` — provides `ContentionQueue`/`ContentionPolicy`
- `archive/specs/S142-contention-event-inspectability.md` — provides facility-queue promotion and contention-resolved events
- `archive/specs/S128-sleep-episode-place-quality.md` — provides the precedent: `SleepEpisode` durable abort contract that this spec mirrors for Wash and Toilet
- `archive/specs/S129-place-dirtiness-facility-wear.md` — provides `WashBasinState`
- `archive/specs/S81-golden-gaps-simulation-remediation.md` — provides the deprivation-death proof that the repeated-interruption collapse scenario extends

## Design Goals

- Every self-care action declares its interruption contract: start state, tick effects, commit effects, abort cleanup, recovery-visible facts, trace surface.
- Mechanically exclusive facilities (`WashBasin`, latrine-tagged place) are reserved on action start and released on commit, abort, or actor incapacitation.
- Eat, Drink, and Wilderness-Relief remain atomic (no partial state) — but their abort handlers explicitly emit the `SelfCareInterrupted` trace event so "interrupted before commit" is distinguishable from "never attempted."
- Sleep retains its existing `SleepEpisode` contract unchanged; this spec layers a uniform trace surface above it so all five families share the same inspection shape.
- Repeated interruption can lawfully drive an agent to deprivation collapse — proven end-to-end by a scenario, not just by the existing simulation-gaps hunger-starvation proof.
- No new abstract score, no hidden rescue, no scenario-specific target injection, no planner intent-as-lock.

## Non-Goals

- No partial-progress state for Eat, Drink, Toilet, or Wilderness-Relief. They remain atomic; the spec explicitly forbids inventing partial bodily-progress math without a durable state carrier (FND-3).
- No `WashSessionProgress` duration-based partial-Wash carrier. Commit-time partial relief when clean water is insufficient (already present) is preserved.
- No social etiquette, privacy, bathroom politics, disease ecology, odor, or social shame system.
- No shelter or sleep-surface scarcity model. Sleep contention remains place-level capacity (or unimplemented) until a future spec proves sleep-surface scarcity matters.
- No queue-jumping policy, patience-threshold negotiation, or social-rank arbitration. First-come first-served via the existing `S44` contention substrate.
- No recovery-memory blocker (avoid recently-failed basin). Agents replan from observation each tick (P1.3 in source report, deferred).
- No backward-compatibility shim around `abort_noop`. Replaced where applicable; removed from the call sites where the new abort handler subsumes it.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Two dirty agents → one occupies basin → other waits or replans → repeated interruption emerges through ordinary world processes, not scripted rescue |
| FND-3 (Concrete state) | `SelfCareOccupancy` is authoritative world state with stable identity; not a score |
| FND-4 (Persistent identity) | Occupancy carries occupant `EntityId`, started-tick, and use kind; release/abandon are explicit transitions |
| FND-8 (Action preconditions, occupancy) | Directly satisfies: every self-care action declares preconditions, duration, occupancy, interruption, contention |
| FND-9 (Scheduling) | Occupy/release/abandon all occur at well-defined tick boundaries (action start, commit, abort, actor death) |
| FND-10 (Aftermath) | Interrupted self-care leaves traceable state (`SelfCareInterrupted` event) and released occupancy, never silent reset |
| FND-11 (Positive feedback) | Repeated interruption → rising deprivation → deprivation wounds is itself the dampener (wounds reduce capacity; eventually death stops the loop) |
| FND-19 (Agent symmetry) | Human and AI agents share identical interruption, occupancy, and contention semantics |
| FND-21 (Intentions revisable) | Losing actor does not silently reserve the basin; explicit occupancy or queue grant is required |
| FND-26 (Systems via state) | Action handlers read/write `SelfCareOccupancy`; planner reads it via belief or co-located observation; no system commands another |
| FND-28 (No backcompat) | `abort_noop` is replaced where the new contract applies; no parallel shim retained |
| FND-29 (Debuggability) | "Why didn't this agent wash?" is answerable from `SelfCareOccupancy` history + `SelfCareInterrupted` events + decision trace |
| FND-29A (Causal history) | Occupy, release, abandon, and interrupted events are append-only and survive replay |
| FND-31 (Validation) | Five scenarios cover atomic-abort, durable-occupancy-release, contested basin, repeated-interruption collapse, and player-POV symmetry |

## Deliverables

### 1. `SelfCareOccupancy` component and `SelfCareUseKind` enum

```rust
/// Authoritative world state. Attached to the facility entity (WashBasin,
/// latrine-tagged Place) while a self-care action is mid-flight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfCareOccupancy {
    pub occupant: EntityId,
    pub use_kind: SelfCareUseKind,
    pub started_tick: Tick,
    pub goal_seed: GoalSeed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfCareUseKind {
    Wash,
    LatrineRelief,
    // Sleep surfaces remain place-level for now; if scarce sleep surfaces are
    // introduced later they extend this enum.
}
```

Component registration:

| Entity kind | Component | Lifecycle |
|-------------|-----------|-----------|
| `WashBasin` | `SelfCareOccupancy` | Present only while basin is occupied; removed on commit/abort/abandon |
| `Place` (latrine-tagged) | `SelfCareOccupancy` | Same lifecycle |

Wilderness-relief facilities and `WashBasin` are the only `SelfCareUseKind` values for this spec. The enum is open for future spec extension (sleep surface, bath, sauna) but those values are not added now.

### 2. Per-action-family interruption contracts

The contract table below is the authoritative source for action-handler implementation. Every self-care action handler must match this table; deviations require a spec amendment.

| Action | Start state written | Tick effects | Commit effects | Abort cleanup | Recovery-visible facts | Trace surface |
|--------|---------------------|--------------|----------------|---------------|------------------------|---------------|
| `eat` | None | None | Consume 1 unit, reduce hunger, apply bladder fill | None (no state was written) | Item still controlled? Yes → may retry. Item gone → replan toward acquisition. | `SelfCareInterrupted { kind: Eat, basin: None }` on abort |
| `drink` | None | None | Consume 1 unit, reduce thirst, apply bladder fill | None | Item still controlled? Yes → may retry. Item gone → replan. | `SelfCareInterrupted { kind: Drink, basin: None }` on abort |
| `sleep` | `SleepEpisode` (existing) | Tick accumulates `Permille` recovery | Commit ends episode with accumulated recovery | `end_sleep_episode(..., WakeReason::LocalDisturbance, ...)` (existing) | `SleepEpisode` removed; partial recovery preserved in `HomeostaticNeeds::fatigue` | Existing `SleepEpisodeEnded` event |
| `toilet` | `SelfCareOccupancy` on the latrine-tagged Place | None | Clear bladder, create Waste, update latrine fullness, increase place dirtiness, **remove occupancy** | **Remove occupancy** | Latrine still available? Yes → may retry. Latrine occupied → wait or replan to alternate. | `SelfCareInterrupted { kind: LatrineRelief, basin: <latrine place> }` |
| `relieve_wilderness` | None | None | Clear bladder, create Waste/evidence, increase actor dirtiness, increase place dirtiness | None | Same place still valid? Replan freely. | `SelfCareInterrupted { kind: WildernessRelief, basin: None }` |
| `wash` | `SelfCareOccupancy` on the `WashBasin` | None | Reduce dirtiness, consume clean water, dirty basin (existing); partial relief when clean water < min (existing); **remove occupancy** | **Remove occupancy** | Basin available + clean water? Retry. Basin occupied by another? Wait or replan. Basin dry → replan to alternate or acquisition. | `SelfCareInterrupted { kind: Wash, basin: <basin entity> }` |

`abort_noop` is removed from the call sites for `toilet` and `wash`; replaced with `abort_release_self_care_occupancy`. For `eat`, `drink`, and `relieve_wilderness`, `abort_noop` is replaced with `abort_emit_self_care_interrupted` — same no-op state effect, but the trace event fires so "interrupted before commit" is distinguishable in decision trace.

### 3. `promotable_contention_kind` extension

`crates/worldwake-systems/src/facility_queue.rs::promotable_contention_kind` is extended to classify:

- `ActionPayload::Wash` → `ContentionKind::SelfCareWash`
- `ActionPayload::Toilet` → `ContentionKind::SelfCareLatrine`

`ActionPayload::Relieve_wilderness` is NOT classified — wilderness relief is location-flexible and does not require occupancy. If a future spec introduces specific scarce wilderness-relief affordances, that classification is added then.

`ActionPayload::Sleep` is NOT classified at the facility-queue layer — sleep already has `SleepEpisode` as its durable carrier, and sleep-surface scarcity is a separate future spec.

### 4. Reservation requirements on `wash` and `toilet` action handlers

The `reservation_requirements: Vec::new()` on the `wash` and `toilet` action handlers in `crates/worldwake-systems/src/needs_actions.rs` is replaced with a single-entry reservation requirement: the target facility must be reservable (no current `SelfCareOccupancy`) for the action to start. On start, `SelfCareOccupancy` is written; on commit or abort, it is removed.

If the contention substrate (`S44` `ContentionQueue`) is used, agents that lose the race join the queue with the existing grant/expiry semantics. The spec reuses S44; it does not introduce a parallel queue.

### 5. Belief-source classification for occupancy

The Wash and Toilet candidate path reads facility occupancy state subject to FND-14B:

| Input | Source class | Stale/unknown behavior |
|-------|--------------|------------------------|
| Own basin occupancy (actor is the occupant) | Self | Always known |
| Co-located basin occupancy | Same-tick local physical observation (FND-14A) | Read from world state when actor is at the basin's place |
| Remote basin occupancy | Belief-backed | If no belief, no candidate (no plan composed assuming the basin is free); revalidation at action start fires `WashContended` if basin is occupied on arrival |

The CLI POV boundary already enforces this gating for remote facility state (S158/S162); this spec confirms no new accessor is required.

### 6. Repeated-interruption deprivation-collapse trace

The spec proves end-to-end that repeated self-care interruption can lawfully drive an agent to deprivation collapse, distinct from sustained hunger starvation (already proven in S81 simulation-gaps golden). The proof shape:

1. Agent has rising dirtiness (or bladder, or fatigue).
2. Agent attempts Wash (or Toilet, or Sleep) and is interrupted before commit by an ordinary world event (hostile presence, urgent self-care of higher priority, local disturbance) — never by a scenario script.
3. Abort cleanup releases occupancy, emits `SelfCareInterrupted`, and the agent replans.
4. The replan attempts the same or alternate facility; interrupted again.
5. After enough cycles, `DeprivationExposure` for the unmet need crosses its critical threshold; deprivation wounds accumulate; wound load eventually exceeds capacity; `DeathCause::NeedDeprivation` fires with `EventTag::Death`.
6. The event log + decision trace expose every step: each interruption, each release, each replan, each failed retry, the accumulating exposure, and the eventual death.

This is implemented by Scenario E below; no new mechanism is required, only a scenario that composes existing carriers.

### 7. Player POV CLI assertions for occupancy

The CLI must not display `SelfCareOccupancy` for a basin the controlled agent has no lawful observation of. The assertion is added to the same Scenario D location as in S172 (extended scope).

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Without per-action interruption contracts and basin occupancy, two dirty agents can "use" the same basin simultaneously without world conflict, an interrupted Wash leaves no trace, and "why didn't this agent wash?" cannot be answered from world state. The collision-proof loop (basin → contention → wait/replan → interrupted → release → retry → eventual relief or lawful failure) is impossible to demonstrate.

2. **New entities/relations/records**:
   - `SelfCareOccupancy` component on `WashBasin` and latrine-tagged `Place`.
   - `SelfCareUseKind` enum (`Wash`, `LatrineRelief`).
   - `SelfCareInterrupted` event payload variant (basin id optional, use kind required).
   - `ContentionKind::SelfCareWash` and `ContentionKind::SelfCareLatrine` (extension of existing `ContentionKind` enum).

3. **Actions that mutate them**:
   - `wash` action start writes `SelfCareOccupancy`; commit and abort remove it.
   - `toilet` action start writes `SelfCareOccupancy`; commit and abort remove it.
   - Actor death or place departure mid-action triggers abandon cleanup (S44 substrate already handles this via grant expiry; spec reuses).
   - Action abort handlers emit `SelfCareInterrupted`.

4. **Information production and travel**:
   - Co-located agents observe basin occupancy via FND-14A same-tick local observation.
   - Remote occupancy must be belief-backed (visits, reports, witness testimony).
   - `SelfCareInterrupted` events travel through the action-trace surface and through the agent's own decision-trace for replanning.

5. **Conserved quantities**: No new conserved resource. `WashBasinState::clean_water` continues to be conserved through Wash commit. `SelfCareOccupancy` is a non-conserved presence claim (no quantity).

6. **Scarce capacities and contention**: `SelfCareOccupancy` is the carrier of exclusive use. One occupant per `WashBasin` or latrine `Place` at a time. Contention via `S44` `ContentionQueue` with grant/expiry. Wilderness-relief is not scarce.

7. **Partial failures and aftermath**: Five lawful abort/failure shapes:
   - Atomic abort with no state change (Eat/Drink/Wilderness-Relief): emits `SelfCareInterrupted`; no further state.
   - Atomic abort with occupancy release (Toilet/Wash): emits `SelfCareInterrupted`; removes `SelfCareOccupancy`.
   - Durable abort with partial-progress preserved (Sleep): existing `SleepEpisode` aftermath.
   - Action start blocked by contention: queue join via S44; no occupancy written.
   - Repeated interruption → deprivation collapse: deprivation wounds accumulate; eventual death (S81 substrate).

8. **Positive feedback loops**:
   - Interruption → replan → interrupted again is the candidate loop. The dampener is point 9.
   - Contention → wait → other agents arrive → longer queue is bounded by FCFS grant expiry and by agents replanning to alternate facilities or absorbing the deprivation.

9. **Physical dampeners**:
   - Deprivation wounds (S17/S81): accumulating unmet need produces concrete wound state that reduces capacity and eventually kills the agent. The interruption→retry loop is bounded by the agent's mortality.
   - `WashBasinState::clean_water` depletion: contested basins also run dry, ending contention by removing the affordance.
   - Travel cost: alternate basin is non-free; agents may absorb dirtiness rather than walk far.
   - Sleep recovery preserved across interruptions: each partial sleep does reduce fatigue, so repeated short sleeps cumulatively help.

10. **Agent learning**: None added by this spec. An agent that keeps trying the same blocked basin will keep getting `WashContended` failures; their next-tick candidate evaluation reads current state and may choose differently based on the updated belief. P1.3 recovery memory remains deferred.

11. **How agents can be wrong**:
    - Believe basin is free when it is occupied (stale belief). Revalidation at action start fires `WashContended`; agent replans.
    - Believe basin has clean water when it is dry. Existing `wash_preconditions` rejection; trace fires `WashBeliefDisconfirmed` (defined in S172).
    - Believe wilderness relief is safe when a predator is en route. Standard interruption; abort cleanup emits `SelfCareInterrupted { kind: WildernessRelief }`.

12. **Lifecycle states**:
    - `SelfCareOccupancy`: `Reserved` (action start) → `Released` (commit) | `AbandonedOnAbort` (abort) | `AbandonedOnIncapacitation` (actor death or place departure).
    - All transitions are explicit world processes — no decay timer, no silent cleanup.

13. **Temporal resolution**: Occupy/release/abandon happen at the action-start, action-commit, action-abort, and incapacitation tick boundaries. Concurrent same-tick attempts on a free basin are resolved by the existing S44 contention tie-break.

14. **Boundary conditions**: Not applicable — self-care is local.

15. **Derived views**: None new. `SelfCareOccupancy` is authoritative. Planner snapshots may read it co-located (FND-14A) or via belief, but the snapshot is not authoritative.

16. **Causal records**:
    - `SelfCareOccupancy` writes/removals appear in the event log.
    - `SelfCareInterrupted` appears as an action-trace event.
    - `ContentionResolved` (existing S142 substrate) fires on grant/expiry.
    - Repeated-interruption deprivation collapse is traceable end-to-end via existing event log.

17. **Target patterns**:
   - Two dirty agents, one basin → one occupies → other waits → first commits → second occupies.
   - Wash interrupted by hostile presence → occupancy released → agent replans to alternate.
   - Repeated Sleep interruption preserves accumulated recovery; eventually agent rests fully.
   - Repeated Toilet interruption → bladder accident → place dirtiness rises → wilderness relief substitution.
   - Repeated Wash interruption + rising dirtiness → deprivation wound (existing severity ladder) → eventual collapse.

18. **Save/load and replay**: `SelfCareOccupancy` is standard ECS state; `SelfCareInterrupted` is standard event-log payload. Both are replay-deterministic.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `SelfCareOccupancy` | Stored authoritative state | Component on facility entity |
| `SelfCareUseKind` | Stored value (inside `SelfCareOccupancy`) | Authoritative |
| `SelfCareInterrupted` | Event-log payload | Authoritative history |
| `ContentionKind::SelfCareWash` / `SelfCareLatrine` | Stored classification (enum variant) | Authoritative |
| Planner-visible basin-free hint | Derived (read of `SelfCareOccupancy` presence via belief or co-location) | View; not authoritative |

## Planner-formalism analysis

Wash and Toilet candidate emission remains plain GOAP. No HTN method is registered. Contention-aware planning is achieved through:

1. Candidate emission filters out basins whose occupancy is known (via belief or co-location) to be held by another actor.
2. If revalidation at action start finds the basin held, the action fails to start and the planner replans next tick from current state.
3. Queue-join via S44 `QueueForFacilityUse` op is already supported (`PlannerOpKind::QueueForFacilityUse` exists). The spec confirms it is reachable for Wash and Toilet by extending the op classification in `WASH_OPS` and `RELIEVE_OPS` only if implementation reveals it is required for budget-bounded plans to compose. If the existing GOAP fallthrough suffices (agent simply replans next tick), no op change is needed and this becomes an audit item.

No method-required goal is introduced. No new HTN schema contract.

## Systemic-validation analysis (FND-31)

| Check | Negative case | Mechanism |
|-------|---------------|-----------|
| No simultaneous use | Two `wash` actions commit at the same basin at the same tick | Reservation requirement; second action start fails at the `wash_preconditions` extension; assertion in Scenario B |
| No silent rescue | Interrupted Wash silently re-runs from start with state restored | Abort cleanup explicitly removes occupancy; replan must go through normal candidate emission; assertion in Scenario C |
| No planner-intent lock | Agent A "plans to use" basin → agent B cannot use it | Spec forbids: intent is not entitlement (FND-21). Only `SelfCareOccupancy` blocks; written only at action start |
| Replay determinism | `SelfCareOccupancy` write/remove order differs across replays | Standard ECS replay invariants; assertion in long-run scenario replay-equivalence test |
| No remote-truth leak | Agent A reads remote basin's `SelfCareOccupancy` directly | Belief-source classification table (Deliverable 5); assertion in Scenario D player-POV test |
| Collapse traceability | Death from repeated interruption without traceable cause | `DeathCause::NeedDeprivation` + accumulated `SelfCareInterrupted` events in trace; Scenario E |
| No backward-compat | `abort_noop` still registered for `wash`/`toilet` after this lands | Compile-time check: the `abort_noop` call sites for those handlers are removed |

## SystemFn Integration

No new SystemFn is introduced. The spec modifies:

- Action handlers for `wash`, `toilet`, `eat`, `drink`, `relieve_wilderness` in `crates/worldwake-systems/src/needs_actions.rs`.
- `promotable_contention_kind` in `crates/worldwake-systems/src/facility_queue.rs`.
- `interrupt_abort` registration in `crates/worldwake-sim/src/interrupt_abort.rs` (extension only).
- Action trace payload in `crates/worldwake-sim/src/action_trace.rs` (new variant).

Ordering against other systems is unchanged. `SelfCareOccupancy` writes/removes happen synchronously inside action start/commit/abort, which already run at well-defined tick boundaries.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SelfCareOccupancy` | `WashBasin`, `Place` (latrine-tagged) | Role-specific | Absent — written only during action lifetime |

Agent-side: no new agent component. The interruption contract reads only `HomeostaticNeeds`, `MetabolismProfile`, and `DeprivationExposure`, all of which exist.

Per `docs/spec-drafting-rules.md` §Agent Profile Scenario Contract, no `AgentDef` change is required.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| `archive/specs/S172-wash-discovery-budget-closure.md` | Wash candidate emission must filter on known occupancy | State-mediated read |
| S44/S142 (contention substrate) | `SelfCareOccupancy` + `ContentionKind::SelfCareWash`/`SelfCareLatrine` participate in existing queue/grant flow | State-mediated |
| S128 (Sleep episode) | Sleep retains its existing contract; `SelfCareInterrupted` event is layered for uniform inspection | Trace-only, no behavior change |
| S129 (WashBasinState) | Basin water/dirtiness state continues to gate `wash_preconditions`; occupancy is a parallel gate | State-mediated |
| S81 (Deprivation death) | Repeated interruption + rising deprivation → existing death pathway | State-mediated; no new mechanism |
| S163 (CLI POV boundary) | UI surfaces `SelfCareOccupancy` only for co-located or belief-known basins | State-mediated via belief view |
| Action engine (existing) | New abort handler variants; existing abort/interrupt machinery used | State-mediated |

## Profile-Driven Parameters

The spec adds no new `MetabolismProfile` field. Existing fields used:

- `MetabolismProfile::wash_duration_ticks` — per-agent Wash duration (already exists).
- `MetabolismProfile::toilet_duration_ticks` — per-agent Toilet duration (already exists).
- `MetabolismProfile::dirtiness_urgency_threshold` — per-agent dirtiness urgency (already exists).
- `DeprivationExposure::critical_threshold` — per-agent ticks at critical exposure before wound (already exists).
- S44 `ContentionPolicy` per facility kind — scenario-configurable grant/expiry (already exists).

No hardcoded constants. No new Permille field. Scenario authors configure all interruption-relevant pressures (hostile presence frequency, deprivation thresholds, basin clean-water capacity) via existing profiles and scenario `.ron` parameters.

## Scenario Validation

### Scenario A — Per-family abort emits `SelfCareInterrupted`

One agent attempts each of the five action families. An external interruption (planner replan to higher-priority goal, or local disturbance) aborts each before commit.

Assertions:
- `SelfCareInterrupted { kind: Eat | Drink | Sleep | LatrineRelief | WildernessRelief | Wash }` fires for each.
- Eat and Drink: no item consumed; possession unchanged.
- Sleep: `SleepEpisode` ends with `WakeReason::LocalDisturbance`; `accumulated_recovery` preserved as a `HomeostaticNeeds::fatigue` reduction.
- Toilet and Wash: `SelfCareOccupancy` written at start, removed at abort.
- Wilderness-Relief: no `SelfCareOccupancy` written; no Waste created.

### Scenario B — Contested basin, one occupant, other waits or replans

Two dirty agents start at the same `WashBasin` with one clean-water unit. Agent A's Wash action starts and writes `SelfCareOccupancy`. Agent B attempts Wash same tick.

Assertions:
- Only Agent A's action commits.
- Agent B's candidate is filtered (or revalidation rejects) — assertion that no second `Wash` action commits on the same basin in the same tick.
- Agent B either joins the contention queue (S44) or replans (alternate basin if known; wait-for-deprivation if not). Both paths are lawful.
- `ContentionResolved` event fires (S142 substrate).

### Scenario C — Interrupted Wash releases basin and recovers

Agent A starts Wash; before commit, a hostile predator (or higher-priority self-care) interrupts. Agent B is queued or present.

Assertions:
- Agent A's abort emits `SelfCareInterrupted { kind: Wash, basin: <id> }`.
- `SelfCareOccupancy` is removed from basin.
- Agent B (if queued) receives a grant within the configured grant-expiry window OR Agent A's next-tick replan re-attempts the basin if it is still free.
- No leftover occupancy at end of run.

### Scenario D — Player POV symmetry for occupancy

Same as S172 Scenario D but extended: controlled agent at a place without a co-located basin and without belief about a remote basin must not see remote `SelfCareOccupancy` state in any CLI accessor output.

### Scenario E — Repeated interruption → lawful deprivation collapse

Harsher scenario, expected to be CI-only (long-running). One agent has rising dirtiness (or fatigue, or bladder). Every Wash (or Sleep, or Toilet) attempt is interrupted before commit by ordinary world events (predator passes, urgent self-care of higher priority, hostile encounter).

Assertions:
- `SelfCareInterrupted` accumulates in the event log; count >= configured threshold.
- `DeprivationExposure` for the unmet need climbs across run; crosses critical threshold.
- Deprivation wounds accumulate (S17 severity ladder).
- Eventually `DeathCause::NeedDeprivation` + `EventTag::Death` fires for the agent.
- No actions start after death.
- Replay determinism holds (state-hash stable across replays).
- Decision trace exposes the chain: target → start → interrupted → release → replan → repeat → exposure → wound → death.

## Risks and Open Questions

1. **`promotable_contention_kind` integration scope.** The current substrate distinguishes Harvest/Craft/Corpse/Care. Adding `SelfCareWash`/`SelfCareLatrine` is straightforward enum extension, but the routing of these variants through `ContentionPolicy` selection needs implementation-time audit — does S44 already support per-kind policy parameters, or does this require a small policy-table extension? Pre-implementation audit required.
2. **Sleep-surface scarcity remains place-level.** The spec explicitly defers sleep-surface identity. If a future scenario reveals two agents trying to sleep at the same scarce surface and the place-capacity model is insufficient, a follow-up spec adds `SleepSurface` and extends `SelfCareUseKind`.
3. **`WashSessionProgress` deferred.** Duration-based partial Wash is interesting (Wash for 5 of 10 ticks, get partial dirtiness reduction) but invisible without a durable carrier. Deferred unless a scenario proves it matters.
4. **Queue-op routing.** Whether Wash and Toilet candidate emission needs to include `PlannerOpKind::QueueForFacilityUse` in their op-relevance lists is an implementation-time audit. The spec's contract is that contention is resolved correctly; the routing detail is left to implementation.
5. **Scenario E run length.** Repeated-interruption collapse may need ~2000-4000 ticks to fire reliably. Whether the scenario is part of the standard golden lane or a separate "ignored CI" lane is a decision for the implementation phase; both are acceptable.
6. **Existing `abort_noop` call sites elsewhere.** The spec replaces `abort_noop` for the five self-care actions but leaves it in place for any non-needs action that legitimately has nothing to clean up. Implementation audits the call sites.

## Out of Scope (Tracked Elsewhere)

- Wash budget closure and discovery — `archive/specs/S172-wash-discovery-budget-closure.md`.
- Recovery-memory blockers (avoid retrying a recently-failed basin) — deferred (P1.3 in source report).
- Sleep-surface scarcity / `SleepSlot` — deferred unless a scenario proves it matters.
- `WashSessionProgress` duration-partial state — deferred.
- Self-care patience profiles (when to abandon a queue) — deferred (P1.2 in source report; existing S44 grant-expiry suffices for first pass).
- Disease, sanitation economy, etiquette, privacy, social shame — deferred (P2 in source report).
- Adjacent-cluster redesign (pursuit, obligation, trade, theft, justice, combat as interruption sources) — out of scope; this spec uses them only as pressure sources, not as redesign targets.
