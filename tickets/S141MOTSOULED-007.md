# S141MOTSOULED-007: Conformance tests, `golden_motive_sources`, ProfileHomogeneity lint extension

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No new engine state — adds test coverage and extends an existing lint
**Deps**: 002 (reads new `UtilityProfile` fields), 004 (exercises `motive_score` refactor end-to-end), 005 (exercises `decisive_motive_sources` payload end-to-end)

## Problem

S141's validation deliverable (D8) closes the contract:
1. Conformance: every emitted `GoalOffer` carries non-empty `motive_sources`; every new `UtilityProfile` motive-class weight has a `#[serde(default)]` helper.
2. Behavioral goldens: five scenarios that exercise per-class scoring, profile-driven variation, observer rendering, and the empty-vec debug assertion.
3. Per-agent diversity: `ProfileHomogeneity` lint (per S111) extends to detect cloned values across the 5 new `UtilityProfile` weight fields.

Without these, the spec's "no silent privilege" + "FND-22 diversity" + FND-28 "no fallback path" promises are unenforced and would erode the next time someone adds an emitter without populating motive sources.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing conformance test precedents: `crates/worldwake-ai/tests/planner_conformance.rs` (workspace-wide planner conformance) and `crates/worldwake-ai/tests/conformance_execution_budget.rs` (planner budget conformance). This ticket adds a sibling file `crates/worldwake-ai/tests/conformance_motive_sources.rs` following the same pattern: spawn a representative scenario, run the planner, assert the invariant across all `GoalOffer`s constructed during the run.
2. `ProfileHomogeneity` lint lives at `crates/worldwake-cli/src/scenario/lints.rs:62-93` and reads 8 agent profile fields via the `option_field_varies()` helper (lines 95–109). The lint currently does NOT inspect `UtilityProfile` subfields — extension scope is field-by-field within the existing `UtilityProfile` comparison.
3. Existing 1440-tick survival goldens at `crates/worldwake-ai/tests/golden_survival_*.rs` already enforce the score-parity gate from 004 — this ticket does NOT duplicate that coverage. New goldens in `golden_motive_sources.rs` exercise behavior that the parity goldens don't cover (Hunger + Greed sum, Pain dominance under wound profile, per-agent profile divergence, empty-vec debug panic).
4. Shared abstraction boundary: this is a (b)+(d) hybrid — production deliverables (lint extension, conformance assertions) plus new test coverage. Per spec D8 the lint inhabits the same "validation surface" as the goldens, so co-locating them in one ticket preserves the spec's organization. Per the precision-rules Coverage Gap Classification rule, the gaps closed here are:
   - **Missing focused/unit coverage**: per-class scoring weights uniformly default-able (`utility_profile_default_for_motive_class`).
   - **Missing runtime trace/integration coverage**: `every_goal_offer_has_motive_sources` (full action registries needed because the planner runs over real emitter coverage, not a needs-only harness).
   - **Missing golden/E2E coverage**: 5 new scenarios in `golden_motive_sources.rs` exercising per-class behavior + observer rendering.

## Architecture Check

1. The conformance test enforces FND-28 (no fallback path post-S141) by failing CI if a future emitter forgets to populate `motive_sources` — the `debug_assert!` in test builds (from 004) catches per-construction-site mistakes; the conformance test catches the "test build forgot to enable debug assertions" loophole.
2. The lint extension enforces FND-22 (agent diversity) by flagging scenario authors whose agent population clones the same motive-class weights across all agents — preserves the spec's "two agents with identical state but different `greed_weight` rank the same opportunity differently" promise.
3. The 5 new goldens validate semantic claims of the spec (per-class summation, dominance under wound profile, profile-driven divergence, debug assertion firing, observer rendering). Each golden corresponds to a spec D8 scenario item.

## Verification Layers

1. Conformance: `every_goal_offer_has_motive_sources` → runtime trace coverage. Boundary: full action registries (not needs-only harness) because the planner must exercise every emitter family.
2. Conformance: `utility_profile_default_for_motive_class` → focused unit coverage in the conformance file (no scenario needed).
3. Behavioral correctness: 5 `golden_motive_sources.rs` scenarios → golden E2E coverage. Each scenario maps one spec D8 bullet:
   - Scenario 1 (Hunger-only commit, score parity) → existing-1440-tick-goldens-as-strict-parity (verified by 004) reused here as a single-tick assertion that the motive_source vec carries exactly one `NeedPressure(Hunger)` and the contribution score matches `motive_score`.
   - Scenario 2 (Hunger + Greed sum) → asserts two motive sources, sum equals score, observer rendering contains both.
   - Scenario 3 (Pain dominates Hunger under wound profile) → asserts `Pain(...)` contribution > `NeedPressure(Hunger)` contribution at scoring time.
   - Scenario 4 (Per-agent `greed_weight` variation) → two agents with identical state but different `greed_weight` produce different commit choices for the same opportunity.
   - Scenario 5 (Empty-motive-sources panic in test build) → `#[should_panic]` test asserting the debug assertion from 004 fires.
4. Lint coverage → focused unit test in `crates/worldwake-cli/src/scenario/lints.rs#[cfg(test)]` constructing two agents with identical new-field values and asserting the lint flags them; constructing two agents with different new-field values and asserting the lint passes.
5. Per `docs/precision-rules.md` Rule 8 (scenario isolation): each golden documents the lawful competing affordances intentionally removed from setup to isolate the intended branch (e.g., scenario 3 removes hunger-relief affordances near the wounded agent so the Pain motive isn't masked by Hunger satisfaction).

## What to Change

### 1. New conformance test file `crates/worldwake-ai/tests/conformance_motive_sources.rs`

Two tests:
- `every_goal_offer_has_motive_sources` — spawn a representative scenario (use the same fixture as `planner_conformance.rs`), run the planner for a fixed number of ticks, intercept every `GoalOffer` construction via the agenda manager surface (or by reading `AgendaEntry.offer` from the decision-runtime per the Read-Only Tooling Consumer pattern), assert each one has non-empty `motive_sources`. Use full action registries (the conformance scope requires emitter coverage across all goal families).
- `utility_profile_default_for_motive_class` — focused unit assertion that each of the 5 new `UtilityProfile` weight fields has a `#[serde(default = "...")]` helper that returns a non-zero `Permille`. Implemented as 5 direct assertions on `UtilityProfile::default()` returning the spec D4 default values.

### 2. New golden test file `crates/worldwake-ai/tests/golden_motive_sources.rs`

Five scenarios as named above. Each scenario is a small focused harness similar to existing `golden_*.rs` files: a custom `WorldBuilder` setup, a fixed tick count, and assertions against the decision-trace + event-log output. Scenarios 2 and 5 also assert observer rendering (scenario 2: motive-source lines appear in Section 3b output; scenario 5: `#[should_panic]` on the synthetic empty-vec construction).

For score-parity (Scenario 1) the assertion compares the per-`MotiveSource`-variant contribution sum to the rendered `motive_score` aggregate — the same value, computed two ways, must match.

### 3. Extend `ProfileHomogeneity` lint at `crates/worldwake-cli/src/scenario/lints.rs:62`

The existing lint checks 8 top-level agent profile fields via `option_field_varies()`. Add a per-`UtilityProfile`-field sub-check that drills into `utility_profile` and tests each of the 5 new fields (`office_duty_weight`, `loyalty_weight`, `greed_weight`, `shame_weight`, `revenge_weight`) for variation across the agent population. Mirror the existing per-field structure: one helper invocation per new field.

Add focused unit tests in the same file (`scenario/lints.rs#[cfg(test)]`):
- `profile_homogeneity_flags_cloned_greed_weights` — two agents with identical `greed_weight`, lint flags.
- `profile_homogeneity_passes_when_greed_weights_vary` — two agents with different `greed_weight`, lint passes.
- One sibling pair for each of the 5 new fields (10 tests total, or one parameterized test covering all 5).

## Files to Touch

- `crates/worldwake-ai/tests/conformance_motive_sources.rs` (new)
- `crates/worldwake-ai/tests/golden_motive_sources.rs` (new)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — extend `ProfileHomogeneity` lint + focused tests)

## Out of Scope

- `GoalOffer.motive_sources` field, `motive_score` body refactor, mapping helper — owned by 004 (must land first).
- `UtilityProfile` 5 new fields, `Default` impl, `#[serde(default)]` helpers — owned by 002 (must land first).
- `GoalCommittedPayload.decisive_motive_sources` and commit-time emission — owned by 005 (must land first).
- Observer Section 3b rendering — owned by 006 (must land first for Scenario 2's observer assertion to work).
- Score parity across existing 1440-tick survival goldens — owned by 004 (validated by `cargo test --workspace`; this ticket does not duplicate that coverage).
- Goldens for the 5 deferred `MotiveSource` variants (`Fear`, `Obligation`, `Debt`, `Habit`, `Curiosity`) — Phase 12 follow-ups; can't be authored until the substrates exist.

## Acceptance Criteria

### Tests That Must Pass

1. `every_goal_offer_has_motive_sources` (new conformance) — passes on the representative scenario; would fail if any emitter is added in the future without populating motive sources.
2. `utility_profile_default_for_motive_class` (new conformance) — 5 assertions, one per new field, all pass.
3. 5 new goldens in `golden_motive_sources.rs` — pass per the scenario assertions above.
4. `ProfileHomogeneity` lint focused tests — flag cloned weights, pass when weights vary.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. The conformance file is the workspace-wide enforcement of S141's "non-empty motive_sources" contract — any future emitter that forgets to populate must fail this test.
2. The lint covers exactly 5 new UtilityProfile fields — extension scope is fully aligned with 002's field additions; no field added in 002 is missed by the lint.
3. Per `docs/precision-rules.md` Rule 8: each golden documents its isolation choice. Scenario 3 (Pain dominance) explicitly removes hunger-relief affordances near the wounded agent in the scenario setup, with a comment naming the excluded lawful branch.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/conformance_motive_sources.rs` (new) — 2 conformance tests.
2. `crates/worldwake-ai/tests/golden_motive_sources.rs` (new) — 5 scenario tests.
3. `crates/worldwake-cli/src/scenario/lints.rs#[cfg(test)]` — focused tests covering the lint extension (one pair per new field, or a parameterized form).

### Commands

1. `cargo test -p worldwake-ai --test conformance_motive_sources`
2. `cargo test -p worldwake-ai --test golden_motive_sources`
3. `cargo test -p worldwake-cli scenario::lints`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh` before push.
