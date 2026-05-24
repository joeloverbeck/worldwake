# S168PARPLASKE-006: Information-barrier segment production

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/agent_tick/planning.rs`, `crates/worldwake-ai/src/partial_plan.rs`, and `crates/worldwake-ai/src/lib.rs`.
**Deps**: `archive/tickets/S168PARPLASKE-002.md` (budget-exhausted skeleton population and construction filter); `archive/tickets/S168PARPLASKE-005.md` (planner-owned skeleton source carrier); `specs/S168-partial-plan-skeleton-reuse.md` (D1.b corrected producer boundary).

## Problem

S168 D1.b still requires populated `PartialPlanSegment.remaining_skeleton` for information-barrier suspensions, but live reassessment during S168PARPLASKE-002 proved the drafted producer path was wrong. `spawn_information_barrier_companions` consumes suspended entries that already have a `PartialPlanSegment`; it cannot be the first producer because it skips entries whose segment is absent.

This ticket implemented the lawful producer boundary for selected plans whose
`PlanTerminalKind` is `InformationBarrier`. The producer preserves the high-level
skeleton from the planner-owned `PartialPlanSkeletonSource`, builds a
`PartialPlanSegment` with `PlanTerminalKind::InformationBarrier { .. }`, and suspends
the matching agenda entry so the existing companion-spawn consumer can discover the
topic and later resume through tickets 001/003.

## Assumption Reassessment (2026-05-24)

1. **Live consumer checked**. `agenda_manager.rs::spawn_information_barrier_companions` iterates suspended agenda entries and requires `entry.partial_plan_segment.as_ref()` before it can inspect `PlanTerminalKind::InformationBarrier { topic }`.
2. **Live budget producer checked**. `agent_tick/planning.rs::write_budget_exhausted_partial_plan_segments` is now the budget-exhausted producer and consumes `CandidatePlanSearch.skeleton_source`. Information barriers need an analogous lawful producer at the barrier-plan selection/completion boundary, not inside the companion consumer.
3. **Shared boundary under audit**. The producer must bridge `PlannedPlan.terminal_kind == PlanTerminalKind::InformationBarrier { .. }`, the matching `CandidatePlanSearch.skeleton_source`, and `AgendaEntry.partial_plan_segment`.
4. **FOUNDATIONS boundary**. The producer may not synthesize a skeleton after the planner referent is gone. It must use the already-carried `PartialPlanSkeletonSource`, and absence of a source must remain lawful by preserving `remaining_skeleton: None` or falling back to ordinary replan behavior.
5. **Sibling impact**. Ticket 003 can consume any populated skeleton through focused tests, but ticket 004's information-barrier reuse/fallback goldens require this producer before the end-to-end chain is truthful.

## Architecture Check

1. Producing information-barrier segments at the plan-selection or plan-completion boundary keeps the agenda companion path a consumer of suspended intentions rather than a circular producer/consumer.
2. Reusing the construction filter from S168PARPLASKE-002 keeps combat and fixed-identity steps out of information-barrier skeletons too.
3. The producer preserves existing agenda arbitration and ranking authority by only suspending the already-selected barrier-blocked pursuit.

## Verified Layers

1. Information-barrier segment production -> focused unit/runtime test showing a selected or completed information-barrier plan parks a suspended agenda entry with `PartialPlanSegment`.
2. Skeleton threading -> assertion that the segment carries filtered `remaining_skeleton` from `CandidatePlanSearch.skeleton_source`.
3. Companion consumer compatibility -> focused agenda test showing `spawn_information_barrier_companions` sees the produced segment and spawns the same `AskWitness` companion as before.
4. Negative no-source case -> focused test showing absence of `PartialPlanSkeletonSource` does not synthesize a skeleton and remains lawful.

## Landed Changes

### 1. Producer seam

Selected `PlanSearchResult::Found(PlannedPlan)` values whose terminal is
`PlanTerminalKind::InformationBarrier { .. }` now write a suspended agenda entry before
normal plan adoption, while the selected `CandidatePlanSearch.skeleton_source` is still
available.

### 2. Information-barrier segment construction

Added `information_barrier_partial_plan_segment`, which builds a `PartialPlanSegmentSeed`
with:

- the selected `GoalOffer`;
- an empty completed prefix for this selected-barrier boundary;
- filtered `remaining_skeleton` from `PartialPlanSkeletonSource`;
- `PlanTerminalKind::InformationBarrier { topic }`;
- `BarrierFact::MissingBelief(...)` for entity-belief topics;
- stable tick/counter identity.

### 3. Agenda suspension

The matching pending agenda entry is copied into `AgendaPhase::Suspended` with the
constructed segment, removed from pending, and left for the existing
`spawn_information_barrier_companions` consumer. `AskWitness` and `ShareBelief`
information-barrier plans are deliberately not suspended by this producer, because those
are the companion/social plans that must remain executable.

### 4. Tests

Added focused tests for segment construction/filtering, selected-plan production,
skeleton threading, no-source fallback, companion-plan non-suspension, and existing
companion consumer compatibility.

## Landed Files

- `crates/worldwake-ai/src/agent_tick/planning.rs` (selected barrier producer and focused tests)
- `crates/worldwake-ai/src/partial_plan.rs` (dedicated information-barrier segment constructor and focused test)
- `crates/worldwake-ai/src/lib.rs` (constructor re-export)
- `archive/tickets/S168PARPLASKE-006.md` (closeout)

## Out of Scope

- Budget-exhausted skeleton population — ticket 002.
- Skeleton revalidation — archived ticket 001.
- Seeded search and resume trace — ticket 003.
- Validation goldens — ticket 004.
- Resource/jurisdiction/coordination barrier skeleton production — S168 non-goals.

## Acceptance Result

### Tests Passed

1. Passed focused information-barrier producer coverage through `cargo test -p worldwake-ai --lib information_barrier`.
2. Passed focused skeleton-threading coverage through `write_information_barrier_partial_plan_segment_suspends_selected_goal_with_skeleton`.
3. Passed focused no-source fallback coverage through `write_information_barrier_partial_plan_segment_allows_missing_skeleton_source`.
4. Passed companion compatibility coverage through existing `agenda_manager` information-barrier companion tests under the same selector, plus the new `AskWitness` non-suspension regression.
5. Passed existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Information-barrier skeletons come only from planner-owned `PartialPlanSkeletonSource`; no source yields `remaining_skeleton: None`.
2. `spawn_information_barrier_companions` remains a consumer of suspended information-barrier segments.
3. No combat or fixed-target-identity skeleton step is preserved.
4. Absence of a skeleton source remains lawful and does not block ordinary fallback behavior.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs::write_information_barrier_partial_plan_segment_suspends_selected_goal_with_skeleton`.
2. `crates/worldwake-ai/src/agent_tick/planning.rs::write_information_barrier_partial_plan_segment_allows_missing_skeleton_source`.
3. `crates/worldwake-ai/src/agent_tick/planning.rs::write_information_barrier_partial_plan_segment_does_not_suspend_ask_witness_companion`.
4. `crates/worldwake-ai/src/partial_plan.rs::information_barrier_partial_plan_segment_preserves_filtered_skeleton`.

### Commands Run

1. Passed `cargo test -p worldwake-ai --lib information_barrier`.
2. Passed `cargo test -p worldwake-ai --lib partial_plan`.
3. Passed `cargo test -p worldwake-ai`.
4. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

## Outcome

Completed on 2026-05-24.

- Added information-barrier segment construction from selected barrier plans while the
  selected `CandidatePlanSearch.skeleton_source` is still live.
- Wired the selected-plan path to suspend primary pursuits with a `PartialPlanSegment`
  instead of adopting an information-barrier plan as executable work.
- Preserved the existing companion path as a consumer: suspended primary entries remain
  visible to `spawn_information_barrier_companions`, while `AskWitness` and `ShareBelief`
  companion/social plans are not suspended by this producer.
- Preserved filtered skeleton semantics by reusing the existing combat/fixed-identity
  filter and allowing `remaining_skeleton: None` when no planner-owned source exists.

## Deviations

- The landed information-barrier constructor creates a `MissingBelief` barrier fact only
  for `TellTopic::EntityBelief`. Other tell-topic families do not produce a segment here
  because this ticket's resume condition needs a concrete belief subject and must not
  synthesize one after the planner referent is gone.
- `AskWitness` and `ShareBelief` plans are excluded from this producer so the companion
  action path remains executable; the producer owns primary pursuits blocked by missing
  information, not the social plan that is supposed to acquire that information.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib information_barrier`.
- Passed `cargo test -p worldwake-ai --lib partial_plan`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
