# S24TYPINVDOM-002: Replace dirty:bool with DirtySet on AgentDecisionRuntime

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `AgentDecisionRuntime.dirty` field type changes from `bool` to `DirtySet`
**Deps**: S24TYPINVDOM-001 (DirtySet type must exist)

## Problem

`AgentDecisionRuntime.dirty` is a `bool` that loses information about *which* domain triggered invalidation. Six sites in `frame.rs`, `active_action.rs`, `mod.rs`, and `failure_handling.rs` set `dirty = true` without any domain annotation. This ticket replaces the boolean with `DirtySet` and tags each mutation site with its typed domain bit, preserving identical behavioral semantics.

## Assumption Reassessment (2026-03-24)

1. `AgentDecisionRuntime.dirty: bool` at `decision_runtime.rs:60`. Confirmed.
2. All 6 `dirty = true` mutation sites confirmed at their spec-listed locations:
   - `frame.rs:197` — queue patience exhaustion → `FRAME_BLOCKAGE` (travel step blockage context per spec; the frame patience line is 405)
   - `frame.rs:405` — assumption failure blocked intent → `FRAME_PATIENCE`
   - `mod.rs:378` — critical assumption failure → `ASSUMPTION_FAILED`
   - `active_action.rs:205` — ProgressBarrier terminal → `PLAN_FINISHED`
   - `active_action.rs:223` — GoalSatisfied/CombatCommitment terminal → `PLAN_FINISHED`
   - `failure_handling.rs:81` — plan failure → `REPLAN_SIGNAL`
3. All 5 `dirty = false` clear sites confirmed:
   - `mod.rs:269` — dead agent early return
   - `planning.rs:273` — snapshot continuation (non-traced)
   - `planning.rs:333` — after plan selection (non-traced)
   - `planning.rs:444` — snapshot continuation (traced)
   - `planning.rs:575` — after plan selection (traced)
4. All 2 `if runtime.dirty` read sites confirmed:
   - `planning.rs:257` — guard in non-traced path
   - `planning.rs:428` — guard in traced path
5. `observation.rs:120` sets `runtime.dirty = runtime.dirty || !dirty_reasons.is_empty()` — this line is NOT changed in this ticket. It will be an `insert()` but the `Vec<DirtyReason>` still exists at this point; the boolean-to-DirtySet conversion on line 120 needs temporary bridging: the observation phase still builds a `Vec<DirtyReason>` which it converts to `DirtySet` bits on the runtime. This ticket handles that by making `observation.rs:120` use `runtime.dirty.insert(...)` for each individual `DirtyReason` in the vec, or equivalently by mapping each `DirtyReason` variant to a `DirtySet` constant. The full observation decomposition happens in S24TYPINVDOM-003.
6. The test file `agent_tick/tests.rs` imports `DirtyReason` at line 19. Tests at lines 1436, 2545, 2644 pass `&[DirtyReason::NoPlan]` as `dirty_reasons` parameter to planning functions. These tests do NOT directly read/write `runtime.dirty` — they pass `dirty_reasons` to `plan_and_validate_next_step_traced`, which is unchanged in this ticket.
7. Test at line 3702 asserts `continuation_read.dirty_reasons == vec![DirtyReason::SnapshotChanged]` on `ReadPhaseResult`. This reads from the vec, not the runtime bool. Unchanged in this ticket.
11. No mismatch.

## Architecture Check

1. The `dirty` field name is preserved to minimize diff noise. The type change from `bool` to `DirtySet` is caught at compile time — every read/write site must be updated or the build fails.
2. No backwards-compatibility shims. The boolean is replaced in-place.

## Verification Layers

1. `runtime.dirty` mutation sites use correct domain bits → compile-time enforcement (type mismatch if missed) + existing golden tests pass unchanged
2. `runtime.dirty` read sites use `!is_empty()` → compile-time + planning behavior preserved
3. `runtime.dirty` clear sites use `DirtySet::default()` → compile-time + golden tests
4. Observation dual-tracking sync line bridges correctly → existing tests verify observation phase produces correct `dirty_reasons` vec
5. Single-layer ticket in the sense that this is a mechanical type replacement; behavioral equivalence is verified by all existing tests passing unchanged.

## What to Change

### 1. Replace field type in `decision_runtime.rs`

Change `pub dirty: bool` to `pub dirty: DirtySet` at line 60.

### 2. Update mutation sites (6 sites)

| File | Line | Before | After |
|------|------|--------|-------|
| `frame.rs` | 197 | `runtime.dirty = true` | `runtime.dirty.insert(DirtySet::FRAME_BLOCKAGE)` |
| `frame.rs` | 405 | `runtime.dirty = true` | `runtime.dirty.insert(DirtySet::FRAME_PATIENCE)` |
| `mod.rs` | 378 | `runtime.dirty = true` | `runtime.dirty.insert(DirtySet::ASSUMPTION_FAILED)` |
| `active_action.rs` | 205 | `runtime.dirty = true` | `runtime.dirty.insert(DirtySet::PLAN_FINISHED)` |
| `active_action.rs` | 223 | `runtime.dirty = true` | `runtime.dirty.insert(DirtySet::PLAN_FINISHED)` |
| `failure_handling.rs` | 81 | `runtime.dirty = true` | `runtime.dirty.insert(DirtySet::REPLAN_SIGNAL)` |

### 3. Update clear sites (5 sites)

All `runtime.dirty = false` → `runtime.dirty = DirtySet::default()`:
- `mod.rs:269`, `planning.rs:273`, `planning.rs:333`, `planning.rs:444`, `planning.rs:575`

### 4. Update read sites (2 sites)

- `planning.rs:257`: `if runtime.dirty` → `if !runtime.dirty.is_empty()`
- `planning.rs:428`: `if runtime.dirty` → `if !runtime.dirty.is_empty()`

### 5. Bridge observation dual-tracking sync line

`observation.rs:120`: `runtime.dirty = runtime.dirty || !dirty_reasons.is_empty()` → convert each `DirtyReason` in the vec to its `DirtySet` bit and insert into `runtime.dirty`. A helper closure or match can map `DirtyReason::NoPlan → DirtySet::NO_PLAN`, etc. This is a temporary bridge removed in S24TYPINVDOM-003.

### 6. Update test setup sites

Any test code that initializes `dirty: false` → `dirty: DirtySet::default()` and `dirty: true` → `dirty: DirtySet::NO_PLAN` (or another non-empty set as appropriate for the test's intent).

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — field type)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — 2 mutation sites)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — 2 mutation sites)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — 1 mutation site, 1 clear site)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — 2 read sites, 4 clear sites)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — 1 bridge at line 120)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — 1 mutation site)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test setup initializers)

## Out of Scope

- Changing `observation_snapshot_changed()` return type from `bool` to `DirtySet` (S24TYPINVDOM-003)
- Removing `Vec<DirtyReason>` from `refresh_runtime_for_read_phase()` (S24TYPINVDOM-003)
- Removing `DirtyReason` enum (S24TYPINVDOM-004)
- Modifying `PlanningPipelineTrace.dirty_reasons` field (S24TYPINVDOM-004)
- Modifying `ReadPhaseResult.dirty_reasons` field (S24TYPINVDOM-003)
- Removing `is_snapshot_changed_only()` function (S24TYPINVDOM-003)
- Removing `dirty_reasons` parameter from planning functions (S24TYPINVDOM-003)
- Modifying trace `format_outcome()` or `summary()` output (S24TYPINVDOM-004)
- Touching any crate other than `worldwake-ai`

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden tests pass unchanged — behavioral equivalence
2. All existing `agent_tick/tests.rs` tests pass with updated initializers
3. Build succeeds with zero `dirty: bool` references remaining in AI crate runtime code
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `DirtySet::default()` (empty/clean) is the only value used at clear sites — semantically equivalent to `false`
2. Every `dirty = true` site now carries a specific domain bit — no information lost
3. The observation dual-tracking sync line faithfully maps every `DirtyReason` variant to a `DirtySet` bit — no reason dropped
4. Planning guards `if !runtime.dirty.is_empty()` are semantically equivalent to `if runtime.dirty` (bool true)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — update `dirty: false` → `DirtySet::default()` and `dirty: true` → non-empty `DirtySet` in test setup structs
2. No new test files — this is a mechanical type replacement verified by existing coverage

### Commands

1. `cargo test -p worldwake-ai` — full crate regression
2. `cargo clippy -p worldwake-ai` — no new warnings
3. `cargo build --workspace` — cross-crate compilation check
