# Worldwake Foundational Principles

These principles define what Worldwake is optimizing for: explainable emergence. The target is not mere surprise, noise, or content density. The target is chains of consequence that are surprising, legible, and fully traceable after the fact.

Designers author the world's nouns, laws, institutions, and initial conditions. They do not author outcomes. Every system, feature, content addition, and optimization must be judged against these principles. They are non-negotiable unless explicitly revised by the project owner. All contributors — human and AI — must internalize them before making design decisions.

---

## I. Causal Standard

### 1. Maximal Emergence Through Local Causality

Worldwake exists to produce emergent behavior through interacting systems and agents, never through authored sequences, hidden quest logic, or one-off story triggers. An event is valid only if it arose from prior world state, agent belief, institutional rule, or natural process already present in the simulation.

Authoring beasts, hunger, roads, caravans, towns, offices, and bounty procedures is correct. Authoring “a beast attack happens so adventurers have content” is forbidden.

**Test**: If the only honest explanation for an event is “the game decided something interesting should happen now,” the design violates this principle.

### 2. No Ungrounded Triggers or Probabilities

No outcome may bottom out at a naked designer dial such as `chanceOfEncounter`, `spawnRate`, `crimeChance`, `questSpawnChance`, or `eventProbability`.

Randomness is allowed only when it stands in for hidden local microstate, noisy perception, uncertain execution, or real variation the simulation does not model explicitly. In those cases, the distribution must still be a function of concrete, local state. Randomness must be seeded, attributable, and never used as a drama generator. Given the same seed and the same causal history, the simulation should reproduce the same outcome. Different seeds may diverge, but only through those same declared local uncertainty paths.

Utility weights, need rates, fear sensitivity, memory fidelity, and skill parameters may exist as concrete agent properties. “Interesting thing happens here 30% of the time” may not.

**Test**: If changing a single abstract constant can create or remove an event without any corresponding change in world state, the design violates this principle.

### 3. Concrete State Over Abstract Scores

Prefer modeling the thing itself over a score that represents it. Danger should come from actual threats on routes, not `danger_score`. Scarcity should come from inventories, queues, failed purchases, and unmet needs, not `scarcity_score`. A price spike should emerge from actual stock, seller beliefs, buyer pressure, and substitute availability, not from `if stock < 50% then price *= 1.5`.

Abstract summaries are allowed only as derived views or caches. They may never become the source of truth.

**Test**: If a system relies on a number that cannot be traced back to concrete entities, relations, or events, the design violates this principle.

### 4. Persistent Identity, Object Permanence, and Explicit Transfer

Every meaningful thing in the simulation has stable identity while it exists: agents, beasts, items, containers, wounds, corpses, notices, contracts, offices, titles, ledgers, debts, rooms, roads, and places. Things do not wink in or out of being because they are offscreen or inconvenient.

Movement, splitting, merging, damage, consumption, creation, transfer, and destruction must be explicit world processes. If gold leaves a stash, there must have been a theft, payment, transfer, misplacement, destruction, or prior accounting error. If a bounty exists, someone or some institution must have created it at a place and time. If a caravan no longer has cargo, that cargo must be somewhere else, destroyed, or consumed.

For quantities the simulation treats as conserved or explicitly accounted — coin, goods, bodies, ingredients, outputs, or claim-like balances — every increase, decrease, split, merge, creation, destruction, and transformation must have an explicit source or sink path. Harvests draw from sources. Crafts transform inputs into outputs. Regeneration, decay, inheritance, spoilage, write-offs, births, and minting must be equally explicit if they exist.

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

### 9. Outcomes Are Granular and Leave Aftermath

Actions are not binary success/fail toggles. They create partial outcomes, side effects, leftovers, and future hooks.

An ambush may kill some targets, wound others, scatter survivors, drop cargo, leave tracks, create rumors, trigger retaliation, and reshape route preferences. A failed theft may still create noise, suspicion, broken locks, bruises, and witness testimony. A completed purchase changes inventories, prices, hunger state, and available coin.

Failure is not a dead end. Failure is new state.

**Test**: If an action leaves the world almost unchanged except for a boolean flag, the model is too thin to support emergence.

### 10. Every Positive Feedback Loop Needs a Physical Dampener

Whenever A increases B and B increases A, a concrete limiting mechanism must exist in the world: resource exhaustion, fatigue, competition, seasonality, distance, maintenance cost, social pushback, succession rules, natural recovery, supply constraints, or other real dampeners.

Never solve runaway loops with invisible caps or clamps. If a crime wave cannot stop except by `min(crime, 1.0)`, the design is broken.

**Test**: For every amplifying loop, identify the world mechanism that slows, reverses, or saturates it. If the only dampener is a numeric cap, the design violates this principle.

### 11. Performance May Compress Computation, Never Causality

Optimization is allowed. Causal cheating is not.

Offscreen simulation may batch, summarize, sleep, or approximate only if causally relevant outcomes remain equivalent to the explicit model. You may compress the math. You may not compress away travel time, information delay, inventory depletion, injury recovery, or other state that agents could later observe and react to.

The same rule applies to save/load, replay, migration, and any other representation boundary. Boundaries may change encoding, batching, or scheduling strategy, never world meaning.

The rule is simple: performance may change how the machine computes a result, never what the world means.

**Test**: If an optimization or boundary causes an agent to observe a state that could not have arisen from any legal sequence of world events, the optimization is invalid.

---

## III. Knowledge, Belief, and Evidence

### 12. World State Is Not Belief State

Ground truth and agent knowledge are separate layers. Agents act on what they believe, remember, infer, suspect, and are told — not on what the simulation knows to be true.

A planner may consult only the agent’s accessible belief state, memory, and known plans. No AI may silently use omniscient world data to make “smarter” choices.

**Test**: If an agent can plan around a fact it has never perceived, inferred, remembered, or been told, the design violates this principle.

### 13. Knowledge Is Acquired Locally and Travels Physically

Knowledge enters an agent through perception, memory retrieval, inference, testimony, documents, traces, and other explicit carriers. Knowledge then moves through the world by physical or social transmission, with delay, distortion, source attribution, and possible loss.

Where relevance matters, beliefs must also carry provenance, acquisition time, confidence, and freshness or chain metadata sufficient for agents to discount stale rumor, prefer direct evidence, and reason about who said what.

Witness testimony, posted notices, letters, ledgers, rumors, tracks, blood trails, empty shelves, missing items, and public speeches are not flavor. They are mechanisms of causal propagation.

**Test**: For any belief that changes an agent’s plan, identify how it was acquired, how it traveled, and what makes it more or less trustworthy than competing claims. If the answer is “the AI system knew,” the design violates this principle.

### 14. Ignorance, Uncertainty, and Contradiction Are First-Class

Agents must be able to not know, to suspect, to misremember, to hold stale beliefs, and to believe false or conflicting reports. Unknown is not false. Unobserved is not empty. Contradiction is not a system error.

Retention is not perfect or free. Beliefs may decay, be overwritten, or be evicted when time passes, memory is weak, or stronger evidence arrives.

The simulation must support cases where one witness says the beast fled east, another says west, and the town reacts imperfectly. It must support an owner believing their gold is home while the gold is already gone.

**Test**: If the architecture forces every proposition into a clean true/false value for each agent at all times, it is too crude for the target simulation.

### 15. Surprise Comes From Violated Expectation

Agents notice anomalies relative to prior expectation, commitment, claim, count, reservation, or routine. A missing stash matters because the owner expected gold there. A market shortage matters because a shopper expected food to be available. A sudden dragon attack interrupts a trip because the agent expected the route to be survivable.

This principle forbids cheap omniscience about absences. Agents do not detect “missing things” globally. They discover mismatch between belief and observation.

**Test**: If an agent can report theft without a prior expectation, claim, or memory concerning the missing goods, the design violates this principle.

### 16. Memory, Evidence, and Records Are World State

Memories, accusations, warrants, contracts, notices, ledgers, titles, debts, and other records are not UI-only abstractions. They are state that can be created, copied, transmitted, forgotten, destroyed, forged, or contested.

Evidence also includes physical aftermath: corpses, tracks, broken locks, spilled grain, scorch marks, blood, missing inventory, and location traces. These are how the world stays legible enough for agents to reason about it.

**Test**: If an important social or investigative process depends on a thing that does not exist anywhere in world state, the design violates this principle.

---

## IV. Agents, Institutions, and Social Order

### 17. Agent Symmetry

The engine makes no rule distinction between human-controlled and AI-controlled agents. Both use the same bodies, inventories, actions, preconditions, consequences, social rules, and world constraints. `ControlSource` changes only who chooses the next action, never what reality allows.

The human may swap into any agent without the world changing its laws.

**Test**: Swap an agent from AI to human or human to AI. The simulation must continue with the same legal action set and the same rule enforcement.

### 18. Resource-Bounded Practical Reasoning Over Scripts

AI agents must reason as limited actors in a dynamic world, using beliefs, priorities, habits, skills, and commitments to choose actions. Plans exist to make reasoning tractable under limited time and limited knowledge, not to hard-script a performance.

Goals name desired world conditions, not privileged one-step solutions. Reaching them may require enabling subchains — travel, acquisition, queueing, bargaining, pickup, treatment, proof, or retreat — through the same lawful affordances everyone else uses.

The implementation may evolve — GOAP, utility systems, BDI, HTN, or hybrids are all acceptable — but the standard does not change: decisions must be explainable as what this agent, with this belief state and these priorities, would try to do.

**Test**: For any decision, you must be able to explain it as “Agent X chose Y because they believed Z and cared about Q.” If the explanation is “the behavior tree hit this node” or “the quest logic told them to,” the design violates this principle.

### 19. Intentions Are Revisable Commitments

Agents need commitments so they do not thrash between options every tick. But commitments are never rails. They are stable intentions held under assumptions.

Intent is not entitlement. A plan reserves nothing unless the world contains an explicit reservation, queue position, contract, assignment, or other claim artifact that other agents can observe or contest. Selecting a plan does not secretly hold the workstation, the bread, the corpse, the patient, or the road.

Agents must monitor the assumptions beneath an active intention and suspend, revise, or replace that intention when new local evidence invalidates it or when another actor lawfully changes the relevant world state. Hungry agent going to market sees dragon -> flee. Guard escorting caravan hears nearby bandits and may tighten formation, investigate, retreat, or continue depending on their beliefs and priorities.

**Test**: If an agent cannot abandon or revise a plan when its assumptions are broken by new information or by another actor legitimately taking the opportunity first, the architecture cannot support emergent interruption.

### 20. Agent Diversity Through Concrete Variation

Agents in the same role must differ in needs, skills, values, loyalties, courage, greed, patience, memory reliability, perception fidelity, and tolerance for risk or ambiguity. These differences come from concrete per-agent parameters, histories, injuries, relationships, and learned experience.

Homogeneous populations collapse into herd behavior and single-path outcomes. Diversity is not garnish. It is one of the engines of emergence.

**Test**: Two agents with the same role and similar beliefs should still sometimes choose differently because they are not the same person.

### 21. Roles, Offices, and Institutions Are World State

Authority is not a global singleton service. It is a socially recognized role embedded in places, organizations, rules, records, and material resources.

A magistrate, captain, guild master, steward, priest, caravan master, and town council exist as agents or offices with jurisdiction, duties, limits, succession rules, and often budgets or assets. A treasury can be empty. An office can be vacant. A jurisdiction can stop at the town gate. A policy can differ by settlement.

Institutions act through agents, artifacts, and rules — never through omniscient manager code.

**Test**: If “the town” can do something without a specific office, rule, record, place, or actor that makes it happen, the design violates this principle.

### 22. Ownership, Custody, Access, Obligation, and Jurisdiction Are Distinct

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

### 23. Social Artifacts Are First-Class: Contracts, Notices, Bounties, Debts, Accusations, Rumors, Warrants

There is no special quest system. There are only world entities and records that people create, discover, believe, dispute, ignore, accept, or fulfill.

A bounty is a public offer or institutional order with an issuer, conditions, reward source, proof requirements, place of posting, expiration, and possible claimants. A rumor is a transmitted claim with a source and credibility. A robbery report is both a record and a social act. A debt can pressure future behavior even when no coin moves right now.

If these are only UI abstractions or hidden controller state, emergence dies.

**Test**: If a bounty can exist without an issuer, a record, a place, conditions, and a possible reward source, it is not a world object — it is scripted content.

---

## V. System Architecture

### 24. Systems Interact Through State, Not Through Each Other

Systems do not call each other’s logic directly to force outcomes. They read world state, local beliefs, and prior records; they write new state, effects, and records. Influence travels through state mutation and event history, not through cross-system command paths.

Combat creates wounds. Needs react to wounds. Planning reacts to needs. Institutions react to reports. None of these systems should need to know each other’s internal logic.

**Test**: If one system must directly invoke another system’s behavior to make the world work, the architecture is too coupled for maximal emergence.

### 25. Derived Summaries Are Caches, Never Truth

Threat maps, route advisories, inventory summaries, market views, reputation views, reservation lists, and other aggregates may exist for performance, UI, or planning convenience. But they must be derived from concrete source state, invalidated when source state changes, and always replaceable by recomputation.

A cached danger estimate is acceptable. A danger estimate that becomes more real than the actual bandits is not.

**Test**: Delete the cache and recompute from source state. If the world’s meaning changes, the cache was illegally promoted to truth.

### 26. No Backward Compatibility in Live Authority Paths

Do not preserve dead abstractions, alias paths, compatibility layers, deprecated shims, or legacy systems inside the live authoritative simulation simply because old code once depended on them. When the design changes, the live authority path changes with it. Broken callers get updated or removed.

Compatibility may exist at boundaries — save migration, import/export, tooling, replay decoding — only if it normalizes into the current model before the world advances. Two live authoritative representations of the same fact may not coexist.

This keeps the simulation honest and prevents fossilized logic from silently bypassing the current world model.

**Test**: If you are adding a wrapper so an obsolete abstraction can continue to mutate live world meaning beside the new one, stop and pay the migration cost now.

### 27. Debuggability Is a Product Feature

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

### 28. Every New System Spec Must Declare Its Causal Hooks

Every system proposal must explicitly state:
1. what concrete entities, relations, and records it introduces,
2. what actions or world processes mutate them,
3. what information it produces, how that information travels, and who can observe it,
4. what quantities it conserves, transfers, transforms, creates, or destroys, and by what source/sink paths,
5. what scarce capacities, exclusive affordances, reservations, queues, or claims it introduces, and how contention, expiry, and invalidation work,
6. what partial failures and aftermath states it creates,
7. what positive feedback loops it amplifies,
8. what physical dampeners limit those loops,
9. what derived views or optimizations are allowed,
10. how agents can become wrong about it, how they can correct those errors, and what provenance or freshness markers matter,
11. and what must survive save/load, replay, and offscreen compression without changing world meaning.

If a proposed system cannot answer those questions, it is not specified well enough to join this simulation.

**Test**: A system spec that has behavior but no declared consequences, knowledge flow, contention rules, or failure states is incomplete by definition.

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

---

## VII. Final Rule of Thumb

When in doubt, choose the design that adds a new carrier of consequence, preserves locality and partial observability, keeps beliefs separate from truth, preserves accounted source/sink paths, resolves contention through world state rather than planner entitlement, and creates more downstream reactions with less special-case code. Reject the design that produces content by exception, authority by singleton, knowledge by omniscience, guarantees by hidden planner state, or outcomes by fiat.