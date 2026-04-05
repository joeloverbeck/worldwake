# AI Architecture Assessment Against FOUNDATIONS

_Principle numbers below refer to `FOUNDATIONS.md`._

## Bottom line

This architecture is already on the right side of the line.

The hard constraints that usually get faked in simulation projects are mostly real here: deterministic replay, stable identity, explicit world state, belief-separated planning, duration-bearing actions, interruptible commitments, contention queues, and state-mediated system interaction.

The real risk now is subtler: as the world gets richer, summary layers and convenience abstractions will start doing semantic work that FOUNDATIONS says must stay in concrete state, evidence, artifacts, and local knowledge. That is where this architecture is most likely to drift.

So the diagnosis is not “patch a weak design.” The diagnosis is: **good architecture, but the next failures will come from abstraction pressure**.

---

## What is already strongly aligned

### Strong alignment with FOUNDATIONS

- **Determinism and replay are real, not aspirational**. Seeded RNG, fixed-point math, ordered authoritative state, logical ticks, and replay hashing are exactly the right base for Principles 2, 9, 12, 29, and 31.
- **Belief and truth are already separated**. Planning through `GoalBeliefView` / `RuntimeBeliefView` is one of the biggest wins in the whole stack. That lines up cleanly with Principles 7, 14, 15, 16, 17, 19, 20, and 21.
- **Actions are world processes, not atomic teleports**. Preconditions, durations, interruptibility, commit conditions, and contention all line up with Principle 8.
- **Intent is not entitlement**. The current contention model already gets an important thing right: planning a use does not silently reserve the world.
- **Systems interact through state**. The crate split and state-mediated cross-system flow are exactly what Principle 26 wants.
- **Per-agent variation is concrete**. The profile-heavy architecture is a strong fit for Principle 22.
- **Institutions already exist in world state**. Offices, records, faction membership, succession, and force-control projection are the right substrate for Principles 23, 24, and 25.
- **The project already treats debugability seriously**. Event log + trace sinks + golden tests are a real foundation for Principles 29 and 31.

That is the good news. It matters.

---

## Where the architecture is still undershooting FOUNDATIONS

## Priority 0: fix these before broadening the AI surface much further

### 1. The belief layer is too summary-centric  
**Principles:** 15, 16, 18, 27, 29

The current belief architecture looks excellent for tractable planning, but it appears too compressed for the world described by FOUNDATIONS.

`AgentBeliefStore` centers on `known_entities: BTreeMap<EntityId, BelievedEntityState>`, with each entity summarized into a current-ish belief bundle: last known place, last known inventory, wounds, activity, artifact state, contention state, and one source/observation time. That is good working memory. It is not enough as the primary representation of knowledge in a contradiction-heavy world.

### Why this matters

FOUNDATIONS wants agents to reason about:

- who told them something,
- when they think it happened,
- when they learned it,
- how stale it is,
- what competing reports exist,
- why one claim outweighed another,
- and how correction propagates unevenly.

A summary-first belief model makes those questions hard. It also makes false rumor, contested evidence, stale testimony, and wrongful accusation much harder to support cleanly.

It is especially risky that inventory belief appears commodity-aggregate-heavy. That is fine for coarse trade planning, but it is not enough for identity-bearing artifacts, proofs, warrants, specific stolen items, or container-specific expectations.

### Proposed change

Introduce a **claim-first belief layer**:

- `BeliefClaim`
  - `claim_id`
  - `subject`
  - `proposition_kind`
  - `value / bindings`
  - `claimed_event_tick`
  - `acquired_tick`
  - `source_kind`
  - `source_chain`
  - `carrier_entity_or_record`
  - `confidence`
  - `freshness`
  - `status` (`active`, `superseded`, `disputed`, `retracted`, etc.)

Then keep `BelievedEntityState` as a **derived working-memory cache**, not the root representation.

That lets you preserve tractable planning while making knowledge paths and contradictions first-class.

### What this unlocks

- Contradictory reports can coexist without becoming bugs.
- Agents can ask, “why do I believe this?”
- Correction can propagate unevenly and lawfully.
- Investigations can reason over claims, not only over world snapshots.
- Debugging gets dramatically better.

### Spec I would write

- **Belief Claims, Provenance, and Working-Memory Summaries**

---

### 2. The goal layer mixes true objectives with action-shaped choices  
**Principles:** 20, 21, 26, 30

The current `GoalKind` enum mixes at least three different things:

- genuine desired conditions,
- durable commitments,
- and direct action-shaped choices.

Examples that look action-shaped or overly privileged:

- `ShareBelief`
- `StealItem`
- `SupportCandidateForOffice`
- `PunishAccused`
- `ClaimOffice`
- `Patrol`

That is a problem because FOUNDATIONS is explicit: **goals should name desired world conditions, not privileged one-step solutions**.

### Why this matters

If the goal layer names actions, the planner starts drifting toward “catalog of authored behaviors” instead of “bounded practical reasoning over lawful affordances.”

That will scale badly. The wrong next move from here is to keep adding more goal variants and more custom emitters every time a new system arrives.

### Proposed change

Refactor top-level planner intent into two explicit layers:

- **Objective kinds**
  - world conditions or commitment states
  - examples: `ResourceUnderControl`, `ThreatReduced`, `AssignmentSatisfied`, `OfficeVacancyResolved`, `BeliefTransferred`, `ArtifactPosted`, `PatientStabilized`

- **Epistemic objective kinds**
  - information states the agent wants to obtain
  - examples: `KnowLocationOf(X)`, `VerifyClaim(Y)`, `FindWitnessFor(V)`, `InspectExpectedContents(C)`, `LearnHolderOfOffice(O)`

Then let action defs remain means.

Also: barrier fallback should only return plans that end in a **concrete gain**:
- new information,
- new queue/grant state,
- new place with observation opportunity,
- new record consulted,
- new control/access relation,
- or other inspectable capability change.

Not just “heuristically closer.”

### Structural recommendation

Replace the hardcoded emitter list with a **registry of deterministic objective providers**.  
Keep ordering explicit if needed, but stop centralizing all future domain growth in one monolithic candidate-generation switchboard.

### Spec I would write

- **Objective Model Refactor**
- **First-Class Epistemic Objectives and Information-Seeking Planning**

---

### 3. Expectation violation exists, but it should be generalized and promoted  
**Principles:** 17, 21, 29, 30

The architecture already has the right instinct here:

- frame assumptions,
- mismatch detection,
- and `emit_expectation_violation_candidates()`.

That is good. But it still reads like a targeted subsystem instead of a general law of the world.

FOUNDATIONS wants surprise to come from violated expectation everywhere, not only from bespoke violation logic.

### Why this matters

You need a general way to represent things like:

- “my stash should still contain this”
- “this seller should have bread”
- “this route should still be survivable”
- “this office should still be occupied”
- “my queue position should still be valid”
- “the caravan should have arrived by now”

Without that, anomaly detection stays domain-fragmented and brittle.

### Proposed change

Introduce first-class:

- `Expectation`
- `PlanAssumption`

Each should carry:

- origin,
- supporting evidence,
- claimed scope,
- freshness,
- invalidation rules,
- observability of violation,
- and relevance to current intention.

Then make anomaly generation generic: compare local observation or consulted record against active expectations and assumptions.

### Specific improvement to blocked intents

`BlockedIntentMemory` is useful, but its expiry should become more causally grounded where possible.

Examples:

- `SellerOutOfStock` should clear on restock evidence, a plausible process window, or fresh inspection.
- `TooExpensive` should clear on new coin, new price evidence, or changed stock.
- `NoKnownSeller` should clear on testimony, record consultation, or exploration results.

Plain TTL fallback is fine as a fallback. It should not be the main truth source.

### Spec I would write

- **Expectations, Plan Assumptions, and General Anomaly Detection**
- **Causally Grounded Blocker Invalidation**

---

### 4. Determinism is strong, but micro-arbitration still looks under-specified  
**Principles:** 8, 9, 21, 29  
**Canonical scenario:** E

The architecture clearly defines macro-order:

- authoritative ticks,
- fixed system order,
- deterministic containers.

That is good. But FOUNDATIONS is clear: **determinism is not enough**.  
Meaning cannot quietly depend on thread order, container iteration order, or incidental engine sequencing.

The architecture report is strong on system order. It is weaker on same-tick micro-arbitration.

### Why this matters

If two agents reach for the same dropped item in the same tick, or two arrivals become visible in the same phase, or two observers sample the same scene in an order-sensitive way, BTreeMap order is not a lawful world rule. It is only a deterministic implementation detail.

That distinction becomes load-bearing as soon as you add more contention, more simultaneous opportunities, and more social response.

### Proposed change

Write an explicit **scheduler and arbitration spec** that defines:

- when observation samples happen,
- when committed effects become visible,
- when arrivals become co-located,
- when plans are reconsidered relative to new evidence,
- what “same-tick” actually means,
- and how conflicts resolve when no explicit reservation already exists.

Add generic arbitration modes:

- explicit queue/grant,
- explicit race,
- simultaneous resolution bucket,
- contested commit with visible winner/loser aftermath.

### Audit targets

Audit at least:

- dropped-item pickup,
- corpse access,
- record access,
- facility use,
- same-place trade initiation,
- patient access,
- pursuit intercepts,
- travel arrival visibility,
- witness resolution order.

### Spec I would write

- **Scheduler Semantics, Simultaneity, and Same-Tick Arbitration**

---

### 5. Contention is currently too facility-centric  
**Principles:** 8, 21, 25  
**Canonical scenario:** E

The current contention queue model is good. Keep it.

But it is explicitly framed around shared facilities: workstations, care facilities, offices.

That is not broad enough for the world FOUNDATIONS describes.

### Why this matters

Scarce opportunities in this world are not just buildings. They are also:

- corpses,
- wounded patients,
- dropped goods,
- witnesses,
- arrest targets,
- escort slots,
- posted proofs,
- conversation partners,
- assignment slots,
- and other mobile or ephemeral opportunities.

If only facilities get first-class contention, other domains will quietly fall back to hidden tick order or planner luck.

### Proposed change

Generalize contention into a family of world-visible claim artifacts:

- `OpportunityClaim`
- `ReservationArtifact`
- `RaceArtifact`
- `AssignmentClaim`

Each should define:

- how it is acquired,
- who can observe it,
- whether it is exclusive or soft,
- how it expires,
- how it is invalidated,
- whether it is transferable,
- and whether it is socially recognized, physically enforced, or purely competitive.

### Important constraint

Do **not** make everything a reservation.  
Some opportunities should remain open races. The point is not “reserve more.” The point is “make the world’s contention rule explicit.”

### Spec I would write

- **Generalized Opportunity Claims, Reservations, and Races**

---

### 6. `can_exercise_control()` is a good primitive, but too collapsed to be the conceptual center  
**Principles:** 23, 24, 25

This is one of the clearest foundational mismatches.

The architecture already has a useful force-control hierarchy:

- possession,
- ownership,
- faction delegation,
- office delegation,
- container traversal.

That is good. But FOUNDATIONS explicitly requires separation between:

- ownership,
- custody,
- access,
- permission,
- obligation,
- capability,
- and jurisdiction.

A single effective “can manipulate” check is not enough as the conceptual center once the world gets more legal, social, and investigative.

### Why this matters

You need to cleanly represent cases like:

- I physically hold it but do not own it.
- I own it but cannot access it.
- I can access it because I have the key, but I am not allowed to take it.
- I owe it to someone else.
- My office authorizes me to open the chest, but only within a place and jurisdiction.
- I can confiscate it legally but cannot carry it physically.

### Proposed change

Build a richer rights lattice with explicit queries for:

- physical capability,
- custody,
- ownership,
- access right,
- delegated authority,
- legal permission,
- obligation / debt / lien,
- jurisdictional authority.

Then keep `can_exercise_control()` as a convenience view over that lattice.

### Spec I would write

- **Rights Lattice: Ownership, Custody, Access, Obligation, and Jurisdiction**

---

## Priority 1: next wave, high leverage

### 7. Social artifact issuance is still underexposed in the planner surface  
**Principles:** 18, 23, 25, 30  
**Canonical scenarios:** A, F, G

The substrate is already there:

- `Record`
- `SocialArtifact`
- `ConsultRecord`
- `PostBounty`
- `PostNotice`

That is all good.

But from the current report, the AI surface area is stronger on **consuming** artifacts than on **creating, maintaining, contesting, and paying through** them.

That is dangerous. It is where simulation projects often reintroduce manager logic.

### Why this matters

FOUNDATIONS does not want hidden quest systems. It wants world artifacts:

- bounties,
- notices,
- accusations,
- warrants,
- orders,
- duties,
- debts,
- contracts,
- payment obligations.

Those should not be side channels.

### Proposed change

Introduce a unified planner-visible artifact lifecycle with fields like:

- issuer,
- sponsor / funding source,
- jurisdiction,
- proof requirements,
- posting place,
- validity window,
- revocation path,
- contest path,
- payment state,
- lineage / copies / reposts.

Then give agents and institutions first-class objectives to:

- create,
- post,
- read,
- copy,
- contest,
- revoke,
- fulfill,
- and discharge.

### Important institutional extension

Operational work should also flow through artifacts:

- patrol orders,
- escort assignments,
- warrants,
- investigation orders,
- staffing duties.

Do not let these remain only static components if you want office vacancy and institutional degradation to emerge correctly.

### Spec I would write

- **Social Artifact Lifecycle**
- **Operational Assignments and Institutional Orders**

---

### 8. Evidence and aftermath need stronger materialization into world state  
**Principles:** 10, 18, 29  
**Canonical scenarios:** A, C, G

The event log is strong for replay and developer introspection.

That is not the same as aftermath being present in the world for agents.

FOUNDATIONS wants aftermath to exist as carriers of consequence, not only as entries in a hidden causal trace.

### Why this matters

Investigations, reports, correction, suspicion, and memory all get cleaner if actions leave inspectable residue when they should.

Not decorative residue. Consequential residue.

### Proposed change

Add a materialization policy layer for aftermath and evidence.

At the architecture level, think in terms of:

- tamper state,
- missing-contents discrepancy,
- movement traces,
- scene evidence bundles,
- disturbance markers,
- container state changes,
- public or private reports derived from events.

Tie them to:

- observability,
- decay,
- copyability,
- contestability,
- and belief acquisition.

### Spec I would write

- **Evidence Artifacts and Aftermath Materialization**

---

### 9. `ReasoningProfile` currently mixes psychology with performance knobs  
**Principles:** 12, 20, 22, 31

This is one of the most important design hygiene issues in the whole architecture.

Right now, fields like these all live together:

- `max_candidates_to_plan`
- `max_plan_depth`
- `snapshot_travel_horizon`
- `max_prerequisite_locations`
- `max_node_expansions`
- `beam_width`
- `switch_margin`

Some of those feel like agent cognition. Some feel like engine budget. In the current system, at least some of them are behavior-changing in ways validated by tests.

That means they are world-meaningful right now.

### Why this matters

If later you tune them “for performance,” you will quietly be changing who the agents are.

That would violate the spirit of FOUNDATIONS even if the code still looks clean.

### Proposed change

Split this into two distinct layers:

- **CognitiveProfile**
  - inspectable, persisted, agent-authored traits
  - bounded foresight style
  - willingness to switch goals
  - patience with uncertainty
  - breadth of consideration
  - planning temperament

- **ExecutionBudget**
  - engine-level compression / budget knobs
  - required to preserve meaning within declared bounds
  - validated explicitly

Then audit each current field and reclassify it.

### Spec I would write

- **Cognitive Profile vs Execution Budget**

---

### 10. Debugging is good on positive paths, but still needs first-class “why not?” support  
**Principles:** 29, 31

The architecture already has better trace infrastructure than most simulation codebases. That is a real asset.

But FOUNDATIONS asks not only:

- why did this happen?

It also asks:

- why did this not happen?
- why did the agent not know?
- why was the bounty not posted?
- why was this goal suppressed?
- why did the office fail to act?

### Proposed change

Add unified structured explanation surfaces for:

- objective emitted / suppressed,
- plan chosen / rejected,
- blocker inserted / cleared,
- belief accepted / discounted / replaced,
- artifact issued / not issued,
- institutional action taken / not taken.

Do not rely only on ad hoc debug logs.  
Make these inspectable structured records.

### Spec I would write

- **Decision Explanations and Negative-Space Introspection**

---

### 11. Source trust and perception exposure are still too static  
**Principles:** 2, 15, 16

The current confidence model is a good baseline:

- direct / report / rumor / inference bases,
- chain penalties,
- staleness penalties.

That is useful. It is not enough long term.

Likewise, `observation_fidelity` is a good trait, but too blunt as the main uncertainty lever.

### Why this matters

A false-rumor world needs more than static trust weights.  
A witness world needs more than flat observation chance.

### Proposed change

Add:

- **SourceReliabilityMemory**
  - local, fallible, context-sensitive trust updates based on later confirmation or contradiction

- **Structured sensory exposure**
  - salience
  - attention / occupancy
  - concealment / obstruction
  - topology / range
  - event visibility
  - personal modifiers like fatigue or panic

Keep the current confidence policy as the base layer, not the whole story.

### Spec I would write

- **Dynamic Source Credibility**
- **Perception Exposure Model**

---

### 12. Off-map and external dependence are not yet architecturally surfaced enough  
**Principles:** 13, 30  
**Canonical scenario:** H

This is probably not urgent if the current map is still closed and locally complete.

But FOUNDATIONS is explicit: off-map is not nowhere.

From the current report, I do not see a planner-visible boundary-process model yet. `ExternalInput` exists in the causal system, but that is not the same as explicit neighboring-region arrivals, expected deliveries, delayed reports, or constrained remote inflow.

### Proposed change

Before the simulation starts depending on off-map trade, migration, taxes, convoys, or remote shocks, add explicit boundary-process support:

- named external sources,
- routes / channels,
- delay,
- observables,
- capacities,
- failure modes,
- evidence of non-arrival,
- and expectations that agents can hold and later see violated.

### Spec I would write

- **Boundary Arrivals and Remote Expectations**

---

## What I would not do

- I would **not** keep solving missing behavior by adding more one-off `GoalKind` variants first.
- I would **not** let deterministic container order stand in for arbitration semantics.
- I would **not** let blocker TTLs become de facto world rules.
- I would **not** let performance tuning quietly mutate psychology.
- I would **not** build institutional behavior in manager code above records, orders, and artifacts.

That path will work for a while. Then it will become the exact kind of authored outcome machinery FOUNDATIONS is trying to prevent.

---

## Specs I would write first

### Immediate

1. **Belief Claims, Provenance, and Working-Memory Summaries**
2. **Objective Model Refactor**
3. **First-Class Epistemic Objectives and Information-Seeking Planning**
4. **Expectations, Plan Assumptions, and General Anomaly Detection**
5. **Scheduler Semantics, Simultaneity, and Same-Tick Arbitration**
6. **Generalized Opportunity Claims, Reservations, and Races**

### Next

7. **Rights Lattice: Ownership, Custody, Access, Obligation, and Jurisdiction**
8. **Social Artifact Lifecycle**
9. **Operational Assignments and Institutional Orders**
10. **Evidence Artifacts and Aftermath Materialization**
11. **Cognitive Profile vs Execution Budget**
12. **Decision Explanations and Negative-Space Introspection**

### After that

13. **Dynamic Source Credibility**
14. **Perception Exposure Model**
15. **Boundary Arrivals and Remote Expectations**

---

## Regression work I would add immediately

The test suite is already serious. Keep that standard. But FOUNDATIONS demands a few scenario classes that still look under-covered from this report.

I would add end-to-end regressions for:

- **Beast starvation -> attack -> report -> bounty issuance -> hunt -> proof -> payment**
- **Stored gold -> empty stash -> discovery -> report -> investigation**
- **Rumor -> travel -> empty source -> contradiction persists -> correction spreads unevenly**
- **False rumor -> accusation -> contested evidence -> correction or miscarriage**
- **Office vacancy -> assignment lapse -> patrol gap -> opportunistic predation**
- **Remote shock -> delayed arrival -> local shortage -> substitution / ration / exit**
- **Same-tick multi-claimant contention where the winner is explained by declared arbitration, not entity order**
- **Cache deletion / rebuild invariance for belief summaries and decision summaries**
- **Execution-budget compression tests proving engine-budget changes do not create illegal world-meaning changes**

---

## Final assessment

This architecture is closer to FOUNDATIONS than most simulation stacks ever get.

The strongest parts are the right parts:
- deterministic replay,
- explicit world state,
- belief-separated planning,
- duration-bearing actions,
- revisable commitments,
- state-mediated systems,
- and inspectable causal traces.

The problem is not that the architecture is cheating today.  
The problem is that **the next cheats will arrive disguised as clean abstractions**.

The pressure points are clear:

- summary beliefs,
- action-shaped goals,
- implicit same-tick arbitration,
- overly collapsed rights/control,
- under-promoted social artifacts,
- and engine budgets that risk doubling as psychology.

If you fix those, you buy a lot more emergent headroom before you need new special cases.

If you do not fix those, the architecture will still look disciplined, but it will gradually stop being fully causality-first in the places that matter most: contradiction, institution, evidence, contested access, and delayed correction.

That is the real fork in the road from here.