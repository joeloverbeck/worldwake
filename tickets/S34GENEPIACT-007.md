# S34GENEPIACT-007: Ranking — verification_motive(), priority class, GoalFamilyPolicy, provenance

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai: ranking additions for VerifyBelief goal family
**Deps**: [archive/tickets/completed/S34GENEPIACT-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-006.md) (completed candidate emission), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

`VerifyBelief` still has an incomplete ranking surface. The family already has policy and low-priority-class wiring, but `motive_score()` still hardcodes `GoalKind::VerifyBelief` to `0`. Once ticket 006 starts emitting candidates, they will be filtered out by the zero-motive path instead of participating in ranking through `VerificationDispositionProfile`.

## Assumption Reassessment (2026-03-28)

1. `ranking.rs` at `crates/worldwake-ai/src/ranking.rs` (~3205 lines) contains `rank_candidates()`, `goal_ranking_provenance()`, and motive scoring functions. The `InvestigateViolation` ranking at `goal_ranking_provenance()` returns provenance, and `investigation_motive()` computes motive from `ViolationDispositionProfile::investigation_motive_weight`. This is the exact pattern for `verification_motive()`.
2. The spec says `GoalPriorityClass::Low` for `VerifyBelief` — same as `InvestigateViolation`. That policy is already live in both `goal_policy.rs` and `ranking.rs`, so this ticket must not re-add it.
3. The spec says motive score comes from `VerificationDispositionProfile::verification_motive_weight`. Per-agent variation in this weight creates diversity (P20).
4. `GoalFamilyPolicy` in `crates/worldwake-ai/src/goal_policy.rs` already includes `GoalKind::VerifyBelief { .. }` under the low-priority suppressed-under-stress family. The architectural gap is motive/provenance, not family registration.
5. `goal_ranking_provenance()` and `motive_score()` in `crates/worldwake-ai/src/ranking.rs` still leave `VerifyBelief` effectively inert: the goal is classified as `Low`, but `motive_score()` returns `0`, and there is no dedicated verification provenance path.
6. The zero-motive filter in `rank_candidates()` is still the right contract. After this ticket, `VerifyBelief` should participate only when the agent’s `VerificationDispositionProfile` supplies positive motive weight; lack of profile or zero weight should continue to suppress the goal, preserving P20 diversity in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md).
7. Mismatch + correction: the original ticket claimed it needed to add a `GoalFamilyPolicy` entry and priority-class assignment. Those are already delivered. The remaining work is motive scoring, provenance, and focused coverage proving that emitted `VerifyBelief` candidates are not silently zeroed out.
8. Follow-on confirmation after ticket 006 completion: [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) now emits `VerifyBelief` candidates, but [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) still returns `0` for `GoalKind::VerifyBelief` in `motive_score()`. This ticket is now the sole active owner of making emitted verification candidates selectable through the normal ranking path.

## Architecture Check

1. The clean fix is to complete the existing ranking path rather than bolting on special exceptions in candidate generation or planner search. `VerifyBelief` should rank the same way other goal families do: policy class from family policy, motive from concrete per-agent profile state, and provenance from explicit ranking inputs.
2. No backwards-compatibility shims. This ticket extends existing ranking dispatch instead of adding a “verification goals bypass zero-motive filter” special case.

## Verification Layers

1. `VerifyBelief` receives non-zero motive from `VerificationDispositionProfile` instead of the current hardcoded zero -> focused ranking test
2. Zero-motive filter still suppresses `VerifyBelief` when profile weight is zero or absent -> focused ranking test
3. `VerifyBelief` remains `GoalPriorityClass::Low` and never preempts higher-priority survival families -> focused ranking test
4. Ranking provenance for `VerifyBelief` is explicit and profile-driven rather than falling through a generic or empty path -> focused ranking test
5. Single-layer ranking ticket. Planner closure belongs to ticket 005 and candidate emission belongs to ticket 006.

## What to Change

### 1. Add verification motive scoring

In `crates/worldwake-ai/src/ranking.rs`, add:

```rust
fn verification_motive(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32 {
    let Some(profile) = context.view.verification_disposition_profile(context.agent) else {
        return 0;
    };
    u32::from(profile.verification_motive_weight.value())
}
```

### 2. Wire into goal_ranking_provenance()

Add `GoalKind::VerifyBelief { .. }` arm in `goal_ranking_provenance()` to return appropriate provenance and wire to `verification_motive()`.

### 3. Wire into motive_score()

Add the `VerifyBelief` dispatch in `motive_score()` to call `verification_motive()`.

### 4. Add focused coverage for the already-live policy contract

Do not change `GoalFamilyPolicy` unless reassessment finds a real bug. Instead, add focused tests proving that:

- `VerifyBelief` remains `GoalPriorityClass::Low`
- non-zero verification motive survives the ranking path
- zero or missing motive still suppresses the goal

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — add verification motive/provenance, tests)
- `crates/worldwake-ai/src/goal_policy.rs` (modify only if reassessment finds a real policy bug; otherwise test-only or no change)

## Out of Scope

- Candidate generation — ticket 006
- Planner ops and goal model — ticket 005
- Action handlers — tickets 003/004
- Golden E2E tests — ticket 008
- Changes to `GoalPriorityClass` enum or `GoalFamilyPolicy` unless reassessment finds a live contradiction
- Interrupt/goal-switching logic for VerifyBelief (the low-priority family contract is already live)
- Feasibility sketching dispatch for VerifyBelief (already exists in `feasibility.rs`)

## Acceptance Criteria

### Tests That Must Pass

1. `verification_motive()` returns `verification_motive_weight` from the agent’s `VerificationDispositionProfile`
2. `verification_motive()` returns `0` when the profile is absent or weight is zero
3. `VerifyBelief` is suppressed by the zero-motive filter when motive is `0`
4. `VerifyBelief` remains `GoalPriorityClass::Low`
5. Two `VerifyBelief` candidates with different motive weights rank correctly relative to each other
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `GoalPriorityClass::Low` remains fixed for `VerifyBelief`
2. Ranking reads belief/profile state only
3. Zero-motive filter still applies to `VerifyBelief`
4. No candidate-generation or planner special case is needed to make verification goals rank

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused tests for verification motive, zero-motive suppression, low priority class, and relative ordering
2. `crates/worldwake-ai/src/goal_policy.rs` — no change unless reassessment finds a live policy mismatch

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo build --workspace`
