# S149PARPLASEG-005: Agenda-manager partial-plan resumption

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — agenda-manager suspended partial-plan resume/abandon lifecycle; follow-up ticket for executable tactical re-entry
**Deps**: archive/tickets/S149PARPLASEG-002.md, archive/tickets/S149PARPLASEG-003.md, archive/tickets/S149PARPLASEG-004.md

## Problem

D5 gave the agenda manager responsibility for evaluating a suspended intention's `PartialPlanSegment` when its resume conditions held and for abandoning it when an abandon condition fired or patience was exhausted. Live reassessment showed the live `PlannedSkeletonStep` carrier was not yet executable: it stored `PlannerOpKind`, `PayloadTemplate`, and expected predicates, but not resolved action definitions, targets, payloads, or the planner context needed to rebuild lawful `PlannedStep`s. The completed slice landed the concrete agenda lifecycle behavior and created the follow-up owner for true tactical suffix re-entry.

## Assumption Reassessment (2026-05-20)

1. `try_resume_partial_plan` and `ResumedPlan` do not yet exist (confirmed during reassessment). `RuntimeBeliefView` is the belief surface, defined at `crates/worldwake-sim/src/belief_view.rs:1596`. The agenda manager's active lifecycle path is `tick_agenda` / `demote_to_pending_or_suspended` at `crates/worldwake-ai/src/agenda_manager.rs`; its inline test block begins in the same file.
2. `IntentionFrame.patience_limit` (`crates/worldwake-core/src/intention_frame.rs:141`) bounds resume retries; exceeding it abandons via `IntentionAbandonCondition::PatienceExhausted`. `PartialPlanSegment.resume_attempt_count` (ticket 002) increments per try.
3. Shared boundary under audit: the agenda-manager suspended-entry lifecycle and the still-missing executable tactical re-entry surface. The current `PartialPlanSegment.remaining_skeleton` is a template carrier, not an executable planner entry point; reconstructing a suffix from it would require a new resolver contract. This ticket performs resume/abandon decision + retry accounting only; a follow-up owns executable skeleton resolution and planner re-entry. Barrier→failure attribution is ticket 004; companion-subgoal/coordination triggers are 006/007.
4. AI regression layer: this is a runtime `agent_tick`/agenda-manager change. Resume/abandon evaluation reads `RuntimeBeliefView` (belief-only planning, FND-14) — no authoritative world read on behalf of the agent.
5. Adjacent contradiction classification: segment construction at barrier time remains required for full D5, but the live code does not yet retain enough executable barrier-time inputs to build all segment fields lawfully at the agenda manager alone. This ticket does not invent placeholder `BarrierFact`s or synthetic `PlannedStep`s; segment writers stay deferred with the executable re-entry follow-up unless a caller already provides a segment.
6. Placeholder relationship: `archive/tickets/S149PARPLASEG-003.md` added `partial_plan_segment: None` on every construction path as a compile-safe default; this ticket is the first writer that populates it with `Some(..)`. No earlier placeholder symbol needs replacing — 003's `None` default is the intended initial state, not a stub.
7. Budget-exhaustion handoff from ticket 001: direct no-plan search-budget exhaustion still returns `PlanSearchResult::BudgetExhausted`, not a terminal-bearing found plan. Turning an eligible budget-exhausted suspension into a terminal-bearing `PartialPlanSegment` requires the same writer/re-entry substrate and is deferred to the follow-up created by this ticket.

## Architecture Check

1. This ticket avoids constructing a speculative second planner path from non-executable skeleton templates. That is cleaner under `docs/FOUNDATIONS.md` than fabricating targets or payloads, because tactical re-entry must be backed by concrete stored planner inputs before it becomes live (FND-3, FND-14, FND-20).
2. Resume/abandon are explicit, bounded, revisable commitments (FND-21): `resume_attempt_count` + `patience_limit` are the concrete dampeners; `PatienceExhausted` is the lawful exit. No unbounded retry loop.

## Verified Layers

1. Existing suspended segment resumes when a condition holds → focused runtime test on `try_resume_partial_plan` (segment present, condition satisfied → `Some(ResumedPlan)`).
2. Abandon fires before resume when an abandon condition holds → focused runtime test (abandon condition true → intention cleared, segment dropped, no resume).
3. Bounded retries → focused runtime test: `resume_attempt_count` exceeding `patience_limit` abandons via `PatienceExhausted`.
4. Tactical re-entry is not proved by this ticket; follow-up owns suffix reconstruction/re-entry and its runtime decision-trace proof.

## Landed Changes

### 1. `try_resume_partial_plan`

Added `fn try_resume_partial_plan(state: &mut AgendaState, actor: EntityId, belief_view: &dyn RuntimeBeliefView, tick: Tick, patience_limit: u32) -> Option<ResumedPlan>`. Live reassessment found no patience limit stored on `AgendaEntry` or `PartialPlanSegment`, so the caller supplies the relevant frame/profile limit when integrating this helper into the broader tick path. For each suspended entry with a segment: (1) if a represented abandon condition holds, abandon and clear; (2) else if a represented resume condition holds, increment `resume_attempt_count`, abandon via `PatienceExhausted` if it exceeds `patience_limit`, else return the segment as a `ResumedPlan`; (3) else leave suspended. Full tick-pass adoption is deferred with executable re-entry because returning a resumed segment is not yet the same as selecting a runnable plan.

`MotiveSourceLost` and `AssumptionPermanentlyBroken` remain outside this partial-plan helper because the current segment carrier has no motive/assumption frame substrate. Those conditions remain frame-lifecycle concerns until the executable re-entry substrate lands.

### 2. Follow-up for executable segment writing and tactical re-entry

Created the now-archived `archive/tickets/S149PARPLASEG-010.md` for segment writer inputs, budget-exhausted segment construction, and tactical suffix re-entry. The follow-up defined the remaining owner for resolving partial-plan re-entry through belief-backed planner inputs without duplicating planner authority.

## Landed Files

- `crates/worldwake-ai/src/agenda_manager.rs` — `try_resume_partial_plan`, retry bounding, `ResumedPlan`, focused tests.
- `crates/worldwake-ai/src/lib.rs` — public re-export for `ResumedPlan` and `try_resume_partial_plan`.
- `archive/tickets/S149PARPLASEG-010.md` — executable segment writer + tactical re-entry follow-up.
- `archive/tickets/S149PARPLASEG-006.md`, `archive/tickets/S149PARPLASEG-007.md`, `archive/tickets/S149PARPLASEG-008.md`, `archive/tickets/S149PARPLASEG-009.md` — dependency and handoff updates for the new executable re-entry owner.
- `specs/S149-partial-plan-segments-and-typed-terminals.md` — active spec wording aligned to the narrowed 005 seam and new 010 owner.

## Out of Scope

- Companion `AskWitness` synthesis for information barriers (ticket 006).
- Coordination watching-list triggers (ticket 007).
- Observer rendering of resume/abandon state (ticket 008).
- Constructing `PartialPlanSegment`s from search/barrier/budget-exhaustion results.
- Re-entering tactical planner search from `completed_prefix` / `remaining_skeleton`.

## Acceptance Criteria

### Verified Acceptance Criteria

1. A suspended intention with a satisfied resume condition returns `Some(ResumedPlan)`; with an unsatisfied condition returns `None`.
2. A satisfied represented abandon condition clears the intention and segment before any resume attempt.
3. `resume_attempt_count > patience_limit` abandons via `PatienceExhausted`.
4. Follow-up ticket exists for executable segment writing, budget-exhausted segment construction, and tactical suffix re-entry.
5. Existing suite passed: `cargo test -p worldwake-ai`

### Invariants

1. Resume/abandon evaluation reads only `RuntimeBeliefView` — never authoritative world state on the agent's behalf (FND-14).
2. Resume retries are bounded by `patience_limit`; no resumption path can loop unboundedly.
3. The implementation does not synthesize executable planner steps from incomplete skeleton templates.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai agenda_manager`.
- Passed `cargo test -p worldwake-ai`.

## Outcome

Completed on 2026-05-20.

This ticket landed the agenda-manager partial-plan lifecycle slice: `try_resume_partial_plan` evaluates represented resume and abandon conditions through `RuntimeBeliefView`, increments bounded resume attempts, records the last resume-attempt tick, returns pending `ResumedPlan` values for callers, and removes abandoned suspended entries.

Deviation from the original D5 draft: executable segment writing, budget-exhausted segment construction, and tactical suffix re-entry were not implemented in this ticket. The live `PlannedSkeletonStep` carrier did not contain executable planner inputs, so those responsibilities moved to the now-archived `archive/tickets/S149PARPLASEG-010.md`. The helper also takes `patience_limit` as an argument because neither `AgendaEntry` nor `PartialPlanSegment` stores that frame/profile value.
