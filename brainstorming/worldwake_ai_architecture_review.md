# Worldwake AI architecture review and spec bundle

## Verdict

The architecture has strong bones: explicit action semantics, deterministic state discipline, intention frames, and a causal event log. But it is **not** ready for real partial observability without a structural decision-layer rewrite.

The central problem is not file size. It is that the current planner still reasons over a **flattened believed world** with **tactic-shaped goals**. That will not survive contradictory reports, stale beliefs, expectation failure, or local coordination without creating more ad hoc patches.

## Good parts worth preserving

- explicit action preconditions, durations, interruptibility, and reservations
- append-only causal event log
- revisable intention frames instead of silent hard reservations
- deterministic-authority discipline

## Main issues

1. **Omniscience is not the only problem.** The belief data model itself is wrong for the target world.
2. **`GoalKind` mixes desired conditions with chosen methods.**
3. **`GoalKey` and `GroundedGoal` are too lossy.**
4. **There is no first-class expectation/discrepancy system.**
5. **Tick phasing will be wrong when perception becomes real.**
6. **Exhaustion backoff is backwards.**
7. **Materialization barriers are too conservative.**
8. **Failure classification is too coarse.**
9. **Semantics are duplicated in too many places.**

## Priority order

1. Evidence-first belief model + deterministic projection
2. Desire/opportunity split
3. Expectation/discrepancy + epistemic actions
4. Repair-first replanning + blocker scope + search reuse
5. Tick phasing + visible intent signals
6. Declarative registration / compile-time completeness
7. Learned local preferences + limited multi-desire side-benefits

---

## Spec WW-AI-001 — Evidence-first belief state and deterministic projection

~~~md
Priority: P0
Status: Mandatory before real perception

Affected crates:
- worldwake-core
- worldwake-sim
- worldwake-ai
- worldwake-systems

Problem:
The current planning layer consumes flattened believed entity state and cannot naturally represent contradiction, provenance, freshness, or competing hypotheses.

Requirements:
1. Introduce first-class per-agent belief facts.
2. Every belief fact MUST include:
   - proposition
   - provenance
   - acquired_at tick
   - confidence
   - freshness metadata
   - whether the source is direct, testimonial, recorded, inferred, or self-state
3. Multiple active facts for the same proposition slot MUST be allowed.
4. The planner MUST NOT read authoritative world truth except through the agent's projected belief view.
5. Each tick, build a deterministic `BeliefProjection` for planning:
   - prefer direct observation over testimony,
   - prefer fresher evidence over staler evidence,
   - prefer shorter testimony chains over longer chains,
   - use stable ID ordering for tie-breaks.
6. The projection MUST retain alternate conflicting facts for traceability even when one fact is preferred for planning.
7. Candidate generation MUST return supporting belief-fact handles, not only entities/places.
8. Save/load, replay, and event history MUST preserve belief facts and provenance.

Non-goals:
- full belief-space contingent planning
- drama-generating probabilities
- hidden omniscient fallback queries

Acceptance:
- An agent can hold two conflicting reports about a commodity source.
- Direct fresh observation beats an older rumor.
- A trace can answer which fact caused a plan choice.
- Belief contradictions survive save/load and replay.
~~~

~~~rust
pub struct BeliefFactId(pub u64);

pub enum BeliefSource {
    SelfState,
    DirectObservation { place: EntityId },
    Testimony { speaker: EntityId, chain_depth: u8 },
    Record { record: EntityId },
    Inference { basis: Vec<BeliefFactId> },
}

pub struct BeliefFact {
    pub id: BeliefFactId,
    pub holder: EntityId,
    pub proposition: BeliefProposition,
    pub source: BeliefSource,
    pub acquired_at: Tick,
    pub confidence: Permille,
    pub freshness_until: Tick,
    pub retracted_by: Option<BeliefFactId>,
}

pub struct BeliefProjection {
    pub preferred: BTreeMap<BeliefSlot, BeliefFactId>,
    pub alternatives: BTreeMap<BeliefSlot, BTreeSet<BeliefFactId>>,
}
~~~

---

## Spec WW-AI-002 — Separate desires from opportunities

~~~md
Priority: P0
Status: Mandatory

Affected crates:
- worldwake-core
- worldwake-ai

Problem:
`GoalKind` currently mixes:
- desired world condition,
- chosen tactic,
- process stage,
- sometimes even a domain-specific action label.

Requirements:
1. Introduce `DesireKey` as the authoritative identity of "what condition the agent wants true".
2. Introduce `OpportunityKey` as the authoritative identity of "which concrete evidence-backed opportunity/tactic the agent is currently pursuing".
3. Intention frames MUST persist on `DesireKey`.
4. A specific plan MAY bind to an `OpportunityKey`.
5. Candidate generation MUST be able to emit multiple opportunities for one desire.
6. Blockers and exhaustion MUST key primarily by opportunity scope, not only by desire.
7. Escalation from opportunity-blocked to desire-blocked MUST happen only when:
   - all known opportunities are blocked, or
   - the blocker is structural at the desire level.
8. Ranking MAY deduplicate for presentation, but the planner MUST preserve distinct opportunities internally.

Migration note:
You do not need to collapse immediately into five mega-goals.
The minimum correct change is to stop using action-verb variants as the authoritative top-level identity when a desired state is what actually matters.

Acceptance:
- Orchard apples blocked does not block market apples.
- A stale rumor and a fresh sighting of the same commodity remain distinct opportunities.
- Frames survive tactic changes when the desire remains the same.
~~~

~~~rust
pub enum DesireKind {
    SatisfyNeed { need: HomeostaticNeedId },
    SecureCommodity {
        commodity: CommodityKind,
        purpose: CommodityPurpose,
        min_qty: Quantity,
    },
    RestoreSafety { subject: EntityId },
    DeliverBelief { listener: EntityId, topic: TellTopic },
    AdvanceCase { violation_id: ViolationId, stage: CaseStage },
    GainOffice { office: EntityId },
    DisposeCorpse { corpse: EntityId },
}

pub struct DesireKey {
    pub kind: DesireKind,
}

pub enum OpportunityAnchor {
    Place(EntityId),
    Entity(EntityId),
    Route { from: EntityId, to: EntityId },
    Record(EntityId),
    None,
}

pub struct OpportunityKey {
    pub desire: DesireKey,
    pub tactic: TacticKind,
    pub anchor: OpportunityAnchor,
    pub supporting_facts: BTreeSet<BeliefFactId>,
}
~~~

---

## Spec WW-AI-003 — Expectations, discrepancies, and epistemic actions

~~~md
Priority: P0
Status: Mandatory

Affected crates:
- worldwake-core
- worldwake-sim
- worldwake-ai
- worldwake-systems/perception

Problem:
The architecture currently has no first-class representation of:
- what the agent expected to find,
- which evidence created that expectation,
- how a mismatch becomes new state.

Requirements:
1. Any plan step that depends on mutable external world facts MUST register one or more expectation records.
2. An expectation MUST include:
   - owning agent,
   - linked desire/opportunity,
   - expected proposition,
   - supporting belief fact(s),
   - establishment tick,
   - expiry/obsolescence rules.
3. Perception and action start/completion MUST be able to generate explicit discrepancy records when observation refutes an expectation.
4. Discrepancy records MUST be usable by:
   - candidate generation,
   - replanning,
   - traces,
   - future testimony/reporting.
5. Introduce concrete epistemic actions/goals, such as:
   - inspect place,
   - inspect container,
   - ask witness,
   - read notice,
   - verify target location,
   - confirm source availability.
6. No plan MAY be silently corrected by authoritative truth without a lawful information path.
7. Ranking SHOULD discount stale/low-confidence opportunities by expected verification cost.

Non-goals:
- full contingent policy planning
- omniscient "missing thing" detection

Acceptance:
- Empty stash discovery produces a discrepancy against prior expectation.
- Rumor-driven travel to an empty source produces a discrepancy record and replanning.
- Other agents may continue to act on stale information until new evidence reaches them.
~~~

~~~rust
pub struct ExpectationRecord {
    pub id: EntityId,
    pub agent: EntityId,
    pub desire: DesireKey,
    pub opportunity: OpportunityKey,
    pub expected: BeliefProposition,
    pub supported_by: BTreeSet<BeliefFactId>,
    pub established_at: Tick,
    pub expires_at: Tick,
}

pub enum DiscrepancyKind {
    ExpectedItemMissing,
    SourceEmpty,
    TargetAbsent,
    OfficeNotVacant,
    ThreatPresent,
    PatientAlreadyHandled,
}

pub struct DiscrepancyRecord {
    pub id: EntityId,
    pub agent: EntityId,
    pub expectation: EntityId,
    pub observed_at: Tick,
    pub kind: DiscrepancyKind,
    pub observation_facts: BTreeSet<BeliefFactId>,
}

pub enum EpistemicTactic {
    InspectPlace,
    InspectContainer,
    AskWitness,
    ReadNotice,
    ConsultRecord,
    VerifyTargetLocation,
}
~~~

---

## Spec WW-AI-004 — Repair-first replanning, blocker scope, and deterministic barrier continuation

~~~md
Priority: P0
Status: Mandatory

Affected crates:
- worldwake-ai

Problem:
The current exhaustion policy reduces search depth after repeated failure.
That is the wrong shape for dynamic but structured worlds.

Requirements:
1. Replace budget-halving exhaustion backoff with retry cooldowns and reusable search artifacts.
2. The planner SHOULD attempt plan repair from the first broken step before full replanning.
3. Blockers MUST carry explicit scope:
   - opportunity,
   - source/place,
   - route,
   - entity,
   - desire.
4. Failure facts MUST be more specific than `Unknown` wherever the simulation can determine a concrete cause.
5. Deterministic materialization actions MUST be eligible for continued hypothetical planning.
6. Materialization barriers SHOULD remain only for:
   - nondeterministic outcomes,
   - negotiated outcomes whose result is not planner-determined,
   - externally contested outcomes whose winner is unresolved.
7. Search traces MUST include:
   - expansions used,
   - frontier max size,
   - repair attempted or not,
   - reused memo yes/no,
   - barrier reason.

Acceptance:
- `Travel -> Harvest -> Consume` can remain one plan when harvest output is deterministic.
- Blocking one source does not suppress all sources for the same desire.
- Repeated similar replans get less frequent, not shallower by default.
- A changed counterparty location repairs the suffix instead of rebuilding the whole plan.
~~~

~~~rust
pub enum BlockerScope {
    Opportunity(OpportunityKey),
    Place(EntityId),
    Route { from: EntityId, to: EntityId },
    Entity(EntityId),
    Desire(DesireKey),
}

pub enum FailureFact {
    SourceDepleted { source: EntityId, commodity: CommodityKind },
    CounterpartyAbsent { target: EntityId },
    ReservationLost { resource: EntityId },
    JurisdictionMissing { office: EntityId },
    ExpectedContradiction { discrepancy: EntityId },
    Unknown,
}

pub struct ExhaustionEntry {
    pub next_retry_tick: Tick,
    pub consecutive_failures: u8,
    pub reusable_search: Option<SearchMemo>,
}

pub struct SearchMemo {
    pub problem_signature: PlanningProblemSignature,
    pub cached_routes: BTreeMap<(EntityId, EntityId), u32>,
}
~~~

---

## Spec WW-AI-005 — Tick phasing and visible local coordination

~~~md
Priority: P0
Status: Mandatory before true perception

Affected crates:
- worldwake-sim
- worldwake-systems
- worldwake-ai

Problem:
Planning before belief refresh will produce one-tick nonsense under real local perception.

Requirements:
1. Split the tick into explicit phases:
   - integrate completed actions / committed world mutations,
   - refresh beliefs from current locally observable world state,
   - deliberate,
   - start actions,
   - advance world systems,
   - emit observations caused by new mutations.
2. Agents MUST perceive their current local context before selecting a new action for that tick.
3. Visible actions SHOULD emit observable intent signals or claim artifacts.
4. Coordination MUST remain local and state-mediated:
   - no telepathic shared planning,
   - no hidden joint planner authority.
5. Candidate generation and ranking MAY use visible intent/claim artifacts to avoid dogpiles.
6. Claims MUST be world state if they matter to others.

Acceptance:
- An agent arriving in a dangerous location can react before issuing the next non-safety action.
- Seeing another agent queue or harvest can alter choice without any hidden reservation.
- Replay preserves identical behavior under the same seed.
~~~

~~~rust
pub enum TickPhase {
    IntegrateCommitted,
    RefreshBeliefs,
    Decide,
    StartActions,
    AdvanceWorld,
    EmitObservations,
}

pub enum IntentSignalKind {
    TravelingTo { destination: EntityId },
    QueueingFor { facility: EntityId },
    Harvesting { source: EntityId, commodity: CommodityKind },
    Treating { patient: EntityId },
    Claiming { target: EntityId },
}

pub struct IntentSignal {
    pub actor: EntityId,
    pub place: EntityId,
    pub kind: IntentSignalKind,
    pub visible_until: Tick,
}
~~~

---

## Spec WW-AI-006 — Declarative registration and compile-time completeness

~~~md
Priority: P1
Status: Strongly recommended

Affected crates:
- worldwake-ai
- worldwake-core

Problem:
Per-goal logic is spread across parallel matches and duplicate tables.

Requirements:
1. Introduce a single declarative registration source for each desire/tactic family.
2. The registry MUST be able to generate or centrally define:
   - candidate generation hooks,
   - ranking family,
   - planner op relevance,
   - satisfaction checks,
   - invalidation conditions,
   - snapshot/belief requirements,
   - trace labels.
3. Exhaustiveness-sensitive matches MUST NOT use catch-all arms.
4. Adding a new desire/tactic MUST fail compilation if required declarations are missing.
5. File splitting SHOULD happen after the registry exists, not before.

Acceptance:
- Adding a new desire without invalidation rules fails compile.
- Goal/op relevance cannot disagree in two different tables.
- Trace labels and ranking hooks exist for every registered desire.
~~~

~~~rust
worldwake_goal! {
    desire SecureCommodity {
        ranking_family = Survival;
        invalidates_on = [
            PositionChanged,
            CommodityChanged,
            SourceStateChanged,
            BlockerExpired,
        ];
        belief_requirements = [
            SelfState,
            Inventory,
            KnownSources,
            RouteKnowledge,
            FacilityQueues,
        ];
        tactics = [
            AcquireByTrade,
            AcquireByHarvest,
            AcquireByCraft,
            AcquireByTheft,
        ];
    }
}
~~~

---

## Optional features after P0

### Feature A — Learned local preferences

~~~md
Add concrete, evidence-backed memories:
- route experience,
- seller reliability,
- witness reliability,
- facility reliability.

These MUST remain beliefs/caches, never truth.
~~~

~~~rust
pub struct RouteExperience {
    pub edge_from: EntityId,
    pub edge_to: EntityId,
    pub last_travel_tick: Tick,
    pub hostile_encounters: u16,
    pub safe_trips: u16,
    pub supported_by: Option<BeliefFactId>,
}
~~~

### Feature B — Limited multi-desire side-benefit scoring

~~~md
Do not jump straight to a full multi-desire planner.
First allow one plan to accrue side-benefit value for secondary desires:
- a market trip can also deliver a report,
- a healing trip can also wash or eat,
- a patrol can also inspect a route.

Selection score should remain primary-goal-first, but secondary benefit should break ties and improve route choices.
~~~

~~~rust
pub struct PlanValue {
    pub primary_desire: DesireKey,
    pub primary_priority: GoalPriorityClass,
    pub side_benefits: BTreeMap<DesireKey, BenefitEstimate>,
    pub expected_ticks: u32,
    pub stability: Permille,
}
~~~

---

## Immediate low-risk patch set

~~~md
Do these even before the larger rewrite:
1. Remove wildcard match arms from exhaustion invalidation logic.
2. Add explicit failure facts beyond `Unknown`.
3. Add trace metrics for expansions, frontier size, barrier reason, repair attempt, and memo reuse.
4. Add regression tests for:
   - rumor_then_empty_source_generates_discrepancy_and_replans
   - source_scoped_blocker_preserves_alternative_market_plan
   - travel_completion_then_perception_before_planning
   - repeated_exhaustion_cools_down_without_shrinking_to_trivial_budget
   - visible_competition_changes_choice_without_hidden_reservation
   - direct_fresh_observation_outranks_stale_testimony
~~~

## Things I would explicitly deprioritize

- Splitting giant files before the semantic refactor
- Goal-scoped snapshot field filtering on the current flattened belief model
- A full cooperative joint planner
- Full contingent/POMDP planning

## Blunt summary

Treat Epic 14 as a **decision-architecture change**, not as a mere `BeliefView` implementation task.
If you ship real perception into the current planner shape unchanged, it will still compile, but it will start producing the wrong kind of world.
