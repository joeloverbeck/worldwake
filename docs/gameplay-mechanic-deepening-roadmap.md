# Gameplay Mechanic Deepening Roadmap

## Purpose

This document organizes the current Worldwake gameplay mechanics into coherent clusters for future deep-research and deepening passes.

It is not a feature wishlist. It is not a final design for each mechanic. It is a guide for deciding which existing mechanics should be researched together, why they collide causally, what current tests/scenarios prove, and what future scenario-backed validation should eventually cover.

The default universe is the current active feature set implied by:

1. `docs/FOUNDATIONS.md`
2. `docs/scenario-roadmap.md`
3. `.github/workflows/*`
4. Active scenario/test files invoked by those workflows
5. Current non-archive code/docs referenced by the above

Do not use `archive/*` to discover mechanics. Archived files may be consulted only when a current non-archive file explicitly names a specific archived file as relevant, and archived material remains lower-trust prior art than active code/docs.

## Relationship to `docs/FOUNDATIONS.md`

`docs/FOUNDATIONS.md` is the design constitution for all mechanic work.

Every mechanic deepening pass must preserve these standards:

- events arise from prior world state, agent belief, institutional rule, or natural process;
- meaningful things have persistent identity;
- quantities and objects have explicit source/sink/transfer paths;
- agents act from belief, memory, testimony, records, local observation, or lawful boundary artifacts;
- communication and knowledge travel through physical or social carriers;
- actions have preconditions, duration, cost, occupancy, and interruption surfaces;
- contention is resolved by explicit world processes, not silent planner intent;
- records, evidence, notices, ledgers, wounds, corpses, debts, claims, and social artifacts are world state;
- AI decisions must be explainable as resource-bounded reasoning over lawful affordances, not scripts.

A mechanic that cannot satisfy `docs/FOUNDATIONS.md` should be redesigned, quarantined, or deleted rather than patched around.

## Relationship to `docs/scenario-roadmap.md`

`docs/scenario-roadmap.md` remains the canonical map of scenario-backed gameplay validation.

This document does not replace it. It adds an organizing layer for future mechanic deepening.

Every future mechanic expansion that changes the behavior, proof status, or scenario coverage of a mechanic cluster must update `docs/scenario-roadmap.md`.

A feature is not considered fully scenario-backed merely because code exists, because a regular golden test passes, or because generated scenario coverage marks it structurally active. A mechanic cluster becomes scenario-backed only when one or more dedicated long-run scenarios are formally registered in `docs/scenario-roadmap.md` and their goldens prove the intended causal branch actually occurs.

## Evidence Policy

Trust sources in this order:

1. `docs/FOUNDATIONS.md`
2. `docs/scenario-roadmap.md`
3. Active `.github/workflows/*`
4. Active test/scenario files invoked by workflows
5. Current non-archive code/docs
6. Archived docs/files only if explicitly referenced by current non-archive files, and only as lower-trust prior art

Generated coverage is useful, but it is structural inventory. It is not behavioral proof.

Regular golden e2e tests are useful, but they mostly prove local correctness. They do not prove whole-system survival, scarcity handling, interruption recovery, multi-agent contention, or cross-mechanic viability.

Long-running 1440-tick scenarios are stronger evidence, but they still prove only the branches they explicitly assert. Passing workflows is necessary. It is not sufficient to conclude that a mechanic is rich, complete, maximally emergent, or fully deepened.

## Coverage Categories

Use these categories when describing any mechanic:

- **Implementation only**: code exists, but no active golden/scenario evidence was found.
- **Regular golden coverage**: covered by normal workspace tests or focused goldens, but not by a long-running scenario.
- **Long-running scenario-backed coverage**: covered by ignored 1440-tick scenario goldens invoked by workflows.
- **Registered roadmap coverage**: formally represented in `docs/scenario-roadmap.md`.
- **Collision-proven coverage**: proven under scarcity, interruption, multi-agent contention, and collision with other mechanics.

Most current mechanics are not collision-proven yet. Treat that as the main reason this document exists.

## Current Mechanic Universe

The active scenario roadmap and workflows imply this current mechanic set:

- basic needs: eat, drink, sleep, relieve, wash;
- metabolism, drive thresholds, travel physiology, deprivation, death traceability;
- drive escalation;
- need-driven exploration;
- activation-decay perception and belief freshness/staleness;
- proactive diversification, curiosity, source reliability, experience preferences;
- production, harvesting, crafting, stock staging, stock/transport;
- merchant selling, trade negotiation, commodity valuation, substitute preferences;
- facility queue contention;
- item decay, disposal, carry-capacity pressure;
- tell / peer information transfer;
- ask-about-person;
- consult-record;
- search, report-found, witness/testimony branches;
- offices, succession, force claims;
- obligation satiation;
- notice posting;
- bounty posting;
- theft, place concealment, physical evidence;
- violation investigation, accusation, punishment, justice records;
- patrol;
- pursuit;
- combat;
- bandit camps;
- escort/coordinated care travel;
- full-stack coexistence.

Generated coverage also mentions `Cognitive archetypes`, but that is not currently a gameplay mechanic cluster in this roadmap. Treat it as AI architecture/support unless `docs/scenario-roadmap.md` explicitly promotes it as a gameplay feature.

## Recommended Work Order

This is a working order, not an eternal absolute order.

Order future deepening passes by:

1. survival-criticality;
2. causal centrality;
3. FOUNDATIONS-risk;
4. player-facing richness.

Recommended order:

1. Embodied Survival and Self-Care
2. Spatial Acquisition, Travel, Exploration, and Belief
3. Material Economy, Production, Trade, Contention, Decay, and Disposal
4. Social Knowledge, Records, Search, and Evidence Transmission
5. Ownership, Institutions, Crime, Justice, Obligations, and Social Artifacts
6. Hostility, Patrol, Pursuit, Combat, Injury, Bandit Camps, and Escort/Care
7. Full-Stack Coexistence and Regression Validation

## Cluster 1 — Embodied Survival and Self-Care

### Mechanics

- hunger / eat;
- thirst / drink;
- fatigue / sleep;
- bladder / relieve;
- dirtiness / wash;
- metabolism;
- drive thresholds;
- drive escalation;
- deprivation and death traceability;
- self-care interruption and recovery.

### Why These Belong Together

These mechanics are one embodied loop.

Relief can create Waste and dirtiness. Dirtiness can force wash. Wash depends on water, facilities, travel, and beliefs. Hunger, thirst, fatigue, bladder, and dirtiness compete for action time. Any deepening pass that treats them as isolated stats will miss the real survival problem.

### Current Evidence

Current long-running survival rows prove that agents can survive baseline, scattered, contested, drive-escalation, and many feature scenarios while satisfying authored survival-health contracts.

Drive escalation has specific evidence that sustained critical dirtiness can increase wash pressure, while a belief-only planning regression ensures escalation does not synthesize remote wash knowledge.

Auxiliary simulation-gap evidence proves death traceability from unmet hunger: durable death state, death event, and no post-death action starts.

### Not Yet Proven Enough

Current evidence does not prove that self-care is fully mature under:

- severe degradation of food, water, sleep, latrine, or wash access;
- agents competing for the same rest/relief facilities;
- injury or pursuit disrupting self-care;
- shelter/safety constraints around sleep.

### Future Deep-Research Questions

A future spec pass should investigate:

- whether each self-care action has adequate physical preconditions, duration, cost, occupancy, interruption, and aftermath;
- whether need relief always comes from concrete world actions and not direct stat satisfaction;
- whether deprivation, exhaustion, collapse, death, waste, dirtiness, and recovery have traceable world-state consequences;
- whether survival priorities remain embodied under travel, trade, combat, escort, and justice pressure.

Do not prescribe exact formulas here. The spec pass should discover the right model from FOUNDATIONS-aligned constraints and current code behavior.

### Scenario Validation Expectations

Future validation should include:

- baseline 1440-tick sustainment;
- harsher scarcity and degradation;
- multi-agent contention for water, relief, and sleep affordances;
- collision with travel, trade, theft, injury, escort, and obligations;
- consequences visible through world state and event logs, not scripted assertions.

## Cluster 2 — Spatial Acquisition, Travel, Exploration, and Belief

### Mechanics

- travel;
- travel physiology;
- place topology;
- need-driven exploration;
- resource discovery;
- perception;
- belief freshness/staleness;
- source reliability;
- proactive diversification;
- experience preferences.

### Why These Belong Together

Survival does not mean “lower hunger.” It means agents can discover, reach, acquire, remember, re-evaluate, and use resources through lawful spatial and epistemic paths.

Food acquisition, water acquisition, trade, theft, patrol, pursuit, and rescue all depend on this cluster.

### Current Evidence

Scattered and contested survival prove multi-hop travel, chokepoints, remote food discovery, and resource-source use under survival pressure.

The scattered belief-envelope regression proves a target-location belief can decay from certain to stale without refresh.

The preferences row proves proactive exploration to a novel grove, later source use, and careful failure accounting.

### Not Yet Proven Enough

Current evidence does not prove:

- robust false-lead handling;
- contradiction between sources;
- route risk and route degradation;
- stale belief causing lawful failure and recovery;
- dynamic disappearance or depletion of believed resources across many agents;
- general learning/preference behavior beyond a focused scenario.

### Future Deep-Research Questions

A future spec pass should investigate:

- which spatial facts are public topology and which require belief;
- how resource beliefs age, conflict, and fail;
- how agents decide between familiar but strained sources and novel sources;
- how travel costs affect survival priorities without becoming hidden global route scoring;
- how discovery works when local searches fail repeatedly;
- how source reliability changes only after lawful expectation failures.

### Scenario Validation Expectations

Future validation should include:

- 1440-tick sustainment with remote resources;
- route chokepoints;
- stale beliefs and belief refresh;
- false or depleted sources;
- multi-agent convergence and divergence;
- interruption during travel;
- collision with trade, theft, pursuit, escort, and patrol.

## Cluster 3 — Material Economy, Production, Trade, Contention, Decay, and Disposal

### Mechanics

- harvesting;
- production and crafting;
- item lots;
- stock staging;
- stock/transport;
- merchant selling;
- trade negotiation;
- commodity valuation;
- substitute preferences;
- facility queue contention;
- carrying pressure;
- disposal;
- item decay.

### Why These Belong Together

This is the material substrate of the simulation.

Food, water, bread, coin, waste, sale lots, staged stock, crafted goods, and decayed items all need persistent identity and source/sink accounting. Trade, theft, production, disposal, and decay must share the same material rules.

### Current Evidence

The production row proves `ProduceCommodity`, `craft:Bake Bread`, Bread materialization, and later Bread consumption.

The trade row proves merchant stock staging, substitute purchase, explicit trade payload, Apple/Coin transfer, eat after trade, and well queue grant before harvest.

The item-decay row proves `FreeCarryCapacity`, `drop_item`, tracked Waste becoming a ground item, and later `ItemDecay`.

The dedicated item-decay workflow proves repeated Waste creation reaches bounded steady state while conservation checks hold.

### Not Yet Proven Enough

Current evidence does not prove:

- complex production chains;
- multi-input/multi-output transformations with lineage;
- multi-seller or multi-buyer market pressure;
- price changes from concrete stock, beliefs, and substitutes rather than abstract scoring;
- transport as a full logistics mechanic;
- disposal consequences beyond tracked Waste;
- queue fairness, abandonment, interruption, and facility failure across many facilities.

### Future Deep-Research Questions

A future spec pass should investigate:

- whether every material change has explicit source, sink, transfer, or transformation provenance;
- how sale listings become known to buyers without omniscience;
- whether trade and production preserve item identity where downstream systems care;
- how substitution and valuation arise from concrete local state and beliefs;
- how queues, grants, locks, reservations, and abandoned intents are represented;
- how decay interacts with ownership, sanitation, evidence, and place state.

### Scenario Validation Expectations

Future validation should include:

- production under ingredient scarcity;
- trade under no direct desired good but lawful substitutes;
- facility contention with queue grants and interruption;
- carried burden and disposal pressure;
- item decay after drop and after repeated production;
- conservation audits at multiple ticks;
- collision with theft, justice, survival, and travel.

## Cluster 4 — Social Knowledge, Records, Search, and Evidence Transmission

### Mechanics

- tell;
- peer information transfer;
- ask-about-person;
- consult-record;
- last-seen memory;
- search;
- report-found;
- witness testimony;
- social observations;
- institutional records;
- belief provenance and freshness.

### Why These Belong Together

These are all knowledge-transfer mechanics.

A spoken report, a record consultation, a search result, a witness statement, and a notice all move information through the world. They must obey locality, provenance, freshness, confidence, and physical/social transmission.

### Current Evidence

The tell row proves accepted transfer of an orchard food belief before the listener travels and eats.

The ask-consult row proves hearsay last-seen acquisition after a witness returns, plus consult-record before office action.

The justice row proves search-place, found-safe expectation resolution, report-found, and office-register status writing.

The theft row proves investigated suspicion can be relayed through accepted testimony to a listener who did not directly perceive the theft.

### Not Yet Proven Enough

Current evidence does not prove:

- conflicting testimony;
- stale reports with wrong locations;
- unreliable witnesses;
- record loss, contradiction, or correction;
- rumor chains beyond focused tell/testimony branches;
- broad report/witness behavior outside the specific landed branches.

### Future Deep-Research Questions

A future spec pass should investigate:

- what information each social action can lawfully transmit;
- how provenance, acquisition tick, claimed event tick, confidence, and source chain are stored;
- how agents choose between direct observation, testimony, and records;
- how social observations become institutional records;
- how failed searches and contradicted reports create new world state;
- how communication occupies time and participants.

### Scenario Validation Expectations

Future validation should include:

- belief transfer before behavior changes;
- stale belief leading to lawful failed action and recovery;
- witness testimony with no direct perception by listener;
- consult-record gating institutional action;
- search/report branches with record writing;
- conflicting reports;
- collision with theft, pursuit, bounty posting, and office duties.

## Cluster 5 — Ownership, Institutions, Crime, Justice, Obligations, and Social Artifacts

### Mechanics

- ownership, custody, access, and displayed stock;
- theft;
- place concealment;
- physical evidence;
- investigation;
- accusation;
- punishment/fines;
- offices;
- succession;
- force claims;
- obligation satiation;
- notices;
- bounties;
- reward encumbrance;
- institutional records.

### Why These Belong Together

This cluster is the social order stack.

Theft requires ownership/custody. Investigation requires evidence and prior expectations. Accusation requires office authority, records, and proof. Bounties require an issuer, artifact, target, reward source, and encumbrance. Offices create duties and powers. Notices and records move institutional claims into world state.

Deepening these separately would create contradictions.

### Current Evidence

Regular office goldens prove support-law claims, loyal support, bribe-backed coalition, and coercion/courage diversity.

The survival-offices row proves force-law claim pressure, notice posting, obligation satiation, office-linked search duty, delayed holder installation, and return to self-care.

The theft row proves concealed theft of a staged lot, possession transfer, eating stolen food, physical evidence, investigation, and testimony.

The justice row proves investigation, accusation, fine, record writing, search/report-found, and institutional bounty posting with reward reservation.

### Not Yet Proven Enough

Current evidence does not prove:

- broad ownership/custody/access distinctions across many item/container cases;
- failed theft consequences;
- contested accusations;
- appeals, false accusation, or contradictory evidence;
- multiple jurisdictions;
- bounty lifecycle after posting;
- obligation conflict across many duties;
- notices aging, being ignored, contradicted, or physically moved.

### Future Deep-Research Questions

A future spec pass should investigate:

- whether ownership, possession, custody, access, jurisdiction, and obligation are distinct in current mechanics;
- how theft changes possession and leaves evidence without granting omniscient knowledge;
- how investigation converts physical evidence and expectations into social/institutional claims;
- how offices authorize actions without becoming magical global powers;
- how social artifacts are created, placed, read, copied, aged, fulfilled, revoked, or destroyed;
- how bounty rewards are reserved, released, paid, or invalidated.

### Scenario Validation Expectations

Future validation should include:

- theft with and without direct witnesses;
- concealment that affects perception but still leaves physical aftermath;
- investigation after a violated expectation;
- accusation and punishment through records;
- office authority gating institutional action;
- obligation actions competing with self-care;
- bounty posting with explicit reward source and reservation;
- collision with trade, evidence transmission, patrol, pursuit, and combat.

### Current Quarantine / Reconciliation Item

The behavioral justice golden proves institutional bounty posting, but generated structural coverage appears to classify bounty posting differently for the justice scenario. Before deepening bounty mechanics, reconcile `scenario_coverage.rs`, `docs/generated/scenario-coverage.md`, and the behavioral golden, or document why structural activation intentionally differs from behavioral proof.

## Cluster 6 — Hostility, Patrol, Pursuit, Combat, Injury, Bandit Camps, and Escort/Care

### Mechanics

- patrol;
- pursuit;
- hostility;
- combat;
- wounds;
- death;
- bandit camps;
- escort to safety;
- care queue / handoff.

### Why These Belong Together

Hostile pressure creates wounds and deaths. Wounds create care needs. Pursuit depends on last-seen memory and travel. Bandit camps change state when members die. Escort depends on injury, spatial movement, and destination care affordances.

This cluster should not become detached combat logic.

### Current Evidence

The patrol row proves patrol at authored waypoints and remote pursuit from hostility plus last-seen memory, including travel-before-attack planning and attack commit.

The combat row proves EngageHostile selection, attack commit, raider death from wounds, camp empty marker, and bandit camp abandonment after grace period.

The escort row proves wounded ward detection, EscortToSafety selection/start/commit, both caretaker and ward reaching the clinic, and care queue installation.

Final integration proves hostile pressure can produce a concrete wound in a full-stack world.

### Not Yet Proven Enough

Current evidence does not prove:

- combat interruption;
- retreat, surrender, pursuit loss, or false last-seen leads;
- wound treatment beyond queue installation;
- multiple wounded actors competing for care;
- bandit camp recruitment, recovery, or retaliation;
- patrol fatigue and duty conflict across longer periods;
- combat consequences feeding justice, reports, bounties, and obligations.

### Future Deep-Research Questions

A future spec pass should investigate:

- how hostility becomes known, remembered, and stale;
- how pursuit respects route knowledge and target uncertainty;
- how attacks create wounds, death, evidence, fear, reports, and obligations;
- how wounded agents remain embodied actors rather than inert quest objects;
- what care handoff and treatment require as concrete world processes;
- how bandit camp state changes propagate through faction, patrol, bounty, and justice systems.

### Scenario Validation Expectations

Future validation should include:

- patrol under survival pressure;
- pursuit from last-seen memory with stale/false leads;
- attack causing wounds and death;
- interrupted combat and recovery;
- wounded escort with travel and care handoff;
- multiple wounded or multiple rescuers;
- bandit camp abandonment after concrete member loss;
- collision with bounty, report/witness, justice, and self-care.

### Inferred Required Support

A deeper care/treatment mechanic is implied by wounds and escort. It is not an arbitrary new feature. It should be introduced only to complete the existing injury/escort/care chain.

## Cluster 7 — Full-Stack Coexistence and Regression Validation

### Mechanics

This cluster does not add mechanics. It validates coexistence.

### Purpose

After deepening any cluster, run final integration and relevant row-specific scenarios to confirm that the full stack still composes.

Final integration should remain a capstone scenario, not a substitute for mechanic-specific proof.

### Current Evidence

The final-integration row structurally activates the full catalog, runs a 1440-tick survival contract, and proves hostile wound pressure in the same full-stack world.

### Future Validation Expectations

Future full-stack validation should include:

- full-catalog structural activation;
- 1440-tick survival-health contract;
- at least one concrete branch from a high-pressure cross-cluster event;
- deterministic replay;
- no scenario-specific rescue rails;
- no hidden omniscient behavior;
- no material conservation violations;
- no stuck or maintenance-starvation anomalies unless intentionally being tested.

## Anti-FOUNDATIONS Mechanic Smells

Treat these as red flags during any mechanic deepening pass:

- resource, item, coin, food, water, wound, notice, bounty, or record appears without source/provenance;
- a need is satisfied directly without a world action;
- a planner uses remote authoritative truth instead of belief, local observation, testimony, record, or lawful boundary artifact;
- communication changes knowledge without co-location, carrier, artifact, or transmission path;
- a record, accusation, bounty, office claim, or notice is UI/debug state rather than world state;
- a queue, grant, lock, or reservation is implicit in planner intent rather than explicit world state;
- an action has no duration, cost, occupancy, interruption surface, or aftermath;
- a scenario succeeds because a test script steers it rather than agents acting through lawful affordances;
- failure leaves only a boolean flag instead of changed world state;
- an institution acts from global truth instead of records, testimony, observation, or jurisdiction;
- trade changes holdings without explicit transfer;
- production creates goods without inputs/source or documented generation path;
- decay deletes items without event/provenance;
- combat causes death without wounds/attack chain;
- escort moves a ward without travel/co-location/care handoff;
- final integration is used to claim deep proof of a mechanic that only appears structurally.

## Rules for Redesign, Quarantine, or Deletion

A mechanic should be **preserved and deepened** when:

- it is already registered in `docs/scenario-roadmap.md`;
- current tests show lawful branch behavior;
- known gaps are about richness, collision coverage, interruption, or scarcity.

A mechanic should be **redesigned** when:

- it cannot explain state changes through concrete world processes;
- it relies on omniscient agent decisions;
- it treats summaries or abstract scores as authoritative truth;
- it bypasses bodily, spatial, material, social, institutional, or temporal causality.

A mechanic should be **quarantined from scenario reliance** when:

- generated coverage and behavioral proof disagree and the disagreement is not documented;
- it works only in a narrow golden but lacks scenario-roadmap registration;
- it is an auxiliary diagnostic/planner/support fixture rather than a landed gameplay mechanic;
- it requires an unresolved known gap that the current scenario explicitly excludes.

A mechanic should be **deleted** only when:

- it is not implied by active workflows, scenario-roadmap rows, tests, or current code/docs;
- it duplicates a better FOUNDATIONS-aligned mechanic;
- it cannot be made causal without corrupting the architecture.

Deletion should be rare. The default action is preserve, deepen, or quarantine until a proper spec pass resolves the issue.

## Rules for Future Mechanic Deepening Passes

Each future deepening pass must:

1. start from `docs/FOUNDATIONS.md`;
2. identify the current mechanic cluster and adjacent clusters it collides with;
3. distinguish implementation evidence, regular golden evidence, long-running scenario evidence, and formal roadmap registration;
4. avoid using generated coverage as behavioral proof;
5. avoid detailed implementation commitments until the spec pass has audited current code and tests;
6. preserve the current feature universe unless a new support mechanic is necessary to complete an existing causal chain;
7. mark inferred support mechanics explicitly;
8. update `docs/scenario-roadmap.md` when scenario coverage changes;
9. add or update scenario-backed validation for major mechanic expansions;
10. keep passing workflows as a floor, not a finish line.

## Scenario Design Guidance

Do not create a rigid maturity ladder, but future scenarios should usually consider:

- baseline 1440-tick sustainment;
- scarcity or degradation;
- interruption and recovery;
- multi-agent contention;
- collision with another mechanic cluster;
- consequences visible through world state rather than scripted assertions;
- deterministic replay;
- proof that the intended branch occurred for the authored causal reason, not because a rival lawful branch happened to satisfy the same high-level assertion.

## Current High-Priority Deepening Targets

These are not final designs. They are the first places future specs should look.

1. **Material provenance across production, trade, theft, disposal, and decay**  
   The current proof is promising. Future passes should harden identity, transfer, source/sink, and lineage across more collisions.

2. **Knowledge provenance across tell, ask, consult, search, report, accusation, and pursuit**  
   This is the main anti-omniscience risk. Every plan-changing belief should have a carrier and freshness/provenance where relevant.

3. **Institutional artifact lifecycle**  
   Notices, bounties, office records, accusations, verdicts, and reward encumbrances need deeper lifecycle scenarios.

4. **Injury-to-care chain**  
   Combat and escort imply care/treatment support. Deepen only as needed to complete the existing wound/escort/care causal chain.

## Non-Goals

This roadmap does not redesign the AI architecture.

It may mention mechanic-specific agent requirements, such as needing to discover food, remember last-seen targets, or choose a trade branch from beliefs. Those are gameplay-mechanic requirements. They are not an AI architecture proposal.

This roadmap also does not import external mechanics just because they sound interesting. New mechanics should appear only when they are implied by active repo evidence or necessary support for an existing mechanic to become causally complete.