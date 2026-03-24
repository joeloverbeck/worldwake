# S22-003: Implement assumption population and evaluation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new AI pipeline stage (assumption evaluation before planning)
**Deps**: S22-002 (IntentionFrame must be the active component, BeliefView must have `route_exists()`)

## Problem

IntentionFrames carry concrete assumptions (`FrameAssumption`) that must be evaluated each tick against the agent's beliefs. When assumptions fail, the frame must transition to `Suspended` (recoverable failure) or `Exhausted` (critical failure). Without this evaluation, frames persist indefinitely regardless of world changes, violating P3 (concrete state over abstract scores) and P19 (intentions are revisable commitments).

## Assumption Reassessment (2026-03-24)

1. `FrameAssumption` enum has 4 variants: `TargetAlive`, `RouteExists`, `NoCriticalThreat`, `CommodityAvailableAt`. Defined in S22-001.
2. `BeliefView` already has `is_alive()` method (from E14). `route_exists()` is added in S22-002.
3. `NoCriticalThreat` is NOT a BeliefView query — it checks ranked candidates for `GoalPriorityClass::Critical`, evaluated during the planning pipeline after candidate generation and ranking.
4. The evaluation must happen in `agent_tick/mod.rs` after observation refresh, before planning.
5. Critical vs recoverable distinction: `TargetAlive` failure → critical (Exhausted). `RouteExists`, `NoCriticalThreat`, `CommodityAvailableAt` failures → recoverable (Suspended).
6. Frame exhaustion from critical assumption failure creates a `BlockedIntent` with `BlockingFact::AssumptionFailed` — but the BlockedIntent creation logic is in S22-005. This ticket only handles the state transition.
7. This is an AI pipeline integration ticket. The intended layer is `agent_tick` runtime. Full action registries are needed for golden test verification.

## Architecture Check

1. Placing assumption evaluation as a pipeline stage (after observation, before planning) follows the existing pattern of per-tick lifecycle stages in `agent_tick/mod.rs`.
2. No backward-compatibility concerns — this is new functionality on the new `IntentionFrame` type.
3. `populate_assumptions()` as a standalone function (not a method on IntentionFrame) keeps worldwake-core free of BeliefView dependencies.

## Verification Layers

1. `TargetAlive` assumption fails when `is_alive()` returns false → frame transitions to Exhausted → focused unit test
2. `RouteExists` assumption fails when `route_exists()` returns false → frame transitions to Suspended with `RouteBlocked` → focused unit test
3. `NoCriticalThreat` assumption fails when Critical candidate exists → frame transitions to Suspended with `SurvivalNeed` → focused unit test
4. Assumption evaluation does not fire when frame is already Exhausted → focused unit test
5. All golden tests pass → `cargo test -p worldwake-ai`

## What to Change

### 1. New functions in `agent_tick/frame.rs`

Add `populate_assumptions()`: given an `IntentionDomain` and the agent's current belief state, return `Vec<FrameAssumption>`:
- `Travel { destination }` → `[RouteExists { from: current_place, to: destination }]`
- `Care { patient }` → `[TargetAlive(patient), RouteExists { from: current_place, to: patient_place }]`
- `Escort { ward, destination }` → `[TargetAlive(ward), RouteExists { from: current_place, to: destination }]`
- `Errand { destination }` → `[RouteExists { from: current_place, to: destination }]`
- `Generic` → `[NoCriticalThreat]`

Add `evaluate_assumptions()`: given a `&[FrameAssumption]`, a `&dyn BeliefView`, and optional ranked candidates, return evaluation result indicating all-pass, recoverable failure (with `SuspensionReason`), or critical failure.

### 2. Integration in `agent_tick/mod.rs`

After the observation/refresh stage and before the planning stage, call `evaluate_assumptions()` on the agent's current `IntentionFrame` (if any, and if state is Active or Suspended). Handle transitions:
- Critical failure → set `frame.state = FrameState::Exhausted`, set `last_frame_clear_reason = AssumptionFailed`
- Recoverable failure → set `frame.state = FrameState::Suspended { reason, suspended_at }`
- All pass + was Suspended → set `frame.state = FrameState::Active` (resume)

### 3. NoCriticalThreat deferred evaluation

Since `NoCriticalThreat` requires ranked candidates (not available until after candidate generation), this assumption is evaluated after ranking, not in the pre-planning stage. Add a second evaluation point in the planning pipeline for this specific assumption kind.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — add `populate_assumptions()`, `evaluate_assumptions()`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — integrate assumption evaluation into per-tick pipeline)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — deferred `NoCriticalThreat` evaluation after ranking)

## Out of Scope

- BlockedIntent creation on frame exhaustion (S22-005 — this ticket only transitions state)
- Progress detection (S22-004)
- Decision trace recording of assumption evaluation (S22-006)
- Non-Travel domain frame creation logic — this ticket implements the evaluation for all domains but frame creation for non-Travel domains is future work
- Changes to `BeliefView` trait (done in S22-002)
- `CommodityAvailableAt` BeliefView implementation — if no existing method supports this, stub it as always-true and document as future work

## Acceptance Criteria

### Tests That Must Pass

1. Focused test: `TargetAlive(dead_entity)` → critical failure → `FrameState::Exhausted`
2. Focused test: `RouteExists` with severed route → recoverable failure → `FrameState::Suspended { reason: RouteBlocked }`
3. Focused test: `NoCriticalThreat` with active Critical candidate → recoverable failure → `FrameState::Suspended { reason: SurvivalNeed }`
4. Focused test: all assumptions pass → frame remains Active
5. Focused test: Suspended frame with all assumptions passing → resumes to Active
6. Focused test: already-Exhausted frame is not re-evaluated
7. `cargo test -p worldwake-ai` — all golden tests pass
8. `cargo clippy --workspace` — no new warnings

### Invariants

1. Assumptions are evaluated through BeliefView (never authoritative world state) — P10 enforcement
2. `NoCriticalThreat` is evaluated after candidate ranking, not as a BeliefView query
3. Critical assumption failure always produces `FrameState::Exhausted`, never `Suspended`
4. Recoverable assumption failure produces `FrameState::Suspended` with the correct `SuspensionReason`
5. Resume does not reset `stalled_ticks` (accumulated patience drain is permanent per spec)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — `populate_assumptions` correctness per domain
2. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — `evaluate_assumptions` focused tests for each assumption kind and failure mode
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — integration test: assumption evaluation fires in pipeline, frame transitions occur

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
