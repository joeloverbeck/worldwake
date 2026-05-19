# S149PARPLASEG-005: Agenda-manager partial-plan resumption

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — agenda-manager resumption path; tactical-planner re-entry; barrier-time segment construction
**Deps**: S149PARPLASEG-002, S149PARPLASEG-003, S149PARPLASEG-004

## Problem

D5 gives the agenda manager the ability to resume a suspended intention from its `PartialPlanSegment` when its resume conditions hold — picking up at the prefix-tail rather than replanning from scratch — and to abandon it when an abandon condition fires or patience is exhausted. This ticket also constructs the segment at barrier time (writing it onto the suspended `AgendaEntry`), since resumption has nothing to read otherwise.

## Assumption Reassessment (2026-05-20)

1. `try_resume_partial_plan` and `ResumedPlan` do not yet exist (confirmed during reassessment). `RuntimeBeliefView` is the belief surface, defined at `crates/worldwake-sim/src/belief_view.rs:1596`. The agenda manager's suspend path is `demote_to_pending_or_suspended` at `crates/worldwake-ai/src/agenda_manager.rs:412`; its inline test block begins at line 596.
2. `IntentionFrame.patience_limit` (`crates/worldwake-core/src/intention_frame.rs:141`) bounds resume retries; exceeding it abandons via `IntentionAbandonCondition::PatienceExhausted`. `PartialPlanSegment.resume_attempt_count` (ticket 002) increments per try.
3. Shared boundary under audit: the agenda-manager tick pass (suspend/resume/abandon) and the tactical-planner re-entry surface (`completed_prefix` applied to planning state, `remaining_skeleton` completed against new world state). Phase distinction: this ticket performs candidate-side resumption decision + plan-search re-entry; barrier→failure attribution is ticket 004; companion-subgoal/coordination triggers are 006/007.
4. AI regression layer: this is a runtime `agent_tick`/agenda-manager change. Resume/abandon evaluation reads `RuntimeBeliefView` (belief-only planning, FND-14) — no authoritative world read on behalf of the agent.
5. Adjacent contradiction classification: segment construction at barrier time is a required consequence of D5 (resumption needs stored segments); it is in-scope here, not a separate ticket. The barrier→condition derivation it calls is owned by ticket 004.
6. Placeholder relationship: ticket 003 added `partial_plan_segment: None` on every construction path as a compile-safe default; this ticket is the first writer that populates it with `Some(..)`. No earlier placeholder symbol needs replacing — 003's `None` default is the intended initial state, not a stub.
7. Budget-exhaustion handoff from ticket 001: direct no-plan search-budget exhaustion still returns `PlanSearchResult::BudgetExhausted`, not a terminal-bearing found plan. To support the S149 `SearchBudgetExhausted` resume/golden path, this ticket owns the first writer that turns an eligible budget-exhausted suspension into a `PartialPlanSegment` whose `terminal_barrier` is `PlanTerminalKind::SearchBudgetExhausted`.

## Architecture Check

1. Resuming from `completed_prefix` re-enters the existing tactical search rather than registering an HTN method — consistent with the spec's plain-GOAP planner-formalism analysis and the "no method-decomposition resumption" non-goal (FND-20).
2. Resume/abandon are explicit, bounded, revisable commitments (FND-21): `resume_attempt_count` + `patience_limit` are the concrete dampeners; `PatienceExhausted` is the lawful exit. No unbounded retry loop.

## Verification Layers

1. Suspend writes a segment; resume returns it when a condition holds → decision-trace / focused runtime test on `try_resume_partial_plan` (segment present, condition satisfied → `Some(ResumedPlan)`).
2. Abandon fires before resume when an abandon condition holds → focused runtime test (abandon condition true → intention cleared, segment dropped, no resume).
3. Bounded retries → focused runtime test: `resume_attempt_count` exceeding `patience_limit` abandons via `PatienceExhausted`.
4. Tactical re-entry completes the suffix → runtime `agent_tick` decision-trace test: resumed plan continues from prefix-tail and reaches `GoalSatisfied` against updated world state (full action registries required — the suffix exercises real affordances).

## What to Change

### 1. Construct segment at barrier time

When a plan reaches a typed barrier and the intention is suspended (`demote_to_pending_or_suspended`), build a `PartialPlanSegment` (prefix steps, terminal barrier, barrier fact, resume/abandon conditions via ticket 004's derivation) and store it on the suspended `AgendaEntry.partial_plan_segment`.

Also handle eligible budget-exhausted suspensions: when the search result is `PlanSearchResult::BudgetExhausted` and the agenda manager parks the intention for retry rather than discarding it, construct the segment with `terminal_barrier: PlanTerminalKind::SearchBudgetExhausted { .. }` so later resume/observer/golden tickets have a typed terminal-bearing record to consume.

### 2. `try_resume_partial_plan`

Add `fn try_resume_partial_plan(state: &mut AgendaState, actor: EntityId, belief_view: &dyn RuntimeBeliefView, tick: Tick) -> Option<ResumedPlan>`. For each suspended entry with a segment: (1) if any abandon condition holds, abandon and clear; (2) else if any resume condition holds, increment `resume_attempt_count`, abandon via `PatienceExhausted` if it exceeds `patience_limit`, else return the segment as a `ResumedPlan`; (3) else leave suspended. Wire the call into the agenda manager's existing tick pass.

### 3. `ResumedPlan` + tactical re-entry

Define `ResumedPlan` and re-enter the tactical planner with `completed_prefix` applied to the planning state, completing `remaining_skeleton` against current world state.

## Files to Touch

- `crates/worldwake-ai/src/agenda_manager.rs` (modify) — segment construction, `try_resume_partial_plan`, tick-pass wiring, `ResumedPlan`
- `Likely: crates/worldwake-ai/src/search/mod.rs` or the tactical-planner entry (modify) — prefix-applied re-entry; grep the tactical-search entry point to pin placement

## Out of Scope

- Companion `AskWitness` synthesis for information barriers (ticket 006).
- Coordination watching-list triggers (ticket 007).
- Observer rendering of resume/abandon state (ticket 008).

## Acceptance Criteria

### Tests That Must Pass

1. New: a suspended intention with a satisfied resume condition returns `Some(ResumedPlan)`; with an unsatisfied condition returns `None`.
2. New: a satisfied abandon condition clears the intention and segment before any resume attempt.
3. New: `resume_attempt_count > patience_limit` abandons via `PatienceExhausted`.
4. New (agent_tick decision trace): a resumed plan continues from the prefix-tail and reaches `GoalSatisfied`.
5. New: an eligible budget-exhausted suspension stores a `PartialPlanSegment` with `PlanTerminalKind::SearchBudgetExhausted`.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Resume/abandon evaluation reads only `RuntimeBeliefView` — never authoritative world state on the agent's behalf (FND-14).
2. Resume retries are bounded by `patience_limit`; no resumption path can loop unboundedly.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (inline) — suspend/resume/abandon + bounded-retry cases.
2. `crates/worldwake-ai/tests/scenarios/` or `agent_tick/tests.rs` — decision-trace re-entry test (prefix-tail continuation).

### Commands

1. `cargo test -p worldwake-ai agenda`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
