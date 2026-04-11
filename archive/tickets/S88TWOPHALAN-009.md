# S88TWOPHALAN-009: Golden tests for two-phase planning

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — golden coverage and helper assertions only
**Deps**: S88TWOPHALAN-007, S88TWOPHALAN-008, S88TWOPHALAN-010

## Problem

The two-phase planner now has live remote-care and remote-production golden scenarios, but those existing goldens still prove only the behavioral outcome. They do not yet assert the newly landed S88 planner-trace contract from `S88TWOPHALAN-008` or the profile-driven landmark diversity/graceful-degradation behavior. Without that additive golden coverage, regressions in the two-phase trace contract and landmark-depth diversity surface would remain under-proved.

## Assumption Reassessment (2026-04-11)

1. Existing golden owners already cover the live two-phase goal families: `golden_healer_acquires_remote_ground_medicine_for_patient` in `crates/worldwake-ai/tests/golden_care.rs` and `golden_remote_acquire_commodity_recipe_input` in `crates/worldwake-ai/tests/golden_production.rs`. The honest missing slice is additive assertions inside those owners, not a brand-new `golden_two_phase_planning.rs` file.
2. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` already owns profile-driven reasoning divergence. The missing S88-specific diversity proof belongs there via `landmark_extraction_depth` / `preferred_operator_boost`, not in a new standalone golden file.
3. `S88TWOPHALAN-008` landed the relevant planner-trace fields (`PlanAttemptTrace.strategic_plan`, `landmarks_extracted`, `landmark_orderings`, `SearchExpansionSummary.preferred_candidates`, `SearchExpansionSummary.landmark_heuristic`) in `crates/worldwake-ai/src/decision_trace.rs`. This ticket should prove those fields through existing golden decision traces rather than exposing lower-layer APIs from the golden surface.
4. Existing exploration and information-locality goldens already own the broad ignorance/no-omniscience contract. This ticket should not duplicate that domain with a synthetic sixth file; it should stay on the missing two-phase-specific golden proof.

## Architecture Check

1. Extending the existing owning golden files is cleaner than creating a duplicate S88-only golden suite because remote care, remote production, and reasoning diversity are already owned there.
2. No backwards-compatibility shims. This is entirely additive golden coverage over live planner-trace and profile-diversity surfaces.

## Verification Layers

1. Remote-care two-phase trace contract → `golden_care.rs`: existing remote-care golden asserts strategic itinerary, landmark counts, preferred-candidate/heuristic trace, and bounded per-expansion candidate counts
2. Remote-production two-phase trace contract → `golden_production.rs`: existing remote-production golden asserts strategic itinerary, landmark counts, preferred-candidate/heuristic trace, and bounded per-expansion candidate counts
3. Landmark-depth diversity and graceful degradation (FND-22) → `golden_reasoning_diversity.rs`: same remote-production belief setup with different `landmark_extraction_depth` values produces different trace profiles while zero-landmark mode still finds the lawful remote plan

## What to Change

### 1. Extend existing remote-care golden ownership

- `crates/worldwake-ai/tests/golden_care.rs`
- Augment `golden_healer_acquires_remote_ground_medicine_for_patient` (or an adjacent same-owner helper) to assert the tick-0 planning trace now includes:
  - non-empty `strategic_plan` with `ORCHARD_FARM` as the prerequisite destination
  - non-zero `landmarks_extracted`
  - at least one root or expansion summary with `preferred_candidates > 0` and `landmark_heuristic > 0`
  - bounded tactical branching (`candidates_generated < 100` for the relevant expansion summaries)

### 2. Extend existing remote-production golden ownership

- `crates/worldwake-ai/tests/golden_production.rs`
- Augment `golden_remote_acquire_commodity_recipe_input` (or an adjacent same-owner helper) to assert the tick-0 planning trace now includes:
  - non-empty `strategic_plan` with `ORCHARD_FARM` as the prerequisite destination
  - non-zero `landmarks_extracted`
  - at least one expansion summary with `preferred_candidates > 0` and `landmark_heuristic > 0`
  - bounded tactical branching (`candidates_generated < 100` for the relevant expansion summaries)

### 3. Add S88-specific reasoning-diversity proof

- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`
- Add a new remote-production planning scenario that keeps world state, beliefs, and seed fixed while varying `landmark_extraction_depth`
- Assert:
  - deeper landmark extraction produces a different planning trace profile than zero-landmark mode
  - zero-landmark mode still selects a lawful remote travel -> pickup -> craft plan
  - zero-landmark mode records `landmarks_extracted == 0`, zero `preferred_candidates`, and zero `landmark_heuristic`
  - deeper mode records non-zero landmark extraction and preferred-operator guidance

### 4. Reuse existing harness profile helpers

- Use the existing `golden_harness` profile setters instead of inventing a new golden file or duplicate setup layer
- Keep perception/profile setup explicit where the owning golden already depends on observed output

## Files to Touch

- `crates/worldwake-ai/tests/golden_care.rs`
- `crates/worldwake-ai/tests/golden_production.rs`
- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`

## Out of Scope

- CLI-level integration tests
- Observer diagnostic tests (deferred per spec non-goals)
- Dedicated no-omniscience/exploration scenarios already owned by the exploration/information-locality goldens
- Raw landmark-set/ordering algorithm proof already owned by focused planner tests
- Performance benchmarks with concrete timing thresholds (this is a correctness-focused spec, not a performance-optimization spec per se — the candidate count reduction is the metric, not wall-clock time)

## Acceptance Criteria

### Tests That Must Pass

1. Existing remote-care golden proves strategic plan + landmark trace fields are populated lawfully
2. Existing remote-production golden proves strategic plan + landmark trace fields are populated lawfully
3. New reasoning-diversity golden proves `landmark_extraction_depth` changes planning-trace profiles under identical world state
4. New/updated graceful-degradation golden proves zero-landmark mode still finds the lawful remote plan with zeroed landmark trace fields
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No golden test accesses world truth on behalf of an agent; all planning assertions are made through existing belief-seeded harness state and decision traces
2. Candidate-count assertions are taken from the live decision-trace contract, not from observer-only tooling
3. Deterministic: same seed produces same results (ChaCha8Rng)
4. Conservation is not violated by any touched scenario

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs` — additive two-phase planner-trace assertions on the existing remote-care golden
2. `crates/worldwake-ai/tests/golden_production.rs` — additive two-phase planner-trace assertions on the existing remote-production golden
3. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` — landmark-depth diversity and zero-landmark graceful-degradation golden coverage

### Commands

1. `cargo test -p worldwake-ai --test golden_care golden_healer_acquires_remote_ground_medicine_for_patient`
2. `cargo test -p worldwake-ai --test golden_production golden_remote_acquire_commodity_recipe_input`
3. `cargo test -p worldwake-ai --test golden_reasoning_diversity`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completion date: 2026-04-11

Implemented as additive coverage in the existing golden owners rather than a new `golden_two_phase_planning.rs` file. `crates/worldwake-ai/tests/golden_care.rs` and `crates/worldwake-ai/tests/golden_production.rs` now assert the live two-phase trace contract through the existing remote-care and remote-production scenarios: strategic itinerary presence, prerequisite destination shape, non-zero landmark extraction, non-zero preferred-candidate / landmark-heuristic guidance, and bounded per-expansion candidate counts. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` now adds a landmark-depth divergence scenario proving that `landmark_extraction_depth = 0` degrades gracefully to zero landmark guidance while preserving the lawful remote travel -> pickup -> craft plan, and that the default depth produces a different trace profile with positive landmark guidance.

One reassessment correction surfaced during implementation: the remote-production landmark-depth scenario lawfully extracts landmarks without requiring a non-zero `landmark_orderings` count, so the final golden proof asserts non-zero extraction and preferred guidance rather than forcing positive ordering count.

Verification passed with:
1. `cargo test -p worldwake-ai --test golden_care golden_healer_acquires_remote_ground_medicine_for_patient`
2. `cargo test -p worldwake-ai --test golden_production golden_remote_acquire_commodity_recipe_input`
3. `cargo test -p worldwake-ai --test golden_reasoning_diversity`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`
