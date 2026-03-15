# E16OFFSUCFAC-004: Add courage to UtilityProfile and New GoalKind Variants

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — UtilityProfile extension in core, GoalKind expansion
**Deps**: E16OFFSUCFAC-001

## Problem

E16 requires two extensions to existing core types:

1. **`courage: Permille`** on `UtilityProfile` — enables agent diversity in Threaten response. Two agents facing the same threat may react differently based on their courage value (Principle 20). Without this field, threaten actions cannot determine yield/resist outcomes.

2. **New `GoalKind` variants** — `ClaimOffice` and `SupportCandidateForOffice` must be added to the goal enum so the AI system can generate and plan toward office-related goals.

## Assumption Reassessment (2026-03-15)

1. `UtilityProfile` in `crates/worldwake-core/src/utility_profile.rs` currently has 9 `Permille` fields — confirmed.
2. `UtilityProfile::default()` sets balanced defaults — confirmed, `courage` default should be `Permille(500)`.
3. `GoalKind` in `crates/worldwake-core/src/goal.rs` currently has 15 variants — confirmed.
4. `GoalKind` uses `EntityId` and `CommodityKind` in variant payloads — confirmed, office variants will use `EntityId`.
5. Adding fields to `UtilityProfile` will affect all existing construction sites — must grep and update.
6. Adding `GoalKind` variants will require match arm updates in AI crate — those are addressed in E16OFFSUCFAC-009.

## Architecture Check

1. `courage` is a per-agent profile parameter, not a transient state — correct placement in `UtilityProfile`.
2. New `GoalKind` variants follow the existing pattern of carrying entity references as payloads.
3. No backward-compatibility shims — all construction sites must be updated.

## What to Change

### 1. Add `courage: Permille` to `UtilityProfile`

In `crates/worldwake-core/src/utility_profile.rs`:

```rust
pub struct UtilityProfile {
    // ... existing 9 fields ...
    pub courage: Permille,
}
```

Default: `Permille(500)` (moderate courage).

### 2. Update all `UtilityProfile` construction sites

Grep for `UtilityProfile {` and `UtilityProfile::default()` and update to include `courage`. This includes:
- Default impl
- Test helpers / test_utils
- CLI world setup
- Any prototype world builders

### 3. Add `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice`

In `crates/worldwake-core/src/goal.rs`:

```rust
pub enum GoalKind {
    // ... existing variants ...
    ClaimOffice { office: EntityId },
    SupportCandidateForOffice { office: EntityId, candidate: EntityId },
}
```

### 4. Update `GoalKind` match exhaustiveness

Any existing `match` on `GoalKind` in `worldwake-core` must handle the new variants (even if as `_ => unreachable!()` stubs for now — the AI-side handling is in E16OFFSUCFAC-009).

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify — add `courage` field and update default)
- `crates/worldwake-core/src/goal.rs` (modify — add 2 new variants)
- `crates/worldwake-core/src/test_utils.rs` (modify — update `UtilityProfile` construction if present)
- Any other core files that construct `UtilityProfile` (grep-determined)
- Downstream crates that match on `GoalKind` or construct `UtilityProfile` (update for exhaustiveness)

## Out of Scope

- Threaten action handler that reads `courage` (E16OFFSUCFAC-006)
- AI goal model mapping for new GoalKind variants (E16OFFSUCFAC-009)
- AI candidate generation for office goals (E16OFFSUCFAC-009)
- Planner ops for Bribe/Threaten/DeclareSupport (E16OFFSUCFAC-009)

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile::default().courage == Permille(500)`.
2. `UtilityProfile` with custom `courage` value roundtrips through bincode.
3. `GoalKind::ClaimOffice { office }` constructs and matches.
4. `GoalKind::SupportCandidateForOffice { office, candidate }` constructs and matches.
5. New `GoalKind` variants roundtrip through bincode serialization.
6. All existing `UtilityProfile` construction sites compile with new field.
7. All existing `GoalKind` matches are exhaustive with new variants.
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. `courage` uses `Permille` — no floats.
2. Default `courage` is `Permille(500)` — moderate, not extreme.
3. No existing `UtilityProfile` field defaults change.
4. No existing `GoalKind` variant semantics change.
5. Determinism preserved — `Permille` is integer-based.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — test `courage` field presence, default value, serde roundtrip.
2. `crates/worldwake-core/src/goal.rs` — test new variant construction and serialization.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
