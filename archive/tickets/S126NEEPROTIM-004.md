# S126NEEPROTIM-004: Golden coverage for need projection

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — golden test only. May add a focused scenario file `scenarios/survival-need-projection.ron` if `survival-baseline.ron`'s rates cannot reproduce the breach within a reasonable plan-completion horizon.
**Deps**: S126NEEPROTIM-002, S126NEEPROTIM-003

## Problem

Tickets 002 and 003 land the per-need projection assumption lifecycle (population, evaluation, recording). This ticket adds the end-to-end golden test that proves the chain holds at the agent-tick level: an agent with a multi-step plan whose completion tick exceeds the projected hunger-high crossing recognizes the breach, records the typed discrepancy, suppresses the original goal for `structural_block_ticks`, and selects a shorter-completion alternative on the next ranking round.

This is the canonical proof surface for spec S126's behavioral contract — the three target patterns named in Motivating Evidence (`harvest_before_sleep`, `wake_early`, `carry_reserve_before_leaving_water`) all reduce to the same chain: `populate → evaluate → record → suppress → re-rank → adopt alternative`. The golden exercises the chain once with hunger as the breaching need; symmetric coverage for thirst/fatigue can be added as separate test cases inside the same file if the harness supports parameterization, or deferred to follow-up tickets if a separate scenario per need is more readable.

## Assumption Reassessment (2026-04-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `scenarios/survival-baseline.ron` exists with 4 places (Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter), 1 agent ("Agent A") with initial needs `hunger=420, thirst=240, fatigue=120, bladder=150, dirtiness=120` (out of 1000 permille). The `MetabolismProfile.hunger_rate` and `DriveThresholds.hunger.high()` values were not directly inspected but typical values (rate ~30-50 per tick, high ~700) suggest a multi-step plan from hunger=420 may not breach the high threshold within a 6-10 tick window. To force the breach reliably, the golden test either (a) authors a focused `scenarios/survival-need-projection.ron` with tightened `MetabolismProfile.hunger_rate` (e.g., 80-100/tick) and a multi-step plan whose completion tick exceeds 4-5 ticks, or (b) reuses `survival-baseline.ron` and arranges the agent in a state where the existing rate produces a breach within the natural plan horizon. Option (a) is more deterministic and is the recommended default — the spec's D8 explicitly authorizes a focused scenario file.
2. Spec authority: `specs/S126-need-projection-time-budget.md` D8 and Motivating Evidence (the three target patterns).
3. Shared abstraction boundary: this ticket exercises the full agent-tick decision cycle (`populate_assumptions` → `evaluate_assumptions` → `record_assumption_failure` → ranking re-adoption) under a controlled scenario, so the proof surface spans the AI crate and the discrepancy storage in `worldwake-core`. The boundary under audit is the typed-discrepancy chain established by S109 + S122, now extended by S126's `NeedHorizonExceeded` variant.
4. The intended invariant motivating the golden: an agent that adopts a long plan with a breaching need-horizon assumption must (a) record the typed discrepancy with TTL suppression, and (b) select a shorter-completion alternative on the next ranking round when one is available. The narrative report cited in Motivating Evidence (`scenario-narrative-survival-baseline-20260425-223423.md`) shows the *current* reactive interruption behavior — the golden's job is to prove the new horizon-aware path supersedes that reactive-only path.
5. `GoalKind` under test: `Sleep` and `AcquireCommodity { commodity: Apple, purpose: SelfConsume }` (or equivalent commodity available at the chosen scenario's locations). The shorter-completion alternative is the harvest-then-sleep pattern; the breaching plan is a direct sleep without first replenishing hunger. Verify the live planner still routes both goals through the expected operator/affordance surface during reassessment at implementation time.
6. The harness boundary: this is a golden E2E test under `crates/worldwake-ai/tests/`. It requires full action registries (not the local needs-only harness) because the agent is committing real plans, executing real action durations, and going through real ranking. Existing pattern: `golden_survival_baseline.rs`, `golden_survival_drive_escalation.rs`, `golden_survival_contested.rs`.
7. Coverage gap classification: this is missing **golden/E2E coverage** for the need-horizon chain. Sibling commodity-availability coverage exists implicitly via `golden_survival_*` tests but no dedicated `golden_need_projection.rs` exists today (verified during reassessment).
8. Scenario isolation: the lawful competing affordances at the scenario's locations include any other need-satisfaction action (drink, eat-from-source, sleep). The golden's contract is "horizon-aware planning replaces reactive interruption when both behaviors lawfully apply." Document explicitly in the scenario rationale which competing affordances were intentionally left in (so the test exercises real ranking) vs. removed (so the test isolates the horizon-aware branch).
9. Live confirmation (2026-04-26, implementation-time): `DriveThresholds::default().hunger.high() = 750` and `CognitiveProfile::default().structural_block_ticks = 200` were verified against `crates/worldwake-core/src/drives.rs` and `crates/worldwake-core/src/cognitive_profile.rs`. `survival-baseline.ron`'s authored hunger_rate=2 with hunger=420 produces a breach in 115 ticks — far longer than any natural plan completion window — so option (a) (focused new scenario `survival-need-projection.ron`) is required. This ticket implements option (a) with: hunger_rate=30, hunger=600, default high=750 → breach at +5 ticks; structural_block_ticks=30 (smaller than the 200 default to keep TTL-expiry phase of the test fast); 1 agent at "Riverside Camp"; "Distant Orchard" the only Apple source; 5-tick travel edge each way; plan completion ~8 ticks (5 travel + 3 harvest), so the projection breach (+5) falls strictly inside the plan window. The agent's fatigue is seeded above the default `high=780` threshold so that, after the AcquireCommodity goal is suppressed, Sleep wins ranking as the alternative goal (the spec's `harvest_before_sleep` motivating-evidence pattern, mirrored by giving the agent a high pre-existing fatigue motive). The closest live sibling that proves a subset of this chain is `golden_goal_switching_during_multi_leg_travel` (`crates/worldwake-ai/tests/golden_ai_decisions.rs:1141`); that test does not assert `is_suppressed`, alternative goal adoption, or TTL expiry — those three milestones are the new proof surface this ticket owns.

## Architecture Check

1. The golden test asserts the spec's behavioral contract end-to-end without requiring new production helpers — all the substrate (population, evaluation, recording, suppression, ranking) lands in tickets 001-003. If the test reveals a gap (e.g., the new shorter alternative loses ranking despite suppression), that is a real spec gap, not a test gap, and warrants a follow-up.
2. New helpers under `crates/worldwake-ai/tests/golden_harness/` (or equivalent test-support module) follow the dual-use read-model pattern: they remain test-facing and compose over runtime types (`AssumptionEvalResult`, `DiscrepancyMemory`, `FrameAssumption`). No runtime API changes are required by this ticket.
3. The `survival-need-projection.ron` scenario (if authored) is a sibling to existing `scenarios/survival-*.ron` files and follows the same structural conventions. It is purely test data; no runtime code depends on its presence outside the golden test.

## Verification Layers

1. `populate_assumptions` adds a `NeedSafeUntilTick { need: Hunger, until_tick: <plan_completion> }` after the agent adopts the breaching plan → decision trace assertion (the assumption's presence in the active frame at the post-adoption tick).
2. `evaluate_assumptions` returns `AssumptionEvalResult::CriticalFailure(NeedSafeUntilTick { .. })` on the next tick → decision trace assertion (the eval result is surfaced via the `PlanInvalidationReason::AssumptionFailed { assumption }` path that observer.rs consumes).
3. `Discrepancy::NeedHorizonExceeded { need: Hunger, projected_breach_tick: <breach> }` lands in `DiscrepancyMemory` with `expires_tick == failure_tick + structural_block_ticks` and `clearing_condition == TtlExpiry` → authoritative world-state assertion against the ECS-stored `DiscrepancyMemory` component.
4. `DiscrepancyMemory::is_suppressed` returns `true` for the original goal's `BlockerKey` for the next `structural_block_ticks` ticks → focused unit assertion against `DiscrepancyMemory` (no event-log scan needed).
5. The shorter-completion alternative wins ranking on the next round → decision trace assertion (the agent's adopted goal at the post-suppression tick is the alternative, not the original).
6. The goal's previously-failed plan does NOT get re-adopted while the suppression is in effect → decision trace + agent-tick driver state assertion. (The `current_plan` field on the runtime should not carry the breaching plan again until TTL expiry.)

## What to Change

### 1. Author or extend the test scenario

Choose one approach based on whether `survival-baseline.ron` can naturally produce the breach:

**Option (a) — Author `scenarios/survival-need-projection.ron`** (recommended): a focused 2-3 place scenario with one agent. Tighten `MetabolismProfile.hunger_rate` (e.g., 90/tick) and ensure the place graph forces a multi-step plan (e.g., 2-tick travel + 3-tick harvest, total ≥ 5 ticks) such that with the agent's initial hunger and high threshold, the projection breaches mid-plan. Provide both a "long" goal path (direct sleep) and a "short" alternative (harvest-then-sleep) so the post-suppression ranking has a real choice to make.

**Option (b) — Reuse `survival-baseline.ron`** if the existing rate profile produces the breach naturally. Verify before committing by computing `breach_tick = current_tick + ⌈(hunger.high() - 420) / hunger_rate⌉` against the natural plan completion tick.

Default to (a) unless inspection of `survival-baseline.ron`'s rates clearly supports (b).

### 2. Write `crates/worldwake-ai/tests/golden_need_projection.rs`

Follow the structure of existing goldens (e.g., `golden_survival_baseline.rs`, `golden_survival_drive_escalation.rs`):
- Load the scenario via `worldwake_cli::scenario::load_scenario_file` and `spawn_scenario`.
- Step the simulation through the agent-tick driver until the agent adopts the breaching plan.
- Assert: `populate_assumptions` placed `NeedSafeUntilTick { need: Hunger, .. }` in the active frame's `assumptions: Vec<FrameAssumption>`.
- Step one more tick.
- Assert: the agent's frame transitioned to `Exhausted` via `AssumptionEvalResult::CriticalFailure`; `DiscrepancyMemory` contains a `NeedHorizonExceeded` entry; `is_suppressed` returns `true` for the original goal's blocker key.
- Step until next adoption.
- Assert: the adopted goal is the shorter-completion alternative (harvest-then-sleep), NOT the original direct-sleep goal.
- Step `structural_block_ticks` ticks forward.
- Assert: `is_suppressed` now returns `false` (TTL expired).

### 3. Add golden harness helpers if missing

If existing `crates/worldwake-ai/tests/golden_harness/` (or equivalent test-support module) lacks helpers for the assertions above, add minimal ones:
- `assert_assumption_present(frame: &IntentionFrame, expected: FrameAssumption)` — convenience for the per-tick assertion.
- `assert_discrepancy_recorded(world: &World, agent: EntityId, expected: Discrepancy)` — reads `DiscrepancyMemory` from the agent's components and asserts the variant + payload.
- `assert_blocker_suppressed(world: &World, agent: EntityId, goal: GoalKind, current_tick: Tick)` — wraps `DiscrepancyMemory::is_suppressed` for readability.

These compose over runtime types and remain reusable for any later assumption-driven golden.

## Files to Touch

- `crates/worldwake-ai/tests/golden_need_projection.rs` (new) — the golden test
- `scenarios/survival-need-projection.ron` (new — likely needed under option (a))
- `crates/worldwake-ai/tests/golden_harness/` (modify or new helper file) — assertion helpers if not already present. Likely path: `crates/worldwake-ai/tests/golden_harness/need_projection_assertions.rs` or extension of an existing helpers file. Confirm during implementation by listing the directory.

## Out of Scope

- Behavioral changes to the spec's chain — tickets 001-003 own that
- Symmetric coverage for thirst/fatigue/bladder/dirtiness need-horizon assumptions — explicit follow-up; this ticket exercises hunger as the canonical case
- Activity-multiplier-aware projection coverage — explicit non-goal per spec Design Goal 7
- Wake-on-projection-breach for sleep episodes — out of scope for S126; that is S128's surface

## Acceptance Criteria

### Tests That Must Pass

1. New golden test: `golden_need_projection_chain` (or similar) passes deterministically against the chosen scenario.
2. Existing suite: `cargo test -p worldwake-ai --test golden_need_projection` passes.
3. Existing suite: `cargo test -p worldwake-ai` passes (no regression in sibling goldens).
4. Existing suite: `cargo test --workspace` passes.
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. The golden runs deterministically across multiple seeds (per the project's deterministic-RNG invariant). If multiple seeds are exercised, all must produce the same chain.
2. The golden does not depend on activity multipliers — projection uses base rate per Design Goal 7.
3. The golden does not assert on wall-clock time, `HashMap`/`HashSet` iteration order, or floats.
4. The golden's scenario file (if new) follows existing `scenarios/survival-*.ron` structural conventions.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_need_projection.rs` — the new golden, structured as an integration test using the scenario loader and agent-tick driver. Sibling pattern: `golden_survival_baseline.rs`, `golden_survival_drive_escalation.rs`.
2. `crates/worldwake-ai/tests/golden_harness/need_projection_assertions.rs` (or extension of existing helpers file) — the three assertion helpers if not already present.

### Commands

1. `cargo test -p worldwake-ai --test golden_need_projection`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-26.

- Authored `scenarios/survival-need-projection.ron`: a 1-agent / 2-place auxiliary scenario that tightens `MetabolismProfile.hunger_rate` to 30/tick so the agent's `hunger=600` projects breach against the default `DriveThresholds::hunger.high()=750` in 5 ticks, well inside the 8-tick plan completion window for the only known apple-acquisition path (5-tick travel to "Distant Orchard" + 3-tick `Harvest Apples`). Sets `cognitive_profile.structural_block_ticks=30` to keep the TTL-expiry phase of the test fast. Suppresses the `UnreachableExplorationDrive` lint via `scenario_lint_overrides` because exploration is intentionally disabled to keep ranking deterministic after suppression.
- Authored `crates/worldwake-ai/tests/golden_harness/need_projection_assertions.rs`: three reusable helpers (`frame_contains_need_safe_until_tick`, `first_need_horizon_entry`, `blocker_is_suppressed`) that compose over `IntentionFrame`, `DiscrepancyMemory`, and `BlockerKey` so future assumption-driven goldens can read the same proof surfaces without duplicating destructuring boilerplate.
- Authored `crates/worldwake-ai/tests/golden_need_projection.rs`: a single end-to-end golden test (`golden_need_projection_chain`) that proves the full S126 chain — populate → evaluate → record → suppress → alternative-goal adoption → TTL expiry — over the new scenario via `load_scenario_file` + `spawn_scenario`. Beliefs about non-co-located entities are seeded post-spawn through `seed_actor_world_beliefs` because the scenario disables curiosity. Suppression status is captured at the discrepancy tick inline (before TTL pruning could remove the entry by test end). Alternative-goal adoption is checked against `runtime.current_plan.goal` rather than `frame.goal` because the existing intention frame transitions to `FrameState::Suspended { reason: PriorityInterrupt }` while keeping its original `goal` field; the agent's actual executed plan is the runtime's `current_plan`. The test also asserts the `expires_tick == observed_tick + structural_block_ticks` invariant and the `DiscrepancyClearing::TtlExpiry` clearing condition.
- Wired the new helper module into `crates/worldwake-ai/tests/golden_harness/mod.rs` (`pub mod need_projection_assertions` + re-export of the three helpers).
- Added `survival-need-projection.ron` as an auxiliary scenario in `docs/scenario-roadmap.md` §5.17 alongside `cli-evaluation.ron`, with explicit rationale for why it is not a roadmap landing (no `survival_health_contract`, deliberately extreme metabolism rate, exploration disabled).
- Refreshed the generated companion at `docs/generated/scenario-coverage.md` via `cargo run -p worldwake-cli --bin scenario-coverage -- --write`.

## Deviations

- **Alternative-goal proof surface (`runtime.current_plan.goal`, not `frame.goal`)**: the spec D8 asks for "a shorter-completion alternative plan wins the next ranking round". Direct observation under the live agent-tick driver shows the original AcquireCommodity intention frame transitions to `FrameState::Suspended { reason: PriorityInterrupt }` and retains its `goal` field while the agent's runtime executes a different `current_plan` — the agent's actual behaviour is the active runtime plan, not the suspended frame's goal. The golden's `saw_alternative_plan` milestone therefore reads `runtime.current_plan.goal` for the alternative-goal check. This is a precision correction to the ticket's narrative — the proof contract still binds at the same architectural seam (a goal whose key differs from the suppressed `BlockerKey.goal_key` is executing under the suppression window).
- **`DriveThresholds::default()` confirmation**: ticket reassessment §1 hypothesised "high ~700"; live values are `hunger.high()=750` and `fatigue.high()=800` per `crates/worldwake-core/src/drives.rs`. The scenario rates and assertions are calibrated against the live values.
- **Suppression status captured inline at the discrepancy tick**: the test loop records `blocker_is_suppressed(...)` immediately when the discrepancy is first observed, rather than asserting it at the end of the loop, because subsequent re-observations refresh `expires_tick` and the post-loop reading would race against TTL pruning. The final assertion compares the captured snapshot.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_need_projection` (1 test: `golden_need_projection_chain`).
- Passed `cargo test -p worldwake-ai` (full ai crate green: 1484 unit tests, 38 golden integration tests including the new `golden_need_projection`, plus conformance and forensic suites).
- Passed `cargo test --workspace` (workspace green).
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo fmt --all -- --check`.
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `./scripts/verify.sh` (full pre-PR gate, exit 0).
