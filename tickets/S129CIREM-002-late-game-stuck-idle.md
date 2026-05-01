# S129CIREM-002: Late-game stuck-idle windows in survival baseline / contested / scattered

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Likely — exploration cooldown vs. low-pressure idle, or planner replan cadence under chronic critical state
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md

## Problem

Three CI golden tests are failing on stuck-idle assertions whose root cause is *not* hygiene-modifier saturation (verified by removing each modifier in isolation, no change). The failure has two distinct sub-shapes that may share or may not share a root cause and must be investigated as two questions before any engine change.

### Sub-shape A — late-game low-pressure idle (`golden_survival_baseline`)

`golden_survival_baseline::no_stuck_idle_windows_with_elevated_needs` reports four idle windows, all clustered at tick 1079+:

| Agent | Window | Length | max_need at start |
| --- | --- | --- | --- |
| Agent B (explorer) | 1079–1136 | 57 ticks | 313 |
| Agent A | 1095–1137 | 42 ticks | 307 |
| Agent C | 1127–1170 | 43 ticks | 392 |
| Agent C | 1220–1262 | 42 ticks | 528 |

`max_need < 600` for three of four windows — the agents are *not* in a critical-need stall. The window contract is `elevated_need_floor: 300`, so any need >= 300 with no action for 40+ ticks counts. Agent B's `exploration_profile` has `max_consecutive_explorations: 4, visit_lookback_ticks: 140` (`scenarios/survival-baseline.ron:266–268`), so an explorer who has burned through four consecutive explores enters a 140-tick cooldown; combined with low needs, the agent has no compelling goal.

### Sub-shape B — chronic critical-need idle (`golden_survival_contested`, `golden_survival_scattered`)

`golden_survival_contested::no_stuck_idle_windows_with_elevated_needs` reports ten windows starting at tick 106; later windows show `max_need_at_start = 1000` for multiple agents. `golden_survival_scattered::no_stuck_idle_windows_with_elevated_needs` reports four windows for Agent A specifically, climbing from `max_need = 489` at tick 454 to `max_need = 1000` at tick 978–1033 (i.e. the agent's max-saturated need stays unrelieved for 56 consecutive ticks past its critical threshold). This is *not* low-pressure idle — agents have actual saturated needs and are still not acting.

The two sub-shapes likely have different causes and the ticket must investigate both before proposing a unified fix.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Pre-S129 baseline**: parent commit (`fa0dd620`) passes `golden_survival_baseline::no_stuck_idle_windows_with_elevated_needs` with zero windows. Verified by checkout-and-run during S129 CI investigation. The windows post-S129 are a regression introduced by some interaction in the S129 changeset, not a pre-existing failure.
2. **Hygiene modifier is not the cause** (verified empirically during S129 CI investigation): removing each branch of `apply_hygiene_motive_modifiers` (`crates/worldwake-ai/src/ranking.rs:1644–1664`) individually did not eliminate the late-game baseline windows. Removing all four modifiers reduced windows from 12 to 5 in one experiment but kept the late-game cluster intact; the parent S129 commit's `HYGIENE_FACTOR_FLOOR = 700` rework leaves baseline at 4 windows. The ranking multiplier saturation is therefore a contributing factor for the *number* of windows, but not the *root cause* of the late-game cluster.
3. **Live exploration profile**: Agent B `curiosity_weight: 750, need_activation_threshold: 325, max_consecutive_explorations: 4, visit_lookback_ticks: 140` (`scenarios/survival-baseline.ron:260–268`). Agent A and Agent C have similar `max_consecutive_explorations: 4`. The `visit_lookback_ticks` and consecutive-cap surface lives in `crates/worldwake-ai/src/candidate_generation.rs` (search for `max_consecutive_explorations`, `acquisition_failure_threshold`).
4. **Live `agent_tick` symbols**: `crates/worldwake-ai/src/agent_tick/active_action.rs`, `crates/worldwake-ai/src/agent_tick/frame.rs`, and `crates/worldwake-ai/src/failure_handling.rs` carry the replan / start-failed / cooldown logic. The reassessment must check these against the late-game tick range to see whether the agent is hitting structural blockers, transient blockers, or simply running out of candidates.
5. **Live action-trace surface**: `crates/worldwake-ai/tests/golden_survival_baseline.rs:179` already enables `agent_has_non_failed_action_or_active`. The investigative test should *also* dump per-tick decision traces for the affected agents in the relevant tick window before proposing the fix.
6. **Sub-shape B difference**: in scattered, Agent A's `start_tick: 978–1033` window has `max_need_at_start: 1000`. The agent is at full pressure and *still* not committing actions. This is not "no candidates"; it must be either "candidates produced but plans fail to start" (e.g. `BindingStrictness` or `Precondition` rejecting at `start_action`) or "plans found but all stale/blocked". The first failure boundary is the `start_action` symbol set: `crates/worldwake-sim/src/action_validation.rs`, `crates/worldwake-sim/src/affordance_query.rs::evaluate_precondition`, or `crates/worldwake-ai/src/agent_tick/active_action.rs::start_step` (depending on layout).
7. **Stale-request / start-failure precision (precision-rules §9)**: the first failure boundary is *probably* in the start path or the candidate-binding path, not in `agent_tick` per se. Verify by extracting `ActionTraceKind::StartFailed` events for the affected agents within the failure windows.
8. **Coverage gap (precision-rules §3)**: the existing goldens enable the action trace and decision trace sinks but do not currently dump them for the failure windows. This ticket must add a focused trace dump (a one-shot investigative test) before any engine change.
9. **Heuristic Removal Discipline (precision-rules §12)**: tempting fixes — bumping `max_consecutive_explorations`, lowering `visit_lookback_ticks`, or removing exploration cooldown entirely — substitute weight tuning for actually fixing the underlying gap. Do not adopt them without naming the substrate they replace.
10. **Cumulative arithmetic (precision-rules §15)**: scattered Agent A reaches `max_need = 1000` at tick 978 and stays idle until 1033 (55 ticks). With `dehydration_tolerance_ticks = 240` and `bladder_accident_tolerance_ticks = 40`, a 55-tick stall at full saturation creates real wound aftermath. The `assert_authored_critical_runs_with_overrides` companion assertion in scattered already passes (3 of 6 contested-survival assertions failed, `all_agents_survive_1440_ticks` now passes after parent S129 commit) — so the agent does survive — but the *stuck-idle* contract is the canary. Investigate whether the agent is in a wound-driven cooldown loop.
11. **Mismatch + correction**: the original S129 spec's golden coverage requirements (S129PLADIRFAC-012) did not anticipate that the new ranking modifiers would alter agent timing enough to expose pre-existing edge cases in exploration cooldown / stale-plan handling. Either the spec under-specified the goldens or the ranking arithmetic interacts with cooldown systems in a way the spec did not analyze. This is a *required consequence* of S129's ranking changes, not a separate bug.

## Architecture Check

1. **Investigation precedes fix**: the two sub-shapes need separate decision-trace dumps before any code change. Without traces, any "obvious" fix (e.g. `max_consecutive_explorations + 1`) is heuristic-tuning that may regress other goldens.
2. **No backwards-compatibility shim**: do not add a "force-replan when stuck idle" hook. If the agent's planner refuses to act, the cause is in the candidate / search / start pipeline and must be fixed at the boundary that actually rejects.
3. **No silent test relaxation**: do not raise `max_idle_window_ticks_with_elevated_need` from 40 to 60 without naming the architectural substrate that justifies the relaxation.

## Verification Layers

1. **Late-game low-pressure idle (sub-shape A)** -> decision-trace dump for Agent B at tick 1079 onwards, naming what candidates *are* generated and which one (if any) is selected. If no candidates are generated, the trace surface is candidate-generation; if candidates are generated but rejected, the surface is ranking; if ranked but plan-not-found, the surface is search.
2. **Chronic critical-need idle (sub-shape B)** -> action-trace dump for the saturated agent, looking specifically for `ActionTraceKind::StartFailed` events. If `StartFailed` appears, the first failure boundary is `start_action`'s precondition evaluation.
3. **Plan freshness** -> if the agent is committed to a stale plan that the ranking would otherwise replace, the `switch_margin` / replan cadence on `agent_tick/frame.rs` is the surface.
4. **Belief currency** -> if remote basins, latrines, or food sources have stale beliefs that fail their preconditions during search, the surface is `planning_snapshot.rs` and the new wash-state belief storage from the parent S129 commit (`crates/worldwake-core/src/belief.rs::BelievedEntityState::wash_basin_state`).
5. **No need projection regression** -> S126's need-projection time-budget should not silently turn off late in the run. Verify `NeedProjectionPolicy` and related thresholds.

## What to Change

### 1. Investigation: dump traces for both sub-shapes

Add a one-shot diagnostic test in `crates/worldwake-ai/tests/` that runs each scenario for the relevant tick range and prints, per affected agent per tick:

- Decision trace: candidates emitted, candidates ranked, selected candidate, rejection reasons.
- Action trace: action lifecycle events (`Started`, `Committed`, `StartFailed`, `Aborted`).
- `DeprivationExposure`, `HomeostaticNeeds`, current place, current active action.

Only after the trace data is in hand should the ticket be split into the actual fix tickets (this ticket may decompose into S129CIREM-002a, -002b on the basis of trace evidence).

### 2. Sub-shape A fix: low-pressure exploration substrate

If the trace shows Agent B is stuck because exploration is on cooldown and other goals are below their activation thresholds, the fix is to give the agent a real low-pressure affordance — e.g.:

- `Wash` candidate at moderate (sub-critical) dirtiness when basin is co-located, not only at critical pressure.
- `Sleep` candidate when fatigue is moderate and a high-quality bed is co-located.
- `Drink` / `Eat` opportunistically when the agent is co-located with a stocked source and the need is non-trivial.

These are *opportunistic self-care* candidates, not a new ranking class. The substrate exists today (the candidate emitter already handles low/medium thresholds for some needs); this ticket may re-tune the activation thresholds rather than adding a new mechanism.

### 3. Sub-shape B fix: critical-need stall

If the trace shows agents at saturated needs failing to commit actions, the fix lives at one of:

- **Candidate generation** — if no candidate is emitted for the critical need, fix the emitter.
- **Ranking** — if the candidate is filtered to `zero_motive`, fix the motive arithmetic.
- **Search** — if the candidate is selected but search fails, fix the operator/precondition that rejects.
- **Start path** — if the plan is found but `start_action` rejects (binding, precondition, contention), fix the boundary that rejects.

Choose exactly one based on trace evidence.

### 4. Targeted golden coverage

After the fix, add focused goldens that isolate each invariant:

- `agent_at_low_pressure_with_co_located_affordance_does_not_stuck_idle` — single-agent, single-place, no critical needs but moderate pressure; assert no idle window > 40 ticks.
- `agent_at_saturated_need_does_not_stuck_idle_under_chronic_state` — single-agent fixture pinned at saturated dirtiness/bladder/etc., assert action committed within the saturation tolerance window.

## Files to Touch

- `crates/worldwake-ai/tests/` (new investigative test file — likely one-shot diagnostic)
- `crates/worldwake-ai/src/candidate_generation.rs` or `crates/worldwake-ai/src/ranking.rs` (if trace points there)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (if trace points there)
- `crates/worldwake-ai/tests/golden_place_dirtiness.rs` or new focused golden file (after fix)

## Out of Scope

- Hygiene-modifier ranking arithmetic — already fixed by parent S129 CI commit.
- Drive-escalation wash recurrence — separate ticket S129CIREM-001.
- Tell-session vs self-care — separate ticket S129CIREM-003.
- Patrol vs self-care — separate ticket S129CIREM-004.
- Survival contract value tuning (`max_idle_window_ticks_with_elevated_need`) — should not be touched without first naming the architectural substrate the relaxation rests on.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_baseline::no_stuck_idle_windows_with_elevated_needs` — zero stuck idle windows.
2. `golden_survival_contested::no_stuck_idle_windows_with_elevated_needs` — zero stuck idle windows.
3. `golden_survival_scattered::no_stuck_idle_windows_with_elevated_needs` — zero stuck idle windows.
4. New focused goldens (one per sub-shape) — pass.
5. Existing suite: `cargo test --workspace`, `./scripts/verify.sh`.

### Invariants

1. **No 40+ tick idle while need is elevated**: under contracts where `max_idle_window_ticks_with_elevated_need = 40` and `elevated_need_floor = 300`, agents do not stuck-idle for the full window. The fix names the substrate (candidate emitter, ranking, search, or start path).
2. **No silent contract relaxation**: this ticket does not raise `max_idle_window_ticks_with_elevated_need` or lower `elevated_need_floor` to make the assertions pass.
3. **No "force-replan" hook**: any replan-cadence change is justified by the failure boundary the trace identifies, not by adding a generic catch-all.

## Test Plan

### New/Modified Tests

1. New diagnostic test (one-shot) — dumps traces for the failure windows. May be deleted after the fix lands; lives in `crates/worldwake-ai/tests/diagnostics_*` or inline as `#[ignore]`.
2. New focused golden for sub-shape A — opportunistic low-pressure self-care.
3. New focused golden for sub-shape B — saturated-need start-path correctness.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1`
3. `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
4. `cargo test --workspace`
5. `./scripts/verify.sh`
