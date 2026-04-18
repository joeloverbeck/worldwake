# S119: Authored Survival Health Contracts

## Summary

Add a shared, profile-driven survival health contract surface for authored scenarios so long-run survival goldens stop hardcoding thresholds or envelope bounds that can drift away from the scenario's actual `DriveThresholds` and intended self-care expectations. The contract lives with the scenario definition, is loaded through the normal CLI scenario path, and is consumed by the existing golden harness and observer-style scenario tooling as a derived read model. This turns "scenario passed healthy recently" from an informal memory into an explicit authored contract that stays aligned with `docs/FOUNDATIONS.md`.

## Phase and Status

Phase 7 Adjunct: Survival Stability Hardening. Status: ✅ COMPLETED.

## Crates

- `worldwake-cli` — scenario schema and loader support for authored survival-health expectations
- `worldwake-ai` — shared golden harness helpers that read authored expectations instead of local magic numbers
- `worldwake-core` — no changes
- `worldwake-sim` — no changes
- `worldwake-systems` — no changes

## Dependencies

- None.
- Intended to retrofit the existing survival scenario families:
  - `scenarios/survival-baseline.ron`
  - `scenarios/survival-scattered.ron`
  - `scenarios/survival-contested.ron`

## Motivating Evidence

(Conditions described below reflect the branch state at spec authoring; the drift was resolved by the implementation landed on 2026-04-18. Kept as the causal motivation for the substrate.)

Implementation of the S116 survival ticket chain exposed that the existing survival golden surface could silently diverge from authored scenario truth:

1. `golden_survival_baseline.rs` was enforcing a hardcoded sustained-critical bound of `pm(750)` even though the authored baseline scenario set higher per-agent critical thresholds in `DriveThresholds` (for example, Agent A thirst critical `pm(820)` and fatigue critical `pm(900)`).
2. The scenario could therefore look "healthy" under one proof surface and unhealthy under another, even when the planner was behaving consistently with the authored scenario profile.
3. The same scenario family also hardcodes envelope constants like `MAX_CRITICAL_RUN_TICKS`, idle-window bounds, and required self-care families directly in the golden files rather than carrying those expectations in one canonical authored place.

This is not merely a test-style issue. It is an architectural truth-carriage problem: the scenario author owns the survival envelope, but the current proof surface can restate that envelope with independent constants.

## Design Goals

1. Every survival scenario can author one canonical survival-health contract that the goldens consume directly.
2. Sustained-critical checks use each agent's authored `DriveThresholds`; no survival golden hardcodes a separate critical permille cutoff.
3. Non-threshold survival envelope checks such as maximum authored-critical run length, idle-window limits, and required self-care action families are authored once and reused.
4. Contract carriage remains scenario-local and deterministic; there is no second config path or CI-only override file.
5. Existing healthy survival scenarios become explicit about what they are proving instead of relying on comments and duplicated file-local constants.

## Non-Goals

- Changing live survival behavior.
- Auto-generating scenario contracts from traces.
- Moving long-run forensic diagnostics into this spec; that belongs in a separate traceability spec.
- Replacing focused lower-layer tests with survival goldens.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-1 (No Magic Numbers) | Survival health bounds move out of file-local test constants and into authored scenario contracts. |
| FND-14 (World State Is Not Belief State) | The contract is authored input and derived test read-model only; it does not change planner information flow. |
| FND-22 (Agent Diversity Through Concrete Variation) | Sustained-critical checks respect each agent's own authored `DriveThresholds` instead of flattening them behind one global test constant. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Golden helpers derive assertions from the authored scenario contract; they do not become an independent truth source. |
| FND-31 (Validation and Falsification Are First-Class) | The scenario now declares what "healthy" means, and goldens mechanically falsify that contract. |

## Deliverables

### D1: Scenario-authored survival health contract

Extend `crates/worldwake-cli/src/scenario/types.rs` with an optional authored contract section:

```rust
pub struct SurvivalHealthContractDef {
    pub max_authored_critical_run_ticks: u32,
    pub max_idle_window_ticks_with_elevated_need: u32,
    pub elevated_need_floor: Permille,
    pub required_self_care_families: Vec<NeedsActionFamily>,
}

pub enum NeedsActionFamily {
    Eat,
    Drink,
    Sleep,
    Relieve,
    Wash,
}

pub struct ScenarioDef {
    // existing fields...
    pub survival_health_contract: Option<SurvivalHealthContractDef>,
}
```

`survival_health_contract` is optional so non-survival scenarios do not carry irrelevant assertions.

Per-need critical-run overrides are out of scope for this spec. They were added as a separate substrate by [specs/S121-per-need-survival-health-contracts.md](/home/joeloverbeck/projects/worldwake/specs/S121-per-need-survival-health-contracts.md), which extends `SurvivalHealthContractDef` with an optional `critical_run_limits: Option<SurvivalCriticalRunLimitsDef>` field carried on the same authored-contract surface.

### D2: Shared golden helper reads authored contract

In `crates/worldwake-ai/tests/golden_harness/` (or the existing shared support file used by survival goldens), add helpers that:

1. load `survival_health_contract`
2. derive per-agent critical thresholds from authored `DriveThresholds`
3. expose one canonical assertion helper for:
   - max authored-critical run ticks
   - max idle window with elevated need
   - required self-care action-family coverage

Survival goldens stop carrying independent copies of these constants.

### D3: Retrofit existing survival scenario files

Add explicit `survival_health_contract` sections to:

- `scenarios/survival-baseline.ron`
- `scenarios/survival-scattered.ron`
- `scenarios/survival-contested.ron`

Each scenario declares:

- max authored-critical run length
- idle-window limit and elevated-need floor
- required self-care families

These values remain scenario-specific. The contract is not normalized across all survival scenarios.

### D4: Retrofit survival goldens

Update:

- `crates/worldwake-ai/tests/golden_survival_baseline.rs`
- `crates/worldwake-ai/tests/golden_survival_scattered.rs`
- `crates/worldwake-ai/tests/golden_survival_contested.rs`

to consume the shared authored contract instead of local constants where the invariant is scenario-owned.

The goldens may still keep file-local constants for test mechanics that are not scenario contracts, but not for survival-health bounds.

### D5: Contract-presence guard

Add a focused regression that fails if a golden survival scenario is missing `survival_health_contract`.

Implemented in the shared golden harness via `expect_survival_health_contract` (panics on `None`) plus a dedicated regression test in `crates/worldwake-ai/tests/golden_survival_baseline.rs` that constructs a synthetic scenario with `survival_health_contract: None` and confirms the guard fires.

### D6: Documentation

Update `docs/golden-e2e-testing.md` to state that long-run survival envelope checks must read authored scenario contracts and authored `DriveThresholds`, not restate them as file-local constants.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: The authored contract flows from `ScenarioDef` into test/read tooling only. It does not enter authoritative world state or planner-visible belief state.
2. **Positive-feedback analysis**: None. The contract is static authored input and cannot influence simulation behavior.
3. **Concrete dampeners**: Scenario-authored `max_authored_critical_run_ticks` and idle-window limits are explicit dampener bounds on what a scenario is allowed to prove healthy. They are not runtime dampeners.
4. **Stored state vs. derived read-model**:
   - **Stored/authored**: `survival_health_contract` in `ScenarioDef`
   - **Derived**: golden observations, run trackers, idle-window summaries, action-family coverage

## SystemFn Integration

None. This spec does not add or modify a `SystemFn`.

## Component Registration

None.

## Cross-System Interactions

- `worldwake-cli` scenario loading exposes the authored contract to callers.
- `worldwake-ai` goldens consume the contract through shared harness helpers.
- No simulation system consumes the contract at runtime.

## Validation and Falsification

### Focused tests

1. A synthetic scenario with `survival_health_contract = None` is accepted by the loader, but the survival-golden inventory guard rejects using it as a survival scenario.
2. Shared helper test: sustained-critical tracking compares against each agent's authored `DriveThresholds`, not a file-local constant.
3. Shared helper test: required self-care family coverage reports missing families deterministically.
4. Loader deserialization test: `crates/worldwake-cli/src/scenario/types.rs::tests::test_scenario_def_deserializes_survival_health_contract` proves the authored contract deserializes through the normal scenario path.

### Golden / integration tests

5. `golden_survival_baseline.rs` reads authored contract values and stays green.
6. `golden_survival_scattered.rs` reads authored contract values and stays green.
7. `golden_survival_contested.rs` reads authored contract values and stays green.

## Outcome

Completed on 2026-04-18.

**What changed**:

- Added `SurvivalHealthContractDef` and `NeedsActionFamily` to [crates/worldwake-cli/src/scenario/types.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/types.rs) and attached an optional `survival_health_contract: Option<SurvivalHealthContractDef>` field to `ScenarioDef`.
- Added shared survival-golden helpers to [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs): `expect_survival_health_contract`, `SurvivalNeedRunTracker`, `assert_authored_critical_runs`, `assert_required_self_care_families`, `assert_no_stuck_idle_windows`.
- Authored explicit `survival_health_contract` sections in [survival-baseline.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-baseline.ron), [survival-scattered.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-scattered.ron), and [survival-contested.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-contested.ron).
- Retrofitted [golden_survival_baseline.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_baseline.rs), [golden_survival_scattered.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_scattered.rs), and [golden_survival_contested.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_contested.rs) to consume the authored contract and removed the stale file-local `NEED_CRITICAL_THRESHOLD` / `MAX_CRITICAL_RUN_TICKS` / `pm(750)` constants.
- Added the contract-presence guard plus a dedicated regression test in `golden_survival_baseline.rs`.
- Updated [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) with the "Survival Health Contracts" section.

**Tickets**:

- [archive/tickets/S119AUTHSURVHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-001.md) — primary substrate and retrofit
- [archive/tickets/S119AUTHSURVHC-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-002.md) — scattered hunger-bound classification (authored bound raised from 400 → 550)

**Deviations from original plan**:

- Post-retrofit verification surfaced two additional blockers that were resolved in their own explicit owners rather than absorbed into this spec:
  1. Scattered hunger overrun → [archive/tickets/S119AUTHSURVHC-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-002.md) classified it as a too-strict authored bound and corrected `survival-scattered.ron`.
  2. Contested dirtiness overrun → [specs/S121-per-need-survival-health-contracts.md](/home/joeloverbeck/projects/worldwake/specs/S121-per-need-survival-health-contracts.md) extended `SurvivalHealthContractDef` with an optional `critical_run_limits: Option<SurvivalCriticalRunLimitsDef>` field so per-need authored-critical bounds could live on the same contract surface without weakening the global envelope.
- `NeedsActionFamily` carries a `Wash` variant even though `survival-contested.ron` authors `required_self_care_families: [Eat, Drink, Sleep, Relieve]` (no `Wash`). The variant exists at the enum level; each scenario declares its own required families.

**Verification**:

- Passed `cargo test -p worldwake-cli scenario`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
