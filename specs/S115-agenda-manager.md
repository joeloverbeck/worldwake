# S115: Agenda Manager with Goal Lifecycle

## Summary

Add an `AgendaState` that tracks goals through a three-state lifecycle (`committed`, `pending`, `suspended`) with origin, freshness, revival trigger, and kill condition per entry. Replaces the current "rank everything fresh every tick" model with an explicit agenda manager: committed goals persist across ticks under margin-based commitment (S74); pending goals wait for a revival condition (resource appears, route becomes safe, counterparty arrives); suspended goals are dormant but not abandoned. The agenda is also the architectural home for distinguishing *satisfied* from *truly-infeasible* from *infeasible-until-belief-changes* goals — a distinction the S112 feasibility probe collapses into a single opaque `RejectedBeforeSearch { reason: Discrepancy }` today, forcing downstream callers to re-derive the intent by inspecting `Discrepancy` variants. Rename `GroundedGoal` → `GoalOffer` and `RankedGoal` → `AgendaEntry` to reflect their lifecycle role. `exhausted` and `abandoned` states are deferred — Phase 7 scenarios have not yet demonstrated a need to distinguish them from `DiscrepancyMemory` entries.

## Phase and Status

Phase 9: Belief-First Continual Planning Structural. Status: Draft.

## Crates

- `worldwake-ai` — `agenda_manager.rs` module; `AgendaState`, `AgendaEntry`, `GoalOffer`; rename of `RankedGoal` / `GroundedGoal` across ranking and candidate-generation surfaces
- `worldwake-core` — `AgendaEntryKey`, lifecycle enum; agenda component on `EntityKind::Agent`
- `worldwake-sim` — belief-view accessors for agenda state
- `worldwake-cli` — scenario contract for agenda-manager profile fields (most are runtime-generated)

## Dependencies

- S110 (Decision History Events) — lifecycle transitions emit `GoalCommitted` / `GoalSuspended` / `GoalAbandoned`.
- S112 (Portfolio Planning) — portfolio still assembles slots per tick, but the commitment slot now reads `AgendaState.committed` directly. The feasibility probe's `RejectedBeforeSearch { reason }` is the primary input to the lifecycle classifier (D4A) that decides whether a rejected goal becomes `Suspended` with no revival trigger (satisfied), `Pending` with a concrete revival trigger (infeasible-until-belief-changes), or killed outright (truly infeasible with no revival path). Hard.
- S114 (Plan Step Guards) — pending goals store their revival conditions as `Invalidator`-kind predicates, reusing the guard infrastructure.
- S123 (Preference-Ordering Authority) — `tick_agenda` ranks its merged candidate pool via `ranking::sort_in_place` and consumes the resulting `OrderedRanked<'_>` for the commit decision. No parallel comparator. Soft (migration ordering only).

## Design Goals

- Committed goals persist across ticks deterministically. No "I committed to goal X but rank re-chose goal Y next tick even though nothing changed."
- Pending goals have explicit revival triggers: "revive when commodity K is believed available at place P" or "revive when target T is observed alive." Not periodic re-ranking.
- Suspended goals stay inspectable in observer output — a plateauing agent is visible as "5 pending goals suspended for these specific reasons."
- Renames `GroundedGoal` / `RankedGoal` cleanly. No compatibility shim (FND-28); the live authority path uses the new names end-to-end.

## Non-Goals

- `Exhausted` / `Abandoned` variants — deferred until a scenario surfaces a case where `DiscrepancyMemory` TTL is insufficient and a goal must be marked structurally dead.
- Concurrent multiple committed goals. The current model commits to one goal at a time; S115 preserves that.
- Replacing margin-based commitment (S74). S74 still decides when to break the commitment; S115 makes the decision surface lifecycle-aware.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-16 (Ignorance, Uncertainty, Contradiction First-Class) | The D4A classifier turns the probe's opaque `RejectedBeforeSearch { reason }` into three explicit lifecycle branches (`Satisfied` / `InfeasibleUntil` / `Dead`). "I can't do this right now" becomes queryable state, not a heuristic the caller re-derives. |
| FND-20 (Resource-Bounded Practical Reasoning) | An explicit agenda lets agents keep track of "what I wanted to do but can't now" without re-ranking from scratch every tick. Lower cognitive cost per tick. |
| FND-21 (Intentions Are Revisable Commitments) | Commitment is explicit state with a kill condition. Revisability is a lifecycle transition (`committed → suspended`), not a silent re-rank. A *satisfied* committed goal (D4A) is preserved as `Suspended` rather than silently dropped, which is the correctness signal the S112 cargo-delivery regression surfaced. |
| FND-22 (Agent Diversity) | `AgendaEntry.origin` records whether the goal came from a need, an obligation, a social commitment, or an opportunity. Per-agent lifecycle policies (S115's agenda profile) let one agent abandon stale pending goals quickly, another patiently. |
| FND-29A (Causal History) | Lifecycle transitions are append-only events (S110). Why a goal was suspended, for what revival condition, and when it revived are all queryable. |

## Deliverables

### D1: `AgendaState` component

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaState {
    pub committed: Option<AgendaEntry>,
    pub pending: BTreeMap<AgendaEntryKey, AgendaEntry>,
    pub suspended: BTreeMap<AgendaEntryKey, AgendaEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaEntry {
    pub key: AgendaEntryKey,
    pub offer: GoalOffer,
    pub phase: AgendaPhase,
    pub origin: AgendaOrigin,
    pub introduced_tick: Tick,
    pub last_reconsidered_tick: Tick,
    pub revival_trigger: Option<RevivalTrigger>,
    pub kill_condition: KillCondition,
}

pub enum AgendaPhase { Committed, Pending, Suspended }

pub enum AgendaOrigin {
    NeedDrive,
    Obligation { artifact: EntityId },
    SocialCommitment { expectation: ExpectationId },
    Opportunity { evidence: EntityId },
    Exploration,
    Enterprise,
}

pub enum RevivalTrigger {
    /// Revive when commodity kind at place reaches quantity.
    CommodityAvailable { place: EntityId, kind: CommodityKind, min: Quantity },
    /// Revive when target is believed present at place.
    TargetPresent { target: EntityId, place: EntityId },
    /// Revive when a route from (from, to) is known.
    RouteLearned { from: EntityId, to: EntityId },
    /// Revive when a counterparty is observed at a place and not busy.
    CounterpartyAvailable { counterparty: EntityId, place: EntityId },
    /// Revive after a deterministic tick (re-evaluate periodically).
    TickElapsed { at_tick: Tick },
}

pub enum KillCondition {
    /// Expire when tick exceeds this bound; fold into discrepancy/abandon later.
    TickExpiry { at_tick: Tick },
    /// Die when the associated expectation/obligation is fulfilled or expires.
    ObligationResolved { expectation: ExpectationId },
    /// Die when target ceases to exist.
    TargetDead { target: EntityId },
    /// Never kill on its own; only revival trigger governs.
    External,
}
```

### D2: `GoalOffer` and `AgendaEntry` (rename targets)

- `GroundedGoal` → `GoalOffer`. Add fields: `obligation_source: Option<EntityId>`, `commitment_impact_if_ignored: Permille`, `required_information_gaps: Vec<BeliefClaimKey>`, `invalidators: Vec<Invalidator>`, `learned_expectation_refs: Vec<ExpectationId>`.
- `RankedGoal` → `AgendaEntry` (re-use S115's type). The scoring fields from `RankedGoal` (`priority_class`, `motive_score`, `provenance`, etc.) move into `AgendaEntry`'s fields alongside the lifecycle data.

Rename mechanics: atomic rename across `candidate_generation.rs`, `ranking.rs`, `agent_tick/planning.rs`, tests. No aliases — FND-28.

### D3: Agenda manager flow

`agenda_manager.rs::tick_agenda`:

```rust
pub fn tick_agenda(
    state: &mut AgendaState,
    fresh_offers: Vec<GoalOffer>,
    beliefs: &impl BeliefView,
    memory: &impl AgendaMemory,
    tick: Tick,
) -> AgendaTransitions {
    // 1. Evaluate kill conditions — drop dead committed/pending/suspended.
    let killed = drain_killed(state, beliefs, tick);

    // 2. Evaluate revival triggers on pending — promote to candidate pool.
    let revived = promote_revived(state, beliefs, tick);

    // 3. Merge fresh offers with existing pending: a fresh offer with the
    //    same key as a pending entry refreshes its `last_reconsidered_tick`
    //    but does not create a duplicate.
    let merged = merge_offers(state, fresh_offers, tick);

    // 4. Rank the candidate pool (committed + revived + merged pending +
    //    survival overrides).
    let ranking = rank_for_commit(state, &merged, &revived, beliefs, memory, tick);

    // 5. Apply margin-based commitment (S74) — keep committed unless a
    //    candidate exceeds it by the switch margin.
    let commit_transition = commit_or_keep(state, ranking, beliefs, tick);

    // 6. Demote remaining candidates to pending or suspended based on
    //    whether their revival trigger is known.
    demote_to_pending_or_suspended(state, ranking.losers, beliefs, tick);

    AgendaTransitions { killed, revived, commit_transition }
}
```

Each transition emits the corresponding S110 `EventTag`. The portfolio assembly (S112) reads the post-tick agenda state: `committed` → commitment slot; fresh offers → survival / economic / information slots.

### D4: Revival-trigger evaluation

For each pending entry, `evaluate_revival_trigger` reads the agent's belief store (via the S113 envelope) and returns whether the trigger fires this tick. Triggers are cheap belief-store lookups; the dominant cost is iterating `pending` map entries (bounded by `agenda_pending_capacity` — see D6). Determinism guaranteed by `BTreeMap` iteration.

### D4A: Feasibility-rejection → lifecycle classifier

The S112 feasibility probe (`feasibility_probe::probe`) returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy }` with five collapsed reason variants (`MissingObservation`, `RouteUnknown`, `PartialExecutionDrift`, `NoLegalBinding`, `ImproperPlanningState`). Three structurally different situations map onto those variants today:

| Situation | Probe signal | Correct lifecycle action |
|-----------|--------------|--------------------------|
| Goal is already *satisfied* (e.g. `MoveCargo` when the agent is at destination with the cargo) | `RejectedBeforeSearch { reason }` where the goal's post-conditions are already true in the agent's belief store | Move to `Suspended` with `KillCondition::External`; `revival_trigger = None`. The commitment persists until `KillCondition` fires (typically `TargetDead` / `ObligationResolved`) or the goal's post-conditions stop holding. |
| Goal is *infeasible now, may become feasible* (e.g. `AcquireCommodity` at a place with `MissingObservation` about stock) | `RejectedBeforeSearch { reason: MissingObservation \| RouteUnknown \| PartialExecutionDrift }` with a belief gap the agent can plausibly close | Move to `Pending` with a `RevivalTrigger` synthesised from the rejection reason: `MissingObservation` on a commodity → `CommodityAvailable`; `RouteUnknown` → `RouteLearned`; `PartialExecutionDrift` on a reservation → `CounterpartyAvailable` or `TickElapsed`. |
| Goal is *structurally infeasible* (e.g. `NoLegalBinding`, `ImproperPlanningState` with no believable resolution path) | `RejectedBeforeSearch { reason: NoLegalBinding \| ImproperPlanningState }` | Kill the entry (drop from agenda). `DiscrepancyMemory` records the failure so the ranker does not re-emit immediately; the goal re-enters naturally after memory TTL. |

New helper `agenda_manager::classify_rejection(probe_verdict: &FeasibilityVerdict, offer: &GoalOffer, beliefs: &impl BeliefView) -> RejectionLifecycle` produces:

```rust
pub enum RejectionLifecycle {
    Satisfied,
    InfeasibleUntil { trigger: RevivalTrigger },
    Dead,
}
```

The classifier is deterministic, pure over (verdict, offer, beliefs), and has no side effects. It is the single authoritative decoder of the probe's rejection reason into agenda lifecycle state — downstream modules do not inspect `Discrepancy` variants to infer the same distinction (closing the Gap-2 gap surfaced by the S112 incident, where the `build_candidate_plans` caller had to special-case "don't exclude the committed opportunity from `search_order` even when the probe rejected it").

Migration note: the S112-era special case in `build_candidate_plans` that kept a rejected *committed* opportunity in `search_order` disappears with S115. Post-S115, a rejected committed goal is demoted to `Suspended` (satisfied) or `Pending` (infeasible-now) by the classifier before `search_order` is built; the search order then trusts the admitted ranking without the special case.

### D5: Suspended vs pending

- **Pending**: has a concrete `RevivalTrigger` that can fire. The agent is actively waiting for the condition.
- **Suspended**: no viable revival trigger (e.g., offer was valid once but the underlying opportunity evidence is now contradicted). Suspended entries stay until `KillCondition` fires, then they are dropped.

Observer tooling distinguishes them when rendering agent decision state. This directly addresses FND-29 ("why is the agent idle?" → "5 pending goals, 2 suspended, here's what they're waiting for").

### D6: Agenda capacity

Per-agent `AgendaProfile` component with `pending_capacity: u32` (default 16) and `suspended_capacity: u32` (default 8). When exceeded, evict the oldest entry (smallest `last_reconsidered_tick`). Capacities are profile-driven, not hardcoded.

### D7: Margin-based commit integration

S74's switch-margin logic stays in place but now reads from `AgendaState.committed` instead of a rank-derived current. The margin check becomes: "does the highest-ranked candidate exceed `committed.score + switch_margin`?" Same behavior; cleaner surface.

### D8: Belief-view accessors

New accessor `agenda_state(agent) -> &AgendaState` on the appropriate belief-view sub-trait. Observer and decision-trace read-only consumers use this path.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Revival triggers read the agent's own belief store. No cross-agent information flow. A social commitment's revival (e.g., counterparty arrives) fires only when the agent perceives the arrival — delegated to the perception system, not omniscient query. The D4A classifier is pure over `(probe_verdict, offer, beliefs)` with no side effects and no cross-agent reads.
2. **Positive-feedback analysis**: Potential loop: pending → revival fires → commit → plan fails → demote to pending → revival fires again. Dampener: S109 `DiscrepancyMemory` records the failure and suppresses re-emission; S115's `kill_condition` can include "after N cycles of commit-then-fail, abandon." A second potential loop is D4A misclassifying a structurally-dead goal as `InfeasibleUntil`, producing endless pending-revive-dead-pending cycles; dampener is `revive_cooldown_ticks` (D6) plus `DiscrepancyMemory` suppression of the emit side.
3. **Concrete dampeners**: `agenda_pending_capacity` + eviction by `last_reconsidered_tick` + discrepancy-memory suppression + `revive_cooldown_ticks`. All profile-driven.
4. **Stored state vs. derived read-model**: `AgendaState` is authoritative stored state on each agent. Lifecycle transitions produce event-log entries (S110). Portfolio slots (S112) are derived read-views over the agenda. The D4A classifier result is *not* stored — it is a per-tick derived decision over inputs that are themselves stored (probe verdict, belief store).

## SystemFn Integration

**New SystemFn**: `agenda_tick_system`. Placement: early in agent-tick phase, after perception/belief-update, before candidate-generation+ranking. Produces the agenda state the portfolio reads.

## Component Registration

- `AgendaState` — register on `EntityKind::Agent`. Runtime-generated, exempt from scenario authoring (§5).
- `AgendaProfile` — register on `EntityKind::Agent`. Universal with `Default`, scenario-authorable.

## Cross-System Interactions

- **Agenda manager ↔ candidate generation**: Generation produces `GoalOffer`s each tick; manager merges them into existing pending.
- **Agenda manager ↔ ranking**: Ranking scores the candidate pool (committed + revived + fresh) for commit decision.
- **Agenda manager ↔ S112 portfolio**: Portfolio reads `AgendaState.committed` and builds slots from `AgendaState.pending` that pass feasibility probes.
- **Agenda manager ↔ S110 event log**: Every lifecycle transition emits an `EventTag` (`GoalCommitted`, `GoalSuspended`, `GoalAbandoned`).
- **Agenda manager ↔ S109 discrepancy memory**: Revival and commit checks read the memory to avoid reviving suppressed goals.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `pending_capacity` | `AgendaProfile` | `u32` | 16 | Max pending entries |
| `suspended_capacity` | `AgendaProfile` | `u32` | 8 | Max suspended entries |
| `revive_cooldown_ticks` | `AgendaProfile` | `u32` | 4 | Minimum ticks between successive revivals of the same key |

## Validation and Falsification

### Unit tests

1. Fresh offer with same key as pending entry updates `last_reconsidered_tick` without duplicating.
2. `RevivalTrigger::CommodityAvailable` fires when belief-store confirms quantity ≥ min.
3. `KillCondition::TickExpiry` drops a pending entry at or after the expiry tick.
4. Capacity overflow evicts the oldest `last_reconsidered_tick`.
5. A revived entry becomes a commit candidate; if it wins, `AgendaState.committed` updates and `EventTag::GoalCommitted` is emitted.
6. D4A `classify_rejection` on a satisfied `MoveCargo` (agent at destination with cargo) returns `RejectionLifecycle::Satisfied`; the entry is demoted to `Suspended` with no revival trigger.
7. D4A `classify_rejection` on `AcquireCommodity` at a place with `MissingObservation` about stock returns `RejectionLifecycle::InfeasibleUntil { trigger: CommodityAvailable { .. } }`; the entry is demoted to `Pending` with a matching trigger that fires when a later belief update resolves the observation gap.
8. D4A `classify_rejection` on `NoLegalBinding` returns `RejectionLifecycle::Dead`; the entry is dropped, and a `DiscrepancyMemory` entry keeps the ranker from re-emitting until TTL.

### Integration tests

9. Two-tick scenario: agent commits goal A at tick 1; tick 2 belief confirms A still viable; `AgendaState.committed == A` at end of tick 2 (no re-commit churn).
10. Deterministic replay: re-running a recorded simulation reproduces identical agenda state at every tick.
11. Cargo-delivery: the S112 `cargo_satisfaction_at_destination_while_carrying` assertion — `active_goal` remains `MoveCargo` after delivery — passes via the D4A classifier demoting the committed goal to `Suspended` (satisfied), not via the search-order special case the S112 incident fix added.
12. Portfolio rejection: the S112 `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` assertion — commit the feasible economic goal within two ticks, ignore the rejected `Sleep` / `ReportMissing` slots — passes via the D4A classifier killing the structurally-infeasible commitment rather than pinning it as the committed priority.
13. Existing goldens pass (`survival-baseline.ron`, `survival-contested.ron`, `golden_planner_pathology.rs`).

### Golden test

14. New scenario `golden_agenda_lifecycle.rs`: agent's purchase goal becomes pending when merchant departs, revives when merchant returns, commits, and completes — all transitions visible in event log and in final agenda state.
15. Extension of `golden_agenda_lifecycle.rs`: agent's cargo-delivery goal reaches destination, the D4A classifier demotes it to `Suspended { kind: Satisfied }`, and the suspended entry appears in observer Section 9 output for one post-satisfaction tick before `KillCondition::External` clears it.

## Outcome

To be filled in at completion.
