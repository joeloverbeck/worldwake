# E16OFFSUCFAC-004: Add courage to UtilityProfile and Political GoalKind Variants

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — UtilityProfile extension in core, GoalKind expansion
**Deps**: E16OFFSUCFAC-001

## Problem

The remaining core work for this slice of E16 is narrower than the epic originally implied:

1. **`courage: Permille`** on `UtilityProfile` — enables agent diversity in Threaten response. Two agents facing the same threat may react differently based on their courage value (Principle 20). Without this field, the planned threaten resolution has no per-agent scalar to vary yield vs resist behavior.

2. **New `GoalKind` variants** — `ClaimOffice` and `SupportCandidateForOffice` must be added to the shared goal enum so the AI layer can generate and plan toward office-related goals.

Office entities, succession data, faction data, and support-declaration relations are already present in `worldwake-core`; this ticket must not re-specify or re-implement them.

## Assumption Reassessment (2026-03-15)

1. `UtilityProfile` in `crates/worldwake-core/src/utility_profile.rs` currently has 9 `Permille` fields and is used as a stable per-agent disposition component — confirmed.
2. `UtilityProfile::default()` currently sets balanced `500` defaults for most weights and `social_weight = 200` — confirmed. `courage` should default to `Permille(500)` to preserve the current moderate baseline.
3. `GoalKind` in `crates/worldwake-core/src/goal.rs` currently has 15 variants — confirmed.
4. `GoalKey::from(GoalKind)` in core performs canonical target extraction and therefore must be updated when adding goal variants — confirmed.
5. `GoalKind` exhaustiveness is mirrored in `worldwake-ai` (`goal_model.rs`, `ranking.rs`, `goal_explanation.rs`, `failure_handling.rs`, and related tests), so adding variants here necessarily causes downstream compile fixes even though the richer AI behavior remains in E16OFFSUCFAC-009.
6. `OfficeData`, `SuccessionLaw`, `EligibilityRule`, `FactionData`, and `support_declarations` already exist in core — confirmed. They are out of scope for this ticket.
7. Explicit `UtilityProfile { ... }` construction sites are limited and grep-discoverable; most call sites use `..UtilityProfile::default()` or `UtilityProfile::default()` already.

## Architecture Check

1. `courage` is a stable per-agent parameter, not a transient state.
2. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` belong in `worldwake-core`, because goals are shared identity types across authoritative memory and AI planning.
3. `courage` is slightly orthogonal to the current meaning of `UtilityProfile`, which today mostly models motive weights. A dedicated social/coercion disposition component would be a cleaner long-term home if more non-utility social traits appear.
4. For this ticket, keep `courage` in `UtilityProfile` to stay aligned with the active E16 spec and to avoid spreading a one-field architectural fork across multiple dependent tickets. If this area grows, replace the profile cleanly rather than adding aliases or compatibility layers.
5. No backward-compatibility shims — all construction sites and exhaustive matches must be updated directly.

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

### 2. Update all explicit `UtilityProfile` construction sites

Grep for `UtilityProfile {` and update every explicit literal to include `courage`. Most `UtilityProfile::default()` call sites do not need source edits because the default implementation carries the new field automatically.

Expected touch points include:
- `crates/worldwake-core/src/test_utils.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/goal_explanation.rs`
- golden tests and other focused test fixtures that build `UtilityProfile` literals

### 3. Add `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice`

In `crates/worldwake-core/src/goal.rs`:

```rust
pub enum GoalKind {
    // ... existing variants ...
    ClaimOffice { office: EntityId },
    SupportCandidateForOffice { office: EntityId, candidate: EntityId },
}
```

### 4. Update `GoalKind` canonicalization and compile exhaustiveness

- Update `GoalKey::from(GoalKind)` in core so the new variants have stable canonical identity.
- Apply the minimum compile-driven exhaustiveness fixes in downstream crates that match on `GoalKind`.
- Do **not** implement political candidate generation, planner semantics, or full motive logic here; richer AI behavior remains in E16OFFSUCFAC-009.

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify — add `courage` field, update default, strengthen tests)
- `crates/worldwake-core/src/goal.rs` (modify — add 2 new variants, update `GoalKey`, strengthen tests)
- `crates/worldwake-core/src/test_utils.rs` (modify — update sample `UtilityProfile`)
- `crates/worldwake-ai/src/` files that must compile after `GoalKind` expansion (minimum exhaustiveness updates only)
- Focused AI/core tests that build explicit `UtilityProfile` literals

## Out of Scope

- Threaten action handler that reads `courage` (E16OFFSUCFAC-006)
- New social/coercion profile extraction or broader disposition refactor
- AI candidate generation for office goals (E16OFFSUCFAC-009)
- Planner ops for Bribe/Threaten/DeclareSupport (E16OFFSUCFAC-009)
- Full political goal ranking semantics beyond the minimum compile-safe placeholder behavior required by the new enum variants

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile::default().courage == Permille(500)`.
2. `UtilityProfile` with custom `courage` value roundtrips through bincode.
3. `GoalKind::ClaimOffice { office }` and `GoalKind::SupportCandidateForOffice { office, candidate }` construct, match, and roundtrip through bincode.
4. `GoalKey::from(GoalKind)` gives stable canonical identity for the new political goal kinds.
5. All explicit `UtilityProfile` construction sites compile with the new field.
6. All affected `GoalKind` matches compile after the enum expansion.
7. `cargo test -p worldwake-core`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo test --workspace`

### Invariants

1. `courage` uses `Permille` — no floats.
2. Default `courage` is `Permille(500)` — moderate, not extreme.
3. No existing `UtilityProfile` field defaults change.
4. Existing `GoalKind` semantics remain unchanged.
5. Political goal identity stays entity-based and deterministic.
6. No compatibility aliases, shadow enums, or duplicate goal representations are introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — test `courage` default and serde roundtrip.
2. `crates/worldwake-core/src/goal.rs` — test political goal construction, serialization, and `GoalKey` canonicalization.
3. Focused `worldwake-ai` tests only where the enum expansion requires explicit fixture or exhaustiveness updates.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added `courage: Permille` to `UtilityProfile` with a default of `Permille(500)`.
  - Added `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` to core.
  - Extended `GoalKey::from(GoalKind)` so the new political goals have deterministic canonical identity.
  - Updated explicit `UtilityProfile` fixtures and scenario RON coverage to include the new field.
  - Applied the minimum downstream `worldwake-ai` exhaustiveness updates required by the enum expansion, without implementing the richer political planning behavior reserved for E16OFFSUCFAC-009.
- Deviations from original plan:
  - The ticket was corrected before implementation because office/faction/support-declaration infrastructure already existed in core.
  - Downstream AI changes were slightly broader than the original draft implied: `GoalKindTag` needed matching placeholder variants so the shared goal enum could remain exhaustively modeled without aliasing.
  - No planner-op, candidate-generation, or political motive logic was implemented here.
- Verification results:
  - `cargo test -p worldwake-core` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_deserialize_full` passed after updating the explicit RON fixture.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
