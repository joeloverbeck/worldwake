# S168PARPLASKE-006: Information-barrier segment production

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/agent_tick/planning.rs` and likely `crates/worldwake-ai/src/partial_plan.rs`.
**Deps**: `archive/tickets/S168PARPLASKE-002.md` (budget-exhausted skeleton population and construction filter); `archive/tickets/S168PARPLASKE-005.md` (planner-owned skeleton source carrier); `specs/S168-partial-plan-skeleton-reuse.md` (D1.b corrected producer boundary).

## Problem

S168 D1.b still requires populated `PartialPlanSegment.remaining_skeleton` for information-barrier suspensions, but live reassessment during S168PARPLASKE-002 proved the drafted producer path was wrong. `spawn_information_barrier_companions` consumes suspended entries that already have a `PartialPlanSegment`; it cannot be the first producer because it skips entries whose segment is absent.

This ticket finds and implements the lawful producer boundary for selected or completed plans whose `PlanTerminalKind` is `InformationBarrier`. The producer must preserve the high-level skeleton from the planner-owned `PartialPlanSkeletonSource`, build a `PartialPlanSegment` with `PlanTerminalKind::InformationBarrier { .. }`, and suspend the matching agenda entry so the existing companion-spawn consumer can discover the topic and later resume through tickets 001/003.

## Assumption Reassessment (2026-05-24)

1. **Live consumer checked**. `agenda_manager.rs::spawn_information_barrier_companions` iterates suspended agenda entries and requires `entry.partial_plan_segment.as_ref()` before it can inspect `PlanTerminalKind::InformationBarrier { topic }`.
2. **Live budget producer checked**. `agent_tick/planning.rs::write_budget_exhausted_partial_plan_segments` is now the budget-exhausted producer and consumes `CandidatePlanSearch.skeleton_source`. Information barriers need an analogous lawful producer at the barrier-plan selection/completion boundary, not inside the companion consumer.
3. **Shared boundary under audit**. The producer must bridge `PlannedPlan.terminal_kind == PlanTerminalKind::InformationBarrier { .. }`, the matching `CandidatePlanSearch.skeleton_source`, and `AgendaEntry.partial_plan_segment`.
4. **FOUNDATIONS boundary**. The producer may not synthesize a skeleton after the planner referent is gone. It must use the already-carried `PartialPlanSkeletonSource`, and absence of a source must remain lawful by preserving `remaining_skeleton: None` or falling back to ordinary replan behavior.
5. **Sibling impact**. Ticket 003 can consume any populated skeleton through focused tests, but ticket 004's information-barrier reuse/fallback goldens require this producer before the end-to-end chain is truthful.

## Architecture Check

1. Producing information-barrier segments at the plan-selection or plan-completion boundary keeps the agenda companion path a consumer of suspended intentions rather than a circular producer/consumer.
2. Reusing the construction filter from S168PARPLASKE-002 keeps combat and fixed-identity steps out of information-barrier skeletons too.
3. The producer must preserve existing agenda arbitration and ranking authority; it should only suspend the already-selected barrier-blocked pursuit.

## Verification Layers

1. Information-barrier segment production -> focused unit/runtime test showing a selected or completed information-barrier plan parks a suspended agenda entry with `PartialPlanSegment`.
2. Skeleton threading -> assertion that the segment carries filtered `remaining_skeleton` from `CandidatePlanSearch.skeleton_source`.
3. Companion consumer compatibility -> focused agenda test showing `spawn_information_barrier_companions` sees the produced segment and spawns the same `AskWitness` companion as before.
4. Negative no-source case -> focused test showing absence of `PartialPlanSkeletonSource` does not synthesize a skeleton and remains lawful.

## What to Change

### 1. Locate the producer seam

Trace selected `PlanSearchResult::Found(PlannedPlan)` values whose terminal is `PlanTerminalKind::InformationBarrier { .. }` through plan adoption and plan completion. Choose the first boundary that still has both the selected plan and the matching `CandidatePlanSearch.skeleton_source`.

### 2. Add information-barrier segment construction

Add or reuse a constructor that builds a `PartialPlanSegmentSeed` with:

- the selected `GoalOffer`;
- any completed prefix available at the suspension boundary;
- filtered `remaining_skeleton` from `PartialPlanSkeletonSource`;
- `PlanTerminalKind::InformationBarrier { topic }`;
- `BarrierFact::MissingBelief(...)` matching the resume subject;
- stable tick/counter identity and causal links when available.

### 3. Wire agenda suspension

Move or copy the matching agenda entry into `AgendaPhase::Suspended` with the constructed segment. Preserve existing companion cleanup and revive behavior.

### 4. Tests

Add focused tests for production, skeleton threading, no-source fallback, and companion compatibility.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — likely producer boundary and tests)
- `crates/worldwake-ai/src/partial_plan.rs` (modify if a dedicated information-barrier constructor is cleaner than local seed construction)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify tests only if companion compatibility is best proved there)
- `tickets/S168PARPLASKE-003.md` and `tickets/S168PARPLASKE-004.md` (truth-sync dependency wording after archival)

## Out of Scope

- Budget-exhausted skeleton population — ticket 002.
- Skeleton revalidation — archived ticket 001.
- Seeded search and resume trace — ticket 003.
- Validation goldens — ticket 004.
- Resource/jurisdiction/coordination barrier skeleton production — S168 non-goals.

## Acceptance Criteria

### Tests That Must Pass

1. Focused information-barrier producer test passes.
2. Focused skeleton-threading test passes.
3. Focused no-source fallback test passes.
4. Focused companion-compatibility test passes.
5. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Information-barrier skeletons come only from planner-owned `PartialPlanSkeletonSource`.
2. `spawn_information_barrier_companions` remains a consumer of suspended information-barrier segments.
3. No combat or fixed-target-identity skeleton step is preserved.
4. Absence of a skeleton source remains lawful and does not block ordinary fallback behavior.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — focused producer and no-source tests.
2. `crates/worldwake-ai/src/agenda_manager.rs` — companion compatibility test if not already covered through the producer test.

### Commands

1. `cargo test -p worldwake-ai --lib information_barrier`
2. `cargo test -p worldwake-ai --lib partial_plan`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
