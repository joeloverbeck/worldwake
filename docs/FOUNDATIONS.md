# Worldwake Foundational Principles

These principles define what Worldwake is optimizing for: explainable emergence. The target is not mere surprise, noise, or content density. The target is chains of consequence that are surprising, legible, and fully traceable after the fact.

Designers author the world's nouns, laws, institutions, and initial conditions. They do not author outcomes. Every system, feature, content addition, and optimization must be judged against these principles. They are non-negotiable unless explicitly revised by the project owner. All contributors — human and AI — must internalize them before making design decisions.

Every change to the simulation — new system, revised spec, implementation plan, or bugfix — must be an architecturally comprehensive solution. Hacks, patches, shims, and workarounds that avoid the root design concern are not acceptable, even when they are faster. The result must leave the architecture clean, robust, and extensible: clean meaning no dead paths or fossilized logic; robust meaning invariants hold under edge cases and future load; extensible meaning new systems can compose with existing ones without surgery.

**Test**: If the most accurate description of a proposed change is "a workaround," "a patch for now," or "a localized fix that avoids the real problem," it violates this mandate. Reframe the change as a proper architectural solution or reject the approach.

---

## I. Causal Standard

### 1. Maximal Emergence Through Local Causality

Worldwake exists to produce emergent behavior through interacting systems and agents, never through authored sequences, hidden quest logic, or one-off story triggers. An event is valid only if it arose from prior world state, agent belief, institutional rule, or natural process already present in the simulation.

Authoring beasts, hunger, roads, caravans, towns, offices, and bounty procedures is correct. Authoring “a beast attack happens so adventurers have content” is forbidden.

**Test**: If the only honest explanation for an event is “the game decided something interesting should happen now,” the design violates this principle.

### 2. No Ungrounded Triggers or Probabilities

No outcome may bottom out at a naked designer dial such as `chanceOfEncounter`, `questSpawnChance`, `interestingEventProbability`, or any similar drama lever.

Probabilistic transitions are allowed only when they belong to an explicit world process or hidden local microstate: imperfect perception, noisy execution, variable travel delay, disease exposure, fertility, collapse risk, weather, or another declared source of uncertainty. In those cases the distribution must still be a function of concrete local or boundary state, elapsed time, and declared inputs. It may not exist solely to pace drama.

Randomness must be seeded, attributable, and replayable. Given the same seed, initial state, schedule, and external inputs, the simulation should reproduce the same outcome. Different seeds may diverge, but only through those same declared uncertainty paths.

Utility weights, need rates, fear sensitivity, memory fidelity, and skill parameters may exist as concrete agent properties. “Interesting thing happens here 30% of the time” may not.

**Test**: If changing a single abstract constant can create or remove an event without any corresponding change in a lawful local or boundary world process, the design violates this principle.

### 3. Concrete State Over Abstract Scores

Prefer modeling the thing itself over a score that represents it. Danger should come from actual threats on routes, not `danger_score`. Scarcity should come from inventories, queues, failed purchases, and unmet needs, not `scarcity_score`. A price spike should emerge from actual stock, seller beliefs, buyer pressure, and substitute availability, not from `if stock < 50% then price *= 1.5`.

Abstract summaries are allowed only as derived views or caches. They may never become the source of truth.

**Test**: If a system relies on a number that cannot be traced back to concrete entities, relations, or events, the design violates this principle.

### 4. Persistent Identity, Object Permanence, and Explicit Transfer

Every meaningful thing in the simulation has stable identity while it exists: agents, beasts, items, containers, wounds, corpses, notices, contracts, offices, titles, ledgers, debts, rooms, roads, and places. Things do not wink in or out of being because they are offscreen or inconvenient.

Movement, splitting, merging, damage, consumption, creation, transfer, and destruction must be explicit world processes. If gold leaves a stash, there must have been a theft, payment, transfer, misplacement, destruction, or prior accounting error. If a bounty exists, someone or some institution must have created it at a place and time. If a caravan no longer has cargo, that cargo must be somewhere else, destroyed, or consumed.

For quantities the simulation treats as conserved or explicitly accounted — coin, goods, bodies, ingredients, outputs, or claim-like balances — every increase, decrease, split, merge, creation, destruction, and transformation must have an explicit source or sink path. Harvests draw from sources. Crafts transform inputs into outputs. Regeneration, decay, inheritance, spoilage, write-offs, births, and minting must be equally explicit if they exist.

When one entity is created from another entity or from a world process — ore into bars, bars into a sword, a cargo stack split into two sacks, a notice copied from an original, a wound becoming a scar, a corpse becoming remains — the model should preserve derivation lineage wherever downstream systems may care. Source/sink accounting says where quantities went. Derivation lineage says what this thing came from.

**Test**: If you cannot answer “where did it go?”, “where did it come from?”, “who changed it?”, or “is this the same entity as before?” the model is too abstract.

---

## II. World Dynamics

### 5. Simulate Carriers of Consequence, Not Decorative Realism

Model only what can propagate downstream effects: goods, containers, tools, wounds, disease, waste, offices, loyalties, debts, contracts, evidence, rumors, records, routes, ownership, access rights, and other carriers of consequence.

Do not simulate weather systems, chemistry, or expansive crafting trees just because they are realistic. Fidelity comes from consequence density, not from the sheer number of subsystems.

**Test**: For any proposed system, ask: “What new downstream consequences does this create?” If the answer is only “it feels more real,” it does not justify its cost.

### 6. World Runs Without Observers

The simulation must continue meaningfully when no human is looking and when no human-controlled agent is present. Villages still deplete inventories. Beasts still roam. Guards still tire. Thieves still steal. Offices still become vacant. Records still age. Debts still come due.

No Schrodinger’s NPCs. No frozen towns. No suspended economics because the player is elsewhere.

**Test**: Advance the simulation for a long interval with no human intervention. The world should still change in ways that remain causally traceable and locally explainable.

### 7. Locality of Motion, Interaction, and Communication

All physical interaction requires co-location or explicit range. All communication requires co-location or a physical carrier moving through the place graph: a witness, rumor chain, letter, notice, messenger, ledger, smoke plume, tracks, corpse, or other evidence carrier.

Agents, institutions, and planners may not query global truth on behalf of a character. A magistrate cannot know a caravan was attacked until some information carrier reaches them. A merchant cannot know a road is unsafe until they perceive evidence or receive a report. A bounty board cannot update itself from global state.

**Test**: For any belief, report, or institutional action, trace the path by which the relevant information arrived. If no path exists, the design violates locality.

### 8. Every Action Has Preconditions, Duration, Cost, and Occupancy

Nothing important is free and nothing important is instantaneous. Actions consume time, energy, materials, opportunities, attention, social availability, or tool access. They also occupy capacities. Travel occupies the body and exposes the agent to what happens en route. Conversation occupies all participants. Rest occupies time that could have been spent earning, guarding, or fleeing.

Long actions must unfold over time and remain interruptible. “Go to market” is not a teleporting atomic call. “Investigate robbery” is not a single instant state flip.

Whenever multiple actors can lawfully attempt the same scarce or exclusive affordance, the resolution mechanism must also be explicit: reservation, queue, grant, lock, contested race, or some other concrete world process. Planner intent is not silent control. “I planned to use the orchard” does not make the orchard unavailable to others.

**Test**: For any action or contested affordance, name its preconditions, its consumed resources, its occupied capacities, its duration, what can interrupt it, and how contention is resolved if more than one actor tries it. If you cannot, the action is too abstract.

### 9. Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model

A causal world needs an authoritative clock, a declared update regime, and explicit tie-break rules. The simulation must specify its temporal resolution, what can happen concurrently, when perception is sampled, when commitments are reconsidered, when effects become visible, and how same-time conflicts are resolved.

Synchronous, asynchronous, event-driven, fixed-step, hybrid, and queued models are all acceptable, but the choice must be explicit and justified as part of the world model, not left as accidental engine behavior. Tick order, thread order, frame delta, and container iteration order may not silently decide who saw the dropped coin first, who entered the doorway first, or whether two blows landed.

For every contested or concurrent case, the world must define a lawful resolution path: ordering rule, initiative rule, arbitration artifact, simultaneous resolution window, or event-queue semantics. Scheduling is not just performance machinery. It changes history.

**Test**: If changing system execution order, thread count, frame rate, or container iteration order changes world meaning without an explicit in-world reason, the design is invalid.

### 10. Outcomes Are Granular and Leave Aftermath

Actions are not binary success/fail toggles. They create partial outcomes, side effects, leftovers, and future hooks.

An ambush may kill some targets, wound others, scatter survivors, drop cargo, leave tracks, create rumors, trigger retaliation, and reshape route preferences. A failed theft may still create noise, suspicion, broken locks, bruises, and witness testimony. A completed purchase changes inventories, prices, hunger state, and available coin.

Failure is not a dead end. Failure is new state.

**Test**: If an action leaves the world almost unchanged except for a boolean flag, the model is too thin to support emergence.

### 11. Every Positive Feedback Loop Needs a Physical Dampener

Whenever A increases B and B increases A, a concrete limiting mechanism must exist in the world: resource exhaustion, fatigue, competition, seasonality, distance, maintenance cost, social pushback, succession rules, natural recovery, supply constraints, or other real dampeners.

Never solve runaway loops with invisible caps or clamps. If a crime wave cannot stop except by `min(crime, 1.0)`, the design is broken.

**Test**: For every amplifying loop, identify the world mechanism that slows, reverses, or saturates it. If the only dampener is a numeric cap, the design violates this principle.

### 12. Performance May Compress Computation, Never Causality

Optimization is allowed. Causal cheating is not.

Offscreen simulation may batch, summarize, sleep, or approximate only if causally relevant outcomes remain equivalent to the explicit model. You may compress the math. You may not compress away travel time, information delay, inventory depletion, injury recovery, or other state that agents could later observe and react to.

The same rule applies to save/load, replay, migration, and any other representation boundary. Boundaries may change encoding, batching, or scheduling strategy, never world meaning.

The rule is simple: performance may change how the machine computes a result, never what the world means.

**Test**: If an optimization or boundary causes an agent to observe a state that could not have arisen from any legal sequence of world events, the optimization is invalid.

### 13. Boundaries and External Inputs Are World Processes

Worldwake may model a region, not the whole universe. Therefore map edges, neighboring settlements, migrating herds, imported goods, weather fronts, refugees, taxes from outside powers, and other extra-local influences must enter through explicit boundary processes.

A border crossing, remote stock, scheduled convoy, seasonal influx, upstream event feed, or lower-fidelity neighboring region is valid only if it has named source regions, travel or transmission delay, capacities, observables, and failure modes. A hidden spawner that creates fully formed actors, goods, or threats with no accountable origin is not.

Off-map is not nowhere. It is either a lower-fidelity part of the same world or an explicitly modeled external driver. The same locality, provenance, and source/sink rules apply across the boundary even if representation changes.

**Test**: If an external arrival cannot answer “from where?”, “by what route or channel?”, “under what constraints?”, and “what evidence of that arrival exists?”, the boundary model is cheating.

---

## III. Knowledge, Belief, and Evidence

### 14. World State Is Not Belief State

Ground truth and agent knowledge are separate layers. Agents act on what they believe, remember, infer, suspect, and are told — not on what the simulation knows to be true.

A planner may consult only the agent’s accessible belief state, memory, and known plans. No AI may silently use omniscient world data to make “smarter” choices.

**Test**: If an agent can plan around a fact it has never perceived, inferred, remembered, or been told, the design violates this principle.

### 15. Knowledge Is Acquired Locally and Travels Physically

Knowledge enters an agent through perception, memory retrieval, inference, testimony, documents, traces, and other explicit carriers. Knowledge then moves through the world by physical or social transmission, with delay, distortion, source attribution, and possible loss.

Where relevance matters, beliefs must also carry provenance, claimed event time, acquisition time, confidence, freshness, and source-chain metadata sufficient for agents to discount stale rumor, reason about transmission delay, prefer direct evidence, compare conflicting witnesses, and ask not only “what do I believe?” but also “when do I think it happened?” and “when did I learn it?”

Witness testimony, posted notices, letters, ledgers, rumors, tracks, blood trails, empty shelves, missing items, and public speeches are not flavor. They are mechanisms of causal propagation.

**Test**: For any belief that changes an agent’s plan, identify how it was acquired, how it traveled, and what makes it more or less trustworthy than competing claims. If the answer is “the AI system knew,” the design violates this principle.

### 16. Ignorance, Uncertainty, and Contradiction Are First-Class

Agents must be able to not know, to suspect, to misremember, to hold stale beliefs, and to believe false or conflicting reports. Unknown is not false. Unobserved is not empty. Contradiction is not a system error.

Retention is not perfect or free. Beliefs may decay, be overwritten, or be evicted when time passes, memory is weak, or stronger evidence arrives.

The simulation must support cases where one witness says the beast fled east, another says west, and the town reacts imperfectly. It must support an owner believing their gold is home while the gold is already gone.

**Test**: If the architecture forces every proposition into a clean true/false value for each agent at all times, it is too crude for the target simulation.

### 17. Surprise Comes From Violated Expectation

Agents notice anomalies relative to prior expectation, commitment, claim, count, reservation, or routine. A missing stash matters because the owner expected gold there. A market shortage matters because a shopper expected food to be available. A sudden dragon attack interrupts a trip because the agent expected the route to be survivable.

This principle forbids cheap omniscience about absences. Agents do not detect “missing things” globally. They discover mismatch between belief and observation.

**Test**: If an agent can report theft without a prior expectation, claim, or memory concerning the missing goods, the design violates this principle.

### 18. Memory, Evidence, and Records Are World State

Memories, accusations, warrants, contracts, notices, ledgers, titles, debts, and other records are not UI-only abstractions. They are state that can be created, copied, transmitted, forgotten, destroyed, forged, or contested.

Evidence also includes physical aftermath: corpses, tracks, broken locks, spilled grain, scorch marks, blood, missing inventory, and location traces. These are how the world stays legible enough for agents to reason about it.

**Test**: If an important social or investigative process depends on a thing that does not exist anywhere in world state, the design violates this principle.

---

## IV. Agents, Institutions, and Social Order

### 19. Agent Symmetry

The engine makes no rule distinction between human-controlled and AI-controlled agents. Both use the same bodies, inventories, actions, preconditions, consequences, social rules, and world constraints. `ControlSource` changes only who chooses the next action, never what reality allows.

The human may swap into any agent without the world changing its laws.

The same restriction applies to normal player-facing information. Outside explicit debug, authoring, or replay tools, the interface may surface only what the currently controlled agent could lawfully perceive, infer, remember, or obtain from records and testimony. UI convenience must not become an omniscient side channel.

**Test**: Swap an agent from AI to human or human to AI. The simulation must continue with the same legal action set and the same rule enforcement.

### 20. Resource-Bounded Practical Reasoning Over Scripts

AI agents must reason as limited actors in a dynamic world, using beliefs, priorities, habits, skills, and commitments to choose actions. Plans exist to make reasoning tractable under limited time and limited knowledge, not to hard-script a performance.

Goals name desired world conditions, not privileged one-step solutions. Reaching them may require enabling subchains — travel, acquisition, queueing, bargaining, pickup, treatment, proof, or retreat — through the same lawful affordances everyone else uses.

The implementation may evolve — GOAP, utility systems, BDI, HTN, or hybrids are all acceptable — but the standard does not change: decisions must be explainable as what this agent, with this belief state and these priorities, would try to do.

Any planner formalism may encode only reusable lawful affordances, decomposition knowledge, or search control. It may not encode plot progression, scene-specific rails, target-specific success paths, or hidden exception logic that bypasses ordinary world causality. HTN methods, utility bonuses, candidate filters, and behavior tree branches are acceptable only when they express how this kind of agent pursues this kind of world condition under these beliefs — not how a desired story beat is supposed to happen.

To remain tractable, agents may use agent-local summaries, heuristics, and bounded lookahead derived from their accessible belief state. These abstractions are legal because they are part of the agent’s reasoning apparatus, not substitutes for authoritative world state. They must remain explainable in terms of what this agent has perceived, remembered, been told, or inferred.

**Test**: For any decision, you must be able to explain it as “Agent X chose Y because they believed Z and cared about Q.” If the explanation is “the behavior tree hit this node” or “the quest logic told them to,” the design violates this principle.

### 21. Intentions Are Revisable Commitments

Agents need commitments so they do not thrash between options every tick. But commitments are never rails. They are stable intentions held under assumptions.

Intent is not entitlement. A plan reserves nothing unless the world contains an explicit reservation, queue position, contract, assignment, or other claim artifact that other agents can observe or contest. Selecting a plan does not secretly hold the workstation, the bread, the corpse, the patient, or the road.

Agents must monitor the assumptions beneath an active intention and suspend, revise, or replace that intention when new local evidence invalidates it or when another actor lawfully changes the relevant world state. Hungry agent going to market sees dragon -> flee. Guard escorting caravan hears nearby bandits and may tighten formation, investigate, retreat, or continue depending on their beliefs and priorities.

**Test**: If an agent cannot abandon or revise a plan when its assumptions are broken by new information or by another actor legitimately taking the opportunity first, the architecture cannot support emergent interruption.

### 22. Agent Diversity Through Concrete Variation

Agents in the same role must differ in needs, skills, values, loyalties, courage, greed, patience, memory reliability, perception fidelity, and tolerance for risk or ambiguity. These differences come from concrete per-agent parameters, histories, injuries, relationships, and learned experience.

Homogeneous populations collapse into herd behavior and single-path outcomes. Diversity is not garnish. It is one of the engines of emergence.

**Test**: Two agents with the same role and similar beliefs should still sometimes choose differently because they are not the same person.

### 22A. Learning, Habits, and Preference Shifts Are Concrete State

Agents may adapt through experience, but adaptation and learning must occur through explicit state change: memory update, skill change, trust revision, habit reinforcement, blocked-intent record, source reliability shift, route preference, institutional doctrine, or similar concrete state. Learning may change what an agent notices, prefers, avoids, retries, or suppresses.

These learned structures must have accountable origin, scope, and decay. The model should be able to say what experience produced the update, when it was acquired, whose state it belongs to, what can revise it, and how it fades or is overwritten.

Agent-local learned summaries are legal even when abstract — route danger expectation, seller reliability, social trust, habit strength — because they are not world truth. They are fallible decision state owned by a particular agent or institution. What is forbidden is hidden global adaptation that silently rewrites behavior for drama or convenience.

**Test**: If the explanation for a changed future choice is only “the AI learned,” without an inspectable experience path and a concrete stored update, the design is cheating.

### 23. Roles, Offices, and Institutions Are World State

Authority is not a global singleton service. It is a socially recognized role embedded in places, organizations, rules, records, and material resources.

A magistrate, captain, guild master, steward, priest, caravan master, and town council exist as agents or offices with jurisdiction, duties, limits, succession rules, and often budgets or assets. A treasury can be empty. An office can be vacant. A jurisdiction can stop at the town gate. A policy can differ by settlement.

Institutions act through agents, artifacts, and rules — never through omniscient manager code.

**Test**: If “the town” can do something without a specific office, rule, record, place, or actor that makes it happen, the design violates this principle.

### 24. Ownership, Custody, Access, Obligation, and Jurisdiction Are Distinct

Possession is not ownership. Ownership is not permission. Permission is not capability. Debt is not payment. Claim is not custody. Jurisdiction is not universal.

These distinctions apply to organizations as well as people. A faction, guild, temple, household, or state may own something that an individual member can access, steward, tax, or use without personally owning it.

To model theft, trade, taxation, inheritance, confiscation, trespass, and robbery correctly, the simulation must separate:
- who owns a thing,
- who currently holds it,
- who can access it,
- who is owed something related to it,
- and which institution can adjudicate disputes about it.

This applies to places, containers, offices, and records as much as to goods.

**Test**: If the model cannot represent “the gold is the guild’s, the chest holds it, my servant has the key, my office lets me open it, and the city watch has jurisdiction,” it is too coarse for the target world.

### 25. Social Artifacts Are First-Class: Contracts, Notices, Bounties, Debts, Accusations, Rumors, Warrants

There is no special quest system. There are only world entities and records that people create, discover, believe, dispute, ignore, accept, or fulfill.

A bounty is a public offer or institutional order with an issuer, conditions, reward source, proof requirements, place of posting, expiration, and possible claimants. A rumor is a transmitted claim with a source and credibility. A robbery report is both a record and a social act. A debt can pressure future behavior even when no coin moves right now.

If these are only UI abstractions or hidden controller state, emergence dies.

**Test**: If a bounty can exist without an issuer, a record, a place, conditions, and a possible reward source, it is not a world object — it is scripted content.

### 25A. Artifact Lifecycle, Visibility, and Actionability Are Distinct

Records and artifacts can remain real after they stop authorizing action. A bounty may be expired yet still posted. A sale listing may persist until invalidated by departure, death, or unstaging. A warning may remain visible after the threat has moved. Evidence may decay before it disappears.

Every artifact class must declare its lifecycle states, transitions, timestamps, invalidators, observers, and legal effects. Existence, visibility, credibility, legality, and actionability are separate axes. Do not collapse them into a single boolean.

Lifecycle transitions must occur through explicit world processes: expiry, fulfillment, revocation, destruction, supersession, departure, death, adjudication, consultation, or decay. Cleanup code may remove representation only after the world meaning of the state transition already exists.

**Test**: If removing an artifact’s active effect requires deleting the artifact itself, or if an artifact can keep generating lawful action after its basis ended because the model knows only “exists/does not exist,” the artifact model is too crude.

---

## V. System Architecture

### 26. Systems Interact Through State, Not Through Each Other

Systems do not imperatively command each other to force outcomes. They read authoritative state, local beliefs, and prior records; they write new state, effects, and records. Influence travels through state mutation and event history, not through hidden cross-system authority.

Shared domain services are allowed when they are generic computations over authoritative state — pathfinding, line-of-sight, legality checks, reservation arbitration, pricing calculations, ballistics, planner search, or similar solvers. Such services compute lawful consequences; they do not grant exceptions or bypass the world model.

Combat creates wounds. Needs react to wounds. Planning reacts to needs. Institutions react to reports. None of these systems should need privileged knowledge of each other’s internals to make the world work.

**Test**: If one system must directly invoke another system’s privileged behavior to make the world work, the architecture is too coupled for maximal emergence.

### 27. Derived Summaries Are Caches, Never Truth

Derived summaries used for performance, UI, or planning convenience — threat heatmaps, pathing cost fields, inventory rollups, market snapshots, planner-side reputation estimates, reservation indices, and similar aggregates — may exist only as views over concrete source state. They must be invalidated when source state changes and always remain replaceable by recomputation.

Do not confuse caches with social artifacts. A posted warning, rumor, public notice, public reputation record, queue token, grant, or reservation artifact is not a cache if it exists as a concrete object, belief, notice, or record in the world. Such artifacts may be stale, false, disputed, or destroyed, but they are real world state precisely because agents can perceive and act on them.

A cached danger estimate is acceptable. A danger estimate that becomes more real than the actual bandits is not.

**Test**: Delete the cache and recompute from source state. If the world’s meaning changes, the cache was illegally promoted to truth.

### 28. No Backward Compatibility in Live Authority Paths

Do not preserve dead abstractions, alias paths, compatibility layers, deprecated shims, or legacy systems inside the live authoritative simulation simply because old code once depended on them. When the design changes, the live authority path changes with it. Broken callers get updated or removed.

Compatibility may exist at boundaries — save migration, import/export, tooling, replay decoding — only if it normalizes into the current model before the world advances. Two live authoritative representations of the same fact may not coexist.

This keeps the simulation honest and prevents fossilized logic from silently bypassing the current world model.

**Test**: If you are adding a wrapper so an obsolete abstraction can continue to mutate live world meaning beside the new one, stop and pay the migration cost now.

### 29. Debuggability Is a Product Feature

Emergence without introspection is indistinguishable from bugs.

The simulation must support questions such as:
- Why did this agent do that?
- Why did this caravan take this road?
- Why is this stash empty?
- Why was this bounty posted?
- Why was this bounty not posted?
- Why was the reward unpaid?
- Who last held this item?
- Who knows about this event?

The answers must be reconstructable from state, beliefs, records, and causal history — not guessed by developers.

**Test**: For any nontrivial event chain, you must be able to inspect both the causal path and the knowledge path separately.

### 29A. Causal History Is Authoritative, Append-Only, and Queryable

Meaningful world changes must leave stable historical records. Events may be summarized, indexed, or compacted for storage, but the authoritative history must behave as append-only: later evidence may supersede an earlier claim, refute a belief, or close a case, yet it does not erase that the earlier event, claim, belief, or judgment occurred.

History must preserve enough structure to answer provenance questions over entities, activities, and responsible agents: what happened, when, where, who acted, what prior event or state it depended on, and what aftermath or records it created. Debug traces are not enough if they are optional, transient, or outside the authoritative model.

This applies equally to social and physical history. A false accusation remains part of history after exoneration. An expired bounty remains a record of having once been active. A looted corpse still has a prior holder chain. A canceled plan still has a blocker or invalidator.

**Test**: If a later state can only be explained by reading ad hoc logs or source code, or if contradicting later facts require rewriting prior history instead of appending new history, the model is not explainable enough.

### 30. Every New System Spec Must Declare Its Causal Hooks

Every system proposal must explicitly state:
1. what specific missing downstream consequence, scenario class, or target pattern motivates the system, and why existing systems cannot already produce it,
2. what concrete entities, relations, and records it introduces,
3. what actions or world processes mutate them,
4. what information it produces, how that information travels, and who can observe it,
5. what quantities it conserves, transfers, transforms, creates, or destroys, and by what source/sink paths,
6. what scarce capacities, exclusive affordances, reservations, queues, or claims it introduces, and how contention, expiry, and invalidation work,
7. what partial failures, degraded states, and aftermath it creates,
8. what positive feedback loops it amplifies,
9. what physical dampeners limit those loops,
10. what agent-local or institutional learning, memory, habit, trust, reliability, or preference updates it creates, how those updates are acquired, revised, and decay, and which of them are summaries versus authoritative state,
11. how agents can become wrong about it, how they can correct those errors, and what provenance, freshness, or source-chain markers matter,
12. what lifecycle states its entities and artifacts can occupy, what transitions move them between states, and how visibility, legality, and actionability differ across those states,
13. what temporal and spatial resolution it assumes, what scheduling regime it depends on, and how simultaneity and tie-breaking are resolved,
14. what boundary conditions, external drivers, or off-map interfaces affect it, and how those inputs are represented, delayed, observed, and rate-limited,
15. what derived views, caches, or optimizations are allowed and what authoritative source state they derive from,
16. what causal records, event identities, and provenance links it emits so later inspection can reconstruct both the causal path and the knowledge path,
17. what target patterns, invariants, regression cases, and falsification checks will be used to tell whether the system is behaving credibly,
18. and what must survive save/load, replay, and offscreen compression without changing world meaning.

If a proposed system cannot answer those questions, it is not specified well enough to join this simulation.

**Test**: A system spec that has behavior but no declared consequences, knowledge flow, contention rules, or failure states is incomplete by definition.

### 31. Validation and Falsification Are First-Class

Interesting-looking output is not evidence that the model is right. Every subsystem and every scenario class must declare multiple independent patterns where possible — from local traces to aggregate world behavior — the artifacts it must never produce, the parameters most likely to destabilize it, and the traces by which developers will detect failure.

Canonical scenario success is necessary but insufficient. The architecture must also support adversarial sampling, sensitivity sweeps, causal trace inspection, and comparison against simplified referents or prior implementations when appropriate.

A believable run is not enough. The standard is fitness for purpose under explicit evaluation criteria.

**Test**: If a feature can only be judged by “it looked plausible in a run I watched,” it is not validated enough to join the authoritative simulation.

---

## VI. Canonical Regression Scenarios

These are permanent acceptance tests for the architecture. They are not authored sequences. They are scenario classes the generic simulation must be capable of producing.

### A. Beast Starvation -> Caravan Attack -> Report -> Bounty -> Hunt -> Reward

The architecture must be able to produce this chain from general-purpose systems:

1. A beast has territory, movement, needs, and food sources.
2. Local food becomes insufficient through actual depletion or competition.
3. The beast chooses to expand range or travel.
4. A caravan physically traverses a route through overlapping space and time.
5. The beast attacks because of local perception, appetite, aggression, or threat logic.
6. Combat produces concrete aftermath: deaths, survivors, wounds, dropped cargo, tracks, fear, damaged property.
7. Survivors carry beliefs and evidence to a settlement through actual travel.
8. An office-holder or institution receives the report, has jurisdiction, and decides whether to act based on rules, priorities, and available resources.
9. A bounty or notice is created as a real record or artifact, with issuer, terms, reward source, proof requirements, and location.
10. Other agents learn of it by seeing, hearing, reading, or being told.
11. One or more adventurers choose whether to pursue it based on their beliefs, needs, courage, skills, and competing commitments.
12. The hunt occurs through actual search, tracking, travel, and confrontation.
13. Completion is verified through accepted evidence or institutional judgment.
14. Payment comes from an actual treasury, sponsor, or obligated issuer.

**Failure smell**: Any implementation that shortcuts this chain with a hidden `post_beast_bounty()` trigger or a dedicated quest pipeline has failed the design goal.

### B. Hungry Agent -> Market Trip -> Dragon Attack -> Interrupted Plan -> Retreat

The architecture must be able to produce this chain from generic planning and interruption:

1. The agent has hunger and believes food can be acquired at the market.
2. The agent adopts a travel-and-purchase intention.
3. Travel is a duration-bearing action that exposes the agent to local events.
4. A dragon enters local perception range, or a credible warning reaches the agent by an explicit channel.
5. The agent’s safety assumptions become invalid.
6. The agent re-evaluates priorities and may flee, hide, seek allies, continue anyway, or change route depending on beliefs and temperament.
7. The abandoned or delayed food-seeking plan remains available for later resumption or replacement.

**Failure smell**: If “go to market” is atomic, if plans cannot be interrupted, or if the agent responds to a dragon it could not possibly know about, the architecture is wrong.

### C. Stored Gold -> Empty Stash -> Discovery -> Robbery Report

The architecture must be able to produce this chain from ownership, belief, and evidence systems:

1. An agent acquires gold through some prior world process.
2. The gold exists as concrete value or items in a specific container or location.
3. Ownership, custody, access rights, and location are represented separately.
4. Another agent or process moves, steals, spends, confiscates, inherits, destroys, or misrecords the gold through actual state transitions.
5. The original owner retains a belief that the gold is still present until new evidence arrives.
6. The owner later inspects the stash and observes a mismatch between belief and reality.
7. That mismatch updates belief and may trigger search, accusation, reporting, concealment, retaliation, or resignation depending on the agent and the institutions available.
8. Authorities can only react if the report reaches them and if their jurisdiction, priorities, and procedures support action.

**Failure smell**: If the gold can disappear without a transfer or destruction path, or if the owner can know theft occurred without prior expectation or new evidence, the architecture is too abstract.

### D. Rumor -> Travel -> Empty Source -> Discovery -> Belief Correction -> Replan

The architecture must be able to produce this chain from belief provenance, travel, perception, and replanning:

1. An agent acquires a belief — from rumor, testimony, memory, or stale prior observation — that a desired resource, person, opportunity, or danger exists at a specific place.
2. That belief carries source, age, and credibility rather than masquerading as ground truth.
3. The agent adopts a plan based on that belief.
4. Before arrival, the relevant world state changes through ordinary local processes.
5. The agent reaches the place and locally observes a mismatch between expectation and reality.
6. That mismatch produces new evidence with explicit provenance rather than teleporting omniscient correction into belief state.
7. The agent revises, abandons, or replaces the old plan based on the new evidence.
8. Other agents can continue to act on the stale report until new evidence reaches them by lawful channels.

**Failure smell**: If the agent is corrected by global truth before any new carrier arrives, if stale beliefs can never survive long enough to waste work, or if contradictory reports cannot coexist, the architecture is too clean for the target world.

### E. Competing Claimants -> Queue or Race -> Expiry/Prune -> Next Actor Acts

The architecture must be able to produce this chain from explicit contention, scarcity, and revisable planning:

1. Multiple agents perceive the same scarce resource, facility, target, or newly materialized output and each forms a lawful intention to use it.
2. Those intentions do not silently reserve the opportunity.
3. Access is resolved through an explicit race, reservation, queue, grant, lock, or other concrete world mechanism.
4. One claimant acts first or receives access while others wait, lose, detour, or replan.
5. Claims can expire, be abandoned, be invalidated by death or incapacity, or be displaced by higher-priority needs.
6. The underlying resource or capacity changes only through the actual winning action, not through planner bookkeeping or hypothetical future consumption.
7. Waiting or losing agents continue from the new world state and may retry, reroute, choose a fallback, or give up.
8. Any resulting line, grant, blocker, or reservation is inspectable world state rather than invisible runtime magic.

**Failure smell**: If selecting a plan secretly guarantees future access, if dead claimants continue blocking the line, or if contention is resolved only by hidden tick order with no inspectable world state, the architecture is wrong.

### F. Office Vacancy -> Succession Delay -> Patrol Gap -> Route Predation

The architecture must be able to produce this chain from generic institutions, succession, budgeting, labor allocation, and opportunism:

1. A settlement office with real duties, jurisdiction, and assets exists through a specific office-holder or chain of delegated agents.
2. The office-holder dies, disappears, resigns, is removed, or becomes incapacitated through ordinary world processes.
3. The office becomes vacant or partially degraded according to actual succession rules, records, and local recognition.
4. Duties previously coordinated through that office are delayed, partially performed, contested, or dropped.
5. Patrol coverage, escort assignment, bounty approval, or treasury release weakens because of that vacancy rather than because a hidden scenario flag fired.
6. Other agents notice the resulting gap only through local evidence, reports, changed routines, or observed non-performance.
7. Opportunistic actors exploit the gap through ordinary planning and local information.
8. The institution either recovers through succession, delegation, improvisation, outside intervention, or continued decline.

**Failure smell**: If “the town” seamlessly continues performing office-dependent functions without a lawful successor, delegated actor, or record path, the institutional model is fake.

### G. False Rumor -> Wrongful Accusation -> Contested Evidence -> Correction or Miscarriage

The architecture must be able to produce this chain from generic belief propagation, contradiction, and institutional judgment:

1. A false or distorted claim enters circulation through rumor, error, forgery, panic, bias, or deliberate deception.
2. Different agents acquire that claim through explicit carriers, with source, age, and confidence metadata that may be incomplete or wrong.
3. One or more agents act on the false claim by accusing, avoiding, pursuing, searching, denying access, or reporting it.
4. Institutions or other agents respond according to their beliefs, procedures, incentives, and available evidence rather than omniscient truth.
5. New evidence later appears through witness conflict, alibi, physical traces, ledger mismatch, confession, or failed prediction.
6. Different actors update at different times depending on what information reaches them.
7. The world supports both correction and non-correction: exoneration, apology, retaliation, bureaucratic inertia, stubborn belief, or punishment that continues despite the truth.
8. The record of the accusation, the later evidence, and the timing of each must remain inspectable.

**Failure smell**: If false reports cannot propagate into real downstream action, or if institutions are corrected instantly by ground truth without an information path, the belief architecture is too clean.

### H. Remote Shock -> Delayed Arrival Failure -> Local Shortage -> Substitution or Exit

The architecture must be able to produce this chain from boundary processes, transport delay, inventories, and replanning:

1. A settlement depends in part on goods, people, animals, or information arriving from beyond the currently simulated core area.
2. Those arrivals are represented through explicit boundary processes, remote stocks, and travel or transmission delay.
3. A disruption occurs in the external region or along the boundary path through ordinary world processes: raid, storm, embargo, bridge collapse, disease, migration pressure, or upstream shortage.
4. The expected arrival is delayed, reduced, rerouted, or canceled through that causal chain.
5. Local agents continue to act on their prior expectations until local evidence or reports update them.
6. Inventories tighten through actual consumption and failed replenishment, not a hidden scarcity flag.
7. Buyers, institutions, and suppliers react by substitution, rationing, queueing, hoarding, theft, departure, repricing, or aid requests depending on beliefs and incentives.
8. Recovery requires actual new inflow, local production, demand destruction, or population movement.

**Failure smell**: If off-map dependence exists only when a content script wants drama, or if replacement goods appear without a lawful source path, the boundary model is fake.

---

## VII. Final Rule of Thumb

When in doubt, choose the design that adds a new carrier of consequence, preserves locality and partial observability, keeps beliefs separate from truth, preserves accounted source/sink paths, resolves contention through world state rather than planner entitlement, and creates more downstream reactions with less special-case code. Reject the design that produces content by exception, authority by singleton, knowledge by omniscience, guarantees by hidden planner state, or outcomes by fiat.