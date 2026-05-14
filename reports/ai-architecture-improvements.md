I’m grounding this in **FOUNDATIONS.md** and the current **GOAP architecture report**. The short verdict: **do not replace the current GOAP stack**. It is already pointed in the right direction. The upgrade path should be a **BDI-shaped deliberation shell over your existing GOAP planner**, with **data-driven goal schemas**, **HTN-style decomposition as lawful search control**, **utility/portfolio triage for hundreds of motives**, **explicit coordination artifacts**, and **much stronger diagnostics and belief-boundary enforcement**. That preserves Worldwake’s causality-first standard while making agents more resilient, realistic, and scalable.

## **1. What the research says, translated for Worldwake**

The original game-GOAP lesson from *F.E.A.R.* is still relevant: real-time planning gives NPCs adaptability, lets goals/actions be modular and reusable, and avoids hand-authored action sequences, but it spends CPU and affects the entire agent architecture rather than just the planner loop. Orkin explicitly frames GOAP as runtime search through action sequences instead of script/FSM selection, and calls out distributed processing and caching as architectural concerns. That maps almost exactly onto your current constraints: many agents, many possible goals, bounded CPU, and a strong need for explainability.

Classical planning research supports your current tactical core. FF-style planning uses forward state-space search guided by relaxed-plan heuristics that ignore delete effects, and your planner already uses the same broad idea through FF delete-relaxation, landmarks, and preferred operators. Your next search upgrade should look more like **multi-heuristic best-first planning** than like a wholesale rewrite: Fast Downward’s design combined causal-graph heuristics, preferred operators, deferred evaluation, multi-heuristic best-first search, and efficient successor data structures, which is directly relevant to your current single combined heuristic and preferred-frontier setup.

HTN research and game implementations point to the right way to handle long, structured tasks: use human-authored decomposition knowledge to reduce search cost, but keep leaves grounded in ordinary actions. SHOP2 is a canonical HTN planner, and game-AI HTN work emphasizes that decomposition makes planning cheaper by encoding reusable domain knowledge. For Worldwake, HTN methods should never be “story beats.” They should be **lawful pursuit patterns**: “how a guard investigates,” “how a hunter fulfills a bounty,” “how a merchant restocks,” “how an office handles succession.”

BDI is the best conceptual wrapper for your existing architecture. BDI separates beliefs, desires, and intentions; it is explicitly designed around resource-bounded practical reasoning. This fits FOUNDATIONS almost perfectly: agents act from beliefs, motives, habits, commitments, and revisable intentions, not omniscient truth or behavior-tree nodes. FOUNDATIONS already allows GOAP, utility, BDI, HTN, or hybrids, as long as decisions remain explainable as “this agent chose this because they believed that and cared about this.”

Utility systems are useful, but only at the **motive triage** layer. The Sims-style utility approach grouped motives and selected within the most urgent bucket, so a starving Sim would not watch TV unless nothing could solve hunger. That is the right pattern for Worldwake’s future “hundreds of possible goals” problem: not one global soup of scores, but **portfolio slots** grounded in concrete needs, obligations, risks, records, and opportunities.

For multi-agent realism, borrow from coordination literature, not from omniscient managers. Contract Net Protocol distributes tasks through negotiation among agents, and market-based robot coordination lets agents locally evaluate tasks and bid based on their own costs/resources. For physical contention, cooperative pathfinding uses reservation tables to avoid route conflicts in space-time. In Worldwake terms: queues, bids, grants, route claims, work orders, patrol assignments, and contracts must be **world artifacts**, not invisible planner locks.

Generative-agent and LLM work is useful, but dangerous for your live authority path. Generative Agents showed that memory, reflection, and planning can improve believable behavior in a simulated town. Voyager showed that an ever-growing library of executable skills can support open-ended Minecraft play. The safe Worldwake translation is: use LLMs for **offline authoring, test generation, trace analysis, and schema suggestions**, not live authoritative decision-making unless every output is deterministic, validated against lawful affordances, replayable, and incapable of inventing facts.

## **2. Current architecture: strong foundation, clear bottlenecks**

The existing architecture is much better than a typical game-AI stack. It has deterministic authoritative state, no floats in authority, seeded RNG, append-only causal history, local affordances, a belief-view wall, two-tier GOAP search, revalidation, interruption, blocker/discrepancy memory, and repair. The current per-agent pipeline is belief view → ranking → candidate generation → affordance enumeration → strategic/tactical search → revalidation → dispatch → failure handling.

The best parts are worth preserving:

The `RuntimeBeliefView` trait wall is central. Planner reads go through belief accessors, with only the FND-14A same-tick local physical observation exception. The plan search stack already uses a strategic itinerary plus tactical best-first search with FF relaxation, landmarks, and preferred operators. Current plan revalidation and failure handling are also architecturally correct for a dynamic world: plans are checked before dispatch, failures become blockers/discrepancies, and failed actions produce events that flow back through perception rather than silently rewriting beliefs.

But the architecture has predictable stress points once you move from “dozens of authored goal variants” to “hundreds of possible motives emitted by dense world state.”

The biggest risks are:

1. **Top-2 candidate planning is too narrow.** `max_candidates_to_plan = 2` gives each agent very little breadth when dozens or hundreds of plausible goals exist. The report itself flags this as small: the agent gets serious budget for only two goals before falling into blocker/failure behavior.  
2. **Goal emission is still too hand-shaped.** The current `emit_*` gate family is acceptable now, but it will become brittle as more institutions, artifacts, resources, social states, and opportunities exist. You need a schema/index-based motive system, not an ever-growing pile of emitter functions.  
3. **Strategic planning is shallow.** The strategic budget defaults to `2 × max_prerequisite_locations`, i.e. six expansions by default, and the report notes this may collapse longer acquisition chains into tactical-only thrashing.  
4. **The tactical plan depth default is too low for real economic/social chains.** `max_plan_depth = 8` is already called out as small for crafting, travel, setup, delivery, and production.  
5. **Blocker matching is too exact.** The current `BlockerKey` can scope by goal/place/target/action, but the report notes cross-goal blockers such as “this place is dangerous for any goal” need a separate mechanism.  
6. **No aggregate diagnostics exist.** Rich per-tick traces exist, but there is no scenario-level dashboard for candidate counts, search exhaustion, beam truncation, repair success, blocker churn, or plan-depth pressure. The report explicitly says the collection infrastructure exists but the aggregator is missing.  
7. **FND-14A is a static-safety risk.** The report calls the same-tick local observation exception “the single biggest discipline risk” because the boundary is enforced by review/tests rather than by a static type split.  
8. **Agent diversity exists but is not generated.** Profiles are per-agent, but all defaults are identical unless scenarios explicitly override them. That means homogeneous herds unless scenario authors do constant work.  
9. **Per-tick snapshots will scale badly.** The report notes fresh `PlanningSnapshot` construction each planning cycle, with Floyd-Warshall-style all-pairs distance precomputation and no incremental snapshot.

My strong recommendation: **treat the current GOAP engine as the tactical executor/planner, not as the whole mind.** Build a deliberation architecture around it.

## **3. Recommended target architecture**

The target architecture should look like this:

Perception / testimony / records / local observation

       ↓

Belief store + provenance + contradiction + source reliability

       ↓

Motive compiler: concrete world evidence → MotiveRecords

       ↓

Goal portfolio triage: survival / safety / duties / commitments / economy / social / exploration

       ↓

Intention manager: adopt / continue / suspend / resume / abandon

       ↓

GoalSchema + HTN method selection: lawful decompositions only

       ↓

GOAP tactical planner: ActionDef leaves, local affordances, queues/reservations, partial plans

       ↓

Revalidation / interruption / repair / failure memory

       ↓

World actions → causal event log → perception again

This is not a replacement. It is a layering change.

## **4. Upgrade A — Add a BDI-shaped deliberation shell**

You already have most of the ingredients, but they are distributed across ranking, candidate generation, intention frames, blockers, discrepancies, and plans. Make the mental model explicit:

struct MotiveRecord {

   id: MotiveId,

   owner: EntityId,

   kind: MotiveKind,

   source_event: Option<EventId>,

   source_entities: BTreeSet<EntityId>,

   source_places: BTreeSet<EntityId>,

   acquired_tick: Tick,

   expires_tick: Option<Tick>,

   confidence: Permille,

   urgency: UrgencyClass,

   obligation_source: Option<EntityId>,

   invalidators: Vec<Invalidator>,

   information_gaps: Vec<BeliefClaimKey>,

   learned_expectation_refs: Vec<ExpectationId>,

}

enum MotiveKind {

   NeedPressure,

   ThreatEvidence,

   InjuryOrCareDuty,

   Obligation,

   ContractOffer,

   Routine,

   InstitutionalDuty,

   EconomicOpportunity,

   SocialRequest,

   InformationGap,

   Discrepancy,

   OpportunisticGain,

}

struct IntentionFrame {

   goal_key: GoalKey,

   motive_refs: Vec<MotiveId>,

   adopted_tick: Tick,

   last_reconsidered_tick: Tick,

   plan_segment: Option<PlanSegmentId>,

   assumptions: Vec<PlanAssumption>,

   resume_conditions: Vec<ResumeCondition>,

   abandon_conditions: Vec<AbandonCondition>,

   explicit_claims: Vec<EntityId>, // reservations, queue tickets, contracts, grants

   suspend_reason: Option<SuspendReason>,

   causal_links: Vec<EventId>,

}

The key distinction:

* **Desires/motives** are many, cheap, persistent, and evidence-backed.  
* **Intentions** are few, stable, revisable commitments.  
* **Plans** are temporary search products under an intention.  
* **Reservations/claims** exist only when there is a world artifact.

This aligns directly with FOUNDATIONS: intentions are revisable commitments, but intent is not entitlement; selecting a plan must not reserve a workstation, bread, corpse, patient, or road unless a concrete reservation/queue/contract exists.

## **5. Upgrade B — Replace emitter sprawl with data-driven `GoalSchema`**

The current `GoalKind` enum is fine as a stable discriminant layer, but goal semantics should move into a registry. Otherwise hundreds of goals will create hundreds of brittle branches.

Add:

struct GoalSchema {

   kind_discriminant: GoalKindDiscriminant,

   candidate_extractors: Vec<CandidateExtractorId>,

   satisfaction_predicate: SatisfactionPredicateId,

   relevant_op_families: BTreeSet<PlannerOpKind>,

   methods: Vec<MethodSchemaId>, // optional HTN-style decompositions

   invalidator_templates: Vec<InvalidatorTemplate>,

   expectation_templates: Vec<ExpectationTemplateSpec>,

   information_gap_templates: Vec<InformationGapTemplate>,

   ranking_features: Vec<RankingFeatureId>,

   explanation_template: ExplanationTemplate,

   causal_hook_spec: CausalHookSpec,

   validation_spec: GoalValidationSpec,

}

Every new goal should declare:

* what evidence can create it,  
* what belief facts satisfy it,  
* what action families may pursue it,  
* what invalidates it,  
* what records/evidence it needs,  
* what downstream consequences it can create,  
* what traces/tests prove it behaves correctly.

This makes FND-30 enforceable as code/data rather than prose. FOUNDATIONS requires every new system to declare causal hooks, information flow, lifecycle, contention, failure states, learning updates, caches, and validation.

Do not keep adding bespoke `emit_*` functions indefinitely. The right future shape is:

BeliefDelta / EventDelta / MemoryDelta

       ↓

CandidateExtractor registry

       ↓

MotiveRecord / GoalOffer

       ↓

Portfolio ranking

Candidate extraction should become **delta-driven**. When a new notice is perceived, only notice-related extractors run. When hunger crosses a threshold, self-care extractors run. When an accusation record arrives, justice extractors run. This is the first major scalability win.

## **6. Upgrade C — Replace “top-2 goals” with a goal portfolio**

`max_candidates_to_plan = 2` is the wrong shape for a dense emergent world. Raising it to 8 is a temporary bandage; the proper fix is **portfolio selection**.

Recommended slots:

enum PortfolioSlot {

   ActiveIntention,

   Survival,

   ImmediateSafety,

   InjuryOrCare,

   ExplicitObligation,

   InstitutionalDuty,

   EconomicMaintenance,

   SocialEpistemic,

   OpportunisticLocal,

   ExplorationOrInformation,

}

Each tick, the agent should produce a bounded planning portfolio:

1 active intention, unless invalidated or emergency-suppressed

+ top survival/safety motives above threshold

+ top explicit obligation/duty motives

+ top economic maintenance motive

+ top social/epistemic motive if budget remains

+ top opportunistic local motive if cheap/local

This gives breadth without letting weak opportunities drown urgent needs. It also creates more believable behavior: agents continue commitments, handle hunger, react to danger, fulfill duties, and opportunistically exploit local affordances without becoming random.

A good default planning portfolio might be:

Emergency mode:

 ActiveIntention only if safety-compatible

 + 2 safety/survival candidates

 + 1 escape/help candidate

Normal mode:

 ActiveIntention

 + 1 survival/self-care

 + 1 obligation/duty

 + 1 economic/work

 + 1 social/epistemic

 + 1 opportunistic/local

Idle/low-pressure mode:

 ActiveIntention if any

 + 1 routine

 + 1 economic

 + 1 social

 + 1 exploration/information

 + 1 opportunistic

This is still resource-bounded. It is just not artificially blind. Utility scores remain legal because they are agent-local summaries derived from concrete motive records, not authoritative world truth. FOUNDATIONS permits bounded heuristics derived from accessible belief state, as long as they are explainable.

## **7. Upgrade D — Add HTN methods as lawful decomposition, not scripts**

Your current two-tier planner is good, but it is not enough for long institutional/social/economic behavior. Add HTN-style methods above GOAP for goals where naive search becomes expensive.

Example:

struct MethodSchema {

   id: MethodId,

   goal_kind: GoalKindDiscriminant,

   preconditions: Vec<BeliefPrecondition>,

   subgoals: Vec<SubgoalTemplate>,

   expected_artifacts: Vec<ArtifactTemplate>,

   required_claims: Vec<ClaimRequirement>,

   failure_modes: Vec<FailureModeTemplate>,

   explanation_template: ExplanationTemplate,

}

Example `FulfillBounty` methods:

Method: Direct hunt

 Preconditions:

   - agent believes bounty exists

   - agent believes target last-seen place or territory

   - agent believes reward/proof requirements

 Subgoals:

   - acquire required supplies/tools if missing

   - travel to last-known place

   - search/track target

   - confront/capture/kill as terms require

   - collect proof artifact

   - return to issuer/jurisdiction

   - submit proof and claim payment

Method: Social investigation

 Preconditions:

   - target location uncertain

   - witnesses or records believed available

 Subgoals:

   - ask witness / inspect notice / consult ledger

   - travel to updated lead

   - search/track

   - continue as direct hunt

Method: Group hunt

 Preconditions:

   - target believed dangerous

   - allies or bounty office available

 Subgoals:

   - recruit / accept assignment / form contract

   - synchronize at staging place

   - travel and confront

Every leaf remains an ordinary `ActionDef`. Every artifact remains world state. Every belief comes through a carrier. The HTN method is just **search control and reusable craft knowledge**, which FOUNDATIONS explicitly allows when it expresses how an agent pursues a world condition under beliefs rather than how a story beat should happen.

This should be applied first to:

* `FulfillBounty`  
* `InvestigateViolation`  
* `Accuse`  
* `PunishAccused`  
* `ProduceCommodity`  
* `RestockCommodity`  
* `MoveCargo`  
* `EscortToSafety`  
* `SearchForMissing`  
* `ClaimOffice`  
* `SupportCandidateForOffice`  
* future caravan, patrol, construction, repair, inheritance, and diplomacy goals

## **8. Upgrade E — Make information barriers first-class plan outcomes**

The current system already has `required_information_gaps` and `SocialQuery`. Expand this aggressively.

Many realistic agents should not fail because they cannot directly plan to the final goal. They should plan to **learn enough to continue**.

Add terminal kinds:

enum PlanTerminalKind {

   GoalSatisfied,

   ProgressBarrier,

   InformationBarrier,

   CoordinationBarrier,

   ResourceBarrier,

   JurisdictionBarrier,

   SafetyBarrier,

}

Examples:

Hungry agent:

 does not know seller stock

 → plan to visit market / inspect stall / ask merchant

Bounty hunter:

 knows bounty but not target location

 → plan to ask witness / read record / inspect tracks

Magistrate:

 has accusation but not proof

 → plan to summon witness / inspect ledger / dispatch investigator

Merchant:

 knows shortage but not supplier

 → plan to consult trade ledger / ask caravan master / inspect notice board

This is crucial for realism. Real agents do not merely choose from known executable plans; they perform epistemic work. Continual-planning research frames this well: realistic environments are dynamic and partially observable, and agents should plan, sense, revise, and continue rather than expect complete all-contingency plans upfront.

Worldwake already has the philosophical basis: ignorance, stale belief, contradiction, surprise, memory, evidence, and records are first-class. The planner should exploit that instead of treating missing knowledge as a planning failure.

## **9. Upgrade F — Add explicit multi-agent coordination artifacts**

Current facility contention is a good start, but future scenarios with many agents need a richer coordination layer. Do **not** add an omniscient job manager. Add concrete artifacts.

Recommended artifacts:

enum CoordinationArtifactKind {

   QueueTicket,

   ReservationGrant,

   WorkOrder,

   Bid,

   ContractAward,

   AssignmentContract,

   PatrolRosterEntry,

   RouteClaim,

   ConvoyMembership,

   MeetingCommitment,

   CompletionProof,

   RevocationNotice,

   FailedDeliveryReport,

}

These artifacts should have:

struct CoordinationArtifact {

   id: EntityId,

   kind: CoordinationArtifactKind,

   issuer: EntityId,

   holder: Option<EntityId>,

   target: Option<EntityId>,

   place: Option<EntityId>,

   valid_from: Tick,

   expires_at: Option<Tick>,

   terms: Terms,

   visibility: VisibilitySpec,

   legal_effects: Vec<LegalEffect>,

   invalidators: Vec<Invalidator>,

   lifecycle_state: ArtifactLifecycleState,

}

Use cases:

* A town office posts a patrol work order.  
* Guards bid/accept based on their beliefs, fatigue, fear, equipment, loyalty.  
* The office awards a patrol assignment.  
* The assignment creates a schedule artifact.  
* Failure to appear creates a record.  
* Opportunistic bandits notice patrol gaps only through observation, reports, or routine mismatch.

For route and facility contention, use reservations as visible or inferable world state. Cooperative pathfinding’s reservation-table idea is useful, but in Worldwake the reservation cannot be hidden engine magic. It must become a queue ticket, route claim, crossing grant, convoy schedule, or other inspectable artifact where agents can observe, contest, ignore, or violate it.

This will directly support canonical scenarios like competing claimants, office vacancy → patrol gap, delayed caravan arrivals, and bounty fulfillment.

## **10. Upgrade G — Improve trust, source reliability, and contradictory belief handling**

Your belief system is already ahead of most game AI. The next step is making source reliability more consequential.

Add:

struct SourceReliabilityMemory {

   source: EntityId,

   topic_scope: TopicScope,

   direct_confirmations: u32,

   direct_refutations: u32,

   stale_claims: u32,

   contradicted_claims: u32,

   last_updated_tick: Tick,

   expertise_tags: BTreeSet<ExpertiseTag>,

   trust: Permille,

   decay_policy: DecayPolicy,

   provenance_events: Vec<EventId>,

}

Use it in:

* candidate ranking,  
* testimony acceptance,  
* accusation confidence,  
* rumor propagation,  
* willingness to ask a source again,  
* institutional judgment,  
* willingness to travel on stale information.

A trust-on-beliefs model should consider source, time, expertise, outdatedness, and conflicting claims; that is exactly the kind of provenance-driven belief handling Worldwake needs.

Concrete examples:

A gullible agent:

 discounts stale claims slowly and trusts social rumors.

An empiricist:

 heavily prefers direct observation and physical traces.

An officialist:

 trusts ledgers, offices, warrants, and posted records.

A partisan:

 trusts faction allies and discounts rivals.

A traumatized survivor:

 overweights threat reports from similar prior events.

This is not flavor. It changes action selection, reporting, accusation, avoidance, trade, and travel. It also supports false rumor → wrongful accusation → contested evidence → correction/miscarriage.

## **11. Upgrade H — Add concrete habits and learned method preferences**

FOUNDATIONS explicitly allows learning only through concrete state: memory update, trust revision, habit reinforcement, blocked-intent record, route preference, institutional doctrine, and similar inspectable structures.

Add:

struct HabitMemory {

   owner: EntityId,

   trigger: HabitTrigger,

   preferred_goal_schema: Option<GoalSchemaId>,

   preferred_method: Option<MethodId>,

   preferred_place: Option<EntityId>,

   strength: Permille,

   learned_from: EventId,

   acquired_tick: Tick,

   last_reinforced_tick: Tick,

   decay: DecayPolicy,

}

struct RoutePreference {

   owner: EntityId,

   route_segment: RouteSegment,

   expectation: RouteExpectation,

   learned_from_events: Vec<EventId>,

   trust: Permille,

   last_confirmed_tick: Tick,

   decay: DecayPolicy,

}

struct SellerReliabilityMemory {

   owner: EntityId,

   seller: EntityId,

   commodity: CommodityKind,

   fulfilled_count: u32,

   failed_count: u32,

   last_price_belief: Option<PriceBelief>,

   learned_from_events: Vec<EventId>,

}

Learning should affect:

* which method the agent tries first,  
* which seller they visit,  
* which route they prefer,  
* when they ask witnesses,  
* when they retry after failure,  
* when they avoid danger,  
* when they abandon an intention.

This gives agents resilience. They stop repeating brittle mistakes without requiring global adaptation or drama dials.

## **12. Upgrade I — Fix search scalability with incremental snapshots and heuristic portfolios**

The current `PlanningSnapshot` rebuild pattern is clean but will become expensive. Do not optimize by compressing causality. Optimize by caching derived belief views and invalidating them correctly.

Recommended changes:

1. **Version belief stores and topology knowledge.**

struct BeliefStoreVersion(u64);

struct BelievedTopologyVersion(u64);

struct LocalOpportunityIndexVersion(u64);

2. **Build an incremental `PlanningIndex`.**

struct PlanningIndex {

   actor: EntityId,

   belief_version: BeliefStoreVersion,

   known_places: BTreeSet<EntityId>,

   local_entities_by_place: BTreeMap<EntityId, Vec<EntityId>>,

   known_sellers_by_commodity: BTreeMap<CommodityKind, Vec<EntityId>>,

   known_sources_by_commodity: BTreeMap<CommodityKind, Vec<EntityId>>,

   known_records_by_topic: BTreeMap<TopicScope, Vec<EntityId>>,

   known_facilities_by_tag: BTreeMap<WorkstationTag, Vec<EntityId>>,

}

3. **Replace all-pairs distance with bounded demand search where possible.**

For most planning, the agent needs distances from current place to candidate anchors, not all-pairs distances among every believed place. Use deterministic bounded Dijkstra/A* over the believed place graph and cache per `(actor, belief_topology_version, source_place, horizon)`.

4. **Add per-goal heuristic policy.**

Right now `use_ff_heuristic` is a single per-agent boolean. The report notes there is no middle ground. Make it per goal/method:

struct HeuristicPolicy {

   use_spatial: bool,

   use_ff: bool,

   use_landmarks: bool,

   use_resource_bottleneck: bool,

   use_method_progress: bool,

   deferred_eval: bool,

   preferred_operator_policy: PreferredOperatorPolicy,

}

5. **Use deterministic multi-queue best-first search.**

Instead of one `max(spatial, ff, landmark)` number, use multiple deterministic queues:

Queue A: low travel cost

Queue B: FF/helpful action progress

Queue C: landmark progress

Queue D: resource bottleneck progress

Queue E: method-stage progress

Interleave them with a deterministic schedule based on `CognitiveProfile`. This keeps agents diverse and avoids overcommitting to one flawed heuristic.

## **13. Upgrade J — Make partial plans a first-class object**

The current `ProgressBarrier` behavior is the right instinct but should be promoted.

Add:

struct PartialPlanSegment {

   id: PlanSegmentId,

   owner: EntityId,

   goal_key: GoalKey,

   steps: Vec<PlannedStep>,

   terminal_barrier: PlanTerminalKind,

   barrier_fact: BarrierFact,

   resume_conditions: Vec<ResumeCondition>,

   abandoned_if: Vec<Invalidator>,

   created_tick: Tick,

   causal_links: Vec<EventId>,

}

Examples:

Production:

 acquire wood → acquire ore → stop at resource barrier “needs furnace access”

Investigation:

 inspect crime scene → ask witness → stop at information barrier “needs suspect location”

Trade:

 travel to market → inspect seller stock → stop at resource barrier “seller empty”

Bounty:

 read notice → travel to last seen → track lost → stop at information barrier “needs new lead”

This makes agents look much more resourceful. They can advance a difficult goal partway, gather new information, suspend it, do something else, and resume later.

## **14. Upgrade K — Strengthen FND-14/FND-14A with static trait separation**

This is a must-fix. The report calls FND-14A the biggest discipline risk.

Split the current belief view into stricter traits:

trait BelievedWorldView {

   fn believed_entity_state(...) -> BeliefRead<BelievedEntityState>;

   fn believed_location(...) -> BeliefRead<Option<EntityId>>;

   fn believed_commodity(...) -> BeliefRead<Quantity>;

}

trait LocalPhysicalObservationView {

   fn colocated_physical_entities(...) -> ObservedRead<Vec<EntityId>>;

   fn observed_item_lot_quantity(...) -> ObservedRead<Quantity>;

   fn observed_workstation_tag(...) -> ObservedRead<Option<WorkstationTag>>;

}

trait BelievedSocialView {

   fn believed_owner(...) -> BeliefRead<Option<EntityId>>;

   fn believed_access_right(...) -> BeliefRead<AccessRight>;

   fn believed_jurisdiction(...) -> BeliefRead<Option<EntityId>>;

}

trait DebugWorldView {

   // unavailable to live worldwake-ai

}

Then enforce:

* `worldwake-ai` live planner cannot import authoritative world state.  
* Co-located physical observation accessors cannot expose ownership, rights, jurisdiction, debt, office legitimacy, or institutional claims.  
* Every belief read should be able to return provenance/freshness/confidence.  
* Debug views should be `#[cfg(debug_assertions)]` or tooling-only and never part of live decision code.

Add regression tests:

Agent stands beside chest but lacks ownership belief:

 may observe chest and contents

 may not infer owner

 may not infer theft legality

 may not infer jurisdiction

Agent sees office building but lacks office-holder record:

 may observe building

 may not know current magistrate

Agent sees item in another agent’s hand:

 may observe possession

 may not know ownership or access rights

This is the kind of architectural fix FOUNDATIONS demands: not “be careful,” but “make the illegal path unavailable.”

## **15. Upgrade L — Add environmental blocker patterns**

Current blocker scoping is too exact for dense emergence. Add pattern blockers that can affect multiple goals while still being belief-local and agent-local.

enum BlockerScope {

   Exact(BlockerKey),

   GoalFamily(GoalFamily),

   Place(EntityId),

   RouteSegment { from: EntityId, to: EntityId },

   Facility(EntityId),

   Counterparty(EntityId),

   ResourceAtPlace { place: EntityId, commodity: CommodityKind },

   LegalAuthority { office: EntityId, jurisdiction: EntityId },

}

struct FailureMemory {

   owner: EntityId,

   scope: BlockerScope,

   fact: BlockingFact,

   observed_tick: Tick,

   expires_tick: Tick,

   clearing_condition: BlockerClearingCondition,

   baseline_snapshot: Option<ClearingBaseline>,

   source_event: EventId,

}

Examples:

RouteSegment blocker:

 affects travel, trade, patrol, escort, bounty pursuit.

Place danger blocker:

 affects market trip, social visit, hauling, sleep-at-inn.

Counterparty blocker:

 affects buying, selling, asking, contract negotiation.

Facility blocker:

 affects crafting, washing, treatment, production.

This will reduce stupid retries and make failure learning more general without inventing global truth.

## **16. Upgrade M — Add default seeded personality/profile generation**

Right now scenarios can diversify agents, but the engine does not. That is too weak for FND-22.

Add explicit profile templates, not random noise:

enum CognitiveArchetype {

   Cautious,

   Bold,

   Stubborn,

   Methodical,

   Opportunistic,

   Sociable,

   Skeptical,

   Dutiful,

   Greedy,

   Fearful,

}

At spawn:

AgentCreated

 → PersonalityAssigned { archetype, seed, profile_values, source }

Concrete variations:

* switch margins,  
* planning breadth,  
* plan depth,  
* guard confidence ceiling,  
* source trust priors,  
* danger tolerance,  
* backoff TTLs,  
* willingness to ask,  
* willingness to detour,  
* repair budget,  
* method preferences.

This remains deterministic, replayable, and inspectable. The assigned profile is world/agent state, not invisible AI tuning.

## **17. Upgrade N — Build the missing aggregate diagnostics now**

Do this before tuning anything. Without aggregate metrics, every planning change will be guesswork.

Create a scenario-runner report that aggregates existing traces:

Goal pressure:

 candidates emitted by schema/slot

 candidates suppressed by reason

 top-K not planned

 active intention continuation rate

Planning:

 plan attempts per tick

 budget exhaustion rate

 frontier exhaustion rate

 beam truncation ratio

 average / p95 plan depth

 GoalSatisfied vs ProgressBarrier vs InformationBarrier

 heuristic helpful-action hit rate

Revalidation / repair:

 invalidation reasons

 repair attempted/succeeded/failed

 repair budget consumed

 full replan frequency

Belief:

 stale belief acted on

 contradicted belief acted on

 source reliability changes

 false rumor propagation count

 correction latency

Coordination:

 queue wait times

 reservation conflicts

 abandoned grants

 dead/incapacitated claimant cleanup

 contract bids/awards/failures

Performance:

 snapshot build cost

 candidate extraction cost

 affordance enumeration cost

 search expansions

 cache hit/miss/invalidation counts

The architecture already has `AgentDecisionTrace`, `PlanSearchTrace`, `SearchExpansionSummary`, `RepairAttemptTrace`, and `CausalLinkCapHit`; the report says only the aggregate consumer is missing.

Add adversarial regression scenarios:

100-goal dense market:

 many objects, sellers, notices, social requests, threats.

20 agents / 3 workstations:

 queue, grant, abandonment, expiry, next actor acts.

False rumor justice:

 rumor → accusation → contested evidence → correction or miscarriage.

Office vacancy:

 office holder dies → succession delay → patrol gap → predation.

Boundary shock:

 expected import fails → shortage → substitution/exit.

Route bottleneck:

 caravans, guards, animals, narrow bridge, reservations.

Long production chain:

 4+ prerequisites, travel, facility queue, partial completion.

Belief-wall trap:

 co-located social facts must not leak through physical observation.

This is not optional polish. For Worldwake, debuggability is a product feature, and FOUNDATIONS requires causal and knowledge paths to be inspectable separately.

## **18. Specific fixes I would make immediately**

### **Fix 1 — Replace `max_candidates_to_plan = 2` with portfolio planning**

Do not merely raise it globally. Implement slot-based candidate selection. As an interim patch while building the portfolio system, use:

planned candidates =

 active intention, if any

 + top survival/safety

 + top obligation/commitment

 + top economic/work

 + top social/epistemic or local opportunity

That likely means 4–6 plan attempts in normal conditions, fewer in emergencies.

### **Fix 2 — Increase strategic budget by number of stages**

Current strategic budget is effectively independent of chain length. Replace:

budget = 2 * max_prerequisite_locations

with something shaped like:

budget =

 min(

   cognitive.max_strategic_expansions,

   2 * stages.len() * max_prerequisite_locations * branching_factor_hint

 )

At minimum:

budget = 2 * stages.len() * max_prerequisite_locations

This directly addresses multi-prerequisite acquisition chains.

### **Fix 3 — Make plan depth goal/method-dependent**

A global default depth of 8 is too blunt. Use:

struct GoalPlanningBudget {

   max_depth: u8,

   max_node_expansions: u16,

   repair_budget_fraction: Permille,

   max_strategic_expansions: u16,

}

Suggested defaults:

Self-care simple:        depth 4–8

Travel/purchase:         depth 8–10

Production/restocking:   depth 12–18

Investigation/justice:   depth 12–20

Bounty/escort/caravan:   depth 16–24, partial plans expected

Institutional succession depth 10–16

Partial plans make high depths less dangerous because agents can stop at meaningful barriers.

### **Fix 4 — Add `InformationBarrier` and epistemic subgoals**

This is the most realism-per-effort upgrade. Agents should plan to inspect, ask, read, verify, scout, and track when they lack information.

### **Fix 5 — Add static belief-view separation**

This is the most important correctness fix. The current belief wall is conceptually right, but the FND-14A exception should be impossible to widen accidentally.

### **Fix 6 — Add aggregate diagnostics before tuning**

Without this, changes to `beam_width`, `max_node_expansions`, `max_plan_depth`, and `max_candidates_to_plan` will be blind.

### **Fix 7 — Add `BlockerScope` patterns**

Exact blockers are not enough. Cross-goal place, route, facility, counterparty, and jurisdiction blockers are necessary for resilient behavior.

### **Fix 8 — Document or replace shared search caches**

The report notes `PlanningState` uses shared `Rc<RefCell<...>>` caches across search siblings and that this is safe only while they remain pure memoization. I would either:

* document the invariant in code and add tests proving cache order does not affect search result; or  
* replace shared mutable caches with immutable per-node/per-search deterministic memo tables.

For a deterministic simulation, accidental cache order dependence is a silent killer.

## **19. What not to do**

Do **not** replace this with pure behavior trees. Behavior trees are good for local reactive execution and animation-ish control, but as the top-level mind they tend to become authored relevance logic. Halo 2’s behavior system handled complexity through competing behaviors, but your FOUNDATIONS target is more demanding: lawful causality, belief provenance, inspectable institutions, and no hidden story rails.

Do **not** add a global job board singleton. Add physical/institutional artifacts: notices, contracts, work orders, ledgers, rosters, warrants, grants, queue tickets.

Do **not** let live LLMs invent plans, motives, facts, or explanations. They can help author schemas, generate tests, summarize traces, or propose decomposition methods offline. In live simulation, any LLM-like output must compile into deterministic, validated, lawful structures before the world advances.

Do **not** solve goal explosion with more hardcoded suppression constants. That will become an invisible drama dial. Use motive records, portfolio slots, evidence provenance, concrete costs, and learned agent-local preferences.

## **20. Recommended implementation order**

1. **Diagnostics first.** Build aggregate reports from existing traces. Measure candidate counts, plan failures, budget exhaustion, repair success, blocker churn, and plan terminal kinds.  
2. **Belief-wall hardening.** Split physical local observation, believed world facts, and believed social facts into separate traits. Add FND-14A trap tests.  
3. **Goal portfolio.** Replace top-2 planning with slot-based motive selection. Keep current `GoalOffer` shape initially.  
4. **Information barriers.** Add epistemic terminal kinds and expand inspect/ask/read/scout/track actions as first-class subgoals.  
5. **Strategic/depth budget fixes.** Scale strategic budget by stages. Make plan depth and expansion budget goal/method-dependent.  
6. **GoalSchema registry.** Move goal semantics out of scattered emitters into data-driven schemas and delta-triggered candidate extractors.  
7. **HTN method layer.** Add lawful decomposition methods for production, bounty, investigation, justice, escort, restocking, and office succession.  
8. **Coordination artifacts.** Add work orders, bids, awards, queue tickets, route claims, patrol roster entries, and assignment contracts.  
9. **Trust/habit learning.** Add source reliability, method preference, seller reliability, and route preference memories with provenance and decay.  
10. **Incremental snapshots and heuristic portfolio.** Optimize planning computation without changing causality.

## **21. Final architecture recommendation**

The best future Worldwake AI architecture is:

BDI deliberation shell

+ portfolio utility triage

+ data-driven GoalSchema registry

+ HTN method decomposition for complex lawful pursuits

+ existing GOAP tactical planner as leaf/action synthesizer

+ continual replanning through information/progress/coordination barriers

+ concrete coordination artifacts for multi-agent work

+ provenance-heavy trust and habit learning

+ static belief-wall enforcement

+ aggregate causal diagnostics

This architecture remains faithful to FOUNDATIONS because it does **not** author outcomes. It authors lawful motives, decomposition knowledge, social artifacts, and action schemas. Agents still act only through ordinary affordances, under local beliefs, with explicit costs, durations, contention, records, and aftermath. It also directly attacks the future scaling problem: hundreds of possible goals become cheap motive records, a bounded portfolio becomes the deliberation frontier, HTN methods keep long tasks tractable, and GOAP remains the grounded executor rather than the entire mind.

