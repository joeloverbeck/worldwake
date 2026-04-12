**Status**: ✅ COMPLETED

# Proposed GOAP architecture upgrades aligned with FOUNDATIONS

Changes are warranted. The current system is already principled, but it will bottleneck (performance) and exhibit brittle choice dynamics (top‑2 cliff) before it reaches your “hundreds of goals” target. fileciteturn0file1

The upgrades below are designed to be “architecturally comprehensive,” preserve determinism and belief-boundedness, and make agent reasoning more scalable without adding outcome-authoring. fileciteturn0file0

```markdown
## Worldwake GOAP Upgrade Proposal (Non‑LLM, Emergence‑First)

### Goals of the upgrade
- Scale to hundreds of simultaneous goal candidates per agent without thrash, stalls, or “top‑2 cliff” behavior.
- Keep strict FOUNDATIONS compliance:
  - Belief-bounded planning only (no omniscient reads).
  - No ungrounded drama triggers or “interestingness” dials.
  - Deterministic + replayable decision outcomes.
  - All reasoning remains inspectable: “Agent X chose Y because they believed Z and cared about Q.”
- Reduce per-tick compute spikes by making planning incremental / anytime and by avoiding repeated full recomputation.

### Architectural changes (high level)
1) Add an explicit **Goal Agenda + Intention Manager** (BDI-like, but still GOAP for action search)
2) Replace single-shot tactical A* calls with an **Anytime Incremental Planner** (resume search across ticks)
3) Introduce a lightweight **Hierarchical Goal Network (HGN) / Skill layer** to reduce branching for deep chains
4) Upgrade heuristics from “count-ish” to **relaxed-plan / landmark-cost** heuristics (FF-style + costed landmarks)
5) Replace brute-force revalidation with **Assumption-Tracked Monitoring** (invalidate by belief diffs, not by full affordance enumeration)
6) Add **Opportunistic Plan Shaping** so agents satisfy secondary goals “for free” when already co-located (without multi-goal global optimization)
7) Add structured metrics for scaling + falsification tests

---

## Goal Agenda + Intention Manager

### New agent-local state
- `GoalAgenda: BTreeMap<GoalKey, GoalAgendaEntry>`
- `GoalAgendaEntry` fields:
  - `status: {Candidate, Suppressed, ActiveIntention, CoolingDown, Blocked, Satisfied}`
  - `last_rank_tick`, `last_motive_score`, `last_priority_class`
  - `evidence_trace_ref` (pointer/id to stored trace)
  - `blocked_intent_ref: Option<BlockedIntentId>`
  - `next_reconsider_tick` (deterministic schedule, no wall-clock)
  - `attempt_history: RingBuffer<PlanAttemptRecord>` (bounded, agent-local learning)

### Core behavior
- Candidate generation continues to emit `GroundedGoal`s, but the agenda becomes the stable “memory” across ticks:
  - merge new candidates into `GoalAgenda`
  - decay / expire stale candidates by explicit tick rules (not random)
- Ranking becomes incremental:
  - recompute motives for goals whose drivers changed (needs ticked, danger changed, new evidence arrived, blocker cleared)
  - keep all others cached
- Intention selection:
  - choose ONE “active intention” plus optional “side-intentions” that are strictly subordinate and co-located
  - enforce hysteresis via existing switch margins, but apply it at agenda level (not only at “top candidates to plan”)

### Fix for the top‑2 cliff
- Replace `max_candidates_to_plan: 2` with:
  - `planning_queue_depth` (e.g., top 8–20 by motive)
  - `planning_effort_scheduler` that allocates limited planning work across those candidates using deterministic round-robin + urgency weighting
- Guarantee: even if the top goal is blocked/unplannable, the agent will eventually attempt lower-ranked viable goals instead of idling.

---

## Anytime Incremental Planner (resume across ticks)

### Replace "plan from scratch" with persistent planning episodes
- New type: `PlanningEpisode`
  - owns frontier(s), closed set, best-so-far plan, and diagnostics
  - keyed by `(GoalKey, OpportunityAnchor, snapshot_signature)`
  - can be resumed for N expansions each tick, then paused
- Each tick:
  - spend a bounded number of node expansions across active planning episodes (deliberation scheduling)
  - if an episode yields a feasible plan, commit to the first step (or first K steps) and keep episode as an improvement candidate

### Anytime mode (fast first plan, refine later)
- Use weighted evaluation early (e.g., `f = g + w*h`) to find a plan quickly.
- If time remains, reduce `w` and continue, reusing prior search (ARA*/ANA* style reuse):
  - update node priorities deterministically
  - improve plan quality without restarting from empty frontier

### Real-time / dynamic world safety
- Plans remain revisable commitments (existing replan model stays).
- If belief changes invalidate the episode’s assumptions, cancel the episode and record a blocker + attempt record.

---

## Hierarchical Goal Networks / Skills layer

### Motivation
Deep multi-step goals (craft chains, institutional procedures, investigations, escort workflows) should not require the tactical search to rediscover “standard operating procedures” every time.

### Add a new authored-but-generic layer: `SkillDef`
- A `SkillDef` is a reusable decomposition of a GoalKind into:
  - a goal network (ordered or partially ordered) of subgoals
  - plus applicability conditions expressed ONLY over the agent’s belief snapshot
  - plus explicit “legal affordance families” it may use (no privileged actions)
- Examples of generic (non-plot) skills:
  - AcquireCommodity via Trade | Harvest | Scavenge | Theft (each is a method)
  - TreatWounds via SelfCare | SeekHealer | SeekFacility | Improvise
  - PostBounty via FindOfficeHolder → PresentEvidence → CreateArtifact → PostAtBoard

### Planning pipeline integration
- Strategic planning becomes HGN planning over places + subgoals (replaces bespoke BFS stages).
- Tactical A* remains, but it is invoked on smaller subproblems:
  - either “satisfy this subgoal here”
  - or “perform this one skill step”
- Allow “action insertion semantics”:
  - if the skill method is missing a model detail, the planner can still insert actions found by GOAP search to satisfy required facts
  - avoids brittle method scripting

### FOUNDATIONS safeguards
- Skills may encode only reusable lawful decomposition knowledge.
- Skills must emit:
  - knowledge-path provenance (why the agent believes the skill applies)
  - explicit failure states and aftermath hooks (for emergence)
- No skill may directly mutate world state; only actions do.

---

## Heuristic upgrades (planner-internal only)

### Add relaxed-plan heuristic (FF-style)
- Build a relaxed planning graph over `PlanningFact`s (delete-relaxation).
- Extract a relaxed plan length as heuristic `h_ff_ticks` (integer).
- Derive “helpful actions” (subset of affordances) for pruning / preferred expansion.

### Add costed landmark heuristic
- Replace landmark count with landmark *cost* lower bounds:
  - each landmark has an estimated minimal cost-to-achieve (ticks + resource costs)
  - combine via admissible aggregation when possible; otherwise use bounded inadmissible but controlled weighting
- Optional later upgrade: operator-counting / LM-cut style heuristic under your restricted PlanningFact vocabulary.

### Determinism constraints
- No floating point.
- Stable ordering for graph layers and achiever enumeration (BTreeMap/BTreeSet iteration).
- Any tie-breaking must be explicit and controlled (never by hash iteration order).

---

## Assumption-Tracked Monitoring (cheap invalidation)

### Replace “enumerate all affordances” revalidation as the default
- When a plan/episode is created, record a **Support Set**:
  - the minimal set of belief claims used:
    - target identity + believed location + confidence + freshness band
    - seller existence + stock belief
    - facility availability
    - required tool presence
    - jurisdiction/legality beliefs for institutional actions
- Maintain a `BeliefDeltaIndex` each perception update:
  - which belief keys changed (entity moved, stock changed, confidence dropped, etc.)
- Invalidate plans/episodes by intersecting belief deltas with support sets.
- Fall back to full affordance enumeration only when:
  - delta is ambiguous
  - handler requires dynamic validity checks

### Benefits
- Orders-of-magnitude reduction in revalidation cost in crowded places.
- Much better introspection: “plan broke because assumption X changed.”

---

## Opportunistic Plan Shaping (multi-goal without global multi-objective search)

### After a primary plan is found, run a cheap “local insert” pass
- Identify side-goals from the agenda that:
  - are below the primary goal’s urgency (no self-care / danger violations)
  - are satisfiable at the same place(s) already visited by the plan
  - have low incremental cost (bounded by a small tick budget)
- Insert actions that satisfy those goals if they do not:
  - violate reservations/occupancy
  - exceed a strict incremental cost cap
  - increase expected danger beyond thresholds

### Result
Agents appear to “remember to do small things while already there”
(e.g., wash while at the well; buy bandages while at the market),
without turning planning into global optimization.

---

## Metrics + falsification tests (must be implemented with the upgrade)
Capture as structured traces per agent and aggregate:
- planning episode count, expansions per tick, frontier peak size, snapshot size
- plan success rate per GoalKind (Found vs BudgetExhausted vs FrontierExhausted)
- revalidation mode counts (support-set invalidations vs full affordance enumeration)
- blocker types and mean time-to-clear
- “agenda starvation” metric: max ticks since last attempt for any high-motive goal

Add golden/regression scenarios aligned with FOUNDATIONS:
- interrupted plans due to new local evidence
- contention resolution via explicit world artifacts (queues/reservations)
- stale belief causing wasted trip, then correction via observation

---

## Migration steps (safe incremental rollout)
1) Introduce GoalAgenda in parallel with current pipeline (observability-only mode)
2) Switch selection from “top-2” to agenda-backed scheduling (keep old single-shot planner)
3) Add PlanningEpisode and resume-across-ticks (keep old A* implementation inside episode)
4) Add support-set monitoring to reduce revalidation cost (keep fallback enumeration)
5) Add FF-style heuristic + helpful-action pruning (measure plan quality + expansions)
6) Add SkillDef/HGN layer for 2–3 deep GoalKinds first (crafting + investigation + institutional posting)
7) Expand SkillDef coverage as needed; retire bespoke strategic BFS when equivalent
```

## Why these changes are the best fit for Worldwake

The strongest upgrade is **not** “switch GOAP to another monolithic planner.” You already have a principled GOAP core. The winning move is to add the missing layers that classical AI planning research has learned to add over decades:

A **goal/intention management layer** is what makes it possible to have *many desires* without recomputing everything every tick and without pathological “I only ever try the top item” behavior. This is the core of BDI-style practical reasoning: intentions as revisable commitments, plus explicit reconsideration when beliefs change. citeturn2search9turn2search6turn2search13 fileciteturn0file0

An **anytime/incremental planning layer** is what makes planning cost predictable in a live simulation. Weighted/anytime heuristic search and ARA*-style reuse specifically target the “find something quickly, improve if time remains” regime that games and real-time sims live in. citeturn0search8turn1search2turn1search10turn0search16

A **hierarchical goal/skill layer** is how you scale to deep chains without writing brittle scripts. Done correctly, it shrinks the effective branching factor while keeping outcomes emergent (because only lawful actions change the world, and decompositions are generic procedures). HGN work is particularly relevant because it keeps “goal-based” semantics while adding hierarchy and enabling heuristic transfer. citeturn3search3turn3search7turn2search1 fileciteturn0file0

A **stronger heuristic layer** gets you “more planning power per expansion.” FF-style relaxed-plan heuristics and LM-cut/operator-counting families are exactly about turning the same operator model into better guidance. citeturn0search2turn0search3turn0search11

Finally, **assumption-tracked monitoring** is what turns replanning from brute-force to principled: plan breaks because an assumption changed, not because a periodic revalidation happened to fail. That is directly aligned with Foundations’ demand for inspectable causal and knowledge paths. citeturn2search2turn2search20 fileciteturn0file0

## Concrete validation criteria that protect emergence

To keep this upgrade from “accidentally becoming a clever hack,” validation must explicitly target both *behavioral credibility* and *FOUNDATIONS invariants*.

The most important behavioral falsification tests are your Foundations canonical scenario classes: interruption by danger during travel, theft discovered only via expectation mismatch, rumor leading to a wasted trip then local correction, contention resolved via explicit queues/reservations, and institutional actions triggered only by physically carried information. fileciteturn0file0

The most important scalability metrics are planner-internal and must be recorded as structured traces (not ad hoc logs): expansions per tick, peak frontier size, snapshot size, plan success rates per goal kind, and rate of “fallback expensive revalidation.” These are exactly the missing structured metrics your GOAP report already identifies as useful for scaling analysis. fileciteturn0file1

A final hard requirement: deterministic replay must remain stable. Anytime and incremental planning are compatible with determinism as long as (a) you schedule expansions by simulation tick, not wall time, and (b) iteration order/tie-breaking is explicit and stable. This is critical because Foundations treats scheduling/tie-breaking as part of world meaning, not an implementation detail. fileciteturn0file0