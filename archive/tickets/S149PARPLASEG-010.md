# S149PARPLASEG-010: Executable partial-plan segment writer and tactical re-entry

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner/barrier segment construction; tactical suffix re-entry resolver
**Deps**: archive/tickets/S149PARPLASEG-005.md

## Problem

Before this ticket, S149PARPLASEG-005 had landed the safe agenda lifecycle slice for stored `PartialPlanSegment`s, but live reassessment showed `PlannedSkeletonStep` was not executable by itself. It carries `PlannerOpKind`, `PayloadTemplate`, and expected predicates, but not the resolved `ActionDefId`, authoritative targets, payload, or planner context needed to reconstruct lawful `PlannedStep`s. This ticket added the concrete segment writer, budget-exhaustion production writer, and resume-to-normal-search re-entry path without fabricating missing skeleton targets.

## Assumption Reassessment (2026-05-20)

1. `PartialPlanSegment` exists in `crates/worldwake-ai/src/partial_plan.rs` and `AgendaEntry.partial_plan_segment` exists in `crates/worldwake-ai/src/agenda_types.rs`, but the only current construction paths default the field to `None`.
2. `try_resume_partial_plan` in `crates/worldwake-ai/src/agenda_manager.rs` evaluates stored segment resume/abandon conditions and returns a `ResumedPlan`; it intentionally does not synthesize runnable steps from skeleton templates.
3. Shared boundary under audit: the planner/barrier result to `PartialPlanSegment` writer, and the tactical re-entry resolver that turns stored prefix/skeleton state into a lawful planner search starting point.
4. Planner contract: per `docs/planner-contracts.md`, planner-visible data must remain snapshot/belief-backed. This ticket must not query authoritative world state to fill missing targets or payloads on behalf of the agent.
5. FOUNDATIONS alignment: executable re-entry must preserve FND-3 concrete state, FND-14 belief-only planning, FND-20 resource-bounded practical reasoning, and FND-28 no duplicate live planner authority paths.
6. Implementation correction: the live lawful re-entry path is not direct replay of `remaining_skeleton`. `PlannedSkeletonStep` remains a stored skeleton/diagnostic carrier until later barrier producers can supply enough concrete data. Resumed segments are re-admitted to the existing belief-backed agenda/ranking/planning pipeline through their stored `GoalOffer`, which preserves the planner contract from `docs/planner-contracts.md` and avoids a second resolver.

## Architecture Check

1. The writer/resolver must make the stored segment concrete enough for lawful re-entry rather than fabricating missing values at resume time.
2. Re-entry should delegate to the existing tactical search/planner state machinery where possible; any new resolver must be a narrow bridge from stored segment fields to existing planner inputs, not a parallel planner.

## Verification Layers

1. Barrier or budget-exhaustion result writes a truthful `PartialPlanSegment` -> focused planner/agenda unit test.
2. Skeleton resolution uses belief-backed planner inputs only -> focused resolver test with no authoritative-only target lookup.
3. Resumed suffix reaches a lawful plan or fails with typed traceable reason -> runtime `agent_tick` / decision-trace test.
4. Budget exhaustion path stores `PlanTerminalKind::SearchBudgetExhausted` segment -> focused regression.

## Landed Changes

### 1. Segment writer

Added `PartialPlanSegmentSeed`, `build_partial_plan_segment`, and `budget_exhausted_partial_plan_segment` in `crates/worldwake-ai/src/partial_plan.rs`. The writer rejects non-barrier terminals, derives existing resume conditions from `BarrierFact`, installs `PatienceExhausted` as the abandon condition, preserves completed prefix and causal links, and assigns deterministic `PartialPlanSegmentId`s.

### 2. Budget-exhaustion production writer

Added `write_budget_exhausted_partial_plan_segments` in `crates/worldwake-ai/src/agent_tick/planning.rs`. Budget-exhausted search outcomes now suspend the ranked agenda entry with a typed `PlanTerminalKind::SearchBudgetExhausted` segment and a profile-backed `TickElapsed` resume condition.

### 3. Runtime integration

Integrated `try_resume_partial_plan` into the agent tick before agenda ranking. A resumed segment is pushed back into ranked candidates with a `REPLAN_SIGNAL`, so the existing belief-backed tactical search owns executable suffix selection. Fresh candidates no longer thaw suspended partial segments unless `try_resume_partial_plan` has made them eligible.

## Landed Files

- `crates/worldwake-ai/src/partial_plan.rs` (modify)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)

## Out of Scope

- Companion `AskWitness` synthesis for information barriers (ticket 006).
- Coordination watching-list triggers (ticket 007).
- Observer rendering (ticket 008).

## Acceptance Result

### Proved Behavior

1. Passed: eligible typed barrier seeds build `PartialPlanSegment`s with truthful barrier fact, resume conditions, abandon condition, completed prefix, causal links, and deterministic identity.
2. Passed: eligible budget-exhausted search outcomes store suspended agenda entries with `PlanTerminalKind::SearchBudgetExhausted` segments.
3. Passed: resumed segments re-enter the existing belief-backed agenda/ranking/planning path; no authoritative target, payload, or action definition is synthesized from `remaining_skeleton`.
4. Passed: `cargo test -p worldwake-ai`.

### Invariants

1. No executable target, payload, or action definition is synthesized without a belief-backed or stored segment source.
2. The re-entry path does not duplicate planner authority or bypass existing tactical search contracts.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/partial_plan.rs` — segment writer, non-barrier rejection, and budget-exhaustion writer tests.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — budget-exhausted search outcome suspends the ranked goal with a typed segment.
3. `crates/worldwake-ai/src/agenda_manager.rs` — fresh candidates cannot bypass unsatisfied partial-plan resume conditions.

### Commands Run

1. Passed `cargo test -p worldwake-ai --lib partial_plan`
2. Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::tick_agenda_keeps_unsatisfied_partial_segment_suspended_despite_fresh_candidate -- --exact`
3. Passed `cargo test -p worldwake-ai`

## Deviations

- `remaining_skeleton` was not made directly executable. The landed resolver re-admits the stored `GoalOffer` to the existing tactical planner after resume conditions hold. This is the lawful S149 boundary because the skeleton carrier still lacks concrete resolved action definitions, targets, payloads, and planner context.
- The production writer is wired for budget-exhausted search outcomes in this ticket. The generic barrier-segment writer is available for typed barrier producers; information-barrier companion synthesis and coordination watching remain owned by tickets 006 and 007.

## Outcome

Completed on 2026-05-20.

- Added the concrete `PartialPlanSegment` writer API and exported it from `worldwake-ai`.
- Stored budget-exhausted planning results as suspended agenda entries with typed `SearchBudgetExhausted` partial-plan segments.
- Re-entered resumed segments through the existing belief-backed agenda/ranking/planning pipeline rather than a parallel skeleton replay path.
- Preserved suspended partial segments against ordinary fresh-candidate refresh until resume conditions pass.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib partial_plan`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::tick_agenda_keeps_unsatisfied_partial_segment_suspended_despite_fresh_candidate -- --exact`
- Passed `cargo test -p worldwake-ai`
