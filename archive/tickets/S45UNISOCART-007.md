# S45UNISOCART-007: Delivery-bounty planner integration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI planner progress/decomposition for delivery-target bounties
**Deps**: S45UNISOCART-003, S45UNISOCART-005

## Problem

`S45UNISOCART-005` corrects bounty AI pursuit to the elimination-bounty slice the live planner can support now. Delivery bounties remain a real world contract in the social-artifact substrate, but the current planner cannot yet treat "bring commodity to destination, satisfy the delivery condition, then travel to claim place and claim the bounty" as one lawful bounty goal family.

Without an explicit follow-up, delivery bounties would remain perceivable and claimable once already satisfied, but not generically pursuable through the same AI contract as elimination bounties.

## Assumption Reassessment (2026-04-04)

1. `claim_bounty` in [`artifact_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/artifact_actions.rs) already supports `BountyTarget::DeliverCommodity`, but only as a terminal authoritative claim after delivery is already true.
2. The current planner already models cargo transitions (`pick_up`, `put_down`, `store_stock`, `collect_display_stock`, etc.) through [`planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), but `GoalKind::FulfillBounty` still advertises the elimination-only op family in [`goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs): `Travel`, `Attack`, `ClaimBounty`.
3. The deeper live blocker is in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), not ranking or invalidation. `PlannerOpKind::MoveCargo` payload shaping is currently implemented only for `GoalKind::MoveCargo` and depends on `restock_gap_at_destination(...)`, which is merchant-demand-specific. Delivery bounties need their own quantity-gap helper tied to `BountyTarget::DeliverCommodity`.
4. Reward ranking in [`ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) and invalidation in [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) are already target-agnostic across `FulfillBounty`; this ticket mainly needs proof updates there, not a new architecture branch.
5. FOUNDATIONS alignment requires explicit causal progress rather than decorative goals. Delivery bounty pursuit must be expressed through known controlled cargo and lawful cargo movement toward `destination`, then the normal terminal `claim_bounty` step. This ticket does not invent a hidden “delivery complete” flag or a separate bounty-only cargo system.
6. Correction applied: this ticket owns delivery-bounty planner integration for known controlled cargo and the normal claim terminal, not a broader commodity-acquisition pipeline from trade or crafting. No remaining active S45 ticket claims that broader acquisition expansion.

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

Wire delivery-target bounty progress through the existing cargo transition and claim contract so the planner can lawfully reach `claim_bounty` only after the destination delivery condition is satisfied. This includes:
- delivery-bounty candidate emission when the agent has enough known controlled cargo to satisfy the bounty
- `FulfillBounty` relevant-op expansion to the cargo ops it actually needs (`MoveCargo`, plus `StockManagement` where facility custody is involved)
- delivery-specific quantity-gap and payload shaping for `PlannerOpKind::MoveCargo`
- delivery-side relevant-place / observed-commodity semantics so search can use the cargo substrate honestly

### 2. Add delivery-specific ranking and invalidation proof

Ensure delivery bounties rank and invalidate coherently under the same `FulfillBounty` goal family. The live ranking and invalidation logic is already shared; this slice mainly adds focused proof that delivery targets follow the same contract.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)

## Out of Scope

- New authoritative bounty actions
- Golden closeout beyond whichever active ticket owns it

## Acceptance Criteria

### Tests That Must Pass

1. Delivery-target active bounties can emit lawful AI candidates when the agent has enough known controlled cargo to satisfy the bounty
2. Planner can find a lawful delivery-then-claim path through the existing cargo transition substrate without fake terminals
3. Existing suite: `cargo test --workspace`

### Invariants

1. Delivery bounty progress is expressed through existing cargo/world-state transitions and known controlled cargo, not hidden planner state
2. `claim_bounty` remains terminal and authoritative; this ticket only teaches the planner how to reach it lawfully

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — delivery-bounty candidate coverage
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` and `crates/worldwake-ai/src/goal_model.rs` — delivery-specific relevant-op, delivery-gap, and satisfaction coverage
3. `crates/worldwake-ai/src/search/tests.rs` — end-to-end delivery-bounty search coverage
4. `crates/worldwake-ai/src/ranking.rs` and `crates/worldwake-ai/src/exhaustion.rs` — target-agnostic proof updates for delivery variants

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai goal_model`
3. `cargo test -p worldwake-ai search`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

Completed: 2026-04-05

- Extended delivery-bounty `FulfillBounty` planning through the existing cargo substrate in `candidate_generation.rs`, `goal_dispatch_decl.rs`, and `goal_model.rs`.
- Added delivery-specific cargo-gap and pickup payload shaping so `MoveCargo` progress is tied to `BountyTarget::DeliverCommodity` instead of merchant restock helpers.
- Tightened planner-root admission in `goal_model.rs` and `search/candidates.rs` so delivery bounties do not leak elimination-side `Attack` candidates and do not surface `claim_bounty` before the delivery gap is closed.
- Added focused proof updates in `search/tests.rs`, `ranking.rs`, and `exhaustion.rs`, plus the bounded decision-trace fallout in `decision_trace.rs`.

Deviations from original plan:

- The ticket landed on the known-controlled-cargo boundary described in reassessment; it does not add a broader acquisition pipeline for unsatisfied delivery bounties.
- A late search failure exposed a second planner-contract mismatch: exact-bound `claim_bounty` binding was already correct, but delivery variants still needed stateful root-candidate availability gating and subtype-specific operator filtering.

Verification:

- `cargo test -p worldwake-ai candidate_generation -- --nocapture`
- `cargo test -p worldwake-ai goal_model -- --nocapture`
- `cargo test -p worldwake-ai search -- --nocapture`
- `cargo test -p worldwake-ai ranking -- --nocapture`
- `cargo test -p worldwake-ai exhaustion -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
