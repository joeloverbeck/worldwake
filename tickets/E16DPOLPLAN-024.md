# E16DPOLPLAN-024: Support-aware `is_satisfied` for `ClaimOffice` and conditional `ProgressBarrier`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — goal_model.rs
**Deps**: E16DPOLPLAN-023

## Problem

`is_satisfied` for `ClaimOffice` always returns `false` (goal_model.rs:645), and `is_progress_barrier` unconditionally returns `true` for `ClaimOffice + DeclareSupport` (goal_model.rs:547-553). This causes the GOAP search to always terminate with a 1-step DeclareSupport ProgressBarrier plan, even when the agent faces competition and needs to build a coalition via Bribe/Threaten first.

The real succession system (`resolve_support_succession` in offices.rs:137-167) installs the candidate with the **most** support declarations, resets the vacancy clock on ties, and does nothing without any declarations. The planner must model this competitive dynamic so it naturally selects coalition-building steps when needed.

## Assumption Reassessment (2026-03-18)

1. `is_satisfied` for `ClaimOffice` returns `false` unconditionally — confirmed (goal_model.rs:645).
2. `is_progress_barrier` returns `true` for `ClaimOffice + DeclareSupport` unconditionally — confirmed (goal_model.rs:547-553).
3. `terminal_kind` in search.rs checks `is_satisfied` BEFORE `is_progress_barrier` — confirmed (search.rs:622-628). GoalSatisfied takes precedence.
4. `PlanningState::has_support_majority(office, candidate)` will be available from E16DPOLPLAN-023.
5. `apply_planner_step` for DeclareSupport under ClaimOffice already writes `with_support_declaration(actor, office, actor)` — confirmed (goal_model.rs:501-502). The planning state after DeclareSupport reflects the actor's self-declaration.
6. `apply_planner_step` for Bribe writes `with_support_declaration(target, office, actor)` — confirmed (E16DPOLPLAN-005 implementation).

## Architecture Check

1. **Principle 3 (Concrete State Over Abstract Scores)**: `is_satisfied` checks actual support declaration counts, not an abstract "political strength" score. The agent wins when they have more supporters — a direct mirror of `resolve_support_succession`.
2. **Principle 18 (Resource-Bounded Practical Reasoning)**: The planner now reasons about whether to invest in coalition-building based on the competitive landscape visible through beliefs. An agent with no competitors self-declares. An agent with competitors bribes or threatens to build majority.
3. **Principle 20 (Agent Diversity)**: Different agents choose different strategies — wealthy agents bribe, strong agents threaten, agents facing weak opposition just self-declare.
4. **Principle 10 (Physical Dampeners)**: Coalition-building is dampened by commodity costs (bribes), courage thresholds (threats), and planning budget limits. No new numeric caps.
5. The ProgressBarrier fallback ensures the planner always produces a plan for ClaimOffice, even when it can't determine a winning coalition. This mirrors the real world: an agent may declare support even without a guaranteed majority, contributing to a tie that resets the vacancy clock.
6. No backwards-compatibility shims.

## What to Change

### 1. Replace `ClaimOffice` in `is_satisfied` (goal_model.rs)

Move `ClaimOffice` out of the catch-all `false` arm at line 641-645 and give it a proper implementation:

```rust
GoalKind::ClaimOffice { office } => {
    state.has_support_majority(*office, actor)
}
```

After a DeclareSupport step, the planning state has the actor's self-declaration. If no competitor has any declarations, `has_support_majority` returns true (1 > 0) → GoalSatisfied. If competitors have equal or more support, it returns false → search continues exploring Bribe/Threaten paths.

### 2. Keep `ClaimOffice + DeclareSupport` as ProgressBarrier (goal_model.rs:547-553)

**Do not remove this.** It serves as a critical fallback. When the planner cannot build a winning coalition (no bribable targets, no threatenable targets, competitors match support), the DeclareSupport ProgressBarrier lets the agent "declare and replan" rather than producing no plan at all.

The key interaction:
- `terminal_kind` checks `is_satisfied` first → if true, GoalSatisfied takes precedence
- If false, falls through to `is_progress_barrier` → ProgressBarrier as fallback
- E16DPOLPLAN-025 changes the search to prefer GoalSatisfied over ProgressBarrier across expansion levels

### 3. Update `SupportCandidateForOffice` check consistency

The `is_progress_barrier` block at line 547-553 covers both `ClaimOffice` and `SupportCandidateForOffice`. `SupportCandidateForOffice` already has a working `is_satisfied` (line 638-640). Verify that the new behavior doesn't regress SupportCandidateForOffice plans — the ProgressBarrier should still be subsumed by GoalSatisfied for that goal kind (since it already works).

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — `is_satisfied` for ClaimOffice)

## Out of Scope

- Changes to search termination logic (E16DPOLPLAN-025)
- Changes to PlanningSnapshot/PlanningState (E16DPOLPLAN-023)
- Changes to belief view (E16DPOLPLAN-022)
- Integration tests (E16DPOLPLAN-006, updated after this chain completes)

## Acceptance Criteria

### Tests That Must Pass

1. `is_satisfied` returns `true` for ClaimOffice when actor has support majority (uncontested: 1 self-declaration, 0 competitors)
2. `is_satisfied` returns `false` for ClaimOffice when tied (actor 1, competitor 1)
3. `is_satisfied` returns `false` for ClaimOffice when actor trails (actor 1, competitor 2)
4. `is_satisfied` returns `true` for ClaimOffice when actor leads after Bribe (actor 2 via self + bribed target, competitor 1)
5. `is_progress_barrier` still returns `true` for `ClaimOffice + DeclareSupport` (unchanged, serves as fallback)
6. `SupportCandidateForOffice` behavior unchanged
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. For uncontested offices, the planner produces a 1-step DeclareSupport GoalSatisfied plan (equivalent to current behavior but with correct terminal kind)
2. ProgressBarrier remains available as a fallback — ClaimOffice always produces SOME plan
3. `is_satisfied` evaluation is purely derived from hypothetical support counts

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — test `is_satisfied` for ClaimOffice with various support configurations
2. `crates/worldwake-ai/src/goal_model.rs` — regression test: `is_satisfied` for SupportCandidateForOffice unchanged

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
