# Per-Agent Narrative Structure

This reference defines the per-agent narrative template used in report Section C, the depth-scaling rule that controls how much narrative each agent receives, and the fixed vocabulary used to classify decision failures.

## Depth Scaling

Every agent in the scenario gets a narrative entry. Depth scales with causal contribution:

- **Tracked principals** (agents named in the scenario's `survival_health_contract` or whose actions own the scenario's primary feature row): full template, all sections below.
- **Supporting causal actors** (witnesses, hostile targets, wards, bystanders whose presence is required for the principal's branch): the Starting State, Critical Decisions, and Final State sections only, focused on their causal role rather than their full lifecycle.
- **Human-controlled agents with no input**: a one-paragraph entry stating that no AI decisions occurred for this agent and naming any human-driven actions present in Section 2. Do not pad.

The roadmap's per-row prose typically names the principals explicitly ("Agent X owns the 1440-tick survival-health envelope while Y is a supporting hostile target"). Use that classification when present; otherwise infer from the survival-health contract field.

## Per-Agent Template (Tracked Principal)

For each principal:

### Starting State

One paragraph. Cover:

- **Who they are**: name, occupation if implied by the scenario, control source, the one or two profile traits that most define their behavior in this scenario (e.g., "Guard Mira begins with an authored patrol route between Watchtower and Crossroads, a `pursuit_profile` that engages hostile targets within 3 hops, and a `combat_profile` capable of melee").
- **Where they begin**: starting place, immediately co-located entities, possessions of any consequence.
- **What they know**: any seeded beliefs that matter for the run (last-seen memory, known recipes, expectations, social observations).
- **The pressure they're under**: which needs are seeded above their critical thresholds, any obligations they carry, any directed hostilities or expectations.

### Critical Decisions

A chronological narrative of the agent's major decisions through the run. Major means: the first selection of each new goal kind, the first commit of each action family, every decision failure that mattered (see vocabulary below), and every inflection moment (replan after a contradiction, behavioral-mode transition, death, post-satiation pivot).

For each decision quoted, name:

- The tick.
- The goal kind selected (or the decision context if no plan was selected).
- The plain-English reason the planner reached for it (drawn from Section 7's "Goals selected" rows, ranking notes, and the active needs at the tick).
- What happened next — committed action, blocked desire, replan, or budget exhaustion.

Avoid blow-by-blow tick-level rehearsal. The narrative arc matters; redundant ticks of the same self-care loop should be summarized ("between ticks 200 and 600 the agent committed `drink` four more times at Stone Well, each preceded by a routine perception sweep at Central Crossing").

### Decision Failures

For every decision failure that mattered, classify the cause using the fixed vocabulary below and state the consequence (replan to what, budget exhaustion at what tick, blocked desire that resolved when, etc.). Do not list every transient `StartFailed` — only those that altered the agent's trajectory.

**Fixed decision-failure vocabulary**:

- **Precondition violation** — the planner selected an action whose authoritative validator rejected it at start time. Name the precondition.
- **Belief-target unknown** — the planner needed a target entity (place, item lot, recipe source) the agent had no belief about. Useful tag for "geographic desert" stories.
- **Frontier exhaustion** — the planner's frontier emptied without finding a viable plan. Frequently signals contention or substrate gaps.
- **Budget exhaustion** — the planner ran out of search budget. Quote the tick and the goal in flight from Section 7's snapshot if Section 8 is present.
- **Contention loss** — the agent committed `queue_for_facility_use` (or its equivalent) and never received a grant within the relevant window.
- **Payload revalidation rejection** — `plan_revalidation` rejected a planner-synthesized payload at execution time. State whether a `with_payload_override_validator` was registered for the action and whether the rejection was structural.
- **Stale belief contradiction** — the agent's plan depended on a belief that the world or perception subsequently invalidated; replan followed.
- **Authority withheld** — the action was structurally available but legitimacy substrate (ownership, office, control) blocked it.
- **Affordance not generated** — `get_affordances` did not surface the candidate the agent needed; the planner therefore never even considered it.

If a failure does not fit the vocabulary, name a new category in plain English and explain it; do not force the misfit. Such cases are also good candidates to flag in `traceability-fix-protocol.md`.

### Belief Evolution

A narrative summary of how the agent's beliefs changed across the run. Cover:

- New entities discovered (place, item lot type, agent, social artifact) — name the discovery tick when legible.
- Hearsay accepted or rejected — quote source agent, content, and the tick of acceptance/rejection.
- Contradictions registered — when a belief was invalidated by direct observation or a later authoritative event.
- Persistent gaps — places visited but never reflected in the belief summary, durable failure memory carried forward, expectations resolved or unresolved.

If the dump's belief summary is end-state-only and cannot tell the evolution story, this is the most common cheap-fix candidate per `traceability-fix-protocol.md`.

### Final State

One short paragraph. Where they ended, what they possessed, the state of each tracked need at end-of-run, whether the survival-health contract was satisfied, and whether their authored purpose for this run was achieved.

## Per-Agent Template (Supporting Actor)

Use the principal template's Starting State, a stripped-down Critical Decisions section narrowly focused on the actor's causal role (e.g., "the bandit raider's only narrative-relevant moment is the tick-840 attack that wounded the ward, triggering the caretaker's escort branch"), and Final State. Skip Belief Evolution and Decision Failures unless the actor's belief or failure is itself the story.

## Tone Rules

- Active voice. "The merchant staged apples for sale at tick 42" not "apples were staged."
- Name the agent by their scenario name, not by ID. Use full names on first reference, short names afterward.
- Quote ticks for landmark events; never quote ticks decoratively.
- When the planner's reason is interesting (substrate isolation, contention pressure, escalation override), narrate it; when the reason is boring (routine self-care selecting the only viable affordance), summarize.
- Do not editorialize on AI competence ("the agent intelligently chose..."). Describe what happened; the reader is doing the deep-research interpretation.

## Cross-Reference With Section B

Each per-agent narrative is concrete *evidence* for the feature rows named in Section B. When the agent's narrative names a committed action, it should be possible to trace that action back to the corresponding feature-row paragraph in Section B without re-explaining the mechanic. Section B explains the *system*; Section C narrates *one agent's encounter* with the system.
