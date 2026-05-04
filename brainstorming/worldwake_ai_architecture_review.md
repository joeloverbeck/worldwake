# Worldwake AI and Simulation Upgrade Proposal

## **Verdict**

Yes, the current AI architecture has issues to fix, but the underlying direction is right. I would **not** replace it with behavior trees, pure utility AI, an LLM controller, or a looser “story director.” The existing GOAP architecture already matches the core Worldwake idea: goals are desired world conditions, actions are lawful affordances, plans are selected under bounded knowledge, and execution is revalidated against local belief. That is exactly the right backbone for “explainable emergence.”

The main problem is scale and brittleness. The current planner looks strong for 35 goal kinds and hand-curated scenarios, but the moment you want agents reasoning among dozens or hundreds of possible goals, exploiting arbitrary scenario affordances, and surviving high contention, the architecture needs a second layer above GOAP: **a persistent belief/desire/intention portfolio, explicit causal-link plan assumptions, reusable HTN-style decomposition methods, continual sensing/replanning, and an affordance-driven opportunity compiler.**

Classic GOAP is valuable because it separates “what condition do I want?” from “what hard-coded sequence do I run?”, and Orkin’s original framing explicitly contrasts GOAP goals with embedded behavior sequences. Your architecture already follows that principle with `GoalKind`, `GoalOffer`, `ActionDef`, affordance queries, and runtime search. The upgrade should deepen that model, not abandon it.

---

## **1. Issues to fix in the current AI architecture**

### **P0: The live refactor must be completed before judging behavior**

The report says `ranking.rs` currently has compile errors around missing `capacity_observation_weight` and `source_composite` fields, and `source_composite.rs` still has unused in-progress helpers. That is not merely cosmetic. Under FOUNDATIONS, live authority paths must not contain half-migrated parallel representations or transitional shims. Finish the refactor, delete obsolete fields/constructors, and make the source-composite model a single authoritative ranking path.

**Fix:** make the ranking refactor atomic: compile cleanly, update all construction sites, add regression tests proving old and new ranking fields are not both consulted, and add a lint/test that rejects unused ranking-source helpers in live code.

---

### **P0: `apply_hypothetical_transition` is a dangerous duplicate simulator**

The report correctly flags that planner hypothetical transitions must match simulator outcomes. The current safety net is conformance testing, which is good, but architecturally this is still the highest-risk design seam: the planner has its own forward model, and the simulator has action handlers. If they drift, the agent “reasons” into impossible plans.

**Fix:** move toward a **single canonical effect schema** per `ActionDef`.

Each action should expose one authoritative semantics object that can be applied in two modes:

Authoritative mode: mutates ECS/event log through the scheduler.  
Hypothetical mode: mutates PlanningState overlays only.

The planner should not hand-reimplement trade, craft, queue, attack, travel, post-bounty, punish, and steal effects. It should call the same declared effect model the simulator uses, with different sinks. This aligns with FND-26 and FND-28: systems interact through state, and live authority paths should not preserve parallel versions of the same fact.

---

### **P0: The snapshot entity cap can become illegal causal compression**

`max_snapshot_entities_per_place = 50` is an obvious performance guard, but it is dangerous. FND-14A says same-tick co-located physical observation is belief-equivalent for directly perceivable facts. If an agent is in a crowded market with 80 relevant entities and the snapshot silently samples 50, then the planner may fail to perceive a colocated enemy, corpse, item, container, workstation, or sale lot for no in-world reason. That violates the spirit of locality and perception.

**Fix:** replace silent snapshot truncation with a concrete **attention/perception budget model**.

If the agent misses something, the answer should be: “because their attention, line of sight, crowd density, lighting, salience, distraction, panic, or search effort did not expose it.” Not: “because the planner cap was 50.”

Add:

PerceptionBudget {  
   max_observations: u16,  
   salience_policy: SaliencePolicy,  
   occlusion_policy: OcclusionPolicy,  
   stress_penalty: Permille,  
}

ObservationOmission {  
   omitted_entity: EntityId,  
   reason: OmissionReason,  
   tick: Tick,  
}

The snapshot may still be capped, but every omission must be explainable as agent-local perception, not CPU optimization.

---

### **P0: Decision traces are opt-in, but causal explainability must be always-on**

The report says rich decision traces exist, but tracing is gated by `enable_tracing()`. That is fine for expansion-level diagnostics, but not sufficient for FND-29/FND-29A. If an agent causes a bounty chain, false accusation, theft report, or failed rescue, you need a permanent minimal explanation record even when debug tracing was off.

**Fix:** split diagnostics into two tiers.

Always-on, append-only, cheap:

DecisionEvent {  
   event_id,  
   agent,  
   tick,  
   selected_goal,  
   selected_plan_id,  
   top_rejected_goals: SmallVec<GoalRejectionSummary>,  
   decisive_beliefs: SmallVec<BeliefRef>,  
   decisive_records: SmallVec<RecordRef>,  
   decisive_world_observations: SmallVec<ObservationRef>,  
   assumptions: SmallVec<PlanAssumptionRef>,  
}

Opt-in, expensive:

SearchExpansionSummary  
AffordanceTrace  
RootCandidateTrace  
BeamPruningTrace  
FFHelpfulActionTrace

That preserves debuggability without paying full trace cost every tick.

---

### **P1: `max_candidates_to_plan = 2` is too brittle for hundreds of goals**

Top-2 planning is aggressive. With many possible goals, it creates a ranking bottleneck: if the first two goals are attractive but infeasible, the agent can stall or waste cycles while a feasible third or fourth goal waits. The report already notices this.

**Fix:** replace “plan top N goals” with **budgeted portfolio deliberation**.

Instead of:

rank all goals  
search top 2  
commit best found

Use:

rank into portfolio slots  
allocate expansion budget across slots  
keep partial search frontiers across ticks  
commit first sufficiently good feasible plan  
continue background deliberation when attention allows

Example portfolio slots:

enum PortfolioSlot {  
   Survival,  
   Safety,  
   ActiveCommitment,  
   Obligation,  
   SocialInstitutional,  
   Economic,  
   Opportunity,  
   Exploration,  
   HabitRoutine,  
}

This lets a hungry agent still notice a dragon, an office-holder still process a murder report, and a merchant still react to a local theft without ranking quality becoming the sole gatekeeper.

---

### **P1: Hard suppression by goal family is too crude**

Current suppression policy drops opportunism at High+ stress and social/political goals at Critical stress. That is clean, but too blunt. Some social actions are survival-relevant: shouting a warning, calling guards, asking for shelter, surrendering, begging for help, reporting an immediate threat, or rallying allies. Some political/institutional actions can be urgent under danger: ordering a gate closed or mustering patrols.

**Fix:** suppress by **expected causal role**, not by goal family.

A goal should not be suppressed because it is “social.” It should be suppressed because, under this agent’s beliefs, it does not reduce a current higher-priority pressure quickly enough.

Replace:

SuppressedAtCritical(ShareBelief)

with something closer to:

GoalInterruptionProfile {  
   can_reduce_current_danger: bool,  
   can_preserve_commitment: bool,  
   requires_stationary_attention: bool,  
   exposes_actor_to_threat: bool,  
   expected_delay_ticks: u32,  
   social_support_effect: Option<SupportEffect>,  
}

Then `ShareBelief { topic: DragonNearby }` can survive critical stress, while `SupportCandidateForOffice` does not.

---

### **P1: `relevant_ops` is currently a hard gate that can block emergent solutions**

The report says each `GoalKind` declares relevant operator kinds. This is useful for performance, but dangerous when it becomes the definition of possibility. For example, `AcquireCommodity` currently maps to travel, trade, queue, harvest, craft, and move-cargo. But a desperate, greedy, criminal, starving, or panicked agent might steal food, beg for food, threaten a merchant, search a corpse, raid a camp, accept debt, or ask an ally. If the operator list omits those paths, the planner cannot discover them, no matter how lawful they would be in the world.

**Fix:** make `relevant_ops` a hint, not the authority.

The authority should be action effects:

ActionDef {  
   effect_schema: EffectSchema,  
   legal_risk_schema: Option<LegalRiskSchema>,  
   social_consequence_schema: Option<SocialConsequenceSchema>,  
}

Then goals query an effect index:

Goal: I want Owns/Controls/CanConsume(food)  
Index returns: buy, harvest, craft, receive gift, steal, loot, beg, confiscate, ration, borrow  
Ranking filters by agent values, law, risk, confidence, urgency

That is the difference between “agents execute designed paths” and “agents exploit what the scenario offers.”

---

### **P1: Candidate generation is too emitter-heavy for future scale**

The architecture has around 50 `emit_*` functions. That is manageable now, but it will become a maintenance trap as goals multiply. Each new goal risks needing bespoke candidate emitters, bespoke ranking, bespoke blocker logic, and bespoke relevant-op mappings. That gradually becomes disguised scripting.

**Fix:** introduce a declarative **goal schema registry**.

Do not replace typed Rust goals with freeform strings. Instead, keep strong typing but require every goal schema to declare:

GoalSchema {  
   satisfaction_predicate,  
   motive_sources,  
   possible_effect_facts,  
   default_operator_hints,  
   required_belief_kinds,  
   canonical_blockers,  
   revival_conditions,  
   trace_labels,  
}

Candidate generation then becomes:

1. Gather motive sources from needs, obligations, threats, records, routines, opportunities.  
2. Gather perceived/remembered affordances and records.  
3. Match motives to goal schemas and action-effect schemas.  
4. Emit GoalOffers with evidence traces.

This aligns directly with FND-30: every system and goal has declared causal hooks, observability, failure states, and downstream consequences.

---

### **P1: Strategic and tactical planning need feedback, not one-way decomposition**

The current split is: strategic itinerary first, tactical A* second. If tactical search fails for a strategic step, the whole plan fails. That is brittle. In a rich world, a failed tactic often means “choose another source,” “ask someone,” “verify rumor,” “detour,” “wait in queue,” “use another tool,” or “try a worse substitute,” not “the whole goal is impossible.”

HTN planning is useful here because it decomposes tasks into subtasks while allowing domain knowledge to shape search; SHOP2, for example, uses methods to recursively decompose tasks into primitive operators and supports partially ordered subtasks and temporal/metric features. That kind of decomposition is appropriate for Worldwake only if methods encode reusable lawful procedures, not story rails.

**Fix:** add **reusable HTN-style methods above GOAP**, not instead of GOAP.

Examples:

AcquireFood  
 - consume carried food  
 - buy from known seller  
 - harvest known source  
 - ask household member  
 - beg from nearby agent  
 - steal from accessible container  
 - travel to rumored source and verify  
 - substitute water/rest/sleep if hunger is not yet critical

InvestigateViolation  
 - inspect scene  
 - interview witness  
 - consult ledger  
 - compare alibi  
 - issue accusation  
 - defer for lack of jurisdiction

Each leaf remains an ordinary `ActionDef`. Each method must declare preconditions, costs, evidence requirements, possible failure states, and legal/social consequences.

---

### **P1: Plan guards should be generated from causal links, not only templates**

Current guards include facts like target present, commodity available, route known, resource access, target moved, commodity depleted, and new blocker. Good start. But robust agents need to know **which prior fact supports which later step**.

Partial-order/causal-link planning explicitly records links from earlier effects to later preconditions; UCPOP describes causal links as representing the assumptions a plan relies on. Worldwake should steal that idea, even if it keeps total-order execution.

**Fix:** store causal links in every plan.

PlanCausalLink {  
   provider: CausalProvider, // prior step, belief, observation, record, carried item, office rule  
   fact: PlanningFact,  
   consumer_step_index: u16,  
   invalidators: Vec<Invalidator>,  
   confidence: Permille,  
   source_time: Tick,  
}

Then revalidation becomes systematic:

Only replan if a causal link supporting the remaining suffix is broken.  
If a link breaks, repair that link first.  
If repair fails, abandon or downgrade the intention.

This makes plans more resilient and more explainable.

---

### **P1: Replanning is too discard-heavy; add plan repair**

Currently, many failures clear the plan and push blockers/discrepancies. That is safe but crude. If step 3 of a 7-step plan fails because the merchant moved, the agent may still keep the goal, the route, the acquired prerequisite, and the later intent. Realistic agents repair plans.

**Fix:** add `PlanRepairContext`.

PlanRepairContext {  
   failed_step,  
   failed_causal_link,  
   preserved_prefix,  
   reusable_suffix,  
   new_evidence,  
   blocker,  
}

Repair attempts:

1. Rebind target.  
2. Replace provider of broken causal link.  
3. Insert verification/sensing step.  
4. Substitute method branch.  
5. Downgrade to progress barrier.  
6. Abandon only if repair search fails.

This pairs naturally with causal links and HTN methods.

---

### **P1: Continual planning should be first-class**

The current architecture already interleaves execution, monitoring, expectations, and replanning. The next step is to let agents plan **to learn**, not merely fail into learning. Brenner and Nebel’s continual-planning work is directly relevant: in dynamic multi-agent worlds, optimal complete plans are often impractical because agents lack knowledge or their knowledge becomes obsolete; their alternative is integrating planning, execution, monitoring, knowledge gathering, and later revision.

**Fix:** introduce explicit epistemic subgoals.

GoalKind::VerifyBelief { claim: BeliefClaimKey }  
GoalKind::ConsultRecord { record: EntityId, topic: RecordTopic }  
GoalKind::AskWitness { witness: EntityId, topic: TellTopic }  
GoalKind::ScoutPlace { place: EntityId, hypothesis: HypothesisKind }  
GoalKind::InspectContainer { container: EntityId, expectation_id: Option<ExpectationId> }

The planner should insert these when the best plan depends on stale, low-confidence, contradicted, or missing beliefs.

Important: do **not** build full probabilistic contingent plans for every possibility. That will explode. Use lightweight continual planning: verify what matters, act, observe, revise.

---

### **P1: Travel pruning is too myopic**

The current travel-pruning rule rejects travel destinations whose remaining cost to goal increases. That is efficient but can suppress realistic detours: avoiding danger, visiting a witness, obtaining permission, getting a tool, asking directions, joining an escort, resting before a fight, or approaching from a safer route.

**Fix:** allow travel away from the immediate goal when it satisfies a causal landmark, reduces risk, improves belief, obtains capacity, or supports a method branch.

Travel pruning should be based on:

distance-to-goal  
+ causal-link progress  
+ risk reduction  
+ information gain  
+ prerequisite acquisition  
+ commitment preservation

not just monotonic distance.

---

### **P1: Budget exhaustion should degrade gracefully**

`max_candidates_per_expansion = 200` currently causes immediate `BudgetExhausted` if exceeded. That is a tripwire. In crowded markets, battles, offices, or rumor hubs, the planner should degrade into “consider the most salient 200 now and continue later,” not fail the whole search.

**Fix:** make search expansion resumable.

SearchContinuation {  
   frontier,  
   unexpanded_candidate_cursor,  
   budget_spent,  
   created_tick,  
   expires_tick,  
}

When an expansion overflows, record a continuation and resume next deliberation slice. The agent can also choose a cheap fallback meanwhile.

---

### **P1: Blocker TTLs are too generic**

`transient_block_ticks = 20` and `structural_block_ticks = 200` are agent-local cognitive settings, so they are not forbidden drama probabilities. But they are still coarse. A blocked route, empty market, dead counterparty, lost office claim, missing witness, and failed theft should not share the same retry logic.

**Fix:** blockers need typed clearing conditions and lifecycles.

Blocker {  
   blocking_fact,  
   observed_tick,  
   source_evidence,  
   clearing_condition,  
   expected_recheck_mode,  
   memory_decay,  
   confidence_decay,  
}

Examples:

CommodityUnavailable clears when agent observes restock, hears credible restock report, or substitutes.  
NoKnownPath clears when route is learned, guide is found, or map/record is consulted.  
CounterpartyRefused clears when relationship changes, price changes, threat changes, or desperation rises.  
OfficeClaimBlocked clears when incumbent dies, support shifts, legal record changes, or force controller changes.

The expiry is memory decay, not world truth.

---

### **P2: Agent diversity exists structurally, but may not exist in practice**

The report says `CognitiveProfile`, `ExecutionBudget`, `UtilityProfile`, `TellProfile`, `EpistemicDispositionProfile`, loyalties, courage, and other per-agent components exist. Good. But if scenarios mostly use defaults, populations will still behave homogenously.

**Fix:** add scenario validation that rejects cloned cognition unless intentionally declared.

Example:

Village may not spawn 20 identical peasants unless AgentDef says homogeneous_population: true.  
Every role archetype must vary at least N concrete dimensions:  
 risk, patience, memory, courage, social trust, greed, duty, curiosity, fatigue tolerance.

This directly supports FND-22 and FND-22A.

---

## **2. How to upgrade the AI for resilient, realistic, intelligent agents**

### **Target architecture**

I would evolve the architecture into this pipeline:

Perception / testimony / records  
       ↓  
Belief store + evidence + contradictions + memory decay  
       ↓  
Motive sources: needs, threats, obligations, habits, values, relationships, opportunities  
       ↓  
Persistent goal portfolio / BDI intention manager  
       ↓  
HTN-style method selection for high-level procedures  
       ↓  
GOAP / A* / FF / landmark planner for primitive lawful actions  
       ↓  
Plan with causal links, expectations, guards, repair handles  
       ↓  
Execution through scheduler, queues, reservations, action durations  
       ↓  
Observation, surprise, blocker/discrepancy memory, learning

This is a BDI-flavored architecture, but not academic ceremony. BDI is useful here because it explicitly distinguishes beliefs, desires, and intentions under bounded deliberation; Rao and Georgeff frame BDI attitudes as information, motivational, and deliberative states, which is exactly the distinction Worldwake needs.

---

### **Keep GOAP as the primitive lawful-action planner**

GOAP should remain the thing that answers:

Given this agent’s belief state,  
this current intention,  
this set of legal affordances,  
and this bounded budget,  
what action sequence could make progress?

Your current FF heuristic and landmark machinery are appropriate. FF’s core idea is forward state-space search guided by a relaxed-plan heuristic that ignores delete effects and extracts useful pruning information; your planner already uses FF-like helpful-action guidance and landmarks.

But GOAP should not own everything. It should not be responsible for long-term personality, institutional workload, social commitments, or evaluating hundreds of desires every tick.

---

### **Add a persistent goal portfolio**

The current agenda lifecycle is close, but I would make it more explicit and more durable.

DesireToken {  
   goal: GoalKind,  
   motive_sources: Vec<MotiveSourceRef>,  
   evidence_trace: Vec<EvidenceRef>,  
   urgency: UrgencyProfile,  
   deadline: Option<Tick>,  
   confidence: Permille,  
   expected_cost_band: CostBand,  
   expected_risk_band: RiskBand,  
   social_legal_exposure: ExposureBand,  
   current_status: DesireStatus,  
   last_attempt: Option<AttemptSummary>,  
}

`MotiveSource` should be concrete:

enum MotiveSource {  
   NeedPressure(HomeostaticNeedId),  
   Pain(WoundId),  
   Fear(ThreatBeliefId),  
   Obligation(ContractId),  
   OfficeDuty(OfficeId, DutyId),  
   Debt(DebtId),  
   Loyalty(EntityId),  
   Greed(OpportunityId),  
   Habit(HabitId),  
   Curiosity(HypothesisId),  
   Shame(ReputationRecordId),  
   Revenge(ViolationId),  
}

Ranking scores become derived views over motive sources, not free-floating truth. That keeps numeric utility compatible with FND-3.

---

### **Add an affordance-to-opportunity compiler**

This is probably the most important upgrade for “agents exploit what scenarios offer.”

Right now, candidate generation is primarily goal-family-driven: emit food goals, emit bounty goals, emit theft goals, emit office goals, and so on. That means scenario opportunities are noticed only if some emitter knows how to look for them.

Add a bottom-up pass:

For every perceived entity, record, place, route, social fact, and affordance:  
   What effects could this enable?  
   What needs/obligations/values could those effects satisfy?  
   What risks or legal consequences would they create?  
   What information could this object/person/place reveal?

Example:

Opportunity {  
   anchor: EntityId,  
   perceived_at: Tick,  
   source_belief: BeliefRef,  
   possible_effects: Vec<PlanningFact>,  
   possible_information: Vec<ClaimTopic>,  
   required_actions: Vec<PlannerOpKind>,  
   legal_status: BelievedLegalStatus,  
   social_exposure: SocialExposure,  
   salience: Salience,  
}

Then the agent can reason:

I am hungry.  
I see bread.  
I believe it belongs to the baker.  
I can buy it, steal it, beg for it, threaten for it, or wait for discard.  
My courage/greed/lawfulness/hunger decide which methods are considered.

That is much more realistic than “AcquireCommodity supports Trade and Harvest.”

---

### **Add HTN methods as lawful reusable know-how**

Agents need domain knowledge. A magistrate, hunter, merchant, thief, priest, guard, and peasant should not all decompose problems the same way.

HTN-style methods are the right way to encode this, as long as they are **generic procedures**, not authored outcomes. SHOP2 shows the core model: nonprimitive tasks decompose recursively into subtasks until primitive operators are reached.

Examples of legal Worldwake methods:

Hunter.HuntBeast  
 - gather latest tracks/reports  
 - travel to last credible evidence  
 - scout adjacent territory  
 - engage if confidence and courage suffice  
 - retreat or request allies if risk too high  
 - return proof to issuer

Magistrate.HandleRobberyReport  
 - receive testimony  
 - check jurisdiction  
 - inspect record/ledger if available  
 - create case record  
 - assign investigation or ignore if resources absent  
 - issue accusation/warrant/bounty if evidence threshold met

Merchant.Restock  
 - check local inventory  
 - check known suppliers  
 - reserve transport  
 - buy cargo  
 - return and post listing

These methods must never teleport facts, skip travel, override rights, or guarantee outcomes. They only guide decomposition.

---

### **Add causal-link plans and assumption monitoring**

Current revalidation checks the next step. Better agents need to monitor the assumptions supporting the entire remaining intention.

A plan should explain itself as:

I will do Step 4 because Step 2 should provide food.  
I believe the food is at Market because witness W said so at tick 120.  
I believe I can access Market because route R is known and not blocked.  
I believe the baker will trade because I saw an active sale listing.

Represent that directly:

PlanAssumption {  
   fact: PlanningFact,  
   source: AssumptionSource,  
   confidence: Permille,  
   freshness: Tick,  
   invalidators: Vec<Invalidator>,  
   consumer_steps: Vec<u16>,  
}

Then interruption becomes principled:

Dragon appears → invalidates route-safety assumption.  
Merchant leaves → invalidates trade-target assumption.  
Food sold out → invalidates commodity-available assumption.  
Witness contradicted → lowers confidence, may insert verification.

This will make agents feel less stupid and less brittle.

---

### **Add planned sensing and verification**

Do not make agents omnisciently smarter. Make them smart enough to know what they do not know.

Useful actions:

Inspect stash  
Check notice board  
Ask witness  
Ask guide for route  
Consult ledger  
Scout road  
Track beast  
Count inventory  
Verify office holder  
Examine corpse  
Check lock

These actions should produce belief updates, not direct truth injection. This supports the canonical rumor and robbery scenarios in FOUNDATIONS.

---

### **Add plan repair before full replanning**

A realistic agent does not throw away an entire intention because one assumption failed.

Example:

Goal: acquire food.  
Plan: travel to market → buy bread → eat.  
Failure: baker has no bread.  
Repair:  
 - buy grain instead  
 - ask baker about supplier  
 - travel to second seller  
 - steal if desperate and immoral  
 - return home if fatigue/danger too high

This should be a small repair search around the broken causal link, not a full reset.

---

### **Make planning itself resource-bounded in-world when appropriate**

The engine’s CPU budget is not the same as the agent’s deliberation budget. But for complex deliberation, the agent should have attention limits.

A panicked, wounded, exhausted agent should plan shallowly. A rested strategist in an office can reason further. The report already has `CognitiveProfile` and `ExecutionBudget`; use them more aggressively as agent traits.

Add:

DeliberationState {  
   current_problem: Option<DeliberationProblem>,  
   expansions_spent: u32,  
   attention_reserved: Permille,  
   started_tick: Tick,  
   can_continue_while_walking: bool,  
}

A guard can patrol and think a bit. A surgeon treating a wound cannot also perform deep political planning. This aligns with FND-8’s attention/occupancy principle.

---

### **Add richer local opportunism**

Opportunism should be a generic interrupt layer, not a few goal kinds.

Local perception can generate interrupt proposals:

I see a dragon → flee/hide/warn/attack.  
I see a wounded ally → treat/carry/report/ignore.  
I see unattended valuables → steal/guard/report/ignore.  
I see a corpse → loot/bury/report/investigate.  
I see a rival vulnerable → attack/blackmail/help/avoid.  
I hear a false rumor → repeat/challenge/verify/exploit.

Each proposal still enters the same portfolio/ranking system. No drama triggers. No “interesting event” rolls. Just local affordances meeting agent motives.

---

### **Upgrade learning without cheating**

The report already has source reliability, blocker memory, discrepancy memory, survey memory, and experience preferences. That is the right shape. Extend it into concrete learned state:

RoutePreference {  
   route,  
   learned_from: EvidenceRef,  
   success_count,  
   failure_count,  
   last_outcome,  
   decay_tick,  
}

SourceReliability {  
   source,  
   topic_class,  
   confirmations,  
   contradictions,  
   last_checked,  
}

Habit {  
   context_signature,  
   action_or_method,  
   reinforcement,  
   last_used,  
   decay,  
}

TrustRelation {  
   other,  
   domain,  
   trust,  
   evidence_history,  
}

The key rule: every learning update must answer “what experience caused this?” That is exactly FND-22A.

---

## **3. Simulation upgrades to better align with FOUNDATIONS**

### **Make scheduling and contention world-visible**

The report emphasizes deterministic iteration with `BTreeMap`, seeded RNG, and stable sorting. That is necessary, but not sufficient. FND-9 says tick order must not silently decide world meaning. If two agents grab the same bread, enter the same doorway, claim the same office, or attack in the same instant, deterministic `EntityId` order is not an in-world explanation.

Add an explicit contention layer:

ContentionEvent {  
   contested_affordance,  
   claimants,  
   resolution_rule,  
   evidence,  
   winner,  
   losers,  
   tick,  
}

Resolution rules might be:

arrival time  
queue position  
reservation token  
office grant  
physical proximity  
initiative  
strength contest  
legal priority  
random seeded microstate, if declared

The outcome must leave an inspectable artifact or event.

---

### **Make institutions task-bearing actors, not just goal sources**

The report includes offices, succession, accusations, punishments, patrols, bounties, and candidates. Good. The next upgrade is institutional workload.

An office should have:

Office {  
   holder,  
   jurisdiction,  
   duties,  
   budget,  
   authority_records,  
   succession_rule,  
   delegation_rules,  
}

InstitutionalTask {  
   issuer_office,  
   duty_kind,  
   priority_basis,  
   required_authority,  
   assigned_agent,  
   status,  
   evidence_refs,  
   deadline,  
}

Then “the town reacts” becomes:

survivor reports attack  
watch office receives testimony  
case record created  
captain assigns patrol or posts bounty  
treasury pays reward if proof accepted

No singleton “town brain.”

---

### **Add explicit artifact lifecycles everywhere**

FOUNDATIONS distinguishes existence, visibility, legality, credibility, and actionability. The current architecture has bounties, notices, records, accusations, expectations, and listings, but every artifact class should formalize lifecycle axes.

Example:

ArtifactLifecycle {  
   existence: Exists | Destroyed,  
   visibility: Hidden | Private | Posted | WidelyKnown,  
   legal_effect: None | Active | Suspended | Expired | Revoked | Fulfilled,  
   credibility: Credible | Disputed | Refuted | Unknown,  
   actionability: Actionable | AwaitingProof | Blocked | Closed,  
}

An expired bounty should remain visible as history. A false accusation should remain inspectable after exoneration. A revoked warrant should not authorize arrest but should remain part of institutional memory.

---

### **Add boundary processes for remote shocks**

FOUNDATIONS includes a canonical scenario for remote disruption causing delayed arrival failure, local shortage, substitution, rationing, hoarding, theft, or exit. The GOAP report does not describe boundary systems in detail.

Add:

BoundaryRegion {  
   id,  
   known_name,  
   remote_stocks,  
   route_channels,  
   travel_delay,  
   reliability,  
   observables,  
}

BoundaryProcess {  
   source_region,  
   output_kind,  
   scheduled_departure,  
   expected_arrival,  
   carrier,  
   capacity,  
   failure_modes,  
   evidence_generated,  
}

Imports, refugees, taxes, herds, letters, weather fronts, and rumors should enter through these processes. No hidden spawners.

---

### **Deepen evidence as physical aftermath**

For the canonical chains, the world needs more than events. It needs carriers:

tracks  
blood  
broken locks  
dropped cargo  
wound records  
corpse state  
drag marks  
empty containers  
ledger mismatch  
rumor copies  
notice copies  
witness memories  
route traces

Agents should plan around these. A hunter follows tracks. A magistrate compares testimony. A thief avoids witnesses. A survivor leaves fear, wounds, missing cargo, and testimony.

This is the substrate that makes emergence legible rather than merely chaotic.

---

### **Upgrade ownership/access/jurisdiction as separate concrete relations**

The report has control/ownership/legal-rights belief views. Make sure the authoritative model fully separates:

owner  
holder  
container  
possessor  
access right  
key/control mechanism  
office authority  
jurisdiction  
debt/obligation  
custody

This is essential for theft, confiscation, taxation, inheritance, trespass, punishment, market access, and office duties. It is also central to the stored-gold canonical scenario.

---

### **Make long actions produce partial state**

FOUNDATIONS says failure is new state. Do not let actions remain atomic if they should create intermediate consequences.

Examples:

Travel: exposes agent to events, fatigue, route evidence, delays.  
Craft: consumes inputs over time, can leave half-finished work, waste, broken tools.  
Combat: wounds, fear, dropped items, noise, witnesses, retreat paths.  
Investigation: creates notes, suspicions, questioned witnesses, contaminated evidence.  
Trade: queue position, negotiation state, reserved goods, refused offers.  
Burial: moved corpse, disturbed witnesses, partial grave.

The planner can still reason in abstract steps, but the simulator should expose partial outcomes to other agents.

---

### **Add routines and expectations as world-facing state**

Realistic agents notice surprise because they expected something else. Worldwake already values this. Build it out:

Routine {  
   agent,  
   expected_place_by_tick,  
   expected_activity,  
   tolerance,  
   observers_who_care,  
}

Expectation {  
   owner,  
   subject,  
   expected_fact,  
   source,  
   created_tick,  
   freshness,  
   violation_policy,  
}

This enables:

guard missing from patrol → merchant worries  
servant absent from kitchen → household searches  
market empty at usual hour → buyers ask why  
gold missing from stash → owner reports robbery  
caravan late → office posts inquiry

No omniscient absence detection.

---

## **4. Additional beneficial changes**

### **Build a formal validation dashboard**

The report lists many golden tests, conformance tests, determinism tests, and a soak. That is excellent. But for this architecture, “looked plausible” is never enough.

Track live metrics:

candidate count distribution  
top-N feasible miss rate  
budget exhaustion rate  
frontier exhaustion rate  
beam pruning rate  
travel-prune regret cases  
plan repair success rate  
assumption invalidation causes  
belief contradiction frequency  
stale-belief wasted travel  
source reliability correction rate  
queue/contention outcomes  
institutional backlog age  
unexplained event count

The key metric I would add immediately:

ranked_goal_feasibility_gap:  
   how often goal rank 3+ had a feasible plan when rank 1-2 failed

If that number is nontrivial, `max_candidates_to_plan = 2` is already harming intelligence.

---

### **Add adversarial scenario fuzzers**

Use property-based tests to generate ugly worlds:

crowded market with 200 entities  
many identical food sources  
stale rumors and contradictory witnesses  
dead office holder during crisis  
two agents racing for one tool  
trade counterparty leaves mid-plan  
route changes while agent travels  
resource appears behind legal access barrier  
false accusation with real punishment  
boundary shipment fails while town consumes stock

Each fuzz run should assert causal validity, not desired story outcome.

---

### **Add architecture lints**

I would add static or test-time lints for:

No ActionDef without canonical effect schema.  
No ActionDef with simulator effect but missing planner effect.  
No GoalKind without satisfaction predicate.  
No GoalKind whose only achievers are hidden special cases.  
No relevant_ops entry that is the sole authority for possibility.  
No belief-view accessor reading nonlocal authoritative state.  
No snapshot truncation without ObservationOmission.  
No artifact with active effect but missing lifecycle state.  
No queue/reservation/grant without expiry/invalidation.  
No decision without minimal DecisionEvent.

These lints directly enforce FOUNDATIONS instead of relying on code review memory.

---

### **Use LLMs only outside live authority paths**

Do not let an LLM choose live actions, invent facts, decide outcomes, summarize beliefs into authority, or “make the story better.” That would collide with locality, determinism, replay, source/sink accounting, and explainability.

Safe uses:

offline scenario generation  
test-case generation  
trace summarization for developers  
authoring assistant for ActionDef/GoalSchema drafts  
fuzzing adversarial social situations  
natural-language explanation of already-recorded causal history

The LLM can describe or propose. It must not become a hidden world process.

---

## **Recommended roadmap**

### **Phase 1: Correctness and architectural hygiene**

Finish the ranking/source-composite refactor. Replace simulator/planner effect duplication with canonical action effect schemas or at least a stricter conformance harness. Make minimal `DecisionEvent` records always-on. Replace silent snapshot caps with explicit perception/attention omissions. Add contention events for same-tick scarce affordances.

### **Phase 2: Scalable deliberation**

Replace top-2 planning with budgeted goal portfolio deliberation. Turn `relevant_ops` into hints backed by effect-schema indexing. Add goal schemas and motive-source ledgers. Add persistent partial search continuations. Track ranked-goal feasibility gaps.

### **Phase 3: Robust intelligence**

Add causal-link plan assumptions. Add plan repair. Add epistemic subgoals: inspect, ask, consult, scout, verify. Add HTN-style reusable methods for roles and institutions. Extend interruption to local opportunism.

### **Phase 4: Richer simulation substrate**

Add institutional task ledgers, boundary processes, richer artifact lifecycles, physical evidence carriers, routines/expectations, and partial outcomes for long actions.

### **Phase 5: Validation at scale**

Convert FOUNDATIONS canonical scenarios A–H into property-based scenario classes, not just hand-authored goldens. Add adversarial sampling, sensitivity sweeps, planner telemetry, and causal-history inspectors.

---

## **The strongest recommendation**

Do **not** think of the future system as “better GOAP.” Think of it as:

BDI-style persistent motives and commitments  
+ HTN-style lawful domain know-how  
+ GOAP primitive action search  
+ continual planning under partial belief  
+ causal-link monitoring and repair  
+ world-state institutions, evidence, contention, and artifact lifecycles

That combination keeps Worldwake aligned with its foundational principles: no authored outcomes, no omniscient intelligence, no hidden drama manager, no abstract scores promoted to truth, no planner entitlement, and no unexplained events. It also gives you a credible path from 35 goals to hundreds without turning the AI into either a brittle script pile or an opaque black box.

