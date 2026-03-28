# S34GENEPIACT-007: Ranking — verification_motive(), priority class, GoalFamilyPolicy, provenance

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai: ranking additions for VerifyBelief goal family
**Deps**: S34GENEPIACT-005 (GoalKindTag::VerifyBelief), S34GENEPIACT-006 (candidates emitted)

## Problem

`VerifyBelief` candidates exist but have no ranking logic. Without a `GoalFamilyPolicy` entry, priority class assignment, and motive scoring function, verification goals cannot be ranked against other goals and will either fail ranking or receive zero motive scores.

## Assumption Reassessment (2026-03-28)

1. `ranking.rs` at `crates/worldwake-ai/src/ranking.rs` (~3205 lines) contains `rank_candidates()`, `goal_ranking_provenance()`, and motive scoring functions. The `InvestigateViolation` ranking at `goal_ranking_provenance()` returns provenance, and `investigation_motive()` computes motive from `ViolationDispositionProfile::investigation_motive_weight`. This is the exact pattern for `verification_motive()`.
2. The spec says `GoalPriorityClass::Low` for `VerifyBelief` — same as `InvestigateViolation`. This is fixed to prevent priority inversion.
3. The spec says motive score comes from `VerificationDispositionProfile::verification_motive_weight`. Per-agent variation in this weight creates diversity (P20).
4. `GoalFamilyPolicy` is in `crates/worldwake-ai/src/goal_policy.rs` (from S02). Each goal family has a policy entry controlling suppression and priority class. `VerifyBelief` needs an entry.
5. `goal_ranking_provenance()` pattern-matches on `candidate.key.kind` to determine motive driver. `VerifyBelief` should return an appropriate provenance variant.
6. The zero-motive filter in `rank_candidates()` will filter out `VerifyBelief` if `verification_motive_weight` is zero or the agent lacks the profile. This is correct behavior (agent without profile never verifies — P20 diversity).

## Architecture Check

1. Following the `InvestigateViolation` pattern exactly — `GoalPriorityClass::Low`, profile-driven motive weight, new `verification_motive()` function. Minimal, clean addition.
2. No backward-compatibility shims. Extends existing ranking dispatch.

## Verification Layers

1. VerifyBelief gets GoalPriorityClass::Low -> focused ranking test
2. verification_motive() returns profile's verification_motive_weight -> focused ranking test
3. Zero-motive filter suppresses VerifyBelief when agent lacks profile -> focused ranking test
4. VerifyBelief never preempts Critical survival goals -> focused ranking test (priority class comparison)
5. Single-layer ticket (ranking only). No cross-layer mapping needed beyond ranking assertions.

## What to Change

### 1. Add GoalFamilyPolicy entry for VerifyBelief

In `crates/worldwake-ai/src/goal_policy.rs`, add the `VerifyBelief` family entry:
- Priority class: `GoalPriorityClass::Low` (fixed, per spec)
- Suppression: Standard Low-priority suppression rules (suppressed when higher-priority goals are active)

### 2. Add verification_motive() function

In `crates/worldwake-ai/src/ranking.rs`, add:

```rust
fn verification_motive(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32 {
    let Some(profile) = context.view.verification_disposition_profile(context.agent) else {
        return 0;
    };
    u32::from(profile.verification_motive_weight.value())
}
```

### 3. Wire into goal_ranking_provenance()

Add `GoalKind::VerifyBelief { .. }` arm in `goal_ranking_provenance()` to return appropriate provenance and wire to `verification_motive()`.

### 4. Wire into motive_score()

Add the `VerifyBelief` dispatch in `motive_score()` to call `verification_motive()`.

### 5. Update GoalFamilyPolicy exhaustiveness

Ensure `GoalKindTag::VerifyBelief` is covered in all policy dispatch sites.

## Files to Touch

- `crates/worldwake-ai/src/goal_policy.rs` (modify — add VerifyBelief family entry)
- `crates/worldwake-ai/src/ranking.rs` (modify — add verification_motive(), wire into provenance and motive dispatch)

## Out of Scope

- Candidate generation — ticket 006
- Planner ops and goal model — ticket 005
- Action handlers — tickets 003/004
- Golden E2E tests — ticket 008
- Changes to `GoalPriorityClass` enum
- Interrupt/goal-switching logic for VerifyBelief (Low priority class means it follows standard Low interruption rules — no special logic needed)
- Feasibility sketching dispatch for VerifyBelief (S25's per-GoalKind dispatch table needs a `VerifyBelief` arm — add it here if the compiler requires it, otherwise defer)

## Acceptance Criteria

### Tests That Must Pass

1. `VerifyBelief` candidates receive `GoalPriorityClass::Low`
2. `verification_motive()` returns `verification_motive_weight` value from agent's `VerificationDispositionProfile`
3. `verification_motive()` returns 0 when agent lacks `VerificationDispositionProfile`
4. `VerifyBelief` is suppressed by zero-motive filter when motive is 0
5. `VerifyBelief` never receives priority class above Low (no priority inversion)
6. Two `VerifyBelief` candidates with different motive weights rank correctly relative to each other
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `GoalPriorityClass::Low` is fixed for VerifyBelief — cannot be promoted to Medium/High/Critical
2. Ranking reads belief state only (P12)
3. `GoalFamilyPolicy` exhaustiveness — all match arms cover VerifyBelief (compiler-enforced)
4. Zero-motive filter applies to VerifyBelief (prevents agents without profiles from ranking verification goals)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (in-module tests) — 4 focused ranking tests: priority class, motive scoring, zero-motive filter, relative ordering
2. `crates/worldwake-ai/src/goal_policy.rs` (in-module tests if applicable) — policy entry existence

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
3. `cargo build --workspace`
