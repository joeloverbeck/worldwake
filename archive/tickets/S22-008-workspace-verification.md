# S22-008: Workspace verification and orphan cleanup

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — verification and cleanup only
**Deps**: S22-001 through S22-007 (all prior tickets complete)

## Problem

Final verification that the S22 migration is complete: no orphaned references to old journey types remain, all tests pass, clippy is clean, and deterministic replay produces consistent hashes. This is the gate ticket that confirms S22 is shippable.

## Assumption Reassessment (2026-03-24)

1. The old types to verify are fully removed: `JourneyCommitment`, `JourneyCommitmentState`, `JourneyPlanRelation`, `JourneyClearReason`, `JourneySwitchMarginSource`, `JourneyDebugSnapshot`, `JourneyRuntimeSnapshot`, `TravelDispositionProfile`.
2. `golden_deterministic_replay_fidelity` in `golden_determinism.rs` verifies replay hash consistency.
3. Archived specs/tickets under `archive/` may still reference old types — that is expected and not an error.
4. This is a verification-only ticket. No production code changes unless orphaned references are found (which would indicate an incomplete prior ticket).

## Architecture Check

1. Gate verification is standard practice for multi-ticket migrations. Catching orphans here is cheaper than discovering them in future tickets.
2. If orphaned references are found, they must be fixed in the appropriate prior ticket (S22-002 most likely), not patched here.

## Verification Layers

1. No orphaned journey references → grep verification
2. Full workspace builds → `cargo build --workspace`
3. Full workspace tests → `cargo test --workspace`
4. Clean clippy → `cargo clippy --workspace`
5. Deterministic replay → `golden_deterministic_replay_fidelity` test
6. Multi-layer gate ticket: covers build, test, lint, determinism, and orphan detection.

## What to Change

### 1. Grep for orphaned references

Run comprehensive grep across all non-archived Rust source for any remaining references to the 8 removed types. If found, fix them (should not happen if S22-002 was complete).

### 2. Full verification battery

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace`
- Verify `golden_deterministic_replay_fidelity` passes

### 3. Spec status update

Update `specs/S22-generalized-intention-frames.md` status from PENDING to COMPLETED.

## Files to Touch

- `specs/S22-generalized-intention-frames.md` (modify — update status to COMPLETED)
- Any files with orphaned references (modify — fix if found, but should be zero)

## Out of Scope

- New feature work beyond S22's scope
- Archiving old tickets (follows separate archival workflow)
- Changes to IMPLEMENTATION-ORDER.md (separate maintenance task)
- Performance optimization of frame lifecycle

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all pass (zero failures)
2. `cargo clippy --workspace` — no warnings
3. `golden_deterministic_replay_fidelity` — deterministic replay produces identical hashes
4. All golden tests in `golden_ai_decisions.rs` pass
5. All golden tests in `golden_determinism.rs` pass

### Invariants

1. Zero references to `JourneyCommitment`, `JourneyCommitmentState`, `JourneyPlanRelation`, `JourneyClearReason`, `JourneySwitchMarginSource`, `JourneyDebugSnapshot`, `JourneyRuntimeSnapshot`, or `TravelDispositionProfile` in non-archived source files
2. Deterministic replay is hash-stable (no non-determinism introduced by migration)
3. Spec S22 status is COMPLETED

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `grep -r "JourneyCommitment\|JourneyCommitmentState\|JourneyPlanRelation\|JourneyClearReason\|JourneySwitchMarginSource\|JourneyDebugSnapshot\|JourneyRuntimeSnapshot\|TravelDispositionProfile" crates/ --include="*.rs"` — must return empty
2. `cargo build --workspace`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-24
- **What changed**: Spec S22 status updated to COMPLETED. No production code changes needed.
- **Deviations**: None. All 8 old types confirmed absent from non-archived source (two doc-comment references explain what the new types replaced — not orphans).
- **Verification results**: `cargo build --workspace` clean, `cargo test --workspace` all pass (0 failures), `cargo clippy --workspace` no warnings, `golden_deterministic_replay_fidelity` passes with stable hashes.
