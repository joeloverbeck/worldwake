# S104SURBASREC-001: Delete hash-dependent golden test files

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The golden test suite contains 19 files (~251 tests) that use StateHash comparisons or tick-specific action sequence assertions. These tests encode the current broken priority ordering and block any behavioral changes to candidate generation, goal ranking, or exploration triggers needed for survival fixes. They must be removed to unblock S104 Phase 2 and Phase 3 work.

## Assumption Reassessment (2026-04-15)

1. All 19 REMOVE files exist in `crates/worldwake-ai/tests/` — confirmed via glob during reassessment. File list: `golden_budget_exhaustion_snapshots.rs`, `golden_care.rs`, `golden_combat.rs`, `golden_commodity_opportunity.rs`, `golden_determinism.rs`, `golden_emergent.rs`, `golden_expectation.rs`, `golden_integration.rs`, `golden_long_scenarios.rs`, `golden_patrol.rs`, `golden_production.rs`, `golden_pursuit.rs`, `golden_reasoning_diversity.rs`, `golden_resilience.rs`, `golden_soak.rs`, `golden_social.rs`, `golden_supply_chain.rs`, `golden_t22_bandit_camp_destruction.rs`, `golden_trade.rs`.
2. Low-risk mismatch auto-correction: the ticket originally assumed a shared test harness entry point with `mod golden_*;` declarations. Live code uses Rust integration-test files directly; each file declares only its own local `mod golden_harness;`, so deleting the 19 REMOVE files removes their harness declarations with them. No shared root file needs modification.
3. KEEP files (`golden_activation_decay.rs`, `golden_exploration.rs`, `golden_perception_exposure.rs`, `golden_travel_physiology.rs`) and TRIAGE files (6 files) must NOT be touched by this ticket.

## Architecture Check

1. Mechanical deletion with no behavioral changes. The cleanest approach — removing entire files rather than surgically editing tests ensures no partial state or dead imports remain.
2. No backwards-compatibility shims introduced. Files are deleted outright.

## Verification Layers

1. Remaining AI golden/integration suites pass after deletion → `cargo test -p worldwake-ai`
2. KEEP tests still pass → `cargo test -p worldwake-ai` runs remaining tests successfully
3. Broad repo verification stays clean → `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`
4. Single-layer ticket — deletion only, no runtime or planning changes.

## What to Change

### 1. Delete 19 golden test files

Remove these files from `crates/worldwake-ai/tests/`:
- `golden_budget_exhaustion_snapshots.rs`
- `golden_care.rs`
- `golden_combat.rs`
- `golden_commodity_opportunity.rs`
- `golden_determinism.rs`
- `golden_emergent.rs`
- `golden_expectation.rs`
- `golden_integration.rs`
- `golden_long_scenarios.rs`
- `golden_patrol.rs`
- `golden_production.rs`
- `golden_pursuit.rs`
- `golden_reasoning_diversity.rs`
- `golden_resilience.rs`
- `golden_soak.rs`
- `golden_social.rs`
- `golden_supply_chain.rs`
- `golden_t22_bandit_camp_destruction.rs`
- `golden_trade.rs`

### 2. Remove orphaned imports

After deletion, run `cargo clippy --workspace --all-targets -- -D warnings` to catch any unused imports or dead code referencing the removed test modules.

## Files to Touch

- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (delete)
- `crates/worldwake-ai/tests/golden_care.rs` (delete)
- `crates/worldwake-ai/tests/golden_combat.rs` (delete)
- `crates/worldwake-ai/tests/golden_commodity_opportunity.rs` (delete)
- `crates/worldwake-ai/tests/golden_determinism.rs` (delete)
- `crates/worldwake-ai/tests/golden_emergent.rs` (delete)
- `crates/worldwake-ai/tests/golden_expectation.rs` (delete)
- `crates/worldwake-ai/tests/golden_integration.rs` (delete)
- `crates/worldwake-ai/tests/golden_long_scenarios.rs` (delete)
- `crates/worldwake-ai/tests/golden_patrol.rs` (delete)
- `crates/worldwake-ai/tests/golden_production.rs` (delete)
- `crates/worldwake-ai/tests/golden_pursuit.rs` (delete)
- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (delete)
- `crates/worldwake-ai/tests/golden_resilience.rs` (delete)
- `crates/worldwake-ai/tests/golden_soak.rs` (delete)
- `crates/worldwake-ai/tests/golden_social.rs` (delete)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (delete)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (delete)
- `crates/worldwake-ai/tests/golden_trade.rs` (delete)

## Out of Scope

- KEEP files (4 files) — not touched
- TRIAGE files (6 files) — handled by S104SURBASREC-002
- Golden harness infrastructure (`golden_harness/mod.rs`, `golden_harness/soak_world.rs`, `golden_harness/timeline.rs`) — not touched
- Regenerating golden docs — handled by S104SURBASREC-002
- Any behavioral or production code changes

## Acceptance Criteria

### Tests That Must Pass

1. All KEEP tests pass: `golden_activation_decay`, `golden_exploration`, `golden_perception_exposure`, `golden_travel_physiology`
2. All TRIAGE tests pass (they are not touched by this ticket)
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo test --workspace`

### Invariants

1. No KEEP or TRIAGE file is deleted or modified
2. Workspace compiles cleanly with no dead imports or orphaned module declarations
3. Golden harness infrastructure files remain unchanged

## Test Plan

### New/Modified Tests

1. None — deletion-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai` — remaining tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo test --workspace` — full suite clean

## Outcome

Completed on 2026-04-15.

- Deleted the 19 REMOVE-category golden integration-test files from `crates/worldwake-ai/tests/`.
- Corrected the ticket's stale assumption about a shared `mod golden_*` harness entry point; live Cargo integration tests are file-based, so no separate root-module edit was needed.
- Left KEEP files, TRIAGE files, and `golden_harness` infrastructure unchanged.
- Deviation from original plan: no shared test-harness entrypoint edit was required after reassessment corrected the live ownership boundary.

## Verification Result

- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
