# S45UNISOCART-003: ClaimBounty action with contention and reward transfer

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action handler with contention integration and commodity transfer
**Deps**: S45UNISOCART-002

## Problem

Bounties can be posted (002) but not claimed. This ticket adds the ClaimBounty action that validates proof, transfers reward from source to claimant, sets bounty state to Fulfilled, and handles contention (race mode — first valid claimant wins).

## Assumption Reassessment (2026-04-04)

1. Commodity transfer uses `transfer_selected_lots()` in `crates/worldwake-systems/src/trade_actions.rs:1110-1160`. This resolves lots from source, splits if needed, and transfers to new holder. Reusable for reward transfer.
2. Contention validation uses `validate_contention_queue_admission()` in `crates/worldwake-systems/src/facility_queue_actions.rs`. Returns `ContentionError::QueueFull` in race mode when another claimant holds the grant.
3. `enqueue_for_contention()` in `facility_queue_actions.rs` enqueues the actor. For race mode (`max_waiters: Some(0)`), this immediately fails if the grant is already held.
4. `ContentionPolicy` on bounty entity has `max_waiters: Some(0)` — set by PostBounty in ticket 002.
5. `ArtifactState` transitions require setting the `ArtifactHeader.state` field via component mutation. `WorldTxn::set_component` pattern.
6. `can_exercise_control()` at `crates/worldwake-core/src/world/ownership.rs:156` chains through containers to verify control over reward source.

## Architecture Check

1. ClaimBounty reuses existing commodity transfer infrastructure (`transfer_selected_lots`) rather than implementing custom reward logic. This keeps reward transfer consistent with trade transfers and respects conservation invariants.
2. Contention uses the generalized S44 substrate (race mode) rather than custom bounty-specific locking. The bounty entity IS the contention target.
3. No backward-compatibility shims.

## Verification Layers

1. Reward transfers from treasury to claimant → authoritative world state (lot ownership) + conservation check
2. Bounty state transitions Active→Fulfilled → authoritative world state (ArtifactHeader.state)
3. Race contention rejects second claimant → action trace (StartFailed with QueueFull)
4. Failed claim on depleted treasury → action trace (commit failure) + bounty remains Active
5. Failed claim with insufficient proof → action trace (precondition failure)

## What to Change

### 1. Implement ClaimBounty action handler

In `crates/worldwake-systems/src/artifact_actions.rs` (extend from ticket 002):

**on_start**:
- Validate claimant co-located with `claim_place` (from BountyTerms).
- Validate bounty entity has `ArtifactState::Active`.
- Validate proof requirement:
  - `PhysicalEvidence`: Check claimant possesses evidence entity (corpse, item) via inventory/co-location.
  - `WitnessTestimony`: Check claimant's belief store for qualifying witness testimony about the target.
  - `SelfReport`: Always passes (lowest bar).
- Validate contention admission via `validate_contention_queue_admission()`. Race mode rejects if grant already held.
- Enqueue via `enqueue_for_contention()`.

**on_commit**:
- Resolve reward from source:
  - `InstitutionalTreasury`: Check treasury entity has sufficient commodity lots. If insufficient, fail with "treasury depleted" — bounty remains Active, claimant gets no reward.
  - `PersonalFunds`: Check issuer entity has sufficient lots.
  - `ReservedLot`: Check reserved lot still exists and has sufficient quantity.
- Transfer reward via `transfer_selected_lots()` from source to claimant.
- Set `ArtifactHeader.state = ArtifactState::Fulfilled`.
- Release contention grant.
- Emit bounty-claimed event.

**on_abort**:
- Release contention grant if held.

Duration: 1-2 ticks.

### 2. Register ClaimBounty action

In `crates/worldwake-systems/src/artifact_actions.rs`: Add to `register_artifact_actions()`.
`ActionDomain::Social`, targets: bounty entity.

### 3. Add affordance enumeration for ClaimBounty

Implement `enumerate_claim_bounty_payloads()` that iterates perceived Active bounties at the agent's location where proof requirements can be met.

## Files to Touch

- `crates/worldwake-systems/src/artifact_actions.rs` (modify — extend from 002)
- `crates/worldwake-systems/src/action_registry.rs` (modify — if not already registered in 002)

## Out of Scope

- AI deciding to pursue bounties — ticket 005
- Perception of bounty state changes — ticket 004
- Golden tests — ticket 006
- Warrant or contract claiming
- Multi-claimant partial reward splitting (only one winner per bounty)

## Acceptance Criteria

### Tests That Must Pass

1. ClaimBounty transfers reward commodity from treasury to claimant
2. ClaimBounty sets bounty state to Fulfilled
3. ClaimBounty with race contention: second claimant receives QueueFull rejection
4. ClaimBounty with depleted treasury: action fails, bounty remains Active
5. ClaimBounty with insufficient proof: precondition rejects
6. ClaimBounty when bounty already Fulfilled: precondition rejects
7. Conservation: total commodity quantity unchanged after claim (reward moves, not created)
8. Existing suite: `cargo test --workspace`

### Invariants

1. Bounty reward comes from a real source (treasury, issuer, reserved lot) — never created from nothing
2. Only one claimant can successfully claim a given bounty (race mode contention)
3. Bounty state is Fulfilled after successful claim — never Active
4. Failed claims leave bounty Active and claimant at claim place with updated beliefs

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_actions.rs` — Unit tests for ClaimBounty success, contention rejection, treasury depletion, proof validation
2. `crates/worldwake-systems/src/artifact_actions.rs` — Conservation invariant test: reward transfer preserves total commodity quantity

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
