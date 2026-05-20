# S149PARPLASEG-010: Executable partial-plan segment writer and tactical re-entry

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner/barrier segment construction; tactical suffix re-entry resolver
**Deps**: archive/tickets/S149PARPLASEG-005.md

## Problem

S149PARPLASEG-005 landed the safe agenda lifecycle slice for stored `PartialPlanSegment`s, but live reassessment showed `PlannedSkeletonStep` is not executable by itself. It carries `PlannerOpKind`, `PayloadTemplate`, and expected predicates, but not the resolved `ActionDefId`, authoritative targets, payload, or planner context needed to reconstruct lawful `PlannedStep`s. S149 still needs a concrete writer and resolver before a resumed segment can re-enter tactical planning from `completed_prefix` / `remaining_skeleton`.

## Assumption Reassessment (2026-05-20)

1. `PartialPlanSegment` exists in `crates/worldwake-ai/src/partial_plan.rs` and `AgendaEntry.partial_plan_segment` exists in `crates/worldwake-ai/src/agenda_types.rs`, but the only current construction paths default the field to `None`.
2. `try_resume_partial_plan` in `crates/worldwake-ai/src/agenda_manager.rs` evaluates stored segment resume/abandon conditions and returns a `ResumedPlan`; it intentionally does not synthesize runnable steps from skeleton templates.
3. Shared boundary under audit: the planner/barrier result to `PartialPlanSegment` writer, and the tactical re-entry resolver that turns stored prefix/skeleton state into a lawful planner search starting point.
4. Planner contract: per `docs/planner-contracts.md`, planner-visible data must remain snapshot/belief-backed. This ticket must not query authoritative world state to fill missing targets or payloads on behalf of the agent.
5. FOUNDATIONS alignment: executable re-entry must preserve FND-3 concrete state, FND-14 belief-only planning, FND-20 resource-bounded practical reasoning, and FND-28 no duplicate live planner authority paths.

## Architecture Check

1. The writer/resolver must make the stored segment concrete enough for lawful re-entry rather than fabricating missing values at resume time.
2. Re-entry should delegate to the existing tactical search/planner state machinery where possible; any new resolver must be a narrow bridge from stored segment fields to existing planner inputs, not a parallel planner.

## Verification Layers

1. Barrier or budget-exhaustion result writes a truthful `PartialPlanSegment` -> focused planner/agenda unit test.
2. Skeleton resolution uses belief-backed planner inputs only -> focused resolver test with no authoritative-only target lookup.
3. Resumed suffix reaches a lawful plan or fails with typed traceable reason -> runtime `agent_tick` / decision-trace test.
4. Budget exhaustion path stores `PlanTerminalKind::SearchBudgetExhausted` segment -> focused regression.

## What to Change

### 1. Segment writer

Add the first production writer that builds `PartialPlanSegment` from typed barrier outcomes and eligible budget-exhausted suspensions. Include completed prefix, barrier fact, resume/abandon conditions from ticket 004 helpers, bounded causal links, and deterministic `PartialPlanSegmentId`.

### 2. Executable re-entry resolver

Define how `remaining_skeleton` resolves to executable planner inputs through existing belief-backed snapshot/search machinery. If the current `PlannedSkeletonStep` fields are insufficient, widen the stored segment contract explicitly before wiring consumers.

### 3. Runtime integration

Integrate `ResumedPlan` from S149PARPLASEG-005 into the planning path so an eligible suspended segment can retry the suffix and either continue lawfully or abandon/record a typed failure.

## Files to Touch

- `crates/worldwake-ai/src/partial_plan.rs` (modify)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/search/` (modify if re-entry requires a narrow planner entry point)

## Out of Scope

- Companion `AskWitness` synthesis for information barriers (ticket 006).
- Coordination watching-list triggers (ticket 007).
- Observer rendering (ticket 008).

## Acceptance Criteria

### Tests That Must Pass

1. New: eligible typed barrier outcome stores a `PartialPlanSegment` with truthful barrier fact and resume/abandon conditions.
2. New: eligible budget-exhausted suspension stores a `PartialPlanSegment` with `PlanTerminalKind::SearchBudgetExhausted`.
3. New: resumed segment re-enters tactical planning without authoritative world reads and reaches a lawful plan when the resume condition holds.
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. No executable target, payload, or action definition is synthesized without a belief-backed or stored segment source.
2. The resolver does not duplicate planner authority or bypass existing tactical search contracts.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/partial_plan.rs` or `agenda_manager.rs` — segment writer and budget-exhaustion writer tests.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` or `agent_tick/tests.rs` — runtime resumed suffix proof.

### Commands

1. `cargo test -p worldwake-ai partial_plan`
2. `cargo test -p worldwake-ai agenda_manager`
3. `cargo test -p worldwake-ai`
