# S45UNISOCART-003: ClaimBounty action with contention and reward transfer

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action handler with contention integration and commodity transfer
**Deps**: S45UNISOCART-002

## Problem

Bounties can be posted (002) but not claimed. This ticket adds the ClaimBounty action that validates proof, transfers reward from source to claimant, sets bounty state to Fulfilled, and handles contention (race mode — first valid claimant wins).

## Assumption Reassessment (2026-04-04)

1. Commodity transfer uses `transfer_selected_lots()` in `crates/worldwake-systems/src/trade_actions.rs:1110-1160`. This resolves lots from source, splits if needed, and transfers to new holder. Reusable for reward transfer.
2. The live race-mode contention contract for `max_waiters: Some(0)` is direct grant acquisition, not queue admission. `ContentionQueue::enqueue(...)` treats `Some(0)` as \"no waiters allowed,\" so `enqueue_for_contention()` cannot lawfully implement first-claimer-wins bounty claims.
3. The nearest live pattern is the unique-item pickup race path in `crates/worldwake-systems/src/transport_actions.rs`, which acquires and clears a direct `ContentionGrant` for `UniqueItem` pickup without using queue admission.
4. `ContentionPolicy` on bounty entity has `max_waiters: Some(0)` — set by PostBounty in ticket 002.
5. `ArtifactState` transitions require setting the `ArtifactHeader.state` field via component mutation. `WorldTxn::set_component` pattern.
6. `can_exercise_control()` at `crates/worldwake-core/src/world/ownership.rs:156` chains through containers to verify control over reward source.
7. The live bounty entity already stores `claim_place`, `proof_requirement`, `reward_source`, and target terms in `BountyTerms`. `ClaimBounty` does not need a new action payload carrier; it can bind the bounty artifact as its sole target and read the rest from authoritative artifact components.
8. `transfer_selected_lots()` is still private to `trade_actions.rs`. Reusing the live commodity-transfer path may require widening helper visibility or performing an equivalent bounded helper extraction rather than treating reward transfer as artifact-handler-local logic.
9. `claim_place` and `posting_place` are distinct fields on the live bounty substrate. `ClaimBounty` therefore cannot be modeled as a same-place artifact-target action unless the artifact is allowed to be targeted by stable entity identity while separately validating the claimant stands at `claim_place`.

## Architecture Check

1. ClaimBounty reuses existing commodity transfer infrastructure (`transfer_selected_lots`) rather than implementing custom reward logic. This keeps reward transfer consistent with trade transfers and respects conservation invariants.
2. Contention uses the generalized S44 substrate (race mode) rather than custom bounty-specific locking. The bounty entity IS the contention target, but race-mode claims must acquire a direct grant rather than enqueue into a queue with `max_waiters: Some(0)`.
3. No backward-compatibility shims.

## Verification Layers

1. Reward transfers from treasury to claimant → authoritative world state (lot ownership) + conservation check
2. Bounty state transitions Active→Fulfilled → authoritative world state (ArtifactHeader.state)
3. Race contention rejects second claimant → action trace (StartFailed with `contention_rejected`)
4. Failed claim on depleted treasury → action trace (commit failure) + bounty remains Active
5. Failed claim with insufficient proof → action trace (precondition failure)

## What to Change

### 1. Implement ClaimBounty action handler

In `crates/worldwake-systems/src/artifact_actions.rs` (extend from ticket 002):

**on_start**:
- Validate claimant co-located with `claim_place` (from `BountyTerms`), even if the artifact itself is posted elsewhere.
- Validate bounty entity has `ArtifactState::Active`.
- Validate proof requirement:
  - `PhysicalEvidence`: Check claimant possesses evidence entity (corpse, item) via inventory/co-location.
  - `WitnessTestimony`: Check claimant's belief store for qualifying witness testimony about the target.
  - `SelfReport`: Always passes (lowest bar).
- Acquire or require the race-mode contention grant directly on the bounty artifact. First valid claimant claims the grant; later claimants receive structured `contention_rejected`.

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
Target: specific bounty artifact entity (`EntityKind::SocialArtifact`) selected from known active bounty artifacts whose `claim_place` matches the actor's current place.

### 2. Register ClaimBounty action

In `crates/worldwake-systems/src/artifact_actions.rs`: Add to `register_artifact_actions()`.
`ActionDomain::Social`, target: stable bounty entity identity.

### 3. Add affordance target enumeration for ClaimBounty

Implement `enumerate_claim_bounty_targets()` that iterates known Active bounty artifacts whose believed `claim_place` matches the actor's current place. No new claim payload type is needed because the artifact already stores the claim terms.

## Files to Touch

- `crates/worldwake-systems/src/artifact_actions.rs` (modify — extend from 002)
- `crates/worldwake-systems/src/action_registry.rs` (modify — registry completeness fallout)
- `crates/worldwake-systems/src/trade_actions.rs` (modify only if helper visibility or bounded helper extraction is needed for shared reward transfer)

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
3. ClaimBounty with race contention: second claimant receives structured `contention_rejected`
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
3. `crates/worldwake-systems/src/action_registry.rs` — Update action catalog completeness assertion for `claim_bounty`

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed on 2026-04-04.
- Added `claim_bounty` in `crates/worldwake-systems/src/artifact_actions.rs` as a stable-identity `SocialArtifact` claim action that:
  - validates the claimant is at `claim_place`
  - checks `ArtifactState::Active`
  - validates bounty target satisfaction and proof requirements
  - acquires and clears a direct race-mode `ContentionGrant` on the bounty artifact
  - transfers reward from a real source to the claimant
  - sets the bounty header state to `Fulfilled`
- Registered `claim_bounty` in `crates/worldwake-systems/src/action_registry.rs` and added dynamic affordance target enumeration for known active bounties whose believed `claim_place` matches the actor's current place.
- Added focused systems proof for reward transfer, fulfillment, race rejection, depleted-source abort, insufficient proof, stale fulfilled-state rejection, conservation, and remote stable-identity claim targeting.
- Absorbed one bounded planner-surface fallout in `crates/worldwake-ai/src/planner_ops.rs` by marking `claim_bounty` intentionally unclassified until `S45UNISOCART-005` lands the bounty-goal/planner integration.

Deviations from original plan:
- The ticket was corrected away from queue admission. Live S44 race mode with `ContentionPolicy { max_waiters: Some(0) }` requires direct grant acquisition rather than `enqueue_for_contention()`.
- The ticket was corrected from same-place artifact targeting to stable bounty-entity targeting because `posting_place` and `claim_place` are distinct substrate fields.
- Reward transfer landed as bounded local helper logic in `artifact_actions.rs` rather than reusing the private `trade_actions.rs` helper directly.

Verification:
- `cargo test -p worldwake-systems artifact_actions`
- `cargo test -p worldwake-systems action_registry`
- `cargo test -p worldwake-systems`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
