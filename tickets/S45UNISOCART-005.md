# S45UNISOCART-005: AI bounty pursuit goals and candidate generation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new GoalKind variant, new candidate emission function, planning operator support
**Deps**: S45UNISOCART-004

## Problem

Agents can perceive bounties (004) but have no AI goal to pursue them. This ticket adds `GoalKind::FulfillBounty`, a candidate generation function that emits bounty-pursuit candidates from believed Active bounties, and planning operator support for the Travel→Act→Travel→Claim plan chain.

## Assumption Reassessment (2026-04-04)

1. `GoalKind` at `crates/worldwake-core/src/goal.rs:16-98` derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. Has 25 variants. `FulfillBounty { bounty: EntityId }` is Copy-compatible (EntityId is Copy).
2. Candidate generation at `crates/worldwake-ai/src/candidate_generation.rs:184-262` uses `generate_candidates_with_travel_horizon()` which calls sequential `emit_*_candidates()` helpers. Pattern: add `emit_bounty_candidates()`.
3. `GoalBeliefView` trait provides access to agent beliefs. The view needs a method to iterate believed Active bounties. This may require extending the trait with `believed_active_bounties()` or iterating known entities and filtering by `believed_artifact`.
4. `GroundedGoal` at `crates/worldwake-ai/src/` wraps `GoalKind` with motive score and metadata. Bounty pursuit uses `enterprise_weight` × reward value for ranking.
5. `UtilityProfile.enterprise_weight` at `crates/worldwake-core/src/utility.rs` provides the weight for enterprise-type goals. Bounty pursuit competes with SellCommodity, RestockCommodity, etc.
6. Plan invalidation: existing `is_goal_invalidated()` pattern checks belief state. FulfillBounty invalidated when `believed_artifact.state != Active`.
7. `handle_plan_failure` at `crates/worldwake-ai/src/agent_tick.rs` triggers replanning when action fails. ClaimBounty precondition failure (bounty Fulfilled by another) causes replan.

## Architecture Check

1. Bounty pursuit uses the generic GOAP planning pipeline — no dedicated bounty-pursuit planner. The plan is: Travel(target_place) → EliminateEntity/DeliverCommodity → Travel(claim_place) → ClaimBounty. All operators already exist except ClaimBounty which is added in 003.
2. Enterprise weight ranking puts bounty pursuit in competition with other enterprise goals. No special priority — agent diversity (Principle 22) naturally produces different bounty pursuit rates across agents.
3. No backward-compatibility shims.

## Verification Layers

1. FulfillBounty candidate emitted when agent believes Active bounty → decision trace (candidate list includes FulfillBounty)
2. FulfillBounty NOT emitted when no believed Active bounties → decision trace (candidate list excludes FulfillBounty)
3. FulfillBounty invalidated when believed bounty Fulfilled → decision trace (goal invalidation)
4. Ranking uses enterprise_weight × reward value → focused unit test on motive score calculation
5. Cross-layer: candidate generation (AI) reads beliefs (core) populated by perception (systems) — verified in golden tests (006).

## What to Change

### 1. Add `GoalKind::FulfillBounty`

In `crates/worldwake-core/src/goal.rs`:
- Add `FulfillBounty { bounty: EntityId }` variant.

### 2. Add `emit_bounty_candidates()`

In `crates/worldwake-ai/src/candidate_generation.rs`:
- Add `emit_bounty_candidates()` function.
- Iterate known entities from `GoalBeliefView`.
- For each entity with `believed_artifact` where `kind == Bounty` and `state == Active`:
  - Check if agent can potentially fulfill the target:
    - `EliminateEntity`: agent has combat ability (CombatProfile present).
    - `DeliverCommodity`: agent has or can acquire the commodity.
  - Compute motive score: `enterprise_weight × reward_quantity` (normalized).
  - Emit `GroundedGoal` with `GoalKind::FulfillBounty { bounty: entity_id }`.
- Call `emit_bounty_candidates()` from `generate_candidates_with_travel_horizon()`.

### 3. Add planning operator for ClaimBounty

Ensure the GOAP planner can construct plans for FulfillBounty:
- Terminal condition: bounty state is Fulfilled (believed).
- Plan chain: Travel(target) → target action → Travel(claim_place) → ClaimBounty.
- The planner needs to know about ClaimBounty as an available action at the claim place.

### 4. Add goal invalidation

In goal invalidation logic:
- `FulfillBounty { bounty }` is invalidated when `believed_artifact.state` for the bounty entity is not `Active` (Fulfilled, Expired, Withdrawn, or Destroyed).

### 5. Update format_goal_kind in CLI display

In `crates/worldwake-cli/src/display.rs`, add `FulfillBounty` variant to `format_goal_kind()` for human-readable display (e.g., "FulfillBounty(bounty at Town Square)").

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify — planning operators if needed)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — goal invalidation)
- `crates/worldwake-cli/src/display.rs` (modify — format_goal_kind)

## Out of Scope

- Golden tests — ticket 006
- AI-driven bounty posting (agents don't autonomously post bounties — only office holders or manual action)
- Bounty competition strategy (agents don't reason about competing claimants — they pursue if terms match their abilities)
- Multi-step bounty targets (e.g., "collect 3 items then deliver" — only single-target bounties supported)

## Acceptance Criteria

### Tests That Must Pass

1. `emit_bounty_candidates()` emits FulfillBounty when agent believes Active bounty with fulfillable target
2. `emit_bounty_candidates()` does NOT emit when no Active bounties believed
3. `emit_bounty_candidates()` does NOT emit when agent lacks capability for target (no CombatProfile for EliminateEntity)
4. FulfillBounty invalidated when believed bounty state changes to Fulfilled
5. Motive score proportional to enterprise_weight × reward value
6. `format_goal_kind` handles FulfillBounty variant
7. Existing suite: `cargo test --workspace`

### Invariants

1. GoalKind::FulfillBounty field is Copy (EntityId is Copy)
2. Bounty pursuit competes with other enterprise goals via standard ranking — no special priority
3. Agent only pursues bounties it believes are Active — never reads authoritative artifact state (Principle 14)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — Unit tests for bounty candidate emission, filtering, capability check
2. `crates/worldwake-ai/src/agent_tick.rs` — Test goal invalidation for FulfillBounty
3. `crates/worldwake-cli/src/display.rs` — Test format_goal_kind handles FulfillBounty

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
