# E16DPOLPLAN-017: Succession resolution verification test

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The spec entry for E16d-017 assumed the Politics system `succession_system()` resolution path (vacancy activation -> support counting -> office holder installation) lacked focused authoritative coverage and needed a new isolated test.

## Assumption Reassessment (2026-03-18)

1. `succession_system()` is in [crates/worldwake-systems/src/offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) and remains the authoritative resolution entrypoint. Confirmed by source.
2. Support declarations are authoritative world relations created through `WorldTxn::declare_support` in [crates/worldwake-core/src/world/social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/social.rs). Confirmed by source.
3. The current code already has focused authoritative succession coverage in [crates/worldwake-systems/src/offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs):
   - `support_succession_installs_unique_top_supported_candidate_and_clears_declarations`
   - `support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes`
   - `support_tie_resets_vacancy_clock_without_installing_anyone`
   - `force_succession_installs_only_uncontested_eligible_present_agent`
   - `force_succession_blocks_when_multiple_contenders_are_present`
4. The originally proposed new test duplicates existing coverage. The majority-wins path, `office_holder` installation, vacancy timer clearing, and support declaration cleanup are already asserted by `support_succession_installs_unique_top_supported_candidate_and_clears_declarations`.
5. The timing contract is also already covered at the authoritative world-state layer. The existing support and force succession tests first trigger vacancy activation, then only resolve after the configured `succession_period_ticks` elapses.
6. This ticket is therefore not an engine-test gap anymore. It is a ticket-correction and verification task.

## Architecture Check

1. Adding another near-identical majority-vote test would not improve the architecture. It would duplicate focused authoritative coverage without adding a new invariant.
2. The current architecture is already clean at this layer:
   - `succession_system()` owns authoritative resolution.
   - support declarations stay in world relations rather than AI/planner state.
   - the focused `offices.rs` tests exercise the system directly without dragging AI or golden harness concerns into the authoritative layer.
3. The cleaner long-term choice is to keep one focused test per succession invariant rather than proliferating overlapping examples. That preserves signal and keeps the suite extensible as political rules evolve.

## Scope Correction

1. Do not add a duplicate succession-resolution unit test.
2. Verify that the existing focused authoritative tests still pass.
3. Verify the containing crate suite and workspace lint still pass.
4. Archive this ticket as already satisfied by existing coverage.

## Files Touched

- `tickets/E16DPOLPLAN-017.md`

## Out of Scope

- Production code changes in `succession_system()`
- Additional duplicate unit tests for already-covered support succession behavior
- AI candidate-generation, ranking, planner-search, or golden E2E behavior

## Acceptance Criteria

### Tests That Must Pass

1. `offices::tests::support_succession_installs_unique_top_supported_candidate_and_clears_declarations`
2. `offices::tests::support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes`
3. `offices::tests::support_tie_resets_vacancy_clock_without_installing_anyone`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. A unique top-supported eligible candidate is installed as `office_holder`.
2. Support succession does not install an ineligible candidate.
3. Ties do not silently assign an office holder.
4. Succession resolves only after vacancy activation and the configured succession delay.

## Tests

### New/Modified Tests

1. None. Existing focused succession tests already covered the intended invariant set.

### Existing Tests Relied On

1. `offices::tests::support_succession_installs_unique_top_supported_candidate_and_clears_declarations`
   Rationale: covers the exact majority-support install path the ticket originally proposed, plus declaration cleanup.
2. `offices::tests::support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes`
   Rationale: covers the edge case that matters more than a duplicate happy-path test, namely invalid support input and vacancy-clock reset behavior.
3. `offices::tests::support_tie_resets_vacancy_clock_without_installing_anyone`
   Rationale: covers the ambiguity branch so support succession remains deterministic and robust under contention.

## Test Plan

### Commands

1. `cargo test -p worldwake-systems support_succession_installs_unique_top_supported_candidate_and_clears_declarations`
2. `cargo test -p worldwake-systems support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes`
3. `cargo test -p worldwake-systems support_tie_resets_vacancy_clock_without_installing_anyone`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-18
- What actually changed: corrected the ticket to match the current codebase; no engine or test code changes were necessary because the authoritative succession invariants were already covered in `offices.rs`.
- Deviations from original plan: the original plan to add a new majority-vote unit test was dropped because it would have duplicated existing focused coverage rather than improving architecture.
- Verification results:
  - `cargo test -p worldwake-systems support_succession_installs_unique_top_supported_candidate_and_clears_declarations` ✅
  - `cargo test -p worldwake-systems support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes` ✅
  - `cargo test -p worldwake-systems support_tie_resets_vacancy_clock_without_installing_anyone` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
