# E15BSOCAIGOA-003: Add social_weight to UtilityProfile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — component struct in core crate
**Deps**: E15 (completed)

## Problem

Agents have no tunable parameter for social motivation. The `UtilityProfile` has 8 weights (hunger, thirst, fatigue, bladder, dirtiness, pain, danger, enterprise) but none for social behavior. Without `social_weight`, all agents would share identical social motivation, violating Principle 20 (agent diversity).

## Assumption Reassessment (2026-03-15)

1. `UtilityProfile` in `crates/worldwake-core/src/utility_profile.rs` has exactly 8 `Permille` fields. Confirmed no social_weight.
2. Default constructor sets all weights to `Permille(500)`. social_weight default should be `Permille(200)` (low priority, below enterprise_weight).
3. UtilityProfile implements `Component`, `Serialize`, `Deserialize`. Adding a field is a breaking change for serialized data — but save/load uses bincode which is not forward-compatible anyway, and no production saves exist.
4. `UtilityProfile` is used in `ranking.rs` for motive score calculation — the ranking ticket (E15BSOCAIGOA-005) will consume this field.

## Architecture Check

1. Pure field addition to an existing struct. Follows the same `Permille` pattern as all other weights.
2. Default value of `Permille(200)` is intentionally low — social goals should never outrank survival.
3. No backwards-compatibility shim needed — binary format is not stable across versions.

## What to Change

### 1. Add social_weight field to UtilityProfile

In `crates/worldwake-core/src/utility_profile.rs`, add:
```rust
pub social_weight: Permille,
```

### 2. Update Default impl

Set `social_weight: Permille(200)` in the Default impl.

### 3. Update all construction sites

Find every place that constructs a `UtilityProfile` (test helpers, golden harness, CLI setup) and add the `social_weight` field. Use `Permille(200)` for default agents.

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — `seed_agent` helper constructs UtilityProfile)
- Any other test files that construct UtilityProfile directly (find via compiler errors after field addition)

## Out of Scope

- Ranking logic that consumes social_weight (E15BSOCAIGOA-005)
- Candidate generation (E15BSOCAIGOA-004)
- GoalKind or PlannerOpKind changes (E15BSOCAIGOA-001, E15BSOCAIGOA-002)
- CLI display of social_weight (future work)
- TellProfile changes (existing struct, unchanged)

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile::default().social_weight == Permille(200)`
2. UtilityProfile with custom social_weight round-trips through serde
3. Existing suite: `cargo test --workspace` — no regressions (all construction sites updated)

### Invariants

1. social_weight uses Permille (never f32/f64) per spec drafting rules
2. social_weight default (200) is strictly less than enterprise_weight default (500)
3. UtilityProfile remains `Copy + Clone + Debug + Eq + PartialEq + Serialize + Deserialize`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` (inline tests) — default value, serde round-trip

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
