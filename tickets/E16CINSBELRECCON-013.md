# E16CINSBELRECCON-013: Ranking Adjustments + Failure Handling for Institutional Beliefs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extend ranking, failure handling, and blocked intent in worldwake-ai/core
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-009, E16CINSBELRECCON-011, E16CINSBELRECCON-012

## Problem

The ranking system must account for institutional belief certainty when scoring political goals, and the failure handling system must recognize stale/conflicted institutional beliefs as specific failure types. Without ranking adjustments, ConsultRecord goals would have no meaningful priority. Without failure handling, agents whose plans break due to stale institutional knowledge would get stuck instead of replanning.

## Assumption Reassessment (2026-03-21)

1. `ranking.rs` in worldwake-ai scores goals by priority class and motive value. Political goals (ClaimOffice, SupportCandidateForOffice) are already ranked. ConsultRecord needs its own priority assignment.
2. `failure_handling.rs` contains `PlanFailureContext` and `handle_plan_failure()`. It updates `BlockedIntentMemory` with `BlockingFact` variants. No institutional belief variants exist yet.
3. `BlockingFact` in `blocked_intent.rs` (worldwake-core) currently has 14 variants (NoKnownPath through Unknown). Must add `InstitutionalBeliefStale` and `InstitutionalBeliefConflicted`.
4. Spec §Phase B2 says: "Ranking reduces motive for Conflicted beliefs" and "Failure handling adds BlockingFact::InstitutionalBeliefStale / InstitutionalBeliefConflicted".
5. N/A — ordering between ConsultRecord and political goals is by priority class, not a custom tiebreaker.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Extending existing ranking and failure handling systems is the right approach — no new systems needed. BlockingFact variants are the standard way to communicate failure reasons.
2. No backward-compatibility shims.

## Verification Layers

1. ConsultRecord ranked at appropriate priority → ranking unit test
2. Political goal motive reduced when institutional belief is Conflicted → ranking unit test
3. Plan failure due to stale belief produces `BlockingFact::InstitutionalBeliefStale` → failure handling test
4. Plan failure due to conflicted belief produces `BlockingFact::InstitutionalBeliefConflicted` → failure handling test
5. Blocked intent with institutional belief facts has appropriate expiration → blocked intent test

## What to Change

### 1. Add `BlockingFact` variants in `blocked_intent.rs` (worldwake-core)

```rust
InstitutionalBeliefStale,
InstitutionalBeliefConflicted,
```

Update any exhaustive match blocks and test helpers.

### 2. ConsultRecord ranking in `ranking.rs`

Assign `ConsultRecord` goals a priority class below survival needs but above idle/enterprise goals. The motive value should reflect the urgency of the institutional knowledge gap (e.g., higher when a political action is blocked by Unknown belief).

### 3. Conflict-based motive reduction in `ranking.rs`

When a political goal's relevant institutional belief is `Conflicted`:
- Reduce the motive score (agent is uncertain and should be reluctant to commit)
- The reduction factor can be influenced by `PerceptionProfile.contradiction_tolerance`

### 4. Failure handling in `failure_handling.rs`

When `handle_plan_failure()` detects that a plan failed because:
- An institutional precondition was checked and the belief was stale → record `BlockingFact::InstitutionalBeliefStale` with appropriate blocking period
- An institutional precondition was checked and the belief was conflicted → record `BlockingFact::InstitutionalBeliefConflicted`

The blocking period for stale beliefs should be shorter (encourages re-consultation), while conflicted beliefs should have a longer period (encourages alternative actions).

## Files to Touch

- `crates/worldwake-core/src/blocked_intent.rs` (modify — add two `BlockingFact` variants)
- `crates/worldwake-ai/src/ranking.rs` (modify — ConsultRecord priority, Conflicted motive reduction)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — detect institutional belief failures, record appropriate BlockingFact)

## Out of Scope

- Candidate generation (ticket -012 — must already exist)
- PlannerOpKind::ConsultRecord (ticket -011 — must already exist)
- PlanningSnapshot/PlanningState (ticket -010 — must already exist)
- Live helper seam removal (ticket -014)
- Golden test updates (ticket -014)

## Acceptance Criteria

### Tests That Must Pass

1. `BlockingFact::InstitutionalBeliefStale` and `InstitutionalBeliefConflicted` roundtrip through bincode
2. `ConsultRecord` goal is ranked below survival needs but above idle enterprise goals
3. Political goal motive is reduced when institutional belief is Conflicted
4. Higher `contradiction_tolerance` results in less motive reduction for Conflicted beliefs
5. Plan failure from stale institutional belief records `InstitutionalBeliefStale` in `BlockedIntentMemory`
6. Plan failure from conflicted institutional belief records `InstitutionalBeliefConflicted`
7. `InstitutionalBeliefStale` has shorter blocking period than `InstitutionalBeliefConflicted`
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `BlockingFact` enum remains exhaustive in all match arms across the workspace
2. ConsultRecord ranking does not interfere with survival goal priorities
3. Conflicted motive reduction does not reduce political goals to zero (agent can still act under contradiction if tolerance is high)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocked_intent.rs` — roundtrip for new variants
2. `crates/worldwake-ai/src/ranking.rs` — ConsultRecord priority class, Conflicted motive reduction
3. `crates/worldwake-ai/src/failure_handling.rs` — stale/conflicted failure detection and BlockingFact recording

### Commands

1. `cargo test -p worldwake-core blocked_intent`
2. `cargo test -p worldwake-ai ranking`
3. `cargo test -p worldwake-ai failure_handling`
4. `cargo clippy --workspace && cargo test --workspace`
