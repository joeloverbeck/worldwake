# Improvements to AI Architecture

I suspected that, as our AI architecture had grown somewhat organically to align with docs/FOUNDATIONS.md , it could be substandard. I generated the report at reports/ai-architecture-analysis.md and fed it to ChatGPT Pro so that it could find issues, possible improvements to make, or even features to add.

## ChatGPT's response

There are real issues. But the architecture is not substandard overall.

The core bet is right: belief-only planning, deterministic/event-sourced causality, no central “story brain,” explicit actions with duration and interruption, and one unified pipeline for physiological, economic, social, and political behavior. That is unusually well aligned with your stated goal of explainable emergence. I would not replace this with behavior trees, a pure utility system, or anything more “clever” on paper. Your main problems are not the paradigm. They are hidden state, duplicated semantics, and a few places where runtime convenience is starting to outrun the causal model.

The pressure system and utility weights are also fine. They are derived decision heuristics over concrete local state, and your foundations explicitly allow derived views and concrete per-agent parameters as long as they never become world truth. So I would keep the pressure/ranking layer.

Here’s where I think the architecture is genuinely weak.

### 1. Too much causally relevant state lives outside authoritative world state

AgentDecisionRuntime is explicitly persistent across ticks and explicitly outside the authoritative simulation, yet it holds current goal/plan, journey commitment, blocker-like state, and queued facility intents. In your foundations, memories, commitments, queues, evidence, and similar state are supposed to be real causal state, and save/load or offscreen compression is not allowed to change world meaning. If this runtime can affect later choices, then it is already authoritative in practice, whether you call it that or not.

This is the biggest structural issue.

What I’d do:
Promote state into three buckets.

1. Authoritative private mind-state: beliefs, blocked assumptions, active commitments, resumable intentions.
2. Authoritative public claim-state: queue positions, reservations, grants, notices, contracts, tickets.
3. Ephemeral caches: last observation signatures, dirty flags, search scratch, rederivable bindings.

Right now those buckets are blurred. They should not be.

### 2. The planner runs on a hand-maintained shadow semantics layer

This is the most dangerous engineering liability in the whole design.

You have real action semantics in handlers (start/tick/commit/abort) and separate planning semantics in PlannerOpSemantics, classify_action_def(), hypothetical transitions, materialization barriers, and state shadowing. The architecture report even calls out the six-point integration burden for every new action. That means every new affordance is a drift risk: the planner can “understand” an action differently from the world that actually executes it.

That is the closest thing in your current design to a fake second authority path.

What I’d do:
Create a single action-semantics contract that both planning and execution consume. At minimum, each action should declare in one place:

- preconditions
- occupied capacities / reservations
- symbolic effects for planning
- materialization schema
- interruption policy
- failure taxonomy

If a full unification is too big right now, do the next best thing immediately: add conformance tests that compare the planner’s hypothetical transition against the authoritative handler outcome on the same setup. That’s a pure win.

### 3. You still have a hidden contention/order-bias problem

Deterministic order is good. Hidden order as the mechanism of reality is bad.

Agents are processed sequentially in deterministic BTreeMap order, and the scheduler consumes deterministic input ordering. That is great for replay. But if two agents lawfully compete for the same scarce affordance and the actual winner is “whoever got processed first this tick,” then you have exactly the kind of invisible authority your foundations reject. Your canonical contention scenario explicitly says queues, races, grants, expiry, and blockers must be inspectable world state, not hidden planner magic or arbitrary tick order. The presence of queued_facility_intents in runtime makes me especially suspicious here.

What I’d do:
For every scarce or exclusive affordance that matters, introduce explicit world artifacts such as QueueEntry, ReservationGrant, ClaimTicket, or Contest. Then resolve access against those artifacts, not against entity-ID order. Tie-breaks should come from observable local rules: arrival, priority, office, force, queue rules, jurisdiction, or agreed policy.

Where no explicit race/queue/grant exists, sequential tick order should never silently become the law of the world.

### 4. The belief model is not rich enough for your own knowledge game

You do separate belief from truth correctly. That part is good. But the belief model still looks too thin for what FOUNDATIONS.md actually demands.

Your foundations want provenance, acquisition time, confidence, freshness, contradiction, and explicit knowledge travel when those matter. The architecture proves stale-belief scenarios and some social transmission, but the described belief structures still read mostly like clean “believed facts” with last-known place/alive state rather than competing claims with source quality and age. That is enough for basic partial observability. It is not enough for rumor, deception, conflicting testimony, institutional reporting, investigative chains, or political maneuvering with evidentiary weight.

This is where I think your future emergence will plateau unless you deepen the model.

What I’d do:
Represent beliefs as records, not just facts. A useful belief atom should carry:

- proposition
- source or carrier
- acquisition tick
- modality: seen / heard / inferred / read
- confidence
- freshness
- maybe chain length or provenance path

Then let contradictory belief records coexist. Planning can still query a compressed view, but the compressed view should be derived from first-class belief records, not replace them.

One conditional warning here: treating topology, facilities, and resource source locations as universal “public structure” is only correct if you truly want universal common knowledge of those things. If you ever want exploration, hidden resources, asymmetric local knowledge, or institutions knowing things peasants do not, that assumption will become a straightjacket.

### 5. Journey commitment is the right instinct, but too narrow

Journey commitment is smart. It solves a real problem: travel thrash. But it is a special-case solution to a more general need.

Your foundations want revisable commitments, not rails. Right now goal switching works at the whole-goal level, and journey commitment only partly protects one class of multi-step effort. That still leaves the system weak at “pause this, do emergency thing, then resume if assumptions still hold” for non-travel chains like care, pickup, bargaining, escort, or political errands.

What I’d do:
Generalize journey commitment into an intention frame model:

- current continuation cursor
- assumptions relied on
- suspension reason
- resume policy
- expiry conditions

That would handle your dragon-interrupts-market-trip class of scenario much more naturally than whole-plan abandonment plus later full replanning.

### 6. BlockedIntentMemory is too blunt

The current blocker model is useful, but it’s too coarse. It suppresses whole goals with TTLs based on a blocking fact. That works as a safety valve. It does not preserve enough causally meaningful detail. “Seller unavailable here,” “route unsafe because of report X,” “prerequisite missing at workstation Y,” and “this exact target is gone” are not the same kind of failure, and they should not all collapse into “don’t pursue this goal for a while.” The Unknown blocker especially is a bad smell, because it turns diagnostic ignorance into long-lived behavior suppression.

What I’d do:
Store failed assumptions, not just failed goals.

Key them more narrowly:

- goal
- method/op kind
- target or place
- blocker type
- source tick / evidence
- confidence / expiry

Then clear them on relevant state changes, not mostly on TTL expiry. And make Unknown short-lived plus loudly traceable.

### 7. Your invalidation model is brittle

A single dirty flag and hand-maintained snapshot signatures is a maintainability trap. The architecture report already admits the problem: add a new causally relevant dimension and forget to thread it into the snapshot comparison, and the AI can quietly stop replanning when it should. That is not a simulation design problem; it is a structural architecture problem.

What I’d do:
Move to dependency-based invalidation.

When a plan is built, record the facts/beliefs/signatures it depended on. When the world or belief store changes, invalidate plans whose dependencies changed. If full dependency tracking is too much right now, at least replace dirty: bool with typed invalidation reasons or version stamps by domain: needs, wounds, inventory, facilities, social facts, political facts, route safety, and so on.

That is nearly pure upside.

### 8. The agenda is too feasibility-blind

Generate candidates, rank them, then search top four is a sensible first architecture. It is not the best mature one.

The report already points out the failure mode: a high-priority but infeasible goal can consume one of the only planning slots while a lower-priority obvious action goes unplanned. That produces avoidable stupidity for reasons that have nothing to do with believable character limitations.

What I’d do:
Keep motive ranking, but insert a cheap feasibility sketch before full search. Not a full plan—just enough to answer “does this look locally actionable from the agent’s current beliefs?” Examples:

- direct affordance exists
- route exists
- required target is known
- blocker memory says no
- one-step prerequisite reachable

Then allocate full GOAP budget using motive plus feasibility hint. That preserves your “goal is desired world condition, not privileged solution” principle while wasting much less search effort.

### What I would add

Your current candidate generation is mostly need/signal driven. I would add a whole family of expectation-violation goals. That is exactly where your foundations are strongest and your current architecture looks underpowered. Your own canonical scenarios want “I expected X, found not-X, now react.” Right now you handle some of this through stale-belief correction and failure recovery, but it should be a first-class goal source.

Examples:

- expected stock missing -> investigate / report / replace supply
- expected person absent -> wait / search / suspect / reroute
- expected payment overdue -> remind / accuse / escalate
- expected route safe -> observe danger -> retreat / warn / choose alternate route
- expected stash full -> empty -> search / report robbery / hide loss / retaliate

That would buy you a lot of emergence without any quest logic.

I would also add knowledge-path traces. Your diagnostics are already a strength, but your foundations want both the causal path and the knowledge path. For every chosen goal and every adopted plan, I want to see the exact beliefs, records, witnesses, notices, or observations that justified it. “Why did this agent do that?” should be answerable as a proof tree, not just a score dump.

### Purely beneficial cleanups

Even if you leave the larger design alone, these are almost all upside:

Split process_agent() into explicit deterministic stages with typed input/output structs. Right now too much integration logic lives in one 950-line function.
Split search_plan() into frontier policy, heuristic policy, transition model, and plan assembly. Same reason.
Add planner-vs-executor conformance tests for every action family.
Replace boolean dirtiness with typed invalidation or dependency stamps.
Make all unknown blocker cases diagnostic hot spots, not long silent suppressions.
Move queue/reservation state into inspectable world state wherever it affects contention.

### What I would prioritize
1. Promote real mental/public claim state out of runtime limbo.
2. Unify planner semantics with executable action semantics, or at least enforce conformance.
3. Make contention explicit and inspectable instead of letting order bias decide reality.
4. Deepen belief records with provenance/freshness/confidence/contradiction.
5. Generalize journey commitment into resumable intention frames.
6. Replace dirty-flag invalidation and coarse blocker memory with dependency/assumption-based versions.

My blunt take: the architecture is already good enough to produce strong emergence, and its foundations are much better than most simulation AI work. But if you want it to stay faithful to FOUNDATIONS.md as the world gets denser, the next big step is not “smarter planning.” It is making the AI’s own causal state more honest, more explicit, and less duplicated. That is where the current design is soft.