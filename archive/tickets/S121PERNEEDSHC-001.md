# S121PERNEEDSHC-001: Implement per-need survival health contracts

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario schema, survival golden harness helpers, contested survival scenario/golden, golden contract docs
**Deps**: archive/specs/S121-per-need-survival-health-contracts.md, archive/tickets/S119AUTHSURVHC-001.md

## Problem

After `S119AUTHSURVHC-001` removed the stale golden-local survival-envelope constants, `golden_survival_contested::all_agents_survive_1440_ticks` still failed because the current contract model is too coarse: the contested scenario intentionally does not require `Wash`, but one global `max_authored_critical_run_ticks` still applies equally to `dirtiness`. Raising that single cap enough to admit lawful contested dirtiness would also weaken the hunger/thirst/fatigue falsification surface. The survival-health contract needs per-need authored-critical bounds.

## Assumption Reassessment (2026-04-18)

1. `specs/S119-authored-survival-health-contracts.md` already landed the first substrate: `ScenarioDef.survival_health_contract`, shared helper consumption, and scenario-owned envelope bounds. The remaining contested failure appeared only after that retrofit removed stale file-local truth.
2. Live contested evidence after the S119 retrofit:
   - `cargo test -p worldwake-ai --test golden_survival_contested all_agents_survive_1440_ticks -- --ignored --exact`
   - failure: `Agent A dirtiness exceeded authored critical pm(900) for 1167 consecutive ticks (max allowed: 400)`
3. The authored contested scenario at `scenarios/survival-contested.ron` currently declares `required_self_care_families: [Eat, Drink, Sleep, Relieve]`, so the scenario no longer claims universal `Wash` coverage after the post-`S116DRIESCSUS-010` wash-rule correction.
4. Shared data contract under audit: survival-health falsification truth. Today one fact still has two incompatible meanings on the same field:
   - current canonical path: `survival_health_contract.max_authored_critical_run_ticks`
   - hidden overloaded meaning: "global cap that should also cover dirtiness even when Wash is not a required scenario family"
   This ticket replaces that overloaded meaning with a richer per-need contract.
5. Intended invariant before trusting the current contested failure: the survival golden should falsify exactly the need envelope the scenario author claims, not a coarser all-needs default that over-asserts unlike self-care families.
6. Live layer under test remains golden E2E, but the implementation work is schema + harness + authored contract carriage, not runtime/system behavior.
7. Reassessment shows this is not a retune-defaults ticket. The same command already has truthful hunger/thirst/fatigue coverage on the current branch; the missing expressive power is in the authored contract shape itself.
8. The current shared helper in `crates/worldwake-ai/tests/golden_harness/mod.rs` enforces one `max_run` equally across all five tracked needs in `assert_authored_critical_runs(...)`; that is the precise helper surface to extend.
9. Adjacent contradiction classification:
   - contested dirtiness overrun under a non-Wash-required scenario: required consequence of the coarse S119 contract and therefore owned here
   - scattered hunger overrun after the S119 retrofit: separate ticket, not owned here
10. No backwards-compatibility alias path is needed. The richer contract can preserve `max_authored_critical_run_ticks` as the scenario-wide default while adding optional per-need overrides.

## Architecture Check

1. Extending the authored contract is cleaner than inflating one global cap, because it keeps unlike needs concrete and separately falsifiable.
2. This approach preserves one canonical scenario-authored truth path instead of layering contested-only exceptions in the goldens.

## Verification Layers

1. Per-need contract deserializes through the CLI scenario path -> focused `worldwake-cli` scenario loader coverage.
2. Shared helper applies a per-need override only to the targeted need -> focused `worldwake-ai` helper/unit coverage.
3. Contested survival no longer over-asserts dirtiness relative to authored scenario truth -> `golden_survival_contested -- --ignored`.
4. Existing survival contract behavior remains canonical for scenarios without overrides -> baseline/scattered focused helper coverage or existing green commands.

## What to Change

### 1. Extend the authored contract schema

Add the per-need critical-run override structure from S121 to `worldwake-cli` scenario types and make it load through the normal scenario path.

### 2. Extend the shared survival-golden helper

Update the shared helper so authored-critical assertions read per-need overrides when present and otherwise use the scenario-wide default.

### 3. Retrofit contested scenario and golden

Author the contested scenario's per-need truth explicitly and update the contested golden to consume it.

### 4. Update docs

Document the richer contract in `docs/golden-e2e-testing.md`.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify if same-crate carriage fallout requires it)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify)
- `scenarios/survival-contested.ron` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- Changing live survival behavior
- Retuning `survival-scattered.ron`
- Replacing the S119 default contract surface for scenarios that do not need per-need overrides

## Acceptance Criteria

### Tests That Must Pass

1. Focused loader/helper coverage for the per-need override path
2. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
3. Existing suite: `cargo test -p worldwake-cli scenario`
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. One canonical authored survival-health contract path remains in force; the richer contract only adds expressive power.
2. Scenarios without per-need overrides still use the scenario-wide authored default.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — prove per-need survival contract deserializes
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` or focused golden test file coverage — prove per-need override beats the default only for the named need
3. `crates/worldwake-ai/tests/golden_survival_contested.rs` — consume the richer contract honestly

### Commands

1. `cargo test -p worldwake-cli scenario::types::tests -- --exact`
2. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
3. `cargo test -p worldwake-cli scenario`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Extended `SurvivalHealthContractDef` with optional per-need `critical_run_limits` in [types.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/types.rs) and added focused deserialization coverage for the new authored contract shape.
- Extended the shared survival-golden helper in [golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) so authored-critical assertions can use per-need overrides while preserving the scenario-wide default for other needs.
- Updated [golden_survival_contested.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_contested.rs) to map the authored per-need contract into the helper and added focused proof that a dirtiness-only override does not weaken the other need envelopes.
- Retrofitted [survival-contested.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-contested.ron) with an authored dirtiness override and updated [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) to document the richer contract.
- Absorbed 2 non-semantic clippy fallout fixes in the shared golden harness / contested helper test so the CI-matching lint surface stayed green.

## Deviations

- The first authored dirtiness override value (`1200`) still underfit the live deterministic contested run (`1277` ticks), so the final scenario-owned contested override shipped as `1300`.

## Verification Result

- Passed `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_deserializes_survival_health_contract -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested per_need_critical_run_limit_override_beats_default_for_dirtiness_only -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
- Passed `cargo test -p worldwake-cli scenario`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
