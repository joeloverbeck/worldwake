---
name: goap-architecture-report
description: "Use when needing to produce a self-contained GOAP architecture report for external LLM evaluation. Triggers: planning pipeline analysis, GOAP scaling concerns, external review of agent decision cycle, preparing context for ChatGPT Pro research on planner improvements."
user-invocable: true
---

# GOAP Architecture Report

Generates a self-contained technical report of the full agent decision cycle — from goal ranking through replanning. The report includes concrete type definitions, function signatures, algorithm descriptions, and relevant FOUNDATIONS.md principles inline. Output is optimized for an external LLM (ChatGPT Pro) that has no repository access.

## Invocation

```
/goap-architecture-report
```

No arguments. Fully autonomous. Writes to `reports/goap-architecture-report.md`.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Reading FOUNDATIONS.md

`docs/FOUNDATIONS.md` exceeds single-read token limits. Read it in 2–3 chunks using offset/limit (e.g., lines 0–150, 150–350, 350+) or target specific principle sections needed for Section 9. All planning-relevant principles (at minimum FND-1, 3, 7, 12, 14, 16, 20, 22, 28, 29) must be read with full text for inline embedding.

## Process

Follow these 3 phases in order. Do not skip any phase.

### Phase 1 — Discovery & Source Tracing

This phase combines test discovery and source tracing. They are deeply interleaved (tests reference the same types being traced) and should be dispatched as a single parallel batch of up to 3 Explore agents, not run sequentially.

**Recommended agent grouping** (adapt if volume shifts):
- **Agent A** — Stages 1–3: goal ranking, candidate generation, affordance queries (pipeline entry).
- **Agent B** — Stage 4: strategic + tactical search internals (landmarks, frontier, heuristic, `PlanningSnapshot`).
- **Agent C** — Stages 5–8 + test discovery: revalidation, dispatch, replanning, cognitive parameters, plus the `tests/golden_*.rs` inventory.

**From golden E2E tests** (`crates/worldwake-ai/tests/`): For each test, extract planning-relevant elements:

1. **Goal kinds triggered** — which `GoalKind` variants appear
2. **Candidate generation paths** — synthesized, affordance-derived, planner-only
3. **Search parameters** — beam width, expansion budget, landmark depth, boost values
4. **Plan outcomes asserted** — success, budget exhaustion, frontier exhaustion, candidate counts
5. **Planning components set up** — `CognitiveProfile`, `ExecutionBudget`, `PlanningSnapshot` configuration

**From source code** (`crates/worldwake-ai/src/`): Grep for all types and functions referenced in the report template sections. At minimum: `search_plan`, `rank_candidates`, `generate_candidates`, `get_affordances`, `handle_plan_failure`, `plan_revalidation`, `DualFrontier`, `LandmarkSet`, `StrategicPlan`, `PlanningSnapshot`, `CognitiveProfile`, `ExecutionBudget`, `GoalKind`, `ActionDef`, `RuntimeBeliefView`, `BlockedIntentMemory`.

For each element found, trace into source and record: concrete type names with field lists, function signatures, algorithm logic, data flow, connections to other elements, constraints and invariants.

**Pipeline stages to trace** (in pipeline order):

1. **Goal ranking** — pressure-based selection, `rank_candidates()`, suppression filters, `GoalKind` variants, how needs become goals, priority ordering
2. **Candidate generation** — `generate_candidates()`, `generate_candidates_with_travel_horizon()`, the 17 `emit_*` gate functions (e.g., `emit_need_candidates()`, `emit_production_candidates()`), blocked-intent filtering, how candidate count relates to branching factor
3. **Affordance queries** — `get_affordances_for_defs()`, `enumerate_targets()`, `RuntimeBeliefView`, locality scoping via `effective_place()`, `ActionDef` structure, `relevant_ops` per goal kind
4. **Plan search** — `search_plan()`, strategic planner (`strategic.rs` — location-visit itinerary from beliefs), tactical planner, landmark extraction (`landmarks.rs` — delete-relaxation, `PlanningFact`, achievers, shared preconditions), dual frontier (`frontier.rs` — preferred/regular queues, boost mechanism), heuristics (`heuristic.rs` — spatial + landmark count), beam truncation, expansion budget, `PlanningSnapshot` as belief surface
5. **Plan revalidation** — `plan_revalidation.rs`, `requested_affordance_matches()`, `with_payload_override_validator`, stale plan detection
6. **Action dispatch** — `BestEffort` action start (find via grep in the `agent_tick` module), what happens between "plan found" and "action executes"
7. **Replanning** — `handle_plan_failure()` (find via grep in `failure_handling.rs` or the ai crate), replan triggers (action failure, budget exhaustion, belief contradiction), belief update on failure, how the agent re-enters the decision pipeline
8. **Cognitive parameters** — all `CognitiveProfile` fields, all `ExecutionBudget` fields, per-agent diversity effects

**Stage → Report Section mapping**: 1→§2, 2→§3, 3→§4, 4→§5, 5+6→§6, 7→§7, 8→§8. Stages 5 (plan revalidation) and 6 (action dispatch) both feed Section 6 of the report — do not split them into separate top-level sections.

### Phase 2 — Live Diagnostics (documentation-only)

This phase is always performed; only *running live tests* is optional. Document the trace infrastructure as it exists on disk — do not skip the phase.

Check if diagnostic trace types exist that capture planning metrics (e.g., `PlanAttemptTrace`, `SearchExpansionSummary`, `CandidateGenerationDiagnostics`). Grep for these types and document their fields and what they capture.

If trace infrastructure exists, describe it in the report. Do not run `cargo test` to extract live metrics — the trace type documentation is sufficient. If no metric capture infrastructure exists, note this as a gap and describe what metrics would be valuable. Do not block report generation on diagnostics.

### Phase 3 — Report Assembly

Write the report to `reports/goap-architecture-report.md`. Do not commit.

## Report Structure

The report MUST include all of the following sections:

```markdown
# GOAP Architecture Reference — YYYY-MM-DD

## 1. Architecture Context
Crate structure (core, sim, systems, ai), ECS basics (BTreeMap-based typed
component storage), determinism guarantees (ChaCha8Rng, no floats, no HashMap).
Key foundational types used by the planner: EntityId, Permille, Quantity, Tick.
Just enough for an external LLM with no repo access to orient.

## 2. Goal Ranking
How needs become goals. Pressure-based ranking algorithm. GoalKind variants
(full enum with fields). Suppression filters (what prevents goals from being
considered). Priority ordering. Key types and function signatures.

## 3. Candidate Generation
How goals become plannable candidates. generate_candidates_with_travel_horizon(),
the 17 emit_* gate functions, blocked-intent filtering. Emission gates.
Locality scoping via RuntimeBeliefView. How candidate count relates to
branching factor. Include actual candidate count ranges if available.

## 4. Affordance Queries
How available actions are discovered from beliefs. get_affordances_for_defs(),
enumerate_targets(), target scoping (EntityAtActorPlace, etc.). ActionDef
structure with key fields. relevant_ops per goal kind.

## 5. Plan Search Pipeline
The core planning algorithm. Strategic planner (location-visit itinerary from
beliefs — StrategicPlan, StrategicStep, TacticalSubGoal types). Tactical
planner (action sequence search at each location). Landmark extraction
(delete-relaxation algorithm, PlanningFact enum, achievers, shared
preconditions, LandmarkSet). Dual frontier (preferred/regular queues, boost
mechanism, DualFrontier struct). Heuristics (spatial + landmark count,
combination logic). Beam truncation. Expansion budget. PlanningSnapshot as
the belief surface. Include algorithm descriptions detailed enough for
external evaluation.

## 6. Plan Revalidation & Execution
How plans are checked before execution. plan_revalidation.rs,
requested_affordance_matches(), payload override validators. BestEffort
action start (in agent_tick module). What happens between "plan found"
and "action executes."

## 7. Replanning
handle_plan_failure() lifecycle. What triggers replanning (action failure,
budget exhaustion, belief contradiction). How beliefs update on failure.
How the agent re-enters the decision pipeline. Replan limits if any.

## 8. Cognitive Parameters
All per-agent planning parameters with their **default values** and effects, organized as a table:
- CognitiveProfile fields (max_node_expansions, landmark_extraction_depth, etc.)
- ExecutionBudget fields (beam_width, preferred_operator_boost, etc.)
Include each field's default value from the `impl Default` block so the
external evaluator can reason about the out-of-the-box planner shape.
How agent diversity (FND-22) manifests in planning behavior.

## 9. FOUNDATIONS Alignment
For each relevant principle (at minimum FND-1, 3, 7, 12, 14, 16, 20, 22,
28, 29):
- The principle text (embedded inline — full text, not summary)
- How the current architecture satisfies or tensions it
- Where alignment is strong vs. where there may be gaps

## 10. Live Diagnostics
(If captured) Candidate counts per expansion, nodes expanded, plan depths
reached, landmarks extracted, plan success/failure rates from representative
scenarios.
(If not captured) Note explaining what metrics would be valuable and why
they couldn't be captured.

## 11. Architectural Observations
Patterns, asymmetries, scaling concerns, or oddities noticed during analysis.
Flags only — no recommendations. The external LLM's job is to evaluate these
and propose changes.
```

## Guardrails

- **Self-contained**: The report must make complete sense without repository access. Never write "see file X" without including the relevant content inline. Include type definitions with fields, enum variants, function signatures, algorithm pseudocode.
- **Planning pipeline only**: Focus on the decision cycle from goal ranking through replanning. Exclude domain-specific system details (needs/metabolism mechanics, production recipes, trade protocols, combat rules). Include domain systems only where they illustrate how the planner interfaces with them (e.g., "production goals use these candidate generation paths").
- **No prescriptions**: Section 11 flags patterns but does not recommend fixes. The external LLM's job is evaluation and recommendation.
- **Concrete over vague**: Exact type names, field lists, function signatures, algorithm pseudocode. `GoalKind::SatisfyNeed { need_kind: NeedKind }` not "goals for satisfying needs."
- **Embedded FOUNDATIONS**: Include full principle text inline in Section 9 for all planning-relevant principles. The external LLM also receives FOUNDATIONS.md separately, but inline embedding provides immediate context next to the architecture it governs.
- **Parallel agents**: Use up to 3 Explore agents in parallel for Phase 1 when volume warrants it (>10 test files).
- **Date stamp**: Always include the current date in the report title.
- **No commit**: Write the file and stop.
- **Previous report diff** (optional): If `reports/goap-architecture-report.md` already exists, briefly note major structural changes (new sections, removed types, renamed functions) at the top of the new report in a `> **Changes since last report (YYYY-MM-DD):**` blockquote. Skip if no previous report exists or if changes are minor.
