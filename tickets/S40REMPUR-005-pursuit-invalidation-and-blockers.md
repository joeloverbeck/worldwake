# S40REMPUR-005: Pursuit invalidation and blocker semantics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Deps**: S40REMPUR-002 (pursuit_target_belief), S40REMPUR-004 (remote candidates exist)

## Problem

Once a remote pursuit plan is active, it must be invalidated when its underlying belief assumptions change: target believed elsewhere, confidence decays below threshold, target believed dead, or target no longer hostile. Without this, agents would pursue stale beliefs indefinitely, violating Principle 14 (World State Is Not Belief State) and Principle 21 (Intentions Are Revisable Commitments).

Additionally, arrival at a believed place where the target is absent must record `BlockingFact::TargetGone` in `BlockedIntentMemory` to suppress immediate re-pursuit at the same stale location. The blocker must be target/place scoped so a new belief about the target at a different place allows fresh pursuit.

## Assumption Reassessment (2026-03-30)

1. `plan_revalidation.rs` contains `revalidate_next_step()` which checks if the next planned step remains executable.
2. `interrupts.rs` contains `evaluate_interrupt()` for goal-switching decisions.
3. `failure_handling.rs` contains `derive_blocking_fact()` and `BlockingFact::TargetGone` (already exists).
4. `BlockedIntentMemory` in `blocked_intent.rs` stores `BlockedIntent` entries keyed by target/place.
5. `agent_tick/frame.rs` manages the per-tick decision loop including dirty-flag checks and replan triggers.
6. The spec requires four invalidation triggers:
   - Target's believed place changes → plan is dirty, replan
   - Confidence drops below `min_location_confidence` → pursuit dropped
   - Target believed dead → pursuit dropped
   - Target no longer hostile → pursuit dropped
7. `BlockedIntentMemory` is already target/place scoped — a blocker at old place must not suppress pursuit at new believed place. Need to verify this is the current scoping contract.
8. No adjacent contradictions exposed.

## Architecture Check

1. Using existing revalidation and interrupt infrastructure is cleaner than adding a parallel pursuit-specific invalidation system. The dirty-flag mechanism in `agent_tick` already handles belief-change-driven replans.
2. `BlockingFact::TargetGone` already exists — no new blocking fact variant needed. The arrival-failure case is an extension of the existing `TargetGone` path in `failure_handling.rs`.
3. No backwards-compatibility shims.

## Verification Layers

1. Belief-place-change invalidation → focused runtime test: change target's believed place mid-pursuit, verify replan triggered
2. Confidence decay invalidation → focused runtime test: advance ticks until staleness drops confidence below threshold, verify pursuit abandoned
3. Target-dead invalidation → focused runtime test: mark target dead in beliefs, verify pursuit dropped
4. Arrival failure → action trace: pursuer arrives, target absent, `TargetGone` recorded in `BlockedIntentMemory`
5. Blocker scoping → focused unit test: blocker at place A does not suppress pursuit to place B for same target
6. Cross-system: belief update (perception) → invalidation (AI tick) → replan (AI tick). All within AI boundary.

## What to Change

### 1. Extend revalidation to check pursuit belief freshness

In `plan_revalidation.rs` or `agent_tick/frame.rs` dirty-flag logic:
- When the active goal is `RaidTarget` or `EngageHostile` with a multi-step (Travel + Attack) plan, check if `pursuit_target_belief(view, actor, target)` still returns a belief for the same place the plan targets.
- If the believed place has changed, mark the plan dirty.
- If `pursuit_target_belief()` returns `None` (target dead, place unknown, or co-located), mark the plan dirty.

### 2. Add confidence-decay check during active pursuit

During each tick's revalidation for an active pursuit plan:
- Re-derive confidence via `belief_confidence(source, current_tick - observed_tick, policy)`.
- If below `PursuitProfile.min_location_confidence`, invalidate the pursuit plan.
- This naturally handles the "long pursuits self-limit" dampener from the spec's H.3 analysis.

### 3. Handle arrival failure in failure_handling.rs

When a pursuit plan's terminal `Attack` step fails because the target is not present at the believed place:
- `derive_blocking_fact()` should return `BlockingFact::TargetGone`.
- This records in `BlockedIntentMemory` with the specific target + place scope.
- Verify that `BlockedIntentMemory` scoping is target + place (not target-only), so pursuit to a different believed place is not blocked.

### 4. Ensure blocker does not suppress new-place pursuit

Verify and if necessary adjust `BlockedIntentMemory` lookup to check both target AND place, so a blocker for (target, place_A) does not suppress a candidate for (target, place_B).

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify) — belief-place-change and confidence-decay checks
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify) — dirty-flag integration for pursuit invalidation
- `crates/worldwake-ai/src/failure_handling.rs` (modify) — arrival-failure to `TargetGone` mapping
- `crates/worldwake-core/src/blocked_intent.rs` (verify/modify) — confirm target+place scoping

## Out of Scope

- Candidate generation (S40REMPUR-004 — already done)
- Decision trace extensions (S40REMPUR-006)
- Golden tests (S40REMPUR-007)
- New `BlockingFact` variants (spec says `TargetGone` suffices)
- Ranking/interrupt hierarchy changes (spec explicitly says no new priority class or interrupt role)
- Guard/justice pursuit invalidation (same mechanism, different goal kinds — future ticket)

## Acceptance Criteria

### Tests That Must Pass

1. Pursuit plan invalidates when believed target place changes.
2. Pursuit plan invalidates when derived confidence decays below `min_location_confidence` during multi-tick travel.
3. Pursuit plan drops when target believed dead.
4. Arrival at believed place without target records `BlockingFact::TargetGone` and triggers replanning.
5. Blocked target/place memory suppresses repeat pursuit only for that same believed place — pursuit to a different place is allowed.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No omniscient chase continuation — if target is absent at believed place, pursuit ends.
2. Confidence is re-derived each tick, never cached.
3. Existing ranking/interrupt hierarchies unchanged.
4. `BlockedIntentMemory` scoping is target + place, not target-only.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` (or test module) — `test_pursuit_invalidated_on_place_change`, `test_pursuit_invalidated_on_confidence_decay`
2. `crates/worldwake-ai/src/failure_handling.rs` (test module) — `test_arrival_failure_records_target_gone`
3. `crates/worldwake-ai/src/candidate_generation.rs` (test module) — `test_blocker_scoped_to_target_and_place`

### Commands

1. `cargo test -p worldwake-ai pursuit`
2. `cargo test -p worldwake-ai revalidat`
3. `cargo test -p worldwake-ai failure`
4. `cargo clippy --workspace && cargo test --workspace`
