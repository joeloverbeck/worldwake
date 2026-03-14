# GOLDEN-001: Treatment Self-Acquisition Through AI Loop (Scenario 12)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only ticket; all system code exists
**Deps**: None (all engine support landed in E14PERBEL series + E12 combat/care)

## Problem

`AcquireCommodity(Treatment)` is the only remaining testable `GoalKind` without golden E2E coverage (15/17 → 16/17). The candidate generation, goal model, ranking, and search code all handle `CommodityPurpose::Treatment` already (`candidate_generation.rs:434`, `goal_model.rs:473,603,652`, `ranking.rs:113,212`, `search.rs:1832`), but no golden test proves the full emergent chain: wounds → pain pressure → treatment candidate → transport(pick-up) → care(heal).

The existing `golden_care.rs` tests (Scenario 2c) prove a *healer treating a patient* — a different agent acquires and applies medicine. This ticket proves the complementary **self-treatment** path where the wounded agent itself generates the `AcquireCommodity(Treatment)` goal and executes the pick-up → heal chain.

## Assumption Reassessment (2026-03-14)

1. `CommodityPurpose::Treatment` emission exists in `candidate_generation.rs:434` — confirmed present in current code.
2. `golden_care.rs` already has `place_ground_commodity()` helper and `seed_wounded_patient()` — reusable for this scenario.
3. The existing care tests use a two-agent setup (healer + patient). This scenario uses a single self-treating agent — structurally different.
4. Pain/danger pressure from wounds drives goal ranking — confirmed in `ranking.rs:113,212` and `pressure.rs`.
5. The `heal` action handler exists in `worldwake-systems/src/combat.rs` (care domain). Transport `pick_up` handler exists in `transport_actions.rs`.

## Architecture Check

1. Test-only change. No new engine code, no new abstractions. Reuses existing `golden_care.rs` helpers.
2. No backwards-compatibility shims. Single new test function + deterministic replay companion.

## What to Change

### 1. Add `run_self_treatment_scenario()` to `golden_care.rs`

Setup:
- Single agent (Rex) at `VILLAGE_SQUARE` with 2+ pre-inflicted wounds (severity pm(300) each, bleed_rate pm(0) — non-bleeding so the agent survives the test window).
- All homeostatic needs low/sated (hunger pm(100), thirst pm(0), fatigue pm(100), bladder pm(0), dirtiness pm(0)) so pain pressure is the dominant driver.
- Ground medicine lot at `VILLAGE_SQUARE` with `Quantity(3)` (enough for multiple heal applications).
- `CombatProfile` set for wound tracking.
- `UtilityProfile::default()` (pain weight is already positive by default).

Assertions (within 80-tick window):
- Rex picks up medicine (transport action — `agent_commodity_qty(rex, Medicine)` increases from 0).
- Rex's wound load decreases (care action — `agent_wound_load(rex)` decreases).
- Medicine conservation holds every tick (`total_live_lot_quantity(Medicine)` never increases).
- Rex stays alive throughout.

### 2. Add `golden_self_treatment_acquires_medicine_and_heals` test

Calls `run_self_treatment_scenario(Seed([30; 32]))`.

### 3. Add `golden_self_treatment_replays_deterministically` test

Two runs with `Seed([31; 32])` produce identical `(world_hash, event_log_hash)`.

### 4. Update `reports/golden-e2e-coverage-analysis.md`

- Move Scenario 12 from "Part 3: Missing Scenarios" to "Part 1: Proven Emergent Scenarios" with full writeup.
- Update GoalKind coverage table: `AcquireCommodity(Treatment)` → **Yes**, coverage 16/17 (94.1%).
- Update Part 4 summary statistics: proven tests count, GoalKind coverage fraction, cross-system chains count.
- Remove Scenario 12 from "Pending Backlog Summary" and "Recommended Implementation Order".

## Files to Touch

- `crates/worldwake-ai/tests/golden_care.rs` (modify — add ~60 lines: scenario runner + 2 test functions)
- `reports/golden-e2e-coverage-analysis.md` (modify — move scenario 12 to proven, update statistics)

## Out of Scope

- No changes to `golden_harness/mod.rs` (existing helpers suffice).
- No changes to any `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai/src/` production code.
- No new `CommodityKind` variants or recipe definitions.
- No changes to other golden test files (`golden_combat.rs`, `golden_production.rs`, etc.).
- Do not add multi-agent treatment scenarios (already covered by existing Scenario 2c).
- Do not add travel-to-distant-medicine variants (keep scope to local self-treatment).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_self_treatment_acquires_medicine_and_heals` — Rex picks up ground medicine via `AcquireCommodity(Treatment)` and heals own wounds.
2. `golden_self_treatment_replays_deterministically` — two runs with same seed produce identical world and event-log hashes.
3. Existing suite: `cargo test -p worldwake-ai --test golden_care` (all 4 existing tests still pass).
4. Full workspace: `cargo test --workspace` passes.

### Invariants

1. Medicine conservation: `total_live_lot_quantity(Medicine)` never increases during the scenario.
2. Agent stays alive throughout the 80-tick window.
3. No manual action queueing — all behavior is emergent through `AgentTickDriver` + `AutonomousControllerRuntime`.
4. Deterministic replay produces identical hashes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs::golden_self_treatment_acquires_medicine_and_heals` — proves `AcquireCommodity(Treatment)` → pick-up → heal chain.
2. `crates/worldwake-ai/tests/golden_care.rs::golden_self_treatment_replays_deterministically` — proves determinism.

### Commands

1. `cargo test -p worldwake-ai --test golden_care`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
