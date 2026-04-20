# S110DECHISEVE-009: Authoritative alternate-target repair events

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — emit `RepairApplied` from the live alternate-target repair seam and transport the completed step index to that seam
**Deps**: archive/tickets/S110DECHISEVE-004.md

## Problem

S110 defines `RepairApplied`, but the live runtime currently proves only one authoritative repair class: a completed plan that succeeds on an alternate target and is later recorded in repair memory. This ticket emits that truthful subset as `RepairApplied` instead of waiting for a broader repair-provenance redesign.

## Assumption Reassessment (2026-04-20)

1. `crates/worldwake-ai/src/failure_handling.rs` classifies failures and records blockers/discrepancies, but it does not expose a successful repair-acceptance result. The first honest repair seam is `crates/worldwake-ai/src/agent_tick/mod.rs::record_repair_memory_from_completed_plan`.
2. The live durable repair substrate is `RepairKey { goal_key, alternate_target }` plus `RepairEntry`; this proves only an `AlternateTarget` repair today. There is no current runtime carrier that distinguishes `AlternateRoute`, `AlternateMerchant`, or `AlternateRecipe` without new provenance substrate.
3. `crates/worldwake-ai/src/agent_tick/observation.rs::CompletedPlanSummary` currently carries `goal_key`, `opportunity`, and terminal kind. This ticket must widen that transport with the completed `step_index` so the repair event can describe the actual repaired step.
4. Shared abstraction boundary under audit: completed-plan reconciliation in `agent_tick/observation.rs` returned into `agent_tick/mod.rs`, plus the repair-memory recording seam that already writes the durable alternate-target success state.
5. Mismatch + correction: the original ticket overstated the live repair taxonomy. This ticket now lands only the authoritative `RepairKind::AlternateTarget` slice and defers richer repair classes to `tickets/S110DECHISEVE-010.md`.

## Architecture Check

1. Emitting from the same seam that records durable alternate-target repair memory is cleaner than inferring repairs from a later read-model or guessing richer repair classes from nearby context.
2. Narrowing to the live `AlternateTarget` subset is cleaner than widening the payload claim beyond what the current runtime actually knows. The broader taxonomy gets its own follow-up ticket instead of being inferred here.

## Verification Layers

1. Completed-plan transport -> focused runtime/unit test proving the repaired step index survives to the repair seam.
2. Event emission -> focused `agent_tick` runtime test.

## What to Change

### 1. Transport the completed repaired step

Widen `CompletedPlanSummary` so the completed-plan reconciliation path preserves the finished `step_index` alongside the goal and opportunity.

### 2. Emit `RepairApplied`

Emit `RepairAppliedPayload` from `record_repair_memory_from_completed_plan` when that seam records a successful alternate-target repair. Populate `repair_kind = RepairKind::AlternateTarget` and `substitute_target = Some(alternate_target)`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)

## Out of Scope

- Richer repair taxonomy beyond `AlternateTarget` (`AlternateRoute`, `AlternateMerchant`, `AlternateRecipe`)
- Invalidation, suppression, or observer work

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proves one successful alternate-target repair emits exactly one `RepairApplied` event with `repair_kind = RepairKind::AlternateTarget`, the correct `goal_key`, the completed `step_index`, and `substitute_target = Some(alternate_target)`.
2. `cargo test -p worldwake-ai`

### Invariants

1. `RepairApplied` is emitted only when the runtime actually records a successful alternate-target repair.
2. The payload reflects the live authoritative alternate target and completed step, not a guessed broader repair taxonomy.

## Test Plan

### New/Modified Tests

1. Focused `agent_tick` repair test.

### Commands

1. `cargo test -p worldwake-ai completed_alternate_plan_records_repair_memory_entry`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Narrowed the ticket to the currently authoritative `RepairKind::AlternateTarget` slice and created `tickets/S110DECHISEVE-010.md` for the deferred richer repair taxonomy.
- Widened `CompletedPlanSummary` with the completed `step_index`, then emitted `RepairApplied` from the same `agent_tick/mod.rs` seam that records durable alternate-target repair memory.
- Added focused proof that a completed alternate-target repair records repair memory and emits exactly one `RepairApplied` event with the correct goal, step index, repair kind, and substitute target.

## Verification Result

Passed:

1. `cargo test -p worldwake-ai agent_tick::tests::completed_alternate_plan_records_repair_memory_entry -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
