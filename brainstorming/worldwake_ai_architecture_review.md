# Worldwake AI and Simulation Upgrade Proposal

## Verdict

Keep the current GOAP foundation. Do **not** replace it with behavior-tree scripting, HTN rails, or LLM improvisation.

The right move is to evolve the current system into a **belief-first continual planning architecture** with:

- uncertainty-aware belief queries,
- explicit information-gathering goals,
- limited contingent policy branches,
- local repair before full replan,
- stricter identity/legality enforcement,
- richer evidence and commitment artifacts,
- and authoritative decision history.

That preserves the good part of the current design: lawful affordances, determinism, locality, belief/world separation, revisable intention, and explicit action costs.

---

## 1. Current architecture issues to fix

### 1.1 Uncertainty exists in the model, but the planner mostly sees collapsed answers
**Alignment:** FND-14, FND-15, FND-16, FND-20

`RuntimeBeliefView` mainly exposes crisp values such as `Option<EntityId>`, `Vec<EntityId>`, and `Quantity`. That is too lossy for the kind of world your FOUNDATIONS require.

Result:
- contradictory reports get flattened too early,
- stale beliefs are hard to reason about except in a few special cases,
- the planner cannot cleanly choose between “act now” and “verify first”,
- and social/investigative intelligence gets capped because provenance and freshness are not first-class in most planner queries.

**Fix:** expose belief objects, not just chosen values.

Proposed shape:

~~~text
BeliefValue<T> {
  value: T,
  confidence: Permille,
  observed_at: Tick,
  claimed_event_time: Option<Tick>,
  acquired_at: Tick,
  status: BeliefStatus, // Certain | Probable | Stale | Disputed | Contradicted
  provenance_chain: BeliefProvenanceId,
}

BeliefSet<T> {
  best: Option<BeliefValue<T>>,
  alternatives: Vec<BeliefValue<T>>,
}
~~~

Use these in planner-facing queries for:
- believed location,
- believed stock,
- believed route condition,
- believed ownership/access,
- believed office-holder / jurisdiction facts,
- believed target presence.

Do **not** jump to full generic epistemic planning everywhere. Use richer belief objects first. That gets most of the win.

---

### 1.2 Plans are too linear for a partial-information, interrupt-heavy world
**Alignment:** FND-7, FND-8, FND-16, FND-20, FND-21

`PlannedPlan` is a linear `Vec<PlannedStep>`. In this world that is too rigid.

The failure mode is obvious:
- the agent plans against a stale belief,
- a local contention outcome changes,
- or a target is absent on arrival,
- and the system must burn a full replan or fail through the step pipeline.

That is the wrong shape for a world where “verify, wait, ask, switch counterparty, take fallback route, back out safely” should be normal.

**Fix:** replace linear-only plans with a `PolicyPlan` that can still stay small and deterministic.

Proposed shape:

~~~text
PolicyPlan {
  entry: PolicyNodeId,
  nodes: Vec<PolicyNode>,
}

PolicyNode {
  action: PlannedAction,
  guards: Vec<PlanGuard>,
  expectations: Vec<PlanExpectation>,
  repair_options: Vec<LocalRepair>,
  on_success: BranchTarget,
  on_failure: BranchTarget,
  on_observation_mismatch: BranchTarget,
  on_contention_loss: BranchTarget,
  on_danger_spike: BranchTarget,
}
~~~

This is **not** a full POMDP rewrite. Keep it limited:
- only uncertain/high-cost steps branch,
- boring low-risk self-care stays straight-line,
- branches cover common local cases, not arbitrary world futures.

---

### 1.3 Next-step-only revalidation is too shallow
**Alignment:** FND-16, FND-20, FND-21, FND-29

Revalidating only the next step is not enough in a world with long actions, stale reports, and competing agents.

Each adopted plan step should carry:
- required believed facts,
- minimum acceptable confidence,
- explicit invalidators,
- expected observations,
- and legal repair targets.

Then the runtime can distinguish:
- irrelevant drift,
- relevant but repairable drift,
- plan-invalidating drift,
- and goal-changing drift.

**Fix:** annotate every planned node with guard conditions and expectation sets.

Minimum expectation taxonomy:
- immediate expectation,
- state expectation,
- informed expectation,
- regression expectation.

For some plan classes, also support:
- “not found where expected”,
- “counterparty unwilling”,
- “resource partially available”,
- “danger rose while en route”.

---

### 1.4 `ActionRequestMode::BestEffort` is too permissive as a default
**Alignment:** FND-4, FND-8, FND-19, FND-21, FND-24

Best-effort substitution is fine for some actions. It is dangerous for others.

Safe-ish:
- eat any fungible bread stack,
- use equivalent workstation at same place,
- travel by equivalent route segment.

Unsafe:
- accuse this specific person,
- transfer this exact item,
- claim this specific office,
- punish this exact accused,
- loot this exact corpse,
- tell this exact witness,
- escort this exact subject.

**Fix:** add per-action binding strictness.

~~~text
enum BindingStrictness {
  ExactIdentity,
  FungibleEquivalentCommodity,
  EquivalentFacilityClassAtSamePlace,
  EquivalentRouteStep,
  AnyLegalTarget,
}
~~~

Revalidation, dispatch, and exact-target fallback must all use the **same** legality path and the same strictness class.

---

### 1.5 The payload-override validator path is a design smell
**Alignment:** FND-26, FND-28

The current “normal affordance match” path plus “payload override validator” path is fragile.

That is exactly the kind of dual live authority path that turns into fossilized logic.

**Fix:** remove the distinction.

Every action should expose one authoritative binding/legality function used by:
- affordance enumeration,
- plan-time successor construction,
- revalidation,
- dispatch.

If the planner needs specialized parameters, those parameters must be typed plan bindings validated by that one function.

---

### 1.6 `BlockingFact::Unknown` collapses too many realities
**Alignment:** FND-16, FND-20, FND-29

Right now “unknown” can mean:
- genuinely unexplained failure,
- stale belief,
- temporary contention loss,
- improper post-failure state,
- structural impossibility,
- missing observation,
- or search-budget exhaustion.

Those should not share one TTL class.

**Fix:** replace `Unknown` with a discrepancy taxonomy.

Minimum set:
- `BeliefStale`
- `BeliefContradicted`
- `ContentionLost`
- `ImproperPlanningState`
- `MissingObservation`
- `NoLegalBinding`
- `NoWillingCounterparty`
- `RouteUnknown`
- `SearchBudgetExhausted`
- `StructurallyImpossible`
- `PartialExecutionDrift`

Each needs:
- distinct retry policy,
- distinct invalidation condition,
- distinct learning update,
- distinct debug explanation.

---

### 1.7 Top-2 planning makes ranking mistakes too expensive
**Alignment:** FND-20, FND-21, FND-22

`max_candidates_to_plan = 2` is too small given your world model.

If the top two are infeasible but the third is trivial, the agent wastes a tick and looks dumb.
If the ranking is slightly noisy, the planner can feel laggy or brittle.
If multiple similar goals crowd the top, diversity collapses.

**Fix:** move to portfolio planning.

Instead of a flat top-2, build a tiny diversified agenda slice:
- best urgent survival goal,
- best current commitment/obligation,
- best feasible background economic goal,
- best information-gathering fallback when confidence is low.

Run cheap feasibility probes across that slice before committing full search budget.

---

### 1.8 Fixed emission order should not have semantic authority
**Alignment:** FND-1, FND-20, FND-28

A fixed ordered chain of `emit_*` functions is fine as an implementation detail.
It should **not** be the thing that decides which provenance “wins” when offers collide.

**Fix:** move candidate generation to an emitter registry with explicit arbitration after collection.

Flow:
1. all emitters produce `GoalOffer`s,
2. offers are grouped by opportunity,
3. arbitration rules combine or suppress them,
4. then ranking runs.

No goal should win because its emitter happened to run earlier.

---

### 1.9 The planner needs a proper pending-goal layer, not just per-tick ranking
**Alignment:** FND-20, FND-21

A world like this needs:
- committed goals,
- pending goals,
- suspended goals,
- abandoned goals,
- exhausted goals.

That is different from “rank everything fresh every tick”.

**Fix:** add an explicit agenda manager.

~~~text
AgendaState {
  committed: Option<GoalCommitment>,
  pending: BTreeMap<GoalKey, PendingGoalState>,
  suspended: BTreeMap<GoalKey, SuspendedGoalState>,
  exhausted: BTreeMap<GoalKey, ExhaustedGoalState>,
}
~~~

Each pending/suspended entry should store:
- origin,
- freshness,
- why it is not active now,
- what would revive it,
- what would kill it permanently.

---

### 1.10 Static search budgets are too blunt
**Alignment:** FND-20, FND-22

Per-agent authored defaults are good.
But static beam width, static top-k, and static horizon on every decision are crude.

Critical danger, quiet loaf-buying, and a messy multi-party investigation should not spend the same shape of thought.

**Fix:** keep deterministic authored profiles, but add runtime modulation from concrete state:
- urgency,
- uncertainty,
- crowding/contention,
- repeated recent failure,
- distance,
- institutional responsibility,
- social stakes.

Knobs to modulate:
- `max_candidates_to_plan`
- `beam_width`
- `max_node_expansions`
- `snapshot_travel_horizon`
- `preferred_operator_boost`

No runtime randomness. No drama dials. Pure state-driven metareasoning.

---

### 1.11 The travel layer needs a hard epistemic fence
**Alignment:** FND-7, FND-14, FND-27

`shortest_travel_ticks` living on `PlanningSnapshot` is dangerous even if current code never reads it.
That is too close to a future leak.

**Fix:**
- remove authoritative distance data from planner-visible snapshot types,
- or put it behind a type boundary unavailable to AI planning code,
- or generate planner-side travel tables only from perceived/believed route state.

Also add a regression test:
- remove authoritative matrix from build,
- AI decisions must not change.

Treat this as mandatory hardening.

---

### 1.12 Optional deep traces are fine; optional decision causality is not
**Alignment:** FND-29, FND-29A

You do not need every search frontier node in the authoritative history.
You **do** need enough always-on reasoning history to answer:
- why this goal was committed,
- why that one was rejected,
- why the plan was invalidated,
- what assumption broke,
- why the agent kept or broke a commitment.

**Fix:** log authoritative decision events, while keeping heavy search traces optional.

Mandatory append-only events:
- `GoalOffered`
- `GoalSuppressed`
- `GoalCommitted`
- `GoalSuspended`
- `GoalAbandoned`
- `PlanAdopted`
- `PlanInvalidated`
- `ExpectationMismatch`
- `RepairApplied`
- `ReplanTriggered`
- `BlockerRecorded`
- `QueueJoined/Left/Expired`
- `PromiseIssued/Accepted/Broken`

---

### 1.13 Profile defaults are too easy to abuse into homogeneity
**Alignment:** FND-22

The architecture allows diversity.
That is not the same as actually producing it.

**Fix:** add scenario lints:
- fail if an agent ships with bare default cognitive/utility/belief profile,
- fail if proactive exploration is enabled without an explicit curiosity/information-seeking trait,
- fail if courage/patience/memory fidelity are all inherited unchanged across a whole population archetype.

---

## 2. AI upgrades that will make agents more resilient, realistic, and intelligent

### 2.1 Upgrade GOAP into a belief-aware continual planner
Keep GOAP as the core search.
Wrap it in four additional layers:

1. `AgendaManager`
2. `InformationPlanner`
3. `PolicyExecutor`
4. `LearningUpdater`

That is the right architecture here.

---

### 2.2 Add explicit information-gathering as first-class planning
**Alignment:** FND-7, FND-14, FND-15, FND-16, FND-20

Right now the architecture can explore and investigate, but it does not look like information acquisition is a first-class general-purpose planning move.

That needs to change.

Add information goals such as:
- `VerifyCommodityAtPlace`
- `VerifyPersonAtPlace`
- `VerifyRouteSafety`
- `VerifyOwnershipOrAccess`
- `VerifyInstitutionalFact`
- `ClarifyConflictingReports`
- `InspectContainer`
- `ScoutArea`
- `AskWitness`
- `ConsultRecord`

Also extend `ActionDef` with observation semantics:

~~~text
ObservationModel {
  confirms: Vec<BeliefPredicate>,
  refutes: Vec<BeliefPredicate>,
  may_discover: Vec<ObservationKind>,
  acquisition_mode: AcquisitionMode, // direct perception, testimony, record, trace
}
~~~

Decision rule:
- if confidence is low and the action is expensive, dangerous, or socially consequential, verify first;
- if the claim is cheap to test on the way, test opportunistically;
- if the claim is stale and low-value, defer or ignore.

This is the biggest practical upgrade for scenarios like:
- rumor-driven travel,
- missing-person search,
- robbery discovery,
- route hazard response,
- stale market beliefs.

---

### 2.3 Add limited contingent policy branches
**Alignment:** FND-8, FND-10, FND-16, FND-20, FND-21

Do **not** try to make the whole planner fully contingent on day one.

Add branching only for common local uncertainty classes:
- target absent,
- stock insufficient,
- facility occupied,
- counterparty unwilling,
- witness contradicts,
- route unsafe,
- danger spike,
- confidence collapse.

That turns “act -> fail -> global replan” into:
- “arrive -> inspect -> branch to fallback”.

That is much more robust and much more believable.

---

### 2.4 Add local repair before full replan
**Alignment:** FND-12, FND-20, FND-21

Repair order should be:

1. exact-binding repair,
2. equivalent-binding repair,
3. suffix repair,
4. bail-out/recovery action,
5. full replan.

Supported repair patterns:
- alternate merchant at same place,
- alternate workstation at same place,
- alternate safe route to same destination,
- alternate evidence source,
- alternate queue entry,
- substitute recipe input,
- retreat to proper state and resume.

Keep repair lawful.
No hidden teleport to a “good planner state”.

---

### 2.5 Add bail-out actions and proper-state recovery
**Alignment:** FND-8, FND-12, FND-20, FND-21

Some failures should not throw the agent into a planner-improper limbo.

For actions whose low-level execution can partially apply effects, add explicit bail-outs:
- stop and restow,
- disengage safely,
- leave queue,
- return borrowed tool,
- abandon search pattern cleanly,
- recover posture,
- withdraw from invalid interaction.

Each bail-out maps the executor to a proper planning state.
That makes integrated planning/execution far more robust.

---

### 2.6 Make route choice multi-criteria, not just travel-time-biased
**Alignment:** FND-3, FND-7, FND-10, FND-22

Travel should be chosen from:
- believed time,
- believed danger,
- confidence/freshness of route knowledge,
- congestion/wait expectation,
- permission/toll barriers,
- patrol coverage,
- escort obligation,
- courage and risk tolerance.

This should come from actual route-condition state and local beliefs, not from a hidden “danger score”.

---

### 2.7 Add social coordination artifacts: promises, requests, orders, assignments
**Alignment:** FND-7, FND-18, FND-21, FND-23, FND-25

Multi-agent coordination is weaker than it should be if agents can only react to present state and informal belief-sharing.

Add first-class artifacts/entities for:
- promise,
- request,
- assignment,
- order,
- escort agreement,
- delivery contract,
- patrol tasking.

Each needs:
- issuer,
- recipient,
- place/time of issue,
- content,
- due window,
- status,
- witnesses or medium,
- fulfillment conditions,
- breach consequences.

Then other agents can plan around accepted commitments **lawfully**.

No telepathic coordination.
No central scheduler sharing hidden intentions.

---

### 2.8 Add perspective-aware social reasoning
**Alignment:** FND-14, FND-15, FND-16, FND-20

Before social actions like `ShareBelief`, `Accuse`, `ReportMissing`, `SupportCandidateForOffice`, or `EscortToSafety`, agents should reason about:
- what the target likely knows,
- how stale that knowledge is,
- whether they trust the source,
- whether they have jurisdiction or power,
- whether a public artifact is better than direct speech.

This can be implemented cheaply:
- not full generic epistemic planning,
- just perspective functions over available evidence and known carriers.

That will massively improve social believability.

---

### 2.9 Add concrete learned memories and habits
**Alignment:** FND-22A

Extend learning beyond blocker backoff.

Add explicit per-agent learned state with origin and decay:
- route reliability,
- merchant reliability,
- witness reliability,
- wait expectation by facility,
- hostile hot spots,
- promise reliability of others,
- preferred supplier and route habits,
- recent contradiction sensitivity.

Each update must record:
- what experience produced it,
- when,
- which agent owns it,
- how it decays or is overwritten.

These are fallible summaries, never world truth.

---

### 2.10 Make commitment persistence causal, not just a switch margin
**Alignment:** FND-21, FND-24, FND-25

Current switch margins give stability.
That is good, but not enough.

Agents should sometimes persist because:
- they made a promise,
- they accepted an assignment,
- abandoning will cause trust loss,
- failing an office duty has legal consequences,
- they already waited in a queue,
- breaking escort leaves another agent exposed.

That means commitment should sometimes survive a mild utility challenge because breaking it has explicit downstream cost.

Do this through real world consequences:
- broken promise record,
- trust update,
- accusation,
- reprimand,
- relation damage,
- unpaid contract.

Not through invisible “sticky plan” magic.

---

## 3. Simulation upgrades that better align with FOUNDATIONS

### 3.1 Add richer evidence carriers
**Alignment:** FND-5, FND-15, FND-18, FND-29

If you want investigations, hunts, robberies, false accusations, and stale reports to actually work, you need richer evidence.

Add first-class evidence entities/artifacts such as:
- tracks,
- broken lock,
- disturbed container,
- blood trail,
- dropped cargo cluster,
- footprint trail,
- eyewitness statement,
- patrol report,
- warning notice,
- route hazard marker,
- corpse condition record.

Each needs:
- source event,
- place,
- creation time,
- decay behavior,
- visibility rules,
- interpretation rules,
- provenance.

This is one of the highest-value simulation upgrades you can make.

---

### 3.2 Make route and place condition state explicit
**Alignment:** FND-3, FND-7, FND-12

Perceived travel cost should come from:
- actual blockage,
- bridge condition,
- congestion,
- recent attack evidence,
- patrol activity,
- territorial hostility,
- legal access state,
- observed weather/season effect only if those systems really exist.

Then agents can:
- avoid roads for actual reasons,
- post warnings,
- spread rumors,
- escort caravans through safer routes,
- misjudge safety if their information is stale.

---

### 3.3 Make institution throughput explicit
**Alignment:** FND-6, FND-7, FND-23, FND-25A

If offices matter, model:
- intake queues,
- backlog,
- delegated duties,
- clerk availability,
- guard availability,
- budget/treasury state,
- delay,
- jurisdiction,
- succession latency.

Then agents can reason about:
- whether reporting now is worthwhile,
- whether a bounty will actually be posted,
- whether a case will stall,
- whether an office vacancy causes a real patrol gap.

That is straight out of your FOUNDATIONS.

---

### 3.4 Separate artifact existence, visibility, credibility, legality, and actionability
**Alignment:** FND-25A

Do not treat “artifact exists” as “artifact is currently actionable”.

For bounties, notices, accusations, warnings, warrants, listings:
- existence,
- visibility,
- credibility,
- legality,
- actionability
must be separate.

AI should be able to reason about:
- stale but still visible bounty,
- visible rumor nobody trusts,
- accusation with no jurisdiction,
- warning that persists after the threat moved,
- expired contract still affecting reputation.

---

### 3.5 Make scarce-affordance claims explicit world state everywhere
**Alignment:** FND-8, FND-21, Canonical Scenario E

Any recurring contested affordance needs an explicit world mechanism:
- queue token,
- reservation artifact,
- grant,
- claim,
- lock,
- turnstile,
- ticket,
- lane right-of-way marker.

Do not rely on planner-local shadow state for things other agents need to observe or contest.

---

### 3.6 Add commitment aftermath
**Alignment:** FND-10, FND-21, FND-22A, FND-25

If a promise, order, debt, escort, or queue commitment is broken, the world should carry aftermath:
- trust loss,
- complaint,
- reprimand,
- sanction,
- retaliation,
- vacancy,
- gossip,
- refusal to cooperate later.

Without this, social persistence stays too abstract.

---

## 4. Changes beyond the previous categories that are still worth doing

### 4.1 Make reasoning history authoritative enough to answer “why not that?”
When committing a plan, log:
- chosen goal,
- top rejected alternatives,
- reason classes for rejection,
- belief sources that mattered,
- blocker state,
- commitment state.

You do not need full frontier dumps for this.
You do need enough to answer:
- why this route,
- why not the other witness,
- why no bounty,
- why abandon escort,
- why keep stale pursuit one more tick.

---

### 4.2 Add scenario lints and architecture lints
Compile-time or scenario-load-time checks should fail if:
- an agent has an all-default profile,
- proactive exploration has no explicit curiosity/information trait,
- an action with socially meaningful identity uses permissive binding strictness,
- a social artifact lacks lifecycle states,
- a contested affordance lacks explicit claim/queue/reservation entity type,
- planner-visible snapshot types expose authoritative world-only helpers.

---

### 4.3 Build falsification harnesses against the canonical scenario classes
Automate FOUNDATIONS scenarios A-H.

For each run, check:
- no knowledge without carrier,
- no item disappearance without source/sink path,
- no bounty without issuer + place + funds + record,
- no interruption without lawful observation/report,
- no blocked opportunity without explicit contention state,
- no institutional action without office/jurisdiction path,
- no instantaneous belief correction from ground truth,
- no cleanup deleting the only explanation.

---

### 4.4 Add metrics that matter
Track at least:
- goal churn rate,
- replan rate,
- local repair rate,
- unknown-discrepancy ratio,
- plan branch count,
- verification-before-high-stakes-action rate,
- stale-pursuit failure rate,
- commitment break rate,
- blocker recurrence rate,
- search budget waste on impossible goals,
- candidate count before/after arbitration,
- successor count before/after pruning,
- route belief error vs observed route state.

---

### 4.5 Do not “solve” this with LLM NPC brains
That would be the wrong move for this project.

Reject:
- omniscient blackboards,
- drama utility terms,
- hidden reservation by intention,
- narrative trigger systems,
- central controller telling institutions what to do,
- non-deterministic text-model decision authority in the live sim.

That would break your foundations and make debugging miserable.

---

## 5. Recommended refactor targets

### 5.1 `GroundedGoal` -> `GoalOffer`
Add:
- confidence summary,
- freshness summary,
- obligation source,
- commitment impact if ignored,
- required information gaps,
- competing claimant estimate,
- invalidators,
- learned expectation references.

---

### 5.2 `RankedGoal` -> `AgendaEntry`
Lifecycle:
- offered,
- pending,
- committed,
- suspended,
- exhausted,
- abandoned.

---

### 5.3 `PlannedPlan` -> `PolicyPlan`
Add:
- dependency structure,
- guards,
- expectations,
- local repairs,
- branch edges,
- bail-out exits.

---

### 5.4 `BlockedIntentMemory` -> split responsibilities
Split into:
- `DiscrepancyMemory`
- `BlockerMemory`
- `RepairMemory`
- `LearnedOpportunityMemory`

Do not use one structure for everything that went wrong.

---

### 5.5 `RuntimeBeliefView` -> uncertainty-aware planner API
Replace raw planner-facing query returns where needed with:
- `BeliefValue<T>`
- `BeliefSet<T>`
- quantity intervals or min/max confidence summaries,
- provenance-bearing route condition views.

---

### 5.6 Unified legality path
One validation/binding path for:
- affordance enumeration,
- revalidation,
- dispatch,
- exact-target recovery.

No side door.

---

## 6. Implementation order

### Phase 1: mandatory cleanup
1. Hard-fence authoritative travel helpers out of planner-visible types.
2. Add per-action binding strictness.
3. Unify action validation/binding.
4. Replace `Unknown` blocker with typed discrepancy classes.
5. Make candidate arbitration independent of emitter order.
6. Add authoritative decision-history events.
7. Add scenario lints for profile homogeneity and proactive exploration grounding.

### Phase 2: highest ROI AI improvements
1. Add agenda manager with pending/suspended goals.
2. Add uncertainty-aware belief objects to planner API.
3. Add information-gathering goals and action observation models.
4. Add cheap feasibility probes and portfolio planning.
5. Add step guards and expectation monitoring.

### Phase 3: resilience upgrade
1. Introduce limited `PolicyPlan` branching.
2. Add local repair before full replan.
3. Add bail-out/proper-state recovery actions.
4. Add learned route/trader/witness/facility expectations.
5. Add promise/request/order artifacts.

### Phase 4: simulation deepening
1. Evidence carriers.
2. Route/place condition state.
3. Institution throughput and delegation.
4. Artifact lifecycle separation.
5. Commitment aftermath and sanction paths.
6. Expanded contention artifacts.

---

## 7. Acceptance criteria

A proposed change is acceptable only if all of these stay true:

- decisions remain explainable as belief + motive + commitment,
- no plan depends on hidden world truth,
- no coordination depends on invisible scheduler state,
- all retries and repairs are lawful actions or lawful state transitions,
- deleting derived caches changes performance only, never meaning,
- interruption and recovery leave inspectable aftermath,
- canonical scenario classes emerge without dedicated scenario scripting.

---

## 8. Bottom line

The architecture is not wrong.
It is just still too **linear**, too **crisp**, and too **forgetful about uncertainty** for the world standard your FOUNDATIONS are demanding.

The right upgrade is:

- **not** “more utility weights,”
- **not** “bigger GOAP search,”
- **not** “LLM NPCs,”
- **not** “more scripted fallbacks.”

It is a clean architectural shift to:

- belief-aware planner inputs,
- information-seeking as a real option,
- limited contingent execution,
- local repair and proper-state recovery,
- explicit social commitments,
- richer evidence carriers,
- and authoritative reasoning history.

That will make the agents tougher across scenarios, more realistic under ignorance, more socially legible, and much closer to the causality-first simulation your repository says it wants.