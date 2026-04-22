# S115: Agenda Manager with Goal Lifecycle

## Summary

Add an `AgendaState` that tracks goals through a three-state lifecycle (`committed`, `pending`, `suspended`) with origin, freshness, revival trigger, and kill condition per entry. Replaces the current "rank everything fresh every tick" model with an explicit agenda manager: committed goals persist across ticks under margin-based commitment (S74); pending goals wait for a revival condition (resource appears, route becomes safe, counterparty arrives); suspended goals are dormant but not abandoned. The agenda is also the architectural home for distinguishing *satisfied* from *truly-infeasible* from *infeasible-until-belief-changes* goals — a distinction the S112 feasibility probe collapses into a single opaque `RejectedBeforeSearch { reason: Discrepancy }` today, forcing downstream callers to re-derive the intent by inspecting `Discrepancy` variants. Rename `GroundedGoal` → `GoalOffer` and `RankedGoal` → `AgendaEntry` to reflect their lifecycle role. `AgendaEntry` absorbs the existing `ActiveGoal` component's role (goal_key + adopted_at) so a single authority tracks the agent's committed goal (FND-28). `exhausted` and `abandoned` states are deferred — Phase 7 scenarios have not yet demonstrated a need to distinguish them from `DiscrepancyMemory` entries.

## Phase and Status

Phase 9: Belief-First Continual Planning Structural. Status: Draft.

## Crates

- `worldwake-ai` — `agenda_manager.rs` module; `AgendaState`, `AgendaEntry`, `GoalOffer`, `RevivalTrigger`, `KillCondition`, `AgendaPhase`, `AgendaOrigin`, `RejectionLifecycle`, `classify_rejection`. `AgendaState` is ai-layer per-agent runtime state (follows `AgentDecisionRuntime` precedent at `decision_runtime.rs:151` — not an ECS component), stored in the existing `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` (`agent_tick/mod.rs:70`) either embedded inside `AgentDecisionRuntime` as a new field or alongside it in a sibling map. Rename of `RankedGoal` / `GroundedGoal` across ranking and candidate-generation surfaces.
- `worldwake-core` — `AgendaProfile` component on `EntityKind::Agent` (scenario-authorable; registered in `component_schema.rs` alongside `ActiveGoal`, `IntentionFrame`, `WoundList`). `ActiveGoal` component is **removed** in this spec — its fields (`goal_key`, `adopted_at`) migrate into `AgendaEntry` (see D2). `AgendaEntryKey` is a type alias or re-export of the existing `OpportunityKey` (`{goal_key, anchor}`) so no parallel key taxonomy is introduced.
- `worldwake-sim` — no new belief-view accessor (FND-26 / crate-layering: `AgendaState` lives in ai, so sim's `GoalBeliefView` cannot expose it without a reverse dependency). Observer, decision-trace, and planning-internal consumers read `AgendaState` directly from the ai runtime map.
- `worldwake-cli` — `AgendaProfile` field on `AgentDef` + `spawn_agent()` wiring (universal, `unwrap_or_default()`). `AgendaState` is exempt from scenario authoring (matches `ActiveGoal`, `IntentionFrame`, `WoundList` per `docs/spec-drafting-rules.md` §5 tail).

## Dependencies

- S110 (Decision History Events) — lifecycle transitions emit `EventTag::GoalCommitted` / `GoalSuspended` / `GoalAbandoned` (variants already present at `crates/worldwake-core/src/event_tag.rs:37-39`; payload structs `GoalCommittedPayload`, `GoalSuspendedPayload`, `GoalAbandonedPayload` already present at `decision_event_payload.rs:80,107,114`). Archived.
- S112 (Portfolio Planning) — portfolio still assembles slots per tick, but the commitment slot now reads `AgendaState.committed` directly. The feasibility probe's `RejectedBeforeSearch { reason }` (see `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-32`) is the primary input to the lifecycle classifier (D4A) that decides whether a rejected goal becomes `Suspended` with no revival trigger (satisfied), `Pending` with a concrete revival trigger (infeasible-until-belief-changes), or killed outright (truly infeasible with no revival path). Hard. Archived.
- S114 (Plan Step Guards) — pending goals may store revival-invalidation signals as `Invalidator` predicates (`crates/worldwake-ai/src/plan_guard.rs:36`), reusing the guard infrastructure. Archived.
- S123 (Preference-Ordering Authority) — `tick_agenda` ranks its merged candidate pool via `ranking::sort_in_place` and consumes the resulting `OrderedRanked<'_>` (`crates/worldwake-ai/src/ranking.rs:271`) for the commit decision. No parallel comparator. Soft (migration ordering only). Archived.

## Design Goals

- Committed goals persist across ticks deterministically. No "I committed to goal X but rank re-chose goal Y next tick even though nothing changed."
- Pending goals have explicit revival triggers: "revive when commodity K is believed available at place P" or "revive when target T is observed alive." Not periodic re-ranking.
- Suspended goals stay inspectable in observer output — a plateauing agent is visible as "5 pending goals suspended for these specific reasons."
- Renames `GroundedGoal` / `RankedGoal` cleanly and removes the redundant `ActiveGoal` component. No compatibility shim (FND-28); the live authority path uses the new names end-to-end and a single authority tracks committed state.

## Non-Goals

- `Exhausted` / `Abandoned` variants — deferred until a scenario surfaces a case where `DiscrepancyMemory` TTL is insufficient and a goal must be marked structurally dead.
- Concurrent multiple committed goals. The current model commits to one goal at a time; S115 preserves that.
- Replacing margin-based commitment (S74). S74 still decides when to break the commitment; S115 makes the decision surface lifecycle-aware by feeding `AgendaState.committed.motive_score` into the switch-margin check at `crates/worldwake-ai/src/agent_tick/active_action.rs:180-199`.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-16 (Ignorance, Uncertainty, Contradiction First-Class) | The D4A classifier turns the probe's opaque `RejectedBeforeSearch { reason }` into three explicit lifecycle branches (`Satisfied` / `InfeasibleUntil` / `Dead`), exhaustively over all nine `Discrepancy` variants. "I can't do this right now" becomes queryable state, not a heuristic the caller re-derives. |
| FND-20 (Resource-Bounded Practical Reasoning) | An explicit agenda lets agents keep track of "what I wanted to do but can't now" without re-ranking from scratch every tick. Lower cognitive cost per tick. Dampeners (`pending_capacity`, `suspended_capacity`, `revive_cooldown_ticks`) bound memory and re-evaluation work. |
| FND-21 (Intentions Are Revisable Commitments) | Commitment is explicit state with a kill condition. Revisability is a lifecycle transition (`committed → suspended`), not a silent re-rank. A *satisfied* committed goal (D4A) is preserved as `Suspended` rather than silently dropped, which is the correctness signal the S112 cargo-delivery regression surfaced. |
| FND-22 (Agent Diversity) | `AgendaEntry.origin` records whether the goal came from a need, an obligation, a social commitment, or an opportunity. Per-agent `AgendaProfile` (D6) lets one agent abandon stale pending goals quickly, another patiently. |
| FND-26 (Systems Interact Through State) | `AgendaState` is ai-layer read/write state; other systems never call into the agenda manager. The portfolio reads `AgendaState.committed`; the observer reads the runtime map directly. No sim-layer belief-view accessor (`worldwake-sim` cannot depend on ai). |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `ActiveGoal` component is removed entirely; no alias, no derived cache, no shim. `AgendaEntry` is the single authority for the agent's committed goal. All ~10 call sites across `agent_tick/planning.rs`, `execution.rs`, `active_action.rs`, `observation.rs` migrate to `AgendaState.committed`. |
| FND-29A (Causal History) | Lifecycle transitions are append-only events (S110). Why a goal was suspended, for what revival condition, and when it revived are all queryable through existing `GoalCommitted`, `GoalSuspended`, `GoalAbandoned` event tags. |

## Deliverables

### D1: `AgendaState` (ai-layer per-agent runtime state)

`AgendaState` is NOT an ECS component (follows the `AgentDecisionRuntime` precedent — see `crates/worldwake-ai/src/decision_runtime.rs:151` and the "is_not_registered_as_a_component" test at `decision_runtime.rs:438`). It lives as per-agent ai-layer state in the existing runtime map (`runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` at `crates/worldwake-ai/src/agent_tick/mod.rs:70`), either embedded inside `AgentDecisionRuntime` as a new field or stored in a sibling `agenda_by_agent: BTreeMap<EntityId, AgendaState>` map. Implementation chooses embedded unless sibling-map yields a materially cleaner save/load split.

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaState {
    pub committed: Option<AgendaEntry>,
    pub pending: BTreeMap<AgendaEntryKey, AgendaEntry>,
    pub suspended: BTreeMap<AgendaEntryKey, AgendaEntry>,
}

/// Reuses the existing OpportunityKey ({goal_key, anchor}) from worldwake-core.
/// This is a type alias, not a new type — AgendaEntry is indexed the same way
/// committed opportunities are indexed elsewhere in the AI crate.
pub type AgendaEntryKey = worldwake_core::OpportunityKey;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaEntry {
    pub key: AgendaEntryKey,
    pub offer: GoalOffer,
    pub phase: AgendaPhase,
    pub origin: AgendaOrigin,
    /// Tick at which this entry was introduced. Absorbs the old
    /// ActiveGoal.adopted_at field when this entry is `committed`.
    pub introduced_tick: Tick,
    pub last_reconsidered_tick: Tick,
    pub revival_trigger: Option<RevivalTrigger>,
    pub kill_condition: KillCondition,
    /// Scoring fields absorbed from the old RankedGoal type.
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    pub feasibility: FeasibilityHint,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgendaPhase { Committed, Pending, Suspended }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgendaOrigin {
    NeedDrive,
    Obligation { artifact: EntityId },
    SocialCommitment { expectation: ExpectationId },
    Opportunity { evidence: EntityId },
    Exploration,
    Enterprise,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

### D2: `GoalOffer` and `AgendaEntry` (rename targets + `ActiveGoal` absorption)

- `GroundedGoal` → `GoalOffer` (current definition at `crates/worldwake-ai/src/goal_model.rs:2306`). Add fields: `obligation_source: Option<EntityId>`, `commitment_impact_if_ignored: Permille`, `required_information_gaps: Vec<BeliefClaimKey>`, `invalidators: Vec<Invalidator>` (reuses existing `Invalidator` from `crates/worldwake-ai/src/plan_guard.rs:36`), `learned_expectation_refs: Vec<ExpectationId>`.
- `RankedGoal` → `AgendaEntry` (current definition at `crates/worldwake-ai/src/goal_model.rs:2528`). The scoring fields from `RankedGoal` (`priority_class`, `motive_score`, `provenance`, `source_reliability_discount`, `competition_discount`, `feasibility`) move into `AgendaEntry`'s fields alongside the lifecycle data (see D1).
- **`ActiveGoal` removed**: the `ActiveGoal` component at `crates/worldwake-core/src/intention.rs:14` is deleted from core entirely. Its fields `goal_key` and `adopted_at` are absorbed: `goal_key` is already present in `AgendaEntry.key.goal_key` (via `OpportunityKey`), and `adopted_at` becomes `AgendaEntry.introduced_tick` when the entry occupies the `committed` slot. All existing call sites migrate:
  - `crates/worldwake-ai/src/agent_tick/planning.rs:1103,1152,1184,1364` — `active_goal: &mut Option<ActiveGoal>` parameters become `agenda: &mut AgendaState` reads/writes of the `committed` slot.
  - `crates/worldwake-ai/src/agent_tick/execution.rs:694-711` — `set_component_active_goal` writes are replaced by mutations on the ai runtime map's `AgendaState`.
  - `crates/worldwake-ai/src/agent_tick/active_action.rs:42,56,206,240` — same migration pattern.
  - `crates/worldwake-ai/src/agent_tick/observation.rs:472` — same.
  - `crates/worldwake-core/src/component_schema.rs:1693-1716` — remove the `active_goals` registration block entirely.
  - All `get_component_active_goal`, `set_component_active_goal` ECS accessors are deleted.

Rename mechanics: atomic rename across `candidate_generation.rs`, `ranking.rs`, `agent_tick/planning.rs`, tests. No aliases — FND-28.

### D3: Agenda manager flow

`agenda_manager.rs::tick_agenda`:

```rust
pub fn tick_agenda(
    actor: EntityId,
    state: &mut AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    profile: &AgendaProfile,
    tick: Tick,
) -> AgendaTransitions {
    // 1. Evaluate kill conditions — drop dead committed/pending/suspended.
    let killed = drain_killed(actor, state, beliefs, tick);

    // 2. Evaluate revival triggers on pending — promote to candidate pool.
    //    Honours AgendaProfile.revive_cooldown_ticks: a pending entry whose
    //    last_reconsidered_tick + cooldown > tick is skipped this tick.
    let revived = promote_revived(actor, state, beliefs, discrepancy_memory, profile, tick);

    // 3. Merge fresh offers with existing pending: a fresh offer with the
    //    same key as a pending entry refreshes its `last_reconsidered_tick`
    //    but does not create a duplicate.
    let merged = merge_candidates(state, fresh_candidates, tick);

    // 4. Rank the candidate pool (committed + revived + merged pending +
    //    survival overrides) via ranking::sort_in_place.
    let ranking = rank_for_commit(state, merged, revived);

    // 5. Apply margin-based commitment (S74) — keep committed unless a
    //    candidate exceeds committed.motive_score + switch_margin.
    let commit_transition = commit_or_keep(state, ranking.as_slice(), tick);

    // 6. Demote remaining candidates to pending or suspended based on
    //    whether their revival trigger is known (D4A classifier output).
    demote_to_pending_or_suspended(state, ranking, profile, tick);

    AgendaTransitions { killed, revived, commit_transition }
}
```

The `&DiscrepancyMemory` parameter replaces the previously-proposed `AgendaMemory` trait — `DiscrepancyMemory` (`crates/worldwake-core/src/discrepancy.rs:53`) already provides `is_suppressed(key, tick)` which is the only memory lookup the agenda manager needs. No new trait required.

Ticket 003 lands this as a pure state transition function over `AgendaState`; caller-side event emission remains owned by ticket 005. The portfolio assembly (S112) reads the post-tick agenda state: `committed` → commitment slot; fresh ranked candidates remain the upstream feed into the manager.

### D4: Revival-trigger evaluation

For each pending entry, `evaluate_revival_trigger` reads the agent's belief store (via `GoalBeliefView`) and returns whether the trigger fires this tick. Triggers are cheap belief-store lookups; the dominant cost is iterating `pending` map entries (bounded by `AgendaProfile.pending_capacity` — see D6). Determinism guaranteed by `BTreeMap` iteration.

Cooldown enforcement: `promote_revived` skips any pending entry whose `last_reconsidered_tick + profile.revive_cooldown_ticks > tick`. This prevents the positive-feedback loop described in Section H §2 where a mis-classified `InfeasibleUntil` entry oscillates between pending and committed every tick.

### D4A: Feasibility-rejection → lifecycle classifier

The S112 feasibility probe (`crates/worldwake-ai/src/feasibility_probe.rs::probe`) returns `FeasibilityVerdict` — a `pub(crate)` enum defined at `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-32`:

```rust
pub(crate) enum FeasibilityVerdict {
    Plausible,
    RejectedBeforeSearch { reason: Discrepancy },
}
```

The classifier `classify_rejection` is called only on the `RejectedBeforeSearch` arm; `Plausible` verdicts require no lifecycle classification (the entry stays Pending→Committed via normal flow). `Discrepancy` (`crates/worldwake-core/src/discrepancy.rs:6-25`) has nine variants; the classifier handles all nine:

| `Discrepancy` variant | Situation | Correct lifecycle action |
|-----------------------|-----------|--------------------------|
| `MissingObservation` | Belief gap the agent can plausibly close by observing | `InfeasibleUntil { CommodityAvailable { .. } }` when the goal is commodity-bound; otherwise `InfeasibleUntil { TargetPresent { .. } }` |
| `RouteUnknown` | Agent doesn't know a route needed for the plan | `InfeasibleUntil { RouteLearned { from, to } }` |
| `PartialExecutionDrift` | Execution partially committed before plan drifted | `InfeasibleUntil { CounterpartyAvailable { .. } }` or `TickElapsed` depending on the drift carrier |
| `BeliefStale` | Previously believed fact has aged out | `InfeasibleUntil { TickElapsed { at_tick: tick + revive_cooldown_ticks } }` — a short delay before re-probing |
| `BeliefContradicted` | New evidence directly contradicts the believed claim that motivated the offer | `Dead` — the offer's premise is refuted; `DiscrepancyMemory` TTL governs re-emission |
| `NoWillingCounterparty` | Needed counterparty exists but is unwilling or unavailable | `InfeasibleUntil { CounterpartyAvailable { counterparty, place } }` |
| `SearchBudgetExhausted` | Planning didn't find a viable plan within budget | `InfeasibleUntil { TickElapsed { at_tick: tick + revive_cooldown_ticks } }` — conservative; some budget configurations recover by the next tick |
| `NoLegalBinding` | World no longer supports the believed legal/institutional binding | `Dead` — structurally infeasible |
| `ImproperPlanningState` | Planning assumed a state that should never have been treated as valid | `Dead` — structurally infeasible |

Additionally, any `RejectedBeforeSearch` verdict whose goal's post-conditions are already true in the agent's belief store is reclassified to `Satisfied` before the variant table applies — this is the cargo-at-destination case, independent of which `Discrepancy` variant the probe returned:

| Situation | Correct lifecycle action |
|-----------|--------------------------|
| Goal's post-conditions are already true in beliefs (e.g., `MoveCargo` when agent is at destination with the cargo) | Move to `Suspended` with `KillCondition::External`; `revival_trigger = None`. The commitment persists until `KillCondition` fires (typically `TargetDead` / `ObligationResolved`) or the goal's post-conditions stop holding. |

`agenda_manager::classify_rejection(probe_verdict: &FeasibilityVerdict, offer: &GoalOffer, beliefs: &impl GoalBeliefView) -> RejectionLifecycle` produces:

```rust
pub enum RejectionLifecycle {
    Satisfied,
    InfeasibleUntil { trigger: RevivalTrigger },
    Dead,
}
```

The classifier is deterministic, pure over (verdict, offer, beliefs), and has no side effects. It is the single authoritative decoder of the probe's rejection reason into agenda lifecycle state — downstream modules do not inspect `Discrepancy` variants to infer the same distinction (closing the Gap-2 gap surfaced by the S112 incident, where `build_candidate_plans` had to special-case rejected committed opportunities in `search_order`).

For the `Dead` branch, `classify_rejection` returns the lifecycle but leaves memory-write to the caller. `tick_agenda` then synthesises a `BlockerKey { goal_key: offer.key, place: offer.anchor.place(), target: offer.anchor.entity(), action_def: None }` and calls `DiscrepancyMemory::record` with `DiscrepancyClearing::TtlExpiry`. This keeps the classifier pure.

Migration note: the S112-era special cases in `build_candidate_plans` that kept a rejected *committed* opportunity alive — **both** the `search_order` exclusion at `agent_tick/planning.rs:400-427` (the `!is_committed` check inside `rejected_opportunities`) **and** the `rejected_by_goal` skip at `agent_tick/planning.rs:875-892` (the `if slot.ranked.grounded.key == committed_goal { continue; }` guard) — disappear with S115. Post-S115, a rejected committed goal is demoted to `Suspended` (satisfied) or `Pending` (infeasible-now) or dropped as `Dead` by the classifier before `search_order` is built; the search order then trusts the admitted ranking without either carve-out.

### D5: Suspended vs pending

- **Pending**: has a concrete `RevivalTrigger` that can fire. The agent is actively waiting for the condition.
- **Suspended**: no viable revival trigger (e.g., offer was valid once but the underlying opportunity evidence is now contradicted, or the goal is already satisfied). Suspended entries stay until `KillCondition` fires, then they are dropped.

Observer tooling distinguishes them when rendering agent decision state. This directly addresses FND-29 ("why is the agent idle?" → "5 pending goals, 2 suspended, here's what they're waiting for"). Observer reads `AgendaState` directly from the ai runtime map — no belief-view accessor.

### D6: Agenda capacity and profile

New per-agent ECS component `AgendaProfile` in `worldwake-core`, registered on `EntityKind::Agent` in `crates/worldwake-core/src/component_schema.rs` following the `ActiveGoal`/`IntentionFrame` pattern (runtime accessors: `insert_agenda_profile`, `get_component_agenda_profile`, etc.). Shape:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaProfile {
    pub pending_capacity: u32,
    pub suspended_capacity: u32,
    pub revive_cooldown_ticks: u32,
}

impl Default for AgendaProfile {
    fn default() -> Self {
        Self {
            pending_capacity: 16,
            suspended_capacity: 8,
            revive_cooldown_ticks: 4,
        }
    }
}

impl Component for AgendaProfile {}
```

`AgendaProfile` is classified as **universal** per `docs/spec-drafting-rules.md` §5. `AgentDef` gains `#[serde(default)] pub agenda_profile: Option<AgendaProfile>` and `spawn_agent()` wires `txn.set_component_agenda_profile(agent_id, agenda_profile.unwrap_or_default())?` (pattern analog: `crates/worldwake-cli/src/scenario/mod.rs:374` for `HomeostaticNeeds`). Runtime access uses `expect()`, not silent fallback.

When `AgendaState.pending` or `AgendaState.suspended` exceeds the respective capacity, evict the oldest entry (smallest `last_reconsidered_tick`).

### D7: Margin-based commit integration

S74's switch-margin logic stays in place but now reads from `AgendaState.committed` instead of the rank-derived current. The margin check at `crates/worldwake-ai/src/agent_tick/active_action.rs:180-199` (currently reading `cognitive.switch_margin` alongside the ranked-candidate loop) becomes: "does the highest-ranked candidate exceed `committed.motive_score + switch_margin`?" Same behavior; cleaner surface — the `switch_margin` / `planning_switch_margin` fields on `CognitiveProfile` (`crates/worldwake-core/src/cognitive_profile.rs:39,41`) are unchanged.

### D8: AgendaState access for read-only consumers

`AgendaState` lives in the ai runtime map and is accessible to:
- **Observer** (`crates/worldwake-cli/src/bin/observer.rs`): reads the runtime map from the ai-facing public API used for decision-trace rendering. No new trait method on `GoalBeliefView`.
- **Decision trace** (`crates/worldwake-ai/src/decision_trace.rs`): reads directly within the ai crate.
- **Agenda manager itself** (`agenda_manager.rs`): reads and writes directly.
- **`build_candidate_plans` / `assemble_portfolio`**: read `AgendaState.committed` to populate the commitment slot. The `committed_opportunity: Option<OpportunityKey>` parameter threaded through `build_candidate_plans` today (`agent_tick/planning.rs:341`) now comes from `AgendaState.committed.as_ref().map(|entry| entry.key)`.

No sim-layer belief-view accessor. `worldwake-sim::GoalBeliefView` cannot expose `&AgendaState` because `AgendaState` is defined in `worldwake-ai` and sim cannot depend on ai (CLAUDE.md crate layering: `core → sim → systems → ai → cli`).

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Revival triggers read the agent's own belief store. No cross-agent information flow. A social commitment's revival (e.g., counterparty arrives) fires only when the agent perceives the arrival — delegated to the perception system (FND-7, FND-15), not omniscient query. The D4A classifier is pure over `(probe_verdict, offer, beliefs)` with no side effects and no cross-agent reads. The `Dead`-branch `DiscrepancyMemory::record` write is performed by the caller (`tick_agenda`) against the agent's own memory component — no cross-agent state mutation.
2. **Positive-feedback analysis**: Potential loop: pending → revival fires → commit → plan fails → demote to pending → revival fires again. Dampener: S109 `DiscrepancyMemory` records the failure and suppresses re-emission; S115's `AgendaProfile.revive_cooldown_ticks` enforces a minimum inter-revival gap (D4). A second potential loop is D4A misclassifying a structurally-dead goal as `InfeasibleUntil`, producing endless pending-revive-dead-pending cycles; dampener is `revive_cooldown_ticks` (D6) plus `DiscrepancyMemory` suppression of the emit side plus `SearchBudgetExhausted`/`BeliefStale` short delays consolidating into `TickElapsed` triggers.
3. **Concrete dampeners**: `AgendaProfile.pending_capacity` + eviction by `last_reconsidered_tick` + `suspended_capacity` + `revive_cooldown_ticks` + `DiscrepancyMemory` suppression. All profile-driven.
4. **Stored state vs. derived read-model**: `AgendaState` is authoritative per-agent runtime state in the ai crate (save/load-compatible via `Serialize/Deserialize` on the containing runtime map, analogous to `AgentDecisionRuntime`). `AgendaProfile` is authoritative ECS component state. Lifecycle transitions produce event-log entries (S110). Portfolio slots (S112) are derived read-views over the agenda. The D4A classifier result is *not* stored — it is a per-tick derived decision over inputs that are themselves stored (probe verdict, belief store).

## SystemFn Integration

**New SystemFn**: `agenda_tick_system`. Placement: early in agent-tick phase, after perception/belief-update (`refresh_runtime_for_read_phase_with_memories` at ~`crates/worldwake-ai/src/agent_tick/mod.rs:930`), before candidate-generation+ranking (which feeds `build_candidate_plans` at `agent_tick/planning.rs:341`). Produces the agenda state the portfolio reads.

## Component Registration

- `AgendaState` — **not a component**. Per-agent ai-layer runtime state stored in `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` at `crates/worldwake-ai/src/agent_tick/mod.rs:70` (either as a new field on `AgentDecisionRuntime` or in a sibling `agenda_by_agent` map). Save/load via the containing type's `Serialize`/`Deserialize`. Follows `AgentDecisionRuntime` precedent (`decision_runtime.rs:151`, with explicit "not-a-component" test at `decision_runtime.rs:438`). Exempt from scenario authoring — see the §5 tail of `docs/spec-drafting-rules.md` (same exemption class as `ActiveGoal`, `IntentionFrame`, `WoundList`, all runtime-only).
- `AgendaProfile` — register on `EntityKind::Agent` in `crates/worldwake-core/src/component_schema.rs`. Universal with `Default`, scenario-authorable. `AgentDef` field; `spawn_agent()` uses `unwrap_or_default()`; runtime access uses `expect()`.

## Cross-System Interactions

- **Agenda manager ↔ candidate generation**: Generation produces `GoalOffer`s each tick (post-rename from `GroundedGoal`); manager merges them into existing pending.
- **Agenda manager ↔ ranking**: Ranking scores the candidate pool (committed + revived + fresh) for commit decision, using the same `ranking::sort_in_place` path (S123).
- **Agenda manager ↔ S112 portfolio**: Portfolio reads `AgendaState.committed` to populate the commitment slot and builds survival/economic slots from fresh offers. The S112-era `!is_committed` and `committed_goal` carve-outs in `build_candidate_plans` are deleted.
- **Agenda manager ↔ S110 event log**: Every lifecycle transition emits an `EventTag` (`GoalCommitted`, `GoalSuspended`, `GoalAbandoned`) with the existing payload types.
- **Agenda manager ↔ S109 discrepancy memory**: Revival and commit checks read the memory to avoid reviving suppressed goals; `Dead` classifications write back to the memory.
- **Agenda manager ↔ ActiveGoal removal**: All code paths that previously read `ActiveGoal` via `get_component_active_goal` now read `AgendaState.committed`. All code paths that wrote via `set_component_active_goal` now mutate the ai-layer `AgendaState.committed` field.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `pending_capacity` | `AgendaProfile` | `u32` | 16 | Max pending entries |
| `suspended_capacity` | `AgendaProfile` | `u32` | 8 | Max suspended entries |
| `revive_cooldown_ticks` | `AgendaProfile` | `u32` | 4 | Minimum ticks between successive revivals of the same key; also the delay used for `TickElapsed` triggers synthesised from `BeliefStale`/`SearchBudgetExhausted` |

## Validation and Falsification

### Unit tests

1. Fresh offer with same key as pending entry updates `last_reconsidered_tick` without duplicating.
2. `RevivalTrigger::CommodityAvailable` fires when belief-store confirms quantity ≥ min.
3. `KillCondition::TickExpiry` drops a pending entry at or after the expiry tick.
4. Capacity overflow evicts the oldest `last_reconsidered_tick`.
5. A revived entry becomes a commit candidate; if it wins, `AgendaState.committed` updates and `EventTag::GoalCommitted` is emitted.
6. D4A `classify_rejection` on a satisfied `MoveCargo` (agent at destination with cargo) returns `RejectionLifecycle::Satisfied`; the entry is demoted to `Suspended` with no revival trigger.
7. D4A `classify_rejection` on `AcquireCommodity` with `MissingObservation` returns `RejectionLifecycle::InfeasibleUntil { trigger: CommodityAvailable { .. } }`; the entry is demoted to `Pending` with a matching trigger that fires when a later belief update resolves the observation gap.
8. D4A `classify_rejection` on `NoLegalBinding` returns `RejectionLifecycle::Dead`; the entry is dropped, and a `DiscrepancyMemory` entry keeps the ranker from re-emitting until TTL.
9. D4A exhaustiveness: each of the nine `Discrepancy` variants produces a deterministic `RejectionLifecycle` per the D4A table.
10. `revive_cooldown_ticks` enforcement: a pending entry revived at tick T is not re-considered for revival before tick T + cooldown.

### Integration tests

11. Two-tick scenario: agent commits goal A at tick 1; tick 2 belief confirms A still viable; `AgendaState.committed == A` at end of tick 2 (no re-commit churn).
12. Deterministic replay: re-running a recorded simulation reproduces identical agenda state at every tick.
13. Cargo-delivery: the existing `cargo_satisfaction_at_destination_while_carrying` assertion (currently an in-crate unit test at `crates/worldwake-ai/src/agent_tick/tests.rs:4710`, to be migrated into this crate's integration suite or kept in-place and adapted) — `AgendaState.committed` still holds `MoveCargo` after delivery — passes via the D4A classifier demoting the committed goal to `Suspended` (satisfied), not via the search-order special case the S112 incident fix added.
14. Portfolio rejection: the `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` assertion (`crates/worldwake-ai/tests/golden_portfolio_planning.rs:210`) — commit the feasible economic goal within two ticks, ignore the rejected `Sleep` / `ReportMissing` slots — passes via the D4A classifier killing the structurally-infeasible commitment rather than pinning it as the committed priority.
15. Existing goldens pass (`scenarios/survival-baseline.ron`, `scenarios/survival-contested.ron`, `crates/worldwake-ai/tests/golden_planner_pathology.rs`).
16. `ActiveGoal` removal validation: grep confirms zero remaining references to `ActiveGoal`, `get_component_active_goal`, `set_component_active_goal` outside this spec and outside archive/.

### Golden test

17. New scenario `golden_agenda_lifecycle.rs`: agent's purchase goal becomes pending when merchant departs, revives when merchant returns, commits, and completes — all transitions visible in event log and in final agenda state.
18. Extension of `golden_agenda_lifecycle.rs`: agent's cargo-delivery goal reaches destination, the D4A classifier demotes it to `Suspended` (satisfied), and the suspended entry appears in observer output for one post-satisfaction tick before `KillCondition::External` clears it.

## Outcome

To be filled in at completion.
