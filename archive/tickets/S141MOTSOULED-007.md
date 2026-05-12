# S141MOTSOULED-007: Conformance tests, `golden_motive_sources`, ProfileHomogeneity lint extension

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No new engine state; added validation coverage and extended an existing lint
**Deps**: `archive/tickets/S141MOTSOULED-002.md`, `archive/tickets/S141MOTSOULED-004.md`, `archive/tickets/S141MOTSOULED-005.md`, `archive/tickets/S141MOTSOULED-006.md`

## Problem

S141's validation deliverable needs proof that production goal offers carry first-class motive-source evidence, the new motive-class profile fields have concrete defaults and scenario-lint coverage, and the golden inventory names the currently inspectable motive-source seams. Without that coverage, future goal emitters could omit `GoalOffer.motive_sources`, or scenario authors could clone the new `UtilityProfile` motive-class weights without a lint warning.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The live `GoalOffer` production assembly contract is the `worldwake_ai::generate_candidates` public seam plus helper-created offers in `candidate_generation.rs`; synthetic focused-test fixtures remain able to build explicit empty vectors so they can exercise `assert_motive_sources_present()`.
2. The live S141 scoring body from `archive/tickets/S141MOTSOULED-004.md` is parity-preserving: `ranking::score_motive_source` delegates current source variants to the extracted `score_goal_kind_motive` body, and `agent_tick::planning::motive_source_contributions_for_summary` assigns the aggregate score to the first source and zero to later sources.
3. Because of that live parity seam, the drafted autonomous goldens for "Hunger + Greed sum", "Pain dominates Hunger under wound profile", and "`greed_weight` changes the committed branch" were not truthful as production behavior in this ticket. The corrected boundary is validation of the representable payload, trace, profile-state, and assertion surfaces.
4. Observer Section 3b rendering is already owned and covered by `archive/tickets/S141MOTSOULED-006.md`; this ticket regenerates the golden inventory and adds fixture-backed motive-source golden detail, but it does not add a separate observer-rendering fixture.
5. `ProfileHomogeneity` lives in `crates/worldwake-cli/src/scenario/lints.rs` and already compared the whole optional `UtilityProfile`; this ticket adds explicit per-field variation checks for the five S141 motive-class fields so the lint remains tied to the new FND-22 variation contract.

## Architecture Check

1. The conformance test exercises the production candidate-generation boundary instead of relying on private constructors, so it fails if an emitted production offer reaches the public planner surface without motive-source evidence.
2. The golden tests stay fixture-backed where the current architecture can represent a motive-source fact without pretending that independent per-source production arithmetic exists.
3. The lint extension drills into the five S141 fields directly, matching the spec-owned motive-class additions without introducing any compatibility aliases or fallback paths.

## Verification Layers

1. Production candidate-generation invariant -> `conformance_motive_sources::every_goal_offer_has_motive_sources` checks every returned public `GoalOffer` in a representative full-registry setup has non-empty `motive_sources`.
2. Profile default invariant -> `conformance_motive_sources::utility_profile_default_for_motive_class` checks the five S141 `UtilityProfile` defaults are non-zero and equal to the documented values.
3. Golden/E2E inventory invariant -> `golden_motive_sources.rs` adds five scenario blocks covering default source mapping, commit-payload source preservation, trace contribution representation, profile-state variation, and empty-source assertion behavior.
4. Scenario-lint invariant -> `scenario::lints` tests prove variation in each of the five motive-class fields satisfies `ProfileHomogeneity`.

## Outcome

1. Added `crates/worldwake-ai/tests/conformance_motive_sources.rs` with production candidate-generation coverage for non-empty motive sources and focused default-value checks for the five new motive-class weights.
2. Added `crates/worldwake-ai/tests/golden_motive_sources.rs` with Scenario 403 through Scenario 407:
   - Scenario 403 proves default hunger mapping and contribution-sum parity for a hunger source.
   - Scenario 404 proves the event-log payload preserves multiple decisive motive sources in insertion order.
   - Scenario 405 proves the trace carrier can represent a pain contribution dominating a hunger contribution.
   - Scenario 406 proves `greed_weight` variation is concrete profile state available to scenarios.
   - Scenario 407 proves the empty-source assertion panics in test builds.
3. Extended `ProfileHomogeneity` with explicit variation checks for `office_duty_weight`, `loyalty_weight`, `greed_weight`, `shame_weight`, and `revenge_weight`.
4. Regenerated the golden inventory, scenario index, coverage matrix, and the per-file scenario detail pages.

## Deviations

1. The original drafted autonomous "Hunger + Greed sum", "Pain dominates under wound profile", and "per-agent `greed_weight` produces different commits" assertions depend on independent per-source production scoring that is not present in the live S141 004 implementation.
2. The remaining independent multi-source scoring and autonomous branch-divergence goldens were split to the now-archived `archive/tickets/S141MOTSOULED-008.md`.
3. `./scripts/verify.sh` was not run because this ticket is not a push preparation pass. Every live gate in the wrapper was run directly and recorded in `Verification Result`.

## Files Touched

- `crates/worldwake-ai/tests/conformance_motive_sources.rs` (added)
- `crates/worldwake-ai/tests/golden_motive_sources.rs` (added)
- `crates/worldwake-cli/src/scenario/lints.rs` (modified)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/*.md` (regenerated detail pages)

## Out of Scope

- Independent per-source production arithmetic for multi-source offers.
- Autonomous scenarios that require the above scoring split to make different branches commit.
- Deferred S141 motive variants whose substrates do not exist yet: `Fear`, `Obligation`, `Debt`, `Habit`, and `Curiosity`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test conformance_motive_sources`
2. `cargo test -p worldwake-ai --test golden_motive_sources`
3. `cargo test -p worldwake-cli scenario::lints`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo test --workspace`
6. `bash scripts/check_active_goal_removed.sh`
7. `bash scripts/check_no_artifact_state.sh`
8. `cargo clippy --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo run -p worldwake-cli --bin scenario-coverage -- --check`

### Invariants

1. Production candidate-generation offers exposed by the public planner seam carry non-empty `motive_sources`.
2. The five S141 motive-class `UtilityProfile` fields have concrete defaults and explicit `ProfileHomogeneity` variation checks.
3. The golden inventory records the current truthful motive-source proof surface without claiming unimplemented independent production scoring.

## Test Plan

### Changed Tests

1. `crates/worldwake-ai/tests/conformance_motive_sources.rs` — production candidate-generation and default-value conformance.
2. `crates/worldwake-ai/tests/golden_motive_sources.rs` — five fixture-backed motive-source golden scenarios.
3. `crates/worldwake-cli/src/scenario/lints.rs` — focused lint coverage for the five motive-class fields.

### Commands

1. `cargo test -p worldwake-ai --test conformance_motive_sources`
2. `cargo test -p worldwake-ai --test golden_motive_sources`
3. `cargo test -p worldwake-cli scenario::lints`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo test --workspace`
6. `bash scripts/check_active_goal_removed.sh`
7. `bash scripts/check_no_artifact_state.sh`
8. `cargo clippy --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo run -p worldwake-cli --bin scenario-coverage -- --check`

## Verification Result

1. Passed `cargo fmt --all`
2. Passed `cargo fmt --all -- --check`
3. Passed `cargo test -p worldwake-ai --test conformance_motive_sources`
4. Passed `cargo test -p worldwake-ai --test golden_motive_sources`
5. Passed `cargo test -p worldwake-cli scenario::lints`
6. Passed `python3 scripts/golden_inventory.py --write --check-docs`
7. Passed `cargo test -p worldwake-ai`
8. Passed `cargo test --workspace`
9. Passed `bash scripts/check_active_goal_removed.sh`
10. Passed `bash scripts/check_no_artifact_state.sh`
11. Passed `cargo clippy --workspace`
12. Passed `cargo clippy --workspace --all-targets -- -D warnings`
13. Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
14. Waived `./scripts/verify.sh` because this was not a push-preparation pass; every underlying gate in the wrapper was run directly.
