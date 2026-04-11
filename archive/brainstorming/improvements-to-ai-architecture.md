# Worldwake AI Architecture Review Against FOUNDATIONS

**Status:** ✅ COMPLETED  
**Scope:** Review of the architecture as described in `ai-architecture-deep-analysis.md`, judged against `FOUNDATIONS.md`  
**Bottom line:** Strong substrate, incomplete architecture

## Executive Assessment

Direct answers:

1. **Does the AI architecture have issues to fix?**  
   **Yes.** Not cosmetic issues. Structural ones.

2. **Are there improvements that would better align it with FOUNDATIONS?**  
   **Yes.** The biggest gains are in epistemics, contention, artifacts/records, and boundaries.

3. **Are there beneficial features to add that align with FOUNDATIONS?**  
   **Yes.** Several of them are important enough that I would treat them as architectural work, not optional content.

My blunt read: **the architecture is already unusually strong at deterministic local-causal simulation**, but it is **not yet fully aligned with FOUNDATIONS**. The substrate is ahead of the deliberation layer. Right now the system is much stronger at bodily needs, combat, travel, and small-scale politics than it is at contradiction, institutions, evidence, property, and extra-local causality.

The problem is **not** that you chose GOAP.  
The problem is that the planner is still reasoning over a world model that is missing some of the most important carriers of consequence demanded by FOUNDATIONS.

## What Is Already Solid

These are real strengths and should be preserved:

- **Deterministic simulation substrate.** Seeded RNG, integer math, deterministic containers, replay, and save/load integrity are all exactly the kind of discipline FOUNDATIONS wants.
- **Explicit action model.** Preconditions, duration, body cost, interruptibility, and action lifecycle are a major alignment win.
- **Belief/world separation.** The AI is not reading raw world truth directly. That is one of the most important architectural choices in the whole stack.
- **Traceability/debuggability.** Event logs, decision traces, action traces, and replay support are a serious advantage.
- **State-mediated system interaction.** The crate structure and action/state/event approach are much closer to FOUNDATIONS than the usual “manager code tells subsystems what to do” mess.
- **Intentions as revisable commitments.** The frame system is directionally correct, even if parts of it need generalization.

In short: **do not tear down the substrate**. It is the right kind of foundation.

## Alignment Snapshot

### Strong alignment
- Local causality
- Determinism and replayability
- Explicit action duration/cost
- Belief vs. ground-truth separation
- Interruptible/revisable planning
- Debuggability
- System interaction through state

### Partial alignment
- Ignorance, uncertainty, contradiction
- Contention and simultaneity
- Institutions and office behavior
- Ownership/custody/access/obligation/jurisdiction
- Derived summaries staying “just summaries”
- Agent diversity in reasoning style
- Validation against all canonical scenario classes

### Weak or missing
- Boundary/off-map processes
- Unified social-artifact substrate
- Several mandatory canonical regression chains

## Issues To Fix

## 1. Decision-time epistemics are too flattened

The belief store appears richer than the decision interface.

The architecture report describes belief provenance, confidence, acquisition paths, stale belief correction, and contradiction tolerance. But the AI-facing belief surfaces are still dominated by crisp returns like `bool`, `Option<T>`, single quantities, and single effective locations. That is enough for stale beliefs. It is **not** enough for the full FOUNDATIONS target of:

- unknown vs false,
- multiple competing reports,
- “I suspect,”
- “I heard two different things,”
- source-weighted contradiction,
- freshness-sensitive reasoning.

### Why this matters
This is a direct pressure point against FOUNDATIONS III.14–18. The foundations do not just want stale knowledge. They want **uncertainty and contradiction as first-class reasoning material**.

### The practical failure mode
You will get agents that can be wrong, but not wrong in a rich enough way. They may correct stale beliefs, but they will struggle to reason about contested testimony, conflicting witness chains, or alternative hypotheses.

### Fix
Introduce a claim-centric epistemic layer:

- `Claim`: proposition, subject, claimed event time, acquisition time, source chain, confidence, freshness, carrier
- `EndorsedBelief`: the agent’s current working view
- `AlternativeClaims`: competing unresolved claims still present in memory

The planner should be able to ask for:
- endorsed view,
- uncertainty level,
- alternative claims,
- provenance,
- freshness,
- and whether a proposition is unknown, contested, or merely low-confidence.

That is a foundational upgrade, not a polish pass.

## 2. Testimony and alarms are too narrow as motivators

The architecture explicitly says **direct observation triggers care**, while indirect reports do not. It also says generic belief-sharing is suppressed under survival stress.

That is too restrictive.

### Why this matters
FOUNDATIONS III.15 and III.18 are explicit: testimony, documents, records, and traces are not flavor. They are causal carriers.

A healer who is told “someone is badly wounded at the mill” should be able to act on that report if the source is credible enough. A fleeing witness should still be able to warn others even while personally stressed. Stress should suppress gossip, not necessarily alarms.

### Practical failure mode
You end up with a locality-respecting system that is paradoxically too deaf to lawful second-hand knowledge.

### Fix
Split communication into distinct classes:

- **Alarm**
- **Report/Testimony**
- **Gossip**
- **Record consultation**
- **Formal accusation**

Each should have its own:
- trust model,
- urgency model,
- suppression rule,
- memory path,
- and planning consequences.

Do not leave “share belief” as a single generic bucket.

## 3. Contention is explicit in some places, but not yet a general world mechanic

Facility queues are good. They are exactly the kind of explicit scarcity-resolution FOUNDATIONS wants.

But the architecture document does **not** show a generic contention substrate for all scarce affordances:
- item pickup,
- corpse access,
- patient attention,
- witness time,
- record access,
- bounty claim competition,
- workstation usage outside facility queues,
- simultaneous arrival cases.

### Why this matters
FOUNDATIONS II.8, II.9, and Canonical Scenario E are clear: plans do **not** reserve outcomes, and contested affordances must resolve through inspectable world processes.

### Practical failure mode
Where there is no explicit arbitration artifact, meaning risks falling back to engine order, input order, or subsystem phase order. Deterministic is not enough. It must be deterministic **for an in-world reason**.

### Fix
Create a reusable contention/arbitration substrate with explicit world objects or records for:
- queue position,
- pending claim,
- grant,
- reservation,
- race window,
- expiry,
- invalidation.

Then make all exclusive affordances use it.

## 4. The goal surface is still too hand-authored

This is one of the biggest long-term architectural risks.

The current architecture has:
- a fixed `GoalKind` enum,
- hand-authored candidate generation buckets,
- per-family suppression policies,
- custom interrupt rules,
- custom planner support,
- and a narrow frame-assumption vocabulary.

That is manageable at current scale. It will get uglier fast.

### Why this matters
FOUNDATIONS wants designers authoring **nouns, laws, institutions, and initial conditions**, not constantly stitching new behavioral categories into multiple AI layers whenever a new system appears.

### Practical failure mode
As you add more world carriers—bounties, contracts, patrol duties, warrants, delegated authority, messenger work, debt collection—you risk rebuilding a disguised quest pipeline inside the AI architecture.

### Fix
Move toward declarative goal and affordance schemas:

- goals defined more as desired world conditions,
- actions define effect semantics and consumed capacities,
- candidate generation derived from motives + visible affordances + believed claims,
- plan assumptions derived from step dependencies rather than mostly hand-authored enums.

I would keep GOAP for now, but I would make the *world it reasons over* more generic and compositional.

## 5. Planner/runtime semantic drift is a real risk

The architecture already admits this risk.

You have:
- hypothetical planning state,
- simplified planner transitions,
- goal-model fallbacks,
- materialization barriers,
- and conformance tests that check **direction agreement**, not exact semantic agreement.

That is useful, but it is not yet tight enough.

### Why this matters
FOUNDATIONS V.26, V.27, V.29, and V.31 demand that the architecture stay explainable and falsifiable. If the planner reasons with one semantics and the runtime executes another, the system will look irrational or mysteriously brittle.

### Practical failure mode
Plans pass search but fail or degrade oddly in execution because the planner’s hypothetical model is only directionally right.

### Fix
Tighten the relationship between planning and execution:

- derive planner effects from the same authoritative action/effect declarations where possible,
- strengthen conformance testing beyond direction-of-change,
- at minimum validate threshold-band agreement, artifact creation/destruction agreement, and critical precondition/effect agreement.

Do not let the planner slowly become a second, approximate simulation.

## 6. Institutions and social artifacts are still thinner than FOUNDATIONS requires

You already have meaningful pieces:
- offices,
- support declarations,
- force claims,
- crime registers,
- accusation/punishment flows,
- institutional beliefs.

That is a strong start.

But FOUNDATIONS IV.23–25 demands a broader and denser social artifact layer:
- bounties,
- notices,
- contracts,
- warrants,
- debts,
- obligations,
- proof rules,
- payout sources,
- public posting places,
- expiration,
- contestability,
- forgery/destruction/copying paths.

### Why this matters
Without a unified artifact/record substrate, the architecture will keep solving social processes one bespoke type at a time.

### Practical failure mode
You will get islands of social simulation instead of a general social world.

### Fix
Introduce a unified artifact/record model with first-class identity and transfer:

- issuer,
- current custodian,
- location,
- authenticity state,
- proof requirements,
- jurisdiction,
- expiration,
- linked claims,
- allowed mutations (copy, destroy, forge, amend, archive).

That one substrate would unlock a huge amount of FOUNDATIONS alignment.

## 7. The rights model is only partially there

The architecture distinguishes placement, ownership, reservation, and social relations. Good.

But FOUNDATIONS IV.24 is stricter:
- ownership,
- custody,
- access,
- obligation,
- and jurisdiction
must be separable.

### Why this matters
Without that separation, theft, confiscation, delegated access, guild property, office property, inheritance, seizure, and legal dispute resolution will stay shallow or collapse into awkward special cases.

### Fix
Add explicit first-class modeling for:
- custody,
- access grants,
- keys/seals/lock state,
- obligations/debts,
- jurisdiction scope,
- and legal basis for access or seizure.

This is one of those additions that looks “administrative” until you realize it massively increases downstream consequence density.

## 8. Boundary processes are a hard missing piece

This is not subtle. The architecture report itself effectively says so.

FOUNDATIONS II.13 and Canonical Scenario H make boundary processes non-optional:
- imported goods,
- remote shortages,
- upstream failures,
- scheduled arrivals,
- migration pressure,
- delayed information,
- cross-boundary institutions.

### Why this matters
Without boundary processes, your world is too sealed. Shortage, substitution, rationing, and delayed shock propagation remain underdeveloped.

### Fix
Build explicit boundary interfaces with:
- source region,
- stock or flow model,
- route/channel,
- delay,
- capacity,
- observables,
- failure modes,
- and evidence of arrival or non-arrival.

Off-map cannot mean “spawn when convenient.”

## 9. Scheduling is explicit, but not fully world-modeled yet

The explicit system order is better than accidental execution order. That is good.

But FOUNDATIONS II.9 asks for more than an engine schedule. It asks for a world model of simultaneity and tie-breaking.

### Why this matters
A global order like `Needs -> Production -> Trade -> Combat -> ...` is fine as engine machinery. It is not enough by itself for every world-meaningful contest.

### Practical failure mode
Two agents reaching the same affordance in the same tick, or two actions becoming mutually incompatible in the same phase, may still be resolved by engine structure rather than explicit world arbitration.

### Fix
Where order changes meaning, define in-world resolution:
- simultaneous windows,
- race tokens,
- arbitration records,
- declared precedence,
- or explicit “who acquired what first” artifacts.

## 10. Agent diversity is stronger in motives than in reasoning style

You already have diversity in:
- drive thresholds,
- utility weights,
- courage,
- perception fidelity,
- memory capacity/retention,
- contradiction tolerance.

That is good.

But deliberation style still looks too uniform:
- shared planning budget,
- shared retry TTLs,
- shared cooldown curves,
- shared switch margins.

### Why this matters
FOUNDATIONS IV.22 is not just about what agents care about. It is also about how differently they behave under uncertainty and pressure.

### Fix
Introduce per-agent reasoning style:
- search depth,
- search patience,
- switch reluctance,
- retry timing model,
- trust weighting,
- consultation habits,
- alarm responsiveness,
- contradiction handling style.

That will create more differentiated emergence without cheating.

## 11. Some useful heuristics are drifting too close to abstract control levers

This is not the worst issue, but it is worth correcting early.

Danger class, competition discount, and enterprise signal are legal as planner-side heuristics. FOUNDATIONS allows that. But they need to stay visibly derived from concrete observable state.

### Why this matters
If these summaries become the real driver of behavior instead of an agent-local compression of evidence, they start violating the spirit of FOUNDATIONS I.2, I.3, and V.27.

### Fix
Push them closer to concrete observed facts:
- visible arms/armor/allies,
- route incidents,
- queue lengths,
- failed purchases,
- stockout observations,
- posted prices,
- seller reliability,
- actual waiting lines,
- and observed competitor claims.

## 12. Validation is good, but not yet aligned with your own acceptance standard

The existing test suite is already stronger than most simulation projects ever get.

But FOUNDATIONS VI does not treat canonical regression chains as optional examples. They are acceptance criteria.

### Current read
- **Strongest support:** C and D
- **Partial support:** B, E, F
- **Missing or unproven as full generic chains:** A, G, H

That matters.

### Fix
Promote missing scenario classes to hard architecture gates:
- A. Beast Starvation → Caravan Attack → Report → Bounty → Hunt → Reward
- F. Office Vacancy → Succession Delay → Patrol Gap → Route Predation
- G. False Rumor → Wrongful Accusation → Contested Evidence → Correction or Miscarriage
- H. Remote Shock → Delayed Arrival Failure → Local Shortage → Substitution or Exit

Also add falsification suites for:
- contradictory testimony,
- memory overflow/eviction,
- 5+ claimant contention,
- materialization-binding races,
- long-horizon plan disruption,
- off-map shock propagation.

## Improvements That Would Most Improve Alignment

## A. Build a claim-centric epistemic substrate
This is the highest-value architecture improvement.

Make beliefs and reports into structured claims with:
- subject,
- proposition,
- claimed event time,
- acquisition time,
- source chain,
- confidence,
- freshness,
- carrier.

Let agents maintain:
- endorsed working beliefs,
- unresolved alternatives,
- confidence gradients,
- and provenance-aware corrections.

## B. Build a unified artifact / record / evidence substrate
Do not implement bounties, debts, warrants, notices, accusations, and contracts as disconnected custom cases.

Give them a shared substrate with:
- stable identity,
- custody,
- location,
- authenticity,
- edit/copy/forge/destroy paths,
- and inspectable linkage to claims and institutions.

## C. Generalize contention into a reusable world mechanic
Every scarce or exclusive affordance should resolve through:
- queue,
- grant,
- reservation,
- race,
- or explicit contest.

No silent entitlement through planning.

## D. Make plan assumptions first-class and derived
Right now the frame system is conceptually correct, but its assumption vocabulary is still narrow.

Let plans automatically track the specific claims, affordances, promises, reservations, and access conditions they depend on. Then invalidate them generically when those supports break.

## E. Unify planner semantics and runtime semantics
Do not let planning become a separate approximate universe.

Either:
- derive planner effects from authoritative action effect declarations,
- or keep a shared semantics source with stricter conformance guarantees.

## F. Separate communication types
At minimum:
- alarm,
- testimony,
- gossip,
- record consultation,
- and formal accusation
should not all share the same treatment.

## G. Make more of “surprise” explicit
FOUNDATIONS wants surprise to arise from violated expectation.

So model expectations explicitly:
- expected stash contents,
- promised deliveries,
- patrol schedules,
- reservations,
- assignments,
- owed payments.

That will make discovery, blame, and investigation more general and more legible.

## H. Put agent diversity into reasoning style, not just motive weights
This is one of the cheapest ways to buy more emergence.

## Features Worth Adding

These are the highest-value additions I would prioritize because they close real FOUNDATIONS gaps.

## 1. Bounty / notice / contract / warrant pipeline
Needed for FOUNDATIONS IV.25 and Scenario A.

Must include:
- issuer,
- jurisdiction,
- reward source,
- proof requirements,
- posting place,
- expiration,
- claimant competition,
- and payout from a real treasury or obligated sponsor.

## 2. Office duty, delegation, and vacancy degradation
Needed for Scenario F.

Add:
- patrol duty,
- escort duty,
- treasury release duty,
- record maintenance duty,
- delegation,
- succession delay,
- recognizable service gaps,
- and recovery paths.

## 3. Beast ecology and nonhuman actor chains
Needed for Scenario A.

Add:
- territory,
- food pressure,
- prey depletion,
- range expansion,
- attack choice,
- aftermath evidence,
- and reportable institutional consequences.

## 4. Boundary trade and remote shock model
Needed for Scenario H.

Add:
- off-map stocks,
- scheduled arrivals,
- convoy failure,
- rerouting,
- import dependence,
- shortage propagation,
- substitution,
- rationing,
- and departure pressure.

## 5. Alarm and messenger carriers
High value even before full institutions.

Add:
- shouts,
- runners,
- letters,
- posted notices,
- town criers,
- and messenger jobs.

That will massively improve information locality without cheating.

## 6. Persistent physical evidence
Needed for FOUNDATIONS III.18 and Scenario G.

Add:
- tracks,
- blood trails,
- broken locks,
- damaged doors,
- tampered seals,
- scorch marks,
- missing-inventory records,
- burial markers.

Evidence should:
- persist,
- decay,
- be misread,
- be contestable,
- and be destroyable.

## 7. Full property-rights and access-control model
Needed for FOUNDATIONS IV.24.

Add:
- custody,
- delegated access,
- keys,
- locks,
- seals,
- obligations,
- debt/lien,
- seizure,
- confiscation,
- jurisdictional legality.

## 8. Contradictory testimony and adjudication
Needed for Scenario G.

Add:
- witness conflict,
- alibis,
- forged records,
- source reputation,
- institutional bias,
- burdens of proof,
- correction,
- and non-correction.

## 9. Generic claim/race/queue tokens across domains
Do this once, then reuse it for:
- workstations,
- corpses,
- patients,
- witness time,
- bounty claims,
- storage access,
- investigation priority,
- output pickup.

## What I Would Not Do

- **I would not replace GOAP right now.**  
  That is not the bottleneck.

- **I would not patch missing scenario chains with bespoke helper pipelines.**  
  That would directly betray FOUNDATIONS.

- **I would not keep widening the goal catalog before fixing carriers.**  
  More goals on top of weak epistemics/artifacts/contention will just create more cross-cutting special cases.

## Recommended Order Of Operations

1. **Keep the substrate.** Do not rewrite the deterministic ECS/action/event architecture.
2. **Fix epistemics first.** Richer uncertainty and contradiction will improve many domains at once.
3. **Add generic contention/arbitration.** That closes a major FOUNDATIONS gap and prevents hidden entitlement.
4. **Add the unified artifact/record/evidence substrate.** This unlocks institutions, justice, bounties, and better knowledge flow.
5. **Expand the rights model.** Ownership alone is not enough.
6. **Add boundary processes.** Otherwise the world stays too sealed.
7. **Promote the missing canonical scenario classes to hard gates.**
8. **Only then widen the behavior surface.**

## Final Verdict

Yes: the current AI architecture has real issues to fix.

But the important distinction is this:

- It is **not** a bad architecture.
- It **is** an incomplete one relative to FOUNDATIONS.

You already have the bones of the right system:
- deterministic causality,
- explicit actions,
- belief separation,
- replay,
- tracing,
- and state-mediated interaction.

What you do **not** yet have is the full carrier ecology that FOUNDATIONS demands:
- rich contradictory claims,
- generalized contention,
- dense social artifacts,
- full property/jurisdiction distinctions,
- and explicit boundary processes.

That is the gap.

If you fix those before continuing to expand behavior, the architecture can grow into the FOUNDATIONS vision cleanly.

If you do not, the likely failure mode is not immediate collapse.  
It is slower and worse: the system will keep growing in ways that look impressive locally while silently accumulating the exact special-case pressure the foundations were written to forbid.