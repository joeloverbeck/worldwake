# S122FRAASSCOM-005: Falsification probes (no-assumption-loss, no-spurious-failure, no-deferred-frozen-frame)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Small — adds opt-in validators to the survival-golden harness and fixes the newly exposed one-tick assumption-hydration gap for adopted acquisition frames.
**Deps**: archive/tickets/S122FRAASSCOM-004.md

## Problem

The architectural fix is in place after S122FRAASSCOM-001..004, but three failure modes remain plausible and silent: (a) a future refactor drops the `CommodityAvailableAt` push from `populate_assumptions` for one acquisition frame shape, leaving a live frame without the assumption (#16); (b) the evaluator records a commodity-assumption failure without the live local refutation that currently justifies `CriticalFailure` (#17); (c) an active frame holding `CommodityAvailableAt` never co-locates with its `place` and remains frozen beyond the intended patience window without being cleared (#18). This ticket lands three opt-in validators in the survival-golden harness so future regressions in any of these modes are detectable on demand.

## Assumption Reassessment (2026-04-20)

1. Existing focused/unit coverage in the survival-golden harness covers `max_authored_critical_run_ticks` bounds via `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`. The harness module under `crates/worldwake-ai/tests/golden_harness/` (e.g., `survival_forensics_assertions.rs`) already hosts existing per-tick forensic assertions over the live `GoldenHarness`. No existing validator sweeps every active acquisition frame to verify assumption presence, every commodity-assumption failure transition to verify local refuting evidence, or long-lived non-co-located frames to verify they are not frozen past patience. These are net-new validators that compose over the existing live harness, decision traces, and authoritative per-tick world state.
2. Spec deliverables Falsification §16 (no-assumption-loss invariant), §17 (no-spurious-failure invariant), §18 (no-deferred-frozen-frame invariant) are defined in `specs/S122-frame-assumption-commodity-availability.md` (lines 263–272).
3. Shared abstraction boundary under audit: the survival-golden harness module(s) under `crates/worldwake-ai/tests/golden_harness/`. The validators are opt-in — gated by an env var (e.g., `WORLDWAKE_FALSIFICATION_PROBES=1`) so default golden runs do not pay the validator cost. The boundary is the validator struct/function signatures and the gating mechanism.
4. The originally drafted posthoc `world_snapshots` / discrepancy-history seam is stale. The current harness does not persist full per-tick world snapshots, but the survival tests already run tick-by-tick against a live `GoldenHarness` with decision tracing enabled. The honest proof seam is therefore per-tick probes over `GoldenHarness` plus `trace_at(agent, tick)`, not posthoc reconstruction of missing snapshot history.
5. Live semantic correction: in the current S122 implementation, `CommodityAvailableAt` becomes `CriticalFailure` only on co-located local refutation. Non-co-located cases stay in `Believed` or `UnknownOrStale`; there is no remote refuting-belief `CriticalFailure` path to validate here.
6. Intended layer: golden E2E with full action registries. Validators inspect live per-tick `IntentionFrame` state, recorded decision traces (`AgentDecisionTrace`), and authoritative world state available through the harness at that tick.
7. Implementation-time finding: once the probes were run against the live baseline scenario, they exposed a real production gap rather than a probe bug. Newly adopted acquisition frames were created with `assumptions: Vec::new()` and only populated on the following tick. This ticket therefore includes the narrow production fix that hydrates assumptions immediately on plan adoption so the probes can validate the real invariant instead of a known one-tick loophole.

## Architecture Check

1. Validators that sweep production behavior at every tick of a survival run are stronger than spot assertions because they catch silent regressions that existing scenarios could otherwise survive for the wrong reason. Opt-in gating preserves default golden run time while making the validators available for regression hunts and on-demand audits.
2. No backwards-compatibility shim — net-new validators added alongside existing harness code, plus one direct production-path fix to hydrate frame assumptions at adoption time. No duplicate authority path introduced.

## Verification Layers

1. No-assumption-loss invariant (#16): for every active `IntentionFrame` in the survival run where `frame.expected_commodity()` returns `(commodity, place)`, `frame.assumptions` MUST contain `FrameAssumption::CommodityAvailableAt { commodity, place }` -> validator scans the live frame component each tick.
2. No-spurious-failure invariant (#17): for every decision trace that clears a frame with `failed_assumption = Some(FrameAssumption::CommodityAvailableAt { commodity, place })`, the agent MUST be co-located with `place` on that tick and authoritative local state at `place` MUST contain no matching lot or live `ResourceSource` support for `commodity` -> validator inspects `trace_at(agent, tick)` and the live harness world immediately after that tick.
3. No-deferred-frozen-frame invariant (#18): an active frame that holds `CommodityAvailableAt`, stays non-co-located with its `place`, and makes no progress (`last_progress_tick` unchanged) MUST not remain frozen for more than its `patience_limit` -> validator tracks same-frame stagnation across ticks instead of treating mere frame age as failure.
4. Per `docs/golden-e2e-testing.md`, these probes are scenario-validity proof surfaces, not structural activation checks. They must prove the authored causal branch when enabled.
5. `SCEROAD` alignment: these probes are opt-in forensic proof surfaces for the survival scenarios, not new scenario-coverage metadata.
6. Multi-layer ticket — validators read decision traces (decision-trace layer), frame state (planning-layer state), and authoritative world state (simulation layer). Each validator targets one invariant; no collapsing.

## What to Change

### 1. Add no-assumption-loss validator (#16)

- File: `crates/worldwake-ai/tests/golden_harness/` (existing module; the analog pattern is in `survival_forensics_assertions.rs`)
- Add a live-harness check that runs per tick:
  - Read `world.get_component_intention_frame(agent)`.
  - If `frame.expected_commodity()` returns `(commodity, place)`, assert `frame.assumptions` contains `FrameAssumption::CommodityAvailableAt { commodity, place }`.
  - Return `Err(...)` naming the tick, agent, goal, commodity, and place if missing.

### 2. Add no-spurious-failure validator (#17)

- File: same harness module
- Add a live-harness check that runs per tick:
  - Look up `trace_at(agent, tick)` and inspect `frame_transition`.
  - For each `FrameTransitionKind::Cleared { reason: AssumptionFailed, failed_assumption: Some(FrameAssumption::CommodityAvailableAt { commodity, place }) }`, assert the agent's live `effective_place(agent)` is `Some(place)`.
  - Assert authoritative local state at `place` has no matching `ItemLot` with positive quantity and no matching `ResourceSource` with positive `available_quantity`, mirroring the live co-located branch of `assess_commodity_availability`.
  - Return `Err(...)` naming the tick, agent, assumption, and the unexpected local support if present.

### 3. Add no-deferred-frozen-frame validator (#18)

- File: same harness module
- Add a stateful live-harness tracker that runs per tick:
  - For each active frame where `frame.expected_commodity()` returns `(commodity, place)` and the agent is not co-located with `place`, track the frame identity plus `last_progress_tick`.
  - Reset the stagnation counter when the frame changes identity, the agent co-locates with `place`, or `last_progress_tick` advances.
  - Return `Err(...)` if the same active non-co-located frame remains without progress for more than `patience_limit` ticks, naming the tick, agent, commodity, place, and stagnant span.

### 4. Fix newly adopted acquisition frames to hydrate assumptions immediately

- File: `crates/worldwake-ai/src/agent_tick/planning.rs`
- When `adopt_selected_plan` / the traced adoption path create or refresh an `IntentionFrame` for a newly selected plan, populate `frame.assumptions` immediately via the existing `populate_assumptions` helper before the frame is persisted.
- Add a focused planning-level regression test proving an adopted `AcquireCommodity` travel frame already carries `CommodityAvailableAt` on the adoption tick.

### 5. Wire validators as opt-in survival-golden hooks

- Files: `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`
- Add an env-var-gated probe object that runs inside the existing tick loop. Use `std::env::var("WORLDWAKE_FALSIFICATION_PROBES").is_ok()` to gate. Default off — existing golden run time is preserved.
- When the gate is set, run all three validators and fail the test with the validator's `Err(...)` message if any reports an issue.

### 6. Validator self-tests

- File: `crates/worldwake-ai/tests/golden_harness/<module>.rs`
- Add focused self-tests around the validator helpers / tracker:
  - missing-assumption acquire frame -> `Err(...)`
  - valid acquire frame -> `Ok(())`
  - commodity-assumption clear without co-location or with unexpected local support -> `Err(...)`
  - valid local refutation clear -> `Ok(())`
  - frozen-frame tracker exceeds patience without progress -> `Err(...)`
  - progress reset / frame replacement / co-location reset -> `Ok(())`

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — immediate assumption hydration on adoption plus focused regression test)
- `crates/worldwake-ai/tests/golden_harness/<module>.rs` (modify or new — three validator checks plus self-tests)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify — gated per-tick probe invocation)
- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify — gated per-tick probe invocation)
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` (modify — gated per-tick probe invocation)

## Out of Scope

- Always-on validation in the existing golden run (would inflate run time).
- Validators for assumptions other than `CommodityAvailableAt` (parity with `TargetAlive`, etc., is a separate enhancement worth a follow-up ticket if the pattern proves useful).
- Production changes beyond the narrow assumption-hydration fix needed to close the newly exposed adoption-time gap.
- Replacing existing forensic assertions in the harness — the new validators are additive.

## Acceptance Criteria

### Tests That Must Pass

1. New: the no-assumption-loss live probe returns `Ok(())` against the `survival-baseline` run when the gate is set.
2. New: the no-spurious-failure live probe returns `Ok(())` against the same run.
3. New: the frozen-frame live probe returns `Ok(())` against the same run.
4. New self-tests: each validator returns `Err(...)` for a hand-crafted violating fixture and `Ok(())` for a satisfying fixture.
5. New focused regression: adopted acquisition frames populate `CommodityAvailableAt` immediately on the adoption tick.
6. Opt-in survival-golden invocations pass when `WORLDWAKE_FALSIFICATION_PROBES=1` is set against `baseline`, `contested`, and `scattered`.
7. Default `cargo test -p worldwake-ai` run does NOT invoke the new validators (preserving existing golden run time).
8. Existing suite: `cargo test -p worldwake-ai` passes (default run, no env var set).

### Invariants

1. Every live acquisition frame that implies an expected commodity/place pair carries the matching `CommodityAvailableAt` assumption. (FND-21.)
2. Every recorded `CommodityAvailableAt` assumption failure corresponds to co-located local refuting evidence at the assumption place. (FND-17.)
3. No active `CommodityAvailableAt` frame remains frozen past patience while non-co-located with its place. (FND-21, FND-11.)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — focused regression proving adopted acquisition frames hydrate assumptions immediately.
2. `crates/worldwake-ai/tests/golden_harness/<module>.rs` — opt-in live probe helpers plus a small set of unit self-tests verifying each validator detects its respective failure mode.
3. `crates/worldwake-ai/tests/golden_survival_baseline.rs` and siblings — opt-in per-tick probe invocation gated on `WORLDWAKE_FALSIFICATION_PROBES`.

### Commands

1. `cargo test -p worldwake-ai commodity_assumption_falsification --test golden_survival_baseline`
2. `cargo test -p worldwake-ai adopt_selected_plan_populates_expected_commodity_assumption_immediately --lib`
3. With opt-in flag set: `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
4. With opt-in flag set: `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1`
5. With opt-in flag set: `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
6. Default (no flag): `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Added the opt-in `CommodityAssumptionFalsificationProbes` harness in `crates/worldwake-ai/tests/golden_harness/commodity_assumption_falsification.rs` and exported it through `golden_harness/mod.rs`.
- Wired the probes into the `survival-baseline`, `survival-contested`, and `survival-scattered` golden runs behind `WORLDWAKE_FALSIFICATION_PROBES=1`, so they sweep live per-tick frame state and decision traces only when explicitly enabled.
- Added focused self-tests for missing-assumption detection, invalid commodity-failure clears, valid co-located refutation, frozen-frame overflow, and tracker reset behavior.
- The new probes exposed a real one-tick production gap: newly adopted acquisition frames were created with empty assumptions until the following tick. Fixed that narrowly in `crates/worldwake-ai/src/agent_tick/planning.rs` by hydrating assumptions immediately on adoption, and added the focused regression `adopt_selected_plan_populates_expected_commodity_assumption_immediately`.
- Outcome amended during closeout: the ticket began as harness-only but honestly landed one narrow production-path fix because the new proof surface exposed a live contradiction. A separate unrelated broader-suite failure now has follow-up ownership in `tickets/AIDECREG-002.md`.

## Verification Result

- Passed `cargo test -p worldwake-ai commodity_assumption_falsification --test golden_survival_baseline`
- Passed `cargo test -p worldwake-ai adopt_selected_plan_populates_expected_commodity_assumption_immediately --lib`
- Passed `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
- Passed `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1`
- Passed `WORLDWAKE_FALSIFICATION_PROBES=1 cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
- Failed broader `cargo test -p worldwake-ai` on an unrelated existing blocker outside this ticket’s owned surface: `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_discrepancy_memory_with_ttl_expiry`
- Confirmed the unrelated blocker in isolation with `cargo test -p worldwake-ai --test golden_ai_decisions golden_discrepancy_memory_with_ttl_expiry`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
