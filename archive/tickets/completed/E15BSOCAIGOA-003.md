# E15BSOCAIGOA-003: Add social_weight to UtilityProfile

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — component struct in core crate
**Deps**: E15 (completed)

## Problem

Agents have no tunable parameter for social motivation. The `UtilityProfile` has 8 weights (hunger, thirst, fatigue, bladder, dirtiness, pain, danger, enterprise) but none for social behavior. `GoalKind::ShareBelief` and `PlannerOpKind::Tell` already exist in the codebase, but the utility layer still cannot express "this agent is chatty" versus "this agent is taciturn." Without `social_weight`, social-goal ranking cannot vary per agent, which weakens Principle 20 (agent diversity).

## Assumption Reassessment (2026-03-15)

1. `UtilityProfile` in `crates/worldwake-core/src/utility_profile.rs` still has exactly 8 `Permille` fields. Confirmed no `social_weight`.
2. The default constructor currently sets every existing weight to `Permille(500)`. `social_weight` should default to `Permille(200)` so social goals stay below the baseline enterprise/self-maintenance drives unless later ranking logic explicitly promotes them.
3. `UtilityProfile` already implements `Component`, `Serialize`, and `Deserialize`. Adding a field is an intentional binary-format break, which is acceptable here because the project explicitly rejects compatibility shims and current save/load uses version-unstable bincode.
4. The original ticket assumption that `GoalKind::ShareBelief`, `GoalKindTag::ShareBelief`, and `PlannerOpKind::Tell` were still pending is stale. Those are already implemented in `goal.rs`, `goal_model.rs`, and `planner_ops.rs`.
5. The original ticket assumption that ranking support for `ShareBelief` was entirely future work is also stale. `ranking.rs` already suppresses `ShareBelief` under high danger/self-care pressure and assigns it `Low` priority with a placeholder motive score.
6. The real remaining gap is narrower than originally written: add the per-agent social utility field and update all explicit `UtilityProfile { ... }` construction sites and tests so the codebase compiles and serialization/component invariants remain covered.

## Architecture Check

1. Pure field addition to an existing struct. This matches the current architecture, which models stable per-agent motive weights explicitly as `Permille` fields rather than through generic maps or ad hoc per-system knobs.
2. Default value of `Permille(200)` is intentionally low. That keeps the base architecture conservative: social behavior is expressible and differentiable without letting a default agent chatter ahead of survival, injury response, or core enterprise loops.
3. This is cleaner than overloading `TellProfile` for motivation. `TellProfile` should remain about communication affordances and filtering (`max_tell_candidates`, relay depth, acceptance), while `UtilityProfile` remains the place for "how much does this agent care."
4. No backwards-compatibility shim needed. Broken callers and serialized fixtures should be updated directly per Principle 26.

## What to Change

### 1. Add social_weight field to UtilityProfile

In `crates/worldwake-core/src/utility_profile.rs`, add:
```rust
pub social_weight: Permille,
```

### 2. Update Default impl

Set `social_weight: Permille(200)` in the Default impl.

### 3. Update all construction sites

Find every place that constructs a `UtilityProfile` directly and add the `social_weight` field. This includes authoritative test fixtures and AI/golden tests with explicit struct literals. Use `Permille(200)` unless a test is intentionally exercising social variation.

### 4. Strengthen utility-profile tests

Extend the existing `utility_profile.rs` tests to cover:
- default `social_weight`
- bincode round-trip with a non-default `social_weight`
- the invariant that `social_weight` defaults below `enterprise_weight`

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify)
- `crates/worldwake-core/src/test_utils.rs` (modify sample fixture)
- Any file with an explicit `UtilityProfile { ... }` literal, especially:
  - `crates/worldwake-ai/src/goal_explanation.rs`
  - `crates/worldwake-ai/src/ranking.rs`
  - `crates/worldwake-ai/tests/golden_ai_decisions.rs`
  - `crates/worldwake-ai/tests/golden_production.rs`

## Out of Scope

- Implementing or changing candidate generation for social goals (`emit_social_candidates()`)
- Revising `ShareBelief` ranking logic beyond making the new field available to later work
- GoalKind or PlannerOpKind changes (already present in the codebase)
- CLI display of social_weight (future work)
- TellProfile changes (existing struct, unchanged)

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile::default().social_weight == Permille(200)`
2. `UtilityProfile::default().social_weight < UtilityProfile::default().enterprise_weight`
3. UtilityProfile with custom `social_weight` round-trips through bincode/serde
4. Existing targeted suites covering touched call sites compile and pass
5. Final verification: `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. social_weight uses Permille (never f32/f64) per spec drafting rules
2. social_weight default (200) is strictly less than enterprise_weight default (500)
3. UtilityProfile remains `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
4. `UtilityProfile` remains the single authoritative home for stable per-agent motive weights; no parallel social-motivation knob is introduced elsewhere

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` (inline tests) — default value, ordering invariant, custom round-trip
2. Any AI/core tests that require `UtilityProfile { ... }` literal updates to compile after the new field addition

### Commands

1. `cargo test -p worldwake-core utility_profile`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- Actual changes:
  - Added `social_weight: Permille` to `UtilityProfile` with a default of `Permille(200)`.
  - Updated authoritative and test fixtures that construct `UtilityProfile` directly.
  - Strengthened `utility_profile.rs` tests to cover the new default and serialization behavior.
  - Updated the CLI scenario RON fixture to include `social_weight`, which was required once the struct shape changed.
- Deviations from original plan:
  - No production `ShareBelief`/`Tell`/goal-model work was needed here; those assumptions were stale and were corrected in the ticket before implementation.
  - The real fallout was broader than the original ticket claimed because direct `UtilityProfile` literals and serialized scenario fixtures also needed updating.
- Verification results:
  - `cargo test -p worldwake-core utility_profile`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-cli test_scenario_def_deserialize_full -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
