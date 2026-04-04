# S45UNISOCART-007: Delivery-bounty planner integration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI planner progress/decomposition for delivery-target bounties
**Deps**: S45UNISOCART-003, S45UNISOCART-005

## Problem

`S45UNISOCART-005` corrects bounty AI pursuit to the elimination-bounty slice the live planner can support now. Delivery bounties remain a real world contract in the social-artifact substrate, but the current planner cannot yet treat "bring commodity to destination, satisfy the delivery condition, then travel to claim place and claim the bounty" as one lawful bounty goal family.

Without an explicit follow-up, delivery bounties would remain perceivable and claimable once already satisfied, but not generically pursuable through the same AI contract as elimination bounties.

## Assumption Reassessment (2026-04-04)

1. `claim_bounty` in [`artifact_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/artifact_actions.rs) already supports `BountyTarget::DeliverCommodity`, but only as a terminal authoritative claim after delivery is already true.
2. The current planner can model cargo transitions (`pick_up`, `put_down`, `store_stock`, etc.) through [`planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) and [`search/transition.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs), but no current `GoalKind` maps those commodity-flow transitions into "delivery bounty progress."
3. FOUNDATIONS alignment requires explicit causal progress rather than decorative goals. A delivery bounty should only become planner-visible once the planner can lawfully connect commodity movement/delivery state to later `claim_bounty`, not via a fake terminal or by assuming out-of-band satisfaction.
4. This follow-up therefore owns the missing planner substrate rather than treating delivery pursuit as "future cleanup."

## Architecture Check

1. This keeps delivery bounties on the canonical cargo and claim paths instead of inventing a bounty-specific delivery shortcut.
2. No backwards-compatibility shims or duplicate planner families should be introduced.

## Verification Layers

1. Delivery-bounty goal emission -> candidate-generation tests
2. Cargo/delivery progress toward bounty satisfaction -> goal-model and search tests
3. Terminal claim visibility after delivery -> search/root-synthesis tests
4. Workspace regression -> `cargo test --workspace`

## What to Change

### 1. Extend `FulfillBounty` planning to `DeliverCommodity`

Wire delivery-target bounty progress through the existing cargo transition and claim contract so the planner can lawfully reach `claim_bounty` only after the destination delivery condition is satisfied.

### 2. Add delivery-specific ranking and invalidation proof

Ensure delivery bounties rank and invalidate coherently under the same `FulfillBounty` goal family.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)

## Out of Scope

- New authoritative bounty actions
- Golden closeout beyond whichever active ticket owns it

## Acceptance Criteria

### Tests That Must Pass

1. Delivery-target active bounties can emit lawful AI candidates
2. Planner can find a lawful delivery-then-claim path without fake terminals
3. Existing suite: `cargo test --workspace`

### Invariants

1. Delivery bounty progress is expressed through existing cargo/world-state transitions, not hidden planner state
2. `claim_bounty` remains terminal and authoritative; this ticket only teaches the planner how to reach it lawfully

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — delivery-bounty candidate coverage
2. `crates/worldwake-ai/src/goal_model.rs` — delivery progress and satisfaction coverage
3. `crates/worldwake-ai/src/search/tests.rs` — end-to-end delivery-bounty search coverage

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai goal_model`
3. `cargo test -p worldwake-ai search`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
