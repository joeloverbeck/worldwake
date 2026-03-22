# S19INSRECCON-003: Scenario 33 — Remote Record → Travel + ConsultRecord + Political Action

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S19INSRECCON-001 (harness helpers); S19INSRECCON-002 (establishes ConsultRecord golden pattern); E14 (perception) — COMPLETED

## Problem

No golden test exercises a multi-hop plan where the agent must travel to a remote record location, consult it, then travel to the office jurisdiction to act politically. Existing Scenario 15 tests travel to a remote office with already-known vacancy. Scenario 33 tests travel to a remote **record** to learn about vacancy, then travel to the office to act. The information-gathering detour through the record location is the novel contract.

This proves Principle 7 (locality — must physically travel to record location to gain institutional knowledge) and Principle 8 (travel + consultation have real duration and cost).

## Assumption Reassessment (2026-03-22)

1. The prototype topology at `topology.rs:460–528` confirms: VillageSquare ↔ RulersHall with 1-tick edges. OrchardFarm → EastFieldTrail (2 ticks) → SouthGate (1 tick) → VillageSquare (1 tick) → RulersHall (1 tick). Total OrchardFarm→RulersHall = 5 ticks. RulersHall→VillageSquare = 1 tick.
2. `seed_office()` creates records at the office jurisdiction. For this scenario, the office jurisdiction is VillageSquare but the consulted record must be at RulersHall. With the new harness helper from S19INSRECCON-001, the clean setup is: create the office normally, then call `seed_office_register(..., RULERS_HALL)` and append the vacancy entry there.
3. The `GoalKind` under test is `ClaimOffice`. The planner must produce a 4-step plan: Travel(→RulersHall) → ConsultRecord → Travel(→VillageSquare) → DeclareSupport. The planner's ability to route through the record location before the office jurisdiction is the contract under test.
4. Live consultation duration comes from `consultation_ticks * consultation_speed_factor / 1000`, floored by integer division and clamped to at least 1 tick. With the harness default `consultation_ticks: 4` and `consultation_speed_factor: pm(500)`, ConsultRecord takes 2 ticks.
5. This is a golden E2E ticket. Full action registries required (provided by `GoldenHarness`).
6. No ordering dependency between agents — single agent scenario.
8. Closure boundary: travel commits reaching RulersHall (action trace) → ConsultRecord committed (action trace) → travel commits reaching VillageSquare (action trace) → DeclareSupport committed (action trace) → succession installs office holder (authoritative relation). AI-layer: plan shape is Travel→ConsultRecord→Travel→DeclareSupport.
10. Isolation: single agent, sated, no competing affordances. The contract is the multi-hop information-gathering path shape.
12. Duration math: OrchardFarm→RulersHall travel = 5 ticks. ConsultRecord = 2 ticks with `consultation_ticks: 4` and `consultation_speed_factor: pm(500)`. RulersHall→VillageSquare travel = 1 tick. DeclareSupport = 1 tick. Succession period = 5 ticks. Total ≈ 14 ticks. Use a 30- to 40-tick run for margin.

## Architecture Check

1. Follows the established `build_*_scenario` + `run_*` + test + replay pattern. The setup is more complex (remote record at different location from office jurisdiction) but the test structure is identical.
2. The OfficeRegister record must be at RulersHall. Since `seed_office()` auto-creates records at the office jurisdiction (VillageSquare), the scenario setup should explicitly use `seed_office_register(..., RULERS_HALL)` and seed the vacancy entry there. The VillageSquare records created by `seed_office()` still exist, but the agent only knows about the RulersHall record.
3. No backward-compatibility shims introduced.

## Verification Layers

1. Travel to RulersHall committed → action trace (`ActionTraceSink`)
2. ConsultRecord committed at RulersHall → action trace
3. Travel to VillageSquare committed → action trace
4. DeclareSupport committed → action trace
5. Sequence: all four action commits in correct order → action trace ordering
6. Agent ends at VillageSquare → authoritative state (`world.effective_place(agent)`)
7. Agent is office holder → authoritative state (`world.office_holder(office)`)
8. Plan shape is Travel→ConsultRecord→Travel→DeclareSupport → decision trace
9. Deterministic → replay companion

## What to Change

### 1. Add `build_remote_record_consultation_scenario()` in `golden_offices.rs`

Setup function creating:
- Single sated agent at `ORCHARD_FARM` with `enterprise_weighted_utility(pm(800))` and perception profile.
- Vacant office ("Village Elder") at `VILLAGE_SQUARE` via `seed_office()`.
- Separate OfficeRegister record at `RULERS_HALL` with vacancy entry. Created via `seed_office_register()` + `seed_office_vacancy_entry()`.
- Entity beliefs: agent has beliefs about the office (at VillageSquare), the RulersHall record, and relevant places (OrchardFarm, RulersHall, VillageSquare, and intermediate places on the route). **No** institutional belief about office holder.

### 2. Add `run_remote_record_consultation()` function

Runs 40 ticks. Asserts:
1. Action traces in sequence: travel (reaching RulersHall), consult_record, travel (reaching VillageSquare), declare_support.
2. Authoritative state: agent at VillageSquare, agent is office holder.
3. Decision trace: initial plan shape includes Travel→ConsultRecord→Travel→DeclareSupport.
4. Returns `(StateHash, StateHash)` for replay.

### 3. Add primary test `golden_remote_record_consultation_political_action`

### 4. Add replay companion `golden_remote_record_consultation_political_action_replays_deterministically`

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- No engine code changes
- No changes to `golden_harness/mod.rs` (handled by S19INSRECCON-001)
- No changes to existing golden scenarios
- No multi-agent scenarios (that's S19INSRECCON-004)
- No changes to topology or prototype world
- No documentation updates (that's S19INSRECCON-005)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action` — new primary test
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically` — new replay test
3. `cargo test -p worldwake-ai` — full AI crate suite (no regressions)
4. `cargo test --workspace` — workspace suite
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint

### Invariants

1. Agent must physically travel to RulersHall before consulting the record — no teleportation (Principle 7)
2. Agent must consult the record before traveling to VillageSquare for DeclareSupport — information locality shapes path
3. The record at RulersHall is a distinct entity from the records auto-created by `seed_office()` at VillageSquare
4. Agent starts with `InstitutionalBeliefRead::Unknown` — belief comes from ConsultRecord, not seeding
5. Succession installs the agent as office holder (full E2E chain completes)
6. Determinism: two runs with same seed produce identical state hashes
7. All existing golden tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs::golden_remote_record_consultation_political_action` — proves Travel→ConsultRecord→Travel→DeclareSupport chain with remote record
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_remote_record_consultation_political_action_replays_deterministically` — deterministic replay

### Commands

1. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action` — targeted
2. `cargo test -p worldwake-ai` — AI crate
3. `cargo test --workspace` — full workspace
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint
