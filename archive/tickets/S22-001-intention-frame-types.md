# S22-001: Define IntentionFrame and IntentionDispositionProfile types in worldwake-core

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component registration (component_schema, component_tables), new blocked_intent variants
**Deps**: S21 (JourneyCommitment must exist as component), S23 (BlockedIntentMemory compound-keyed system)

## Problem

S22 requires new generalized types (`IntentionFrame`, `IntentionDispositionProfile`, supporting enums) to replace travel-specific `JourneyCommitment` and `TravelDispositionProfile`. This ticket adds the types and component registrations without removing the old types — removal happens in S22-002. This two-phase approach keeps the workspace buildable at every ticket boundary.

## Assumption Reassessment (2026-03-24)

1. `JourneyCommitment` currently exists in `crates/worldwake-core/src/intention.rs` with fields: `committed_goal`, `destination`, `state`, `established_at`, `last_progress_tick`, `consecutive_blocked_leg_ticks`. Confirmed via grep.
2. `TravelDispositionProfile` currently exists in `crates/worldwake-core/src/travel_disposition.rs` with fields: `route_replan_margin`, `blocked_leg_patience_ticks`. Confirmed via grep.
3. `BlockingFact` enum currently has 14 variants in `crates/worldwake-core/src/blocked_intent.rs`. New variants `PatienceExhausted` and `AssumptionFailed` must be added.
4. Component registration uses macro-generated storage in `component_tables.rs` and schema in `component_schema.rs`. Both files need new entries for `IntentionFrame` and `IntentionDispositionProfile`.
5. This is a type-definition ticket — no AI pipeline changes. Single-layer verification (build + focused unit tests on new types).

## Architecture Check

1. Adding new types alongside old types (not replacing yet) avoids a massive cross-crate change in one ticket. The old types are removed in S22-002 once all consumers migrate.
2. No backward-compatibility aliasing — the new types are entirely fresh, not wrappers around old ones.

## Verification Layers

1. `IntentionFrame` component registered on Agent entities → `cargo build --workspace` (compilation proof)
2. `IntentionDispositionProfile` component registered on Agent entities → `cargo build --workspace`
3. `IntentionDomain::domain_tag()` returns correct discriminant → focused unit test
4. `IntentionDispositionProfile::patience_for()` fallback logic → focused unit test
5. `BlockingFact::PatienceExhausted` and `AssumptionFailed` have `blocks_goal_generation() == true` → focused unit test
6. Single-layer ticket: no AI pipeline integration, no golden test changes.

## What to Change

### 1. New module: `intention_frame.rs` in worldwake-core

Add `IntentionFrame`, `IntentionDomain`, `IntentionDomainTag`, `FrameAssumption`, `FrameState`, `SuspensionReason`, `FrameClearReason` with all derives per spec. Add `IntentionDomain::domain_tag()` method.

### 2. New module: `intention_disposition.rs` in worldwake-core

Add `IntentionDispositionProfile` struct with `domain_patience: BTreeMap<IntentionDomainTag, NonZeroU32>`, `default_patience_ticks: NonZeroU32`, `commitment_switch_margin: Permille`. Add `patience_for()` helper method.

### 3. Component registration

Register `IntentionFrame` and `IntentionDispositionProfile` as Agent components in `component_schema.rs` and `component_tables.rs`. Add get/set/remove accessors following existing patterns.

### 4. BlockingFact variants

Add `PatienceExhausted` and `AssumptionFailed` to `BlockingFact` in `blocked_intent.rs`. Both must return `true` from `blocks_goal_generation()`.

### 5. Module declarations and re-exports

Update `crates/worldwake-core/src/lib.rs` to declare `intention_frame` and `intention_disposition` modules and re-export all public types.

## Files to Touch

- `crates/worldwake-core/src/intention_frame.rs` (new)
- `crates/worldwake-core/src/intention_disposition.rs` (new)
- `crates/worldwake-core/src/blocked_intent.rs` (modify — add 2 variants to `BlockingFact`)
- `crates/worldwake-core/src/component_schema.rs` (modify — register 2 new components)
- `crates/worldwake-core/src/component_tables.rs` (modify — register 2 new components)
- `crates/worldwake-core/src/lib.rs` (modify — add module declarations, re-exports)
- `crates/worldwake-core/src/delta.rs` (modify — add `ComponentDelta` variants for new components if needed)
- `crates/worldwake-core/src/world.rs` (modify — add component accessors if not macro-generated)
- `crates/worldwake-core/src/world_txn.rs` (modify — add transactional accessors if not macro-generated)

## Out of Scope

- Removing `JourneyCommitment`, `JourneyCommitmentState`, or `TravelDispositionProfile` (S22-002)
- Any changes to worldwake-ai or worldwake-sim (S22-002+)
- `FramePlanRelation` enum (lives in worldwake-ai, added in S22-002)
- BeliefView changes (S22-002)
- Assumption evaluation logic (S22-003)
- Progress detection logic (S22-004)
- Decision trace integration (S22-006)
- Golden test updates (S22-002+)

## Acceptance Criteria

### Tests That Must Pass

1. Focused unit test: `IntentionDomain::Travel { .. }.domain_tag() == IntentionDomainTag::Travel` (and all other variants)
2. Focused unit test: `IntentionDispositionProfile::patience_for()` returns domain-specific value when present, default when absent
3. Focused unit test: `BlockingFact::PatienceExhausted.blocks_goal_generation() == true`
4. Focused unit test: `BlockingFact::AssumptionFailed.blocks_goal_generation() == true`
5. `cargo build --workspace` succeeds
6. `cargo clippy --workspace` — no new warnings
7. All existing tests: `cargo test --workspace`

### Invariants

1. `IntentionFrame` and `IntentionDispositionProfile` are registered on `EntityKind::Agent` only
2. All new types derive `Serialize, Deserialize` for save/load compatibility
3. `IntentionDomain`, `IntentionDomainTag`, `FrameAssumption` derive `Ord, PartialOrd` for deterministic ordering
4. `IntentionDomainTag` derives `Hash` for `BTreeMap` key use
5. No `HashMap` or `HashSet` used in any new type (determinism invariant)
6. Existing `JourneyCommitment` and `TravelDispositionProfile` remain untouched and functional

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/intention_frame.rs` (inline `#[cfg(test)]` module) — domain_tag mapping, FrameState transitions, derive coverage
2. `crates/worldwake-core/src/intention_disposition.rs` (inline `#[cfg(test)]` module) — patience_for fallback, BTreeMap lookup
3. `crates/worldwake-core/src/blocked_intent.rs` (existing test module) — add assertions for new `BlockingFact` variants

### Commands

1. `cargo test -p worldwake-core`
2. `cargo build --workspace && cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-24
- **What changed**:
  - Created `crates/worldwake-core/src/intention_frame.rs` — `IntentionFrame`, `IntentionDomain`, `IntentionDomainTag`, `FrameAssumption`, `FrameState`, `SuspensionReason`, `FrameClearReason`
  - Created `crates/worldwake-core/src/intention_disposition.rs` — `IntentionDispositionProfile` with `patience_for()` helper
  - Added `PatienceExhausted` and `AssumptionFailed` variants to `BlockingFact` in `blocked_intent.rs`
  - Registered both new components on `EntityKind::Agent` in `component_schema.rs`
  - Updated `component_tables.rs`, `delta.rs`, `world.rs`, `lib.rs` with imports and re-exports
  - Updated `failure_handling.rs` in worldwake-ai to handle new `BlockingFact` variants (structural TTL, not auto-resolvable)
- **Deviations**: None. The worldwake-ai `failure_handling.rs` match arms needed updating for exhaustiveness — this was not listed in the ticket's "Files to Touch" but was a necessary consequence of adding `BlockingFact` variants.
- **Verification**: `cargo build --workspace`, `cargo clippy --workspace`, `cargo test --workspace` — all 766+ tests pass, 0 failures, 0 clippy warnings.
