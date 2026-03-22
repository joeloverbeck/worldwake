# S19INSRECCON-002: Scenario 32 — Local ConsultRecord Prerequisite → Political Action

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S19INSRECCON-001 (harness helpers); E16c (ConsultRecord action) — COMPLETED; E16d (political goldens) — COMPLETED

## Problem

No golden E2E test exercises the ConsultRecord action through the AI planner. All existing political goldens (Scenarios 11–19, 22–25, 28) seed institutional beliefs directly via `seed_office_holder_belief()`, bypassing the ConsultRecord prerequisite path. This means the full chain — Unknown belief → planner inserts ConsultRecord → handler projects belief → political action — has zero E2E coverage despite having unit tests (`search.rs:5330`, `search.rs:5448`, `goal_model.rs:2717`).

Scenario 32 proves that when an agent has Unknown institutional belief about an office holder, the planner inserts ConsultRecord as a mid-plan prerequisite, the agent physically consults the record (with duration cost), acquires the Certain(None) belief, and then proceeds to DeclareSupport → succession installation.

## Assumption Reassessment (2026-03-22)

1. Unit test `search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown` at `search.rs:5330` confirms the planner inserts ConsultRecord when `InstitutionalBeliefRead::Unknown`. This unit test validates the planning layer; the golden test validates end-to-end execution.
2. Unit test `search_political_goal_skips_consult_record_when_vacancy_belief_is_already_certain` at `search.rs:5448` confirms the planner skips ConsultRecord when belief is `Certain`. This is the contrast behavior exercised by existing Scenario 11.
3. The `GoalKind` under test is `ClaimOffice`. The planner operator surface is: `ConsultRecord` (prerequisite when Unknown) → `DeclareSupport` (terminal). The affordance surface requires: the agent has entity belief about the office, the office has `SuccessionLaw::Support`, and the agent is at the office jurisdiction (for `DeclareSupport`) and at the record's `home_place` (for `ConsultRecord`).
4. Live consultation duration comes from `consultation_ticks * consultation_speed_factor / 1000` in `action_semantics.rs`, floored by integer division and clamped to at least 1 tick. With the harness default `consultation_ticks: 4` and `consultation_speed_factor: pm(500)`, ConsultRecord takes 2 ticks, not 8.
5. This is a golden E2E ticket. Full action registries are required (already provided by `GoldenHarness`).
6. No ordering dependency between agents — single agent scenario.
8. The closure boundary asserted is: ConsultRecord committed (action trace) → DeclareSupport committed (action trace) → succession resolution installs office holder (authoritative relation: `world.office_holder(office) == Some(agent)`). AI-layer symbols: `ClaimOffice` candidate in `DecisionOutcome::Planning`, plan shape includes `ConsultRecord` step. Authoritative-layer: `office_holder()` relation query.
10. Isolation: single agent, sated (no competing needs), single office, no other agents (no competing claimants). The only lawful branch is the ConsultRecord → DeclareSupport chain.

## Architecture Check

1. This follows the established golden test pattern from Scenario 11 (`build_*_scenario` + `run_*` + primary test + replay companion). The only difference is omitting `seed_office_holder_belief()` and adding `seed_office_vacancy_entry()`.
2. No backward-compatibility shims introduced.

## Verification Layers

1. ConsultRecord committed before DeclareSupport → action trace ordering (`ActionTraceSink::events_for()`)
2. Agent becomes office holder → authoritative world state (`world.office_holder(office) == Some(agent)`)
3. Plan includes ConsultRecord step → decision trace (`DecisionOutcome::Planning`, plan shape inspection)
4. Deterministic → replay companion (two runs with same seed produce identical world + event log hashes)
5. Single-agent scenario, so no multi-agent ordering concerns.

## What to Change

### 1. Add `build_consult_record_prerequisite_scenario()` in `golden_offices.rs`

Setup function creating:
- Single sated agent at `VILLAGE_SQUARE` with `enterprise_weighted_utility(pm(800))` and `PerceptionProfile { institutional_memory_capacity: 20, consultation_speed_factor: pm(500), ... }`. In current code this makes consultation faster than baseline, not slower.
- Vacant office at `VILLAGE_SQUARE` via `seed_office()` (creates OfficeRegister record with empty entries).
- Vacancy entry in the OfficeRegister via `seed_office_vacancy_entry(world, event_log, office, VILLAGE_SQUARE)` from S19INSRECCON-001.
- Entity beliefs about office and record via `seed_actor_beliefs()`.
- **No** `seed_office_holder_belief()` — agent starts with `InstitutionalBeliefRead::Unknown`.

### 2. Add `run_consult_record_prerequisite()` function

Runs 20 ticks (ConsultRecord 2 ticks + DeclareSupport 1 tick + succession period 5 ticks + margin). Asserts:
1. Action trace: `consult_record` committed before `declare_support` (using `ActionTraceSink`).
2. Authoritative state: `world.office_holder(office) == Some(agent)`.
3. Decision trace: early-tick plan includes ConsultRecord step.
4. Returns `(StateHash, StateHash)` for replay.

### 3. Add primary test `golden_consult_record_prerequisite_political_action`

Calls `run_consult_record_prerequisite(Seed([...]))`.

### 4. Add replay companion `golden_consult_record_prerequisite_political_action_replays_deterministically`

Standard two-run hash comparison + non-trivial simulation check, matching Scenario 11b pattern.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- No engine code changes
- No changes to `golden_harness/mod.rs` (handled by S19INSRECCON-001)
- No changes to existing golden scenarios (11–28)
- No multi-agent scenarios (that's S19INSRECCON-004)
- No remote record scenarios (that's S19INSRECCON-003)
- No documentation updates (that's S19INSRECCON-005)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_consult_record_prerequisite_political_action` — new primary test
2. `cargo test -p worldwake-ai --test golden_offices golden_consult_record_prerequisite_political_action_replays_deterministically` — new replay test
3. `cargo test -p worldwake-ai` — full AI crate suite (no regressions)
4. `cargo test --workspace` — workspace suite
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint

### Invariants

1. Agent must not have any seeded institutional belief about the office holder — the belief must come exclusively from ConsultRecord execution
2. ConsultRecord action must commit before DeclareSupport action (causally required — agent cannot support without knowing vacancy)
3. Succession installs the agent as office holder (the full E2E chain completes)
4. Determinism: two runs with the same seed produce identical state hashes
5. All existing golden tests (Scenarios 11–28) continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs::golden_consult_record_prerequisite_political_action` — proves ConsultRecord → belief acquisition → DeclareSupport → succession chain end-to-end
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_consult_record_prerequisite_political_action_replays_deterministically` — proves deterministic replay of the above

### Commands

1. `cargo test -p worldwake-ai --test golden_offices golden_consult_record_prerequisite_political_action` — targeted
2. `cargo test -p worldwake-ai` — AI crate
3. `cargo test --workspace` — full workspace
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint
