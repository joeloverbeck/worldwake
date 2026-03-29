# S35OBSACTSIG-002: Add `activity_awareness_weight` to `UtilityProfile`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core UtilityProfile
**Deps**: None (parallel with S35OBSACTSIG-001)

## Problem

There is no per-agent parameter controlling how much observed competition influences goal ranking. The spec requires `activity_awareness_weight: Permille` on `UtilityProfile` to implement P20 (Agent Diversity) — agents should differ in how much they care about competition.

## Assumption Reassessment (2026-03-29)

1. `UtilityProfile` is defined at `crates/worldwake-core/src/utility_profile.rs:8` with 11 `Permille` fields (hunger, thirst, fatigue, bladder, dirtiness, pain, danger, enterprise, social, courage, care weights). No `activity_awareness_weight` exists.
2. `UtilityProfile` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
3. `Default` impl is at the same file — sets balanced defaults (most at 500, social/care at 200).
4. `UtilityProfile` is read by `rank_candidates()` in `worldwake-ai/src/ranking.rs` for motive score computation.
5. `UtilityProfile` is constructed in test harnesses and golden tests. All construction sites must be updated.
6. No ranking-sensitive changes — this ticket only adds a field with a default, no behavioral change yet.

## Architecture Check

1. Adding a `Permille` field to an existing profile struct is the cleanest approach — follows the established pattern of per-agent utility weights.
2. `#[serde(default)]` ensures backward compatibility without shims. Default `Permille(200)` per spec means 20% discount per competitor.
3. No alternative considered — this is the only sensible location per P20.

## Verification Layers

1. Field existence and default value -> focused unit test
2. Serialization round-trip -> focused unit test
3. Single-layer ticket: data definition only, no behavioral change

## What to Change

### 1. Add field to `UtilityProfile`

In `crates/worldwake-core/src/utility_profile.rs`, add:
```rust
#[serde(default = "default_activity_awareness_weight")]
pub activity_awareness_weight: Permille,
```
With helper: `fn default_activity_awareness_weight() -> Permille { Permille(200) }`

Update `Default` impl to include `activity_awareness_weight: Permille(200)`.

### 2. Update all explicit `UtilityProfile` construction sites

Search for all struct-literal constructions of `UtilityProfile` across the workspace and add `activity_awareness_weight: Permille(200)` (or a test-specific value).

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify — add field and default)
- All test files constructing `UtilityProfile` explicitly (modify — add field)

## Out of Scope

- Ranking discount logic using this weight (S35OBSACTSIG-006)
- Perception changes (S35OBSACTSIG-003)
- `BelievedActivity` type (S35OBSACTSIG-001)
- Golden tests exercising diversity (S35OBSACTSIG-007)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `UtilityProfile::default().activity_awareness_weight == Permille(200)`.
2. Unit test: `UtilityProfile` with `activity_awareness_weight: Permille(0)` constructs correctly (zero-awareness agent).
3. Existing suite: `cargo test --workspace`

### Invariants

1. `UtilityProfile::default()` returns `activity_awareness_weight: Permille(200)`.
2. All existing explicit constructions compile with the new field.
3. Old serialized `UtilityProfile` data deserializes cleanly via `#[serde(default)]`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — unit test for default value and explicit construction.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
