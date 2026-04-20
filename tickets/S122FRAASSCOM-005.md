# S122FRAASSCOM-005: Falsification probes (no-assumption-loss, no-spurious-failure, no-deferred-frozen-frame)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — adds opt-in validators to the survival-golden harness only. No production code changes.
**Deps**: S122FRAASSCOM-004

## Problem

The architectural fix is in place after S122FRAASSCOM-001..004, but three failure modes remain plausible and silent: (a) a future refactor drops the `CommodityAvailableAt` push from `populate_assumptions` for one goal kind, leaving frames without the assumption (#16); (b) the evaluator over-eagerly records assumption failures that don't correspond to genuine refuting belief or perception (#17); (c) a frame holding `CommodityAvailableAt` never co-locates with its `place` and persists indefinitely without being torn down by `patience_limit` (#18). This ticket lands three opt-in validators in the survival-golden harness so future regressions in any of these modes are detectable on demand.

## Assumption Reassessment (2026-04-20)

1. Existing focused/unit coverage in the survival-golden harness covers `max_authored_critical_run_ticks` bounds via `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`. The harness module under `crates/worldwake-ai/tests/golden_harness/` (e.g., `survival_forensics_assertions.rs`) hosts existing per-tick forensic assertions. No existing validator sweeps every plan adoption to verify assumption presence, every `record_assumption_failure` call to verify refuting evidence, or frame timeouts when co-location never happens. These are net-new validators that compose over the existing decision-trace and discrepancy-memory snapshots.
2. Spec deliverables Falsification §16 (no-assumption-loss invariant), §17 (no-spurious-failure invariant), §18 (no-deferred-frozen-frame invariant) defined in `specs/S122-frame-assumption-commodity-availability.md` (lines 263–272).
3. Shared abstraction boundary under audit: the survival-golden harness module(s) under `crates/worldwake-ai/tests/golden_harness/`. The validators are opt-in — gated by an env var (e.g., `WORLDWAKE_FALSIFICATION_PROBES=1`) so default golden runs do not pay the validator cost. The boundary is the validator function signatures and the gating mechanism.
6. Intended layer: golden E2E with full action registries. Validators inspect the recorded decision traces (`AgentDecisionTrace` records produced during the run) and `DiscrepancyMemory` snapshots at each tick.

## Architecture Check

1. Validators that sweep production behavior at every tick of a survival run are stronger than spot assertions because they catch silent regressions (e.g., a future refactor that drops the assumption push from `populate_assumptions` for one goal kind would still pass spot-tested cases but would fail the sweep). Opt-in gating preserves existing golden run time while making the validators available for regression hunts and on-demand audits.
2. No backwards-compatibility shim — net-new validators added alongside existing harness code. No production code touched.

## Verification Layers

1. No-assumption-loss invariant (#16): for every plan adoption in the survival-baseline run, every `IntentionFrame` whose committed goal is `AcquireCommodity` MUST have a `CommodityAvailableAt` assumption present in `frame.assumptions` -> validator scans the recorded decision-trace frame snapshots per tick.
2. No-spurious-failure invariant (#17): for every `record_assumption_failure` call recorded during the run, the corresponding frame at the failure tick MUST have either (a) co-located perception confirming absence of the commodity at the assumption's `place`, or (b) a refuting belief in `agent_belief_store(agent)` for the assumption's `(commodity, place)` pair -> validator inspects the frame, the belief store, and the agent's place at the failure tick using the world snapshot for that tick.
3. No-deferred-frozen-frame invariant (#18): a frame that holds `CommodityAvailableAt` and never co-locates with its `place` MUST be torn down by the existing `IntentionFrame::patience_limit` / `stalled_ticks` mechanism within `patience_limit` ticks of frame establishment -> validator scans for `CommodityAvailableAt`-holding frames whose lifespan exceeds `patience_limit` without progress or assumption-failure clear.
6. Multi-layer ticket — validators read decision traces (decision-trace layer), frame snapshots (planning-layer state), and discrepancy memory (component state). Each validator targets one invariant; no collapsing.

## What to Change

### 1. Add no-assumption-loss validator (#16)

- File: `crates/worldwake-ai/tests/golden_harness/` (existing module, exact location to be confirmed during implementation; the analog pattern is in `survival_forensics_assertions.rs`)
- New function `assert_acquire_commodity_frames_have_assumption(traces: &[AgentDecisionTrace]) -> Result<(), String>`:
  - Walk each tick's frame snapshot.
  - For each frame where `frame.goal.kind` matches `GoalKind::AcquireCommodity { .. }`, assert `frame.assumptions.iter().any(|a| matches!(a, FrameAssumption::CommodityAvailableAt { .. }))`.
  - Return `Err(...)` naming the tick, agent, and goal if missing.

### 2. Add no-spurious-failure validator (#17)

- File: same harness module
- New function `assert_assumption_failures_have_refuting_evidence(traces: &[AgentDecisionTrace], world_snapshots: &[WorldSnapshot], discrepancy_memories: &[(Tick, EntityId, &DiscrepancyMemory)]) -> Result<(), String>`:
  - Walk recorded `record_assumption_failure` events (as captured via `DiscrepancyMemory::record` calls during the run).
  - For each, look up the agent's belief store and physical place at the failure tick using the world snapshot.
  - Assert that either (a) co-located with `place` AND no entity at the place satisfies the commodity per the same logic as `assess_commodity_availability` from S122FRAASSCOM-001, OR (b) the belief store has no supporting entry for the assumption.
  - Return `Err(...)` naming the tick, agent, assumption, and the missing/contradicting evidence.

### 3. Add no-deferred-frozen-frame validator (#18)

- File: same harness module
- New function `assert_no_frame_outlives_patience_without_co_location(traces: &[AgentDecisionTrace]) -> Result<(), String>`:
  - Walk frame establishment events (frames that gained `CommodityAvailableAt`).
  - Track each such frame's `last_progress_tick` and `stalled_ticks` across subsequent ticks.
  - Assert that no such frame survives > `patience_limit` ticks without either making progress (`last_progress_tick` updated) or being torn down (frame cleared, `last_frame_clear_reason` set).
  - Return `Err(...)` naming the tick, agent, and frame age.

### 4. Wire validators as opt-in survival-golden hooks

- Files: `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`
- Add an env-var-gated test invocation that runs all three validators against the captured run output. Use `std::env::var("WORLDWAKE_FALSIFICATION_PROBES").is_ok()` to gate. Default off — existing golden run time is preserved.
- When the gate is set, run all three validators and fail the test with the validator's `Err(...)` message if any reports an issue.

### 5. Validator self-tests

- File: `crates/worldwake-ai/tests/golden_harness/<module>.rs`
- Synthesize minimal trace fixtures that violate each invariant and assert the validator returns `Err(...)` (so a future refactor that breaks the validator itself is caught).
- Synthesize fixtures that satisfy each invariant and assert `Ok(())`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/<module>.rs` (modify or new — three validator functions plus self-tests)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify — gated test invocation)
- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify — gated test invocation)
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` (modify — gated test invocation)

## Out of Scope

- Always-on validation in the existing golden run (would inflate run time).
- Validators for assumptions other than `CommodityAvailableAt` (parity with `TargetAlive`, etc., is a separate enhancement worth a follow-up ticket if the pattern proves useful).
- Modifying production code — this ticket is test-harness-only.
- Replacing existing forensic assertions in the harness — the new validators are additive.

## Acceptance Criteria

### Tests That Must Pass

1. New: `assert_acquire_commodity_frames_have_assumption` validator returns `Ok(())` against the `survival-baseline` run when the gate is set.
2. New: `assert_assumption_failures_have_refuting_evidence` validator returns `Ok(())` against the same run.
3. New: `assert_no_frame_outlives_patience_without_co_location` validator returns `Ok(())` against the same run.
4. New self-tests: each validator returns `Err(...)` for a hand-crafted violating fixture and `Ok(())` for a satisfying fixture.
5. Opt-in survival-golden invocations pass when `WORLDWAKE_FALSIFICATION_PROBES=1` is set against `baseline`, `contested`, and `scattered`.
6. Default `cargo test -p worldwake-ai` run does NOT invoke the new validators (preserving existing golden run time).
7. Existing suite: `cargo test -p worldwake-ai` passes (default run, no env var set).

### Invariants

1. Every `AcquireCommodity` frame carries the `CommodityAvailableAt` assumption (sweep-validated). (FND-21.)
2. Every recorded assumption failure corresponds to refuting evidence in the agent's belief or co-located perception. (FND-17.)
3. No `CommodityAvailableAt`-holding frame survives indefinitely without co-locating with its place. (FND-21, FND-11.)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/<module>.rs` — three new validator functions plus a small set of unit self-tests verifying each validator detects its respective failure mode.
2. `crates/worldwake-ai/tests/golden_survival_baseline.rs` and siblings — opt-in test invocations gated on `WORLDWAKE_FALSIFICATION_PROBES`.

### Commands

1. `cargo test -p worldwake-ai --test '*' --features falsification_probes` (or whichever gating mechanism is used)
2. With opt-in flag set: `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
3. With opt-in flag set: `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1`
4. With opt-in flag set: `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
5. Default (no flag): `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
