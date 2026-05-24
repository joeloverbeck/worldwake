# S168PARPLASKE-007: Re-enable info-barrier suspension producer after end-to-end D7 validation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/agent_tick/planning.rs`, `crates/worldwake-ai/src/agenda_manager.rs`.
**Deps**: `archive/specs/S168-partial-plan-skeleton-reuse.md` D1.b; `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` D7; `archive/tickets/S168PARPLASKE-006.md` (the disabled producer).

## Problem

S168PARPLASKE-006 added `write_information_barrier_partial_plan_segment` to activate the previously-dormant S149 D7 mechanism: when a plan terminal is `InformationBarrier { topic }`, the agent should suspend the primary pursuit so `spawn_information_barrier_companions` can spawn an `AskWitness` companion that learns the topic, after which the primary resumes via `BeliefStatusChanged`.

Activating the producer broke 6 gated goldens (planner-pathology degenerate, simulation-gaps multi-agent-convergence, scenario-diagnostics fixture, survival-tell, survival-baseline drink + dirtiness). The fix in this branch disabled the producer; the D7 chain returned to its pre-S168 dormant state (consumer + resume conditions present, but never driven).

The disable is conservative — the D7 mechanism is architecturally meaningful and should be re-enabled — but the producer cannot ship until the chain has been validated end-to-end. The specific failure modes the disable papered over need each to be diagnosed and fixed before the producer is reactivated.

## Failure Modes To Resolve Before Re-enable

1. **Witness unavailable at suspension time.** `select_information_barrier_witness` requires a co-located, alive, agent witness who plausibly knows the topic and has not already been asked. When no witness exists, no companion spawns and the suspended entry sits forever (`revival_trigger: None`, `kill_condition: External`). Reproduced by `planner_pathology_degenerate::degenerate_zero_step_loop_blocks_actionable_goals` (Lina alone in Eldergrove with no co-located agents).
2. **Multi-agent scenarios where witnesses ARE co-located but the chain still fails.** Survival baseline and simulation-gaps had multiple agents in shared places yet still failed. The producer's promise to the consumer (a companion will spawn and resolve the barrier) was not honored in practice. Diagnose whether the companion AskWitness spawns, whether it executes successfully, whether the witness's belief actually contains the topic, and whether `BeliefStatusChanged` fires on the primary afterward.
3. **No safety net for stuck suspensions.** The current `KillCondition::External` makes suspended info-barrier entries unrecoverable except through the companion path. If that path is broken or never fires, the agent is trapped.

## Architecture Check

1. **Witness gating must be symmetric with the consumer.** If the producer suspends only when `spawn_information_barrier_companions` could plausibly spawn a companion (same witness selector applied), suspension and companion creation become guaranteed-symmetric — no orphaned suspensions.
2. **Safety-net kill condition.** Even with a witness present at suspension time, the chain can break later (witness leaves, companion fails). Adding `KillCondition::TickExpiry { at_tick: tick + N }` (or a TickElapsed resume condition) bounds the trap — at worst the agent loses N ticks before re-creating the goal.
3. **Skeleton reuse already in place.** The skeleton-source carrier and the seeded-search consumer are unaffected by the producer's disabled state; budget-exhausted skeleton reuse continues to work. The D7-producer's contribution is purely about activating the suspension/companion/resume cycle.

## Verification Layers

1. **Witness-gate symmetry** → focused unit test: producer suspends iff `select_information_barrier_witness(actor, beliefs, topic)` returns `Some`.
2. **Companion → resume chain** → new golden: agent A has plan terminating at `InformationBarrier { topic: EntityBelief { subject } }`; agent B is co-located and plausibly knows the topic. Expectation: agent A suspends, AskWitness companion is spawned at the next tick, A travels/talks to B, B shares belief, A resumes the original pursuit. Trace should show `PartialPlanResumeTrace { decision: ReusedSeededSearch | FallbackToReplan... }`.
3. **No-witness fallback** → focused integration: agent A has the same plan but no co-located plausible witness. Expectation: no suspension, plan adopts and executes toward the barrier; the agent makes incidental progress and re-plans next tick.
4. **Safety-net expiry** → focused unit test: a suspended info-barrier entry with no companion progress is auto-cleared after `TickExpiry` so the agent regenerates the candidate.
5. **All 6 originally-regressed goldens** must remain green after re-enable (planner-pathology degenerate; simulation-gaps multi-agent-convergence; scenario-diagnostics fixture; survival-tell row-five; survival-baseline drink + dirtiness).

## What to Change

### 1. Witness gate in producer

`write_information_barrier_partial_plan_segment` (currently a stub returning `false` in `agent_tick/planning.rs`) gains a belief-view parameter. Before suspending, it calls `select_information_barrier_witness` (made `pub(crate)` in `agenda_manager.rs`) and returns `false` if `None`.

### 2. Kill-condition safety net

Replace `KillCondition::External` with `KillCondition::TickExpiry { at_tick: tick + cognitive.search_exhaustion_backoff_ticks }` on the suspended entry, so an info-barrier suspension is bounded even if the companion path subsequently breaks.

### 3. End-to-end companion-chain golden

A focused authored-scenario golden demonstrating the full D7 chain: producer suspends → consumer spawns → AskWitness commits → primary resumes. Lives next to the existing partial-plan-terminals goldens in `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs`.

### 4. Restore producer call sites

The two call sites in `plan_and_validate_next_step_*_with_opportunity_index` (currently calling the stubbed producer that always returns `false`) remain; once the gate + safety-net are in place, the call sites perform their original early-return-with-trace behavior.

### 5. Restore the focused producer unit tests

The 3 producer tests deleted in this branch (`write_information_barrier_partial_plan_segment_suspends_selected_goal_with_skeleton`, `..._allows_missing_skeleton_source`, `..._does_not_suspend_ask_witness_companion`) need to be rewritten to provide a belief view fixture that controls witness availability, then re-added.

## Out of Scope

- Skeleton reuse for budget-exhausted suspensions (already working).
- Other barrier kinds (Coordination, Resource, Jurisdiction).
- Replacing the AskWitness companion mechanism with a different information-acquisition primitive.

## Notes

- The disabled producer leaves the consumer (`spawn_information_barrier_companions`) as dead code at present — same state as pre-S168. The skeleton-source carrier on `CandidatePlanSearch` is still alive (used by `write_budget_exhausted_partial_plan_segments`).
- `information_barrier_partial_plan_segment` (the constructor in `partial_plan.rs`) is kept for the producer's re-enable; its focused construction test in `partial_plan.rs` remains green.
