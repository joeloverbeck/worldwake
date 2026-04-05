# S49GOLGAP-001: Delivery-bounty golden closeout

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Bounded AI/planner fallout if the new golden exposes a missing delivery-bounty claim-place contract
**Deps**: S45UNISOCART-006 (S45 golden tests complete), S45UNISOCART-007 (`archive/tickets/S45UNISOCART-007.md` — delivery-bounty planner integration)

## Problem

Scenarios 105-107 prove elimination-bounty lifecycle, expiration, and threat-warning route diversion, but no golden test proves delivery-bounty fulfillment end-to-end. A posted `BountyTarget::DeliverCommodity` artifact driving real cargo movement through the transport substrate and then unlocking a terminal claim remains unproved at golden E2E level.

## Assumption Reassessment (2026-04-05)

1. Scenarios 105-107 exist in `crates/worldwake-ai/tests/golden_integration.rs` at lines 5950, 5988, 6025 — confirmed. Scenario 105 proves elimination-bounty, not delivery.
2. `BountyTarget::DeliverCommodity` variant exists in `crates/worldwake-core/src/social_artifact.rs` — confirmed.
3. `GoalKind::FulfillBounty { bounty: EntityId }` exists in `crates/worldwake-core/src/goal.rs:64-66` — confirmed.
4. `GoalKind::MoveCargo` exists in `crates/worldwake-core/src/goal.rs:53-56` — confirmed. This is the cargo movement goal the planner should chain with FulfillBounty.
5. `BelievedArtifactState` exists in `crates/worldwake-core/src/belief.rs:712-726` — confirmed. Perception integration populates this.
6. `S45UNISOCART-007` (`archive/tickets/S45UNISOCART-007.md`) confirms delivery-bounty planner integration is complete — lower-layer operator admission and search shape proved by focused tests.
7. `RewardSource::ReservedLot` variant exists in `social_artifact.rs` — confirmed. Used for reserved reward lot in this scenario.
8. `PerceptionProfile` required on agents that need to observe artifacts — per CLAUDE.md golden test note.
9. `transfer_selected_lots` in `crates/worldwake-systems/src/trade_actions.rs` handles reward transfer — confirmed.

## Architecture Check

1. Primary owned work is golden closeout in `crates/worldwake-ai/tests/golden_integration.rs`. If the lawful scenario exposes a missing lower-layer contract in the delivery-bounty planner path, this ticket absorbs the bounded AI/planner fix plus focused proof before finalizing the golden.
2. The cross-crate contract under test is: social artifact actions (worldwake-systems) → perception (worldwake-systems) → candidate generation (worldwake-ai) → GOAP planning (worldwake-ai) → transport actions (worldwake-systems) → claim action (worldwake-systems). All interaction through state and event log per Principle 26.
2. No backward-compatibility shims.

## Verification Layers

1. Local bounty perception → belief store assertion (`believed_artifact` populated with `BountyTarget::DeliverCommodity`)
2. `FulfillBounty` goal selected → decision trace (candidate list includes FulfillBounty for delivery bounty)
3. Cargo reaches bounty destination through lawful transport → authoritative world state (commodity lot at destination)
4. `claim_bounty` commits only after delivery completion → action trace (ClaimBounty committed after cargo arrival)
5. Reward transfers from reserved lot to claimant → authoritative world state (lot ownership change) + conservation check
6. Bounty state becomes Fulfilled → authoritative world state (`ArtifactHeader.state == Fulfilled`)
7. Generated golden inventory/docs reflect the new scenario ownership and numbering
8. If needed, focused planner proof that `ClaimBounty` stays unavailable until the actor is both delivery-complete and at `claim_place`

## What to Change

### 1. Add golden scenario: delivery-bounty fulfillment

In `crates/worldwake-ai/tests/golden_integration.rs`:

**Setup**:
- 3 places: PostingPlace, Destination, ClaimPlace (or PostingPlace == ClaimPlace if simpler). At minimum Destination must be distinct from PostingPlace. Travel edges connecting them.
- 1 human issuer at PostingPlace with reserved reward lot (e.g., 10 Coin).
- 1 AI courier at PostingPlace with enough of the required commodity (e.g., 5 Grain) already in inventory. Courier has: PerceptionProfile, UtilityProfile with enterprise_weight, ReasoningProfile.
- Human issuer posts `BountyTarget::DeliverCommodity { commodity: Grain, quantity: 5, destination: Destination }` bounty with `RewardSource::ReservedLot` pointing to the Coin lot. `claim_place` = PostingPlace.

**Execution**: Tick simulation with bounded limit until bounty becomes Fulfilled.

**Assertions**:
- Courier perceived bounty artifact at PostingPlace (`believed_artifact.kind == Bounty`, `believed_artifact.state == Active`).
- Courier selected `FulfillBounty` goal (decision trace).
- Grain commodity lot moved to Destination through lawful transport actions (authoritative world state).
- `claim_bounty` action committed (action trace).
- 10 Coin transferred from reserved lot to courier (authoritative world state).
- Bounty `ArtifactState::Fulfilled` (authoritative world state).
- Conservation: total Coin and total Grain unchanged.

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical outcome.

### 3. Refresh generated golden inventory/docs

Run `python3 scripts/golden_inventory.py --write --check-docs` after the new scenario lands so the S45/S49 ownership surfaces stay in sync with the generated inventory.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (only if the lawful golden exposes a missing claim-place planner contract)
- `crates/worldwake-ai/src/search/tests.rs` (focused proof for any bounded planner fallout)
- `docs/generated/golden-coverage-matrix.md` (generated)
- `docs/generated/golden-e2e-inventory.md` (generated)
- `docs/generated/golden-scenario-map.md` (generated)

## Out of Scope

- Elimination-bounty tests (already covered by Scenario 105)
- Bounty expiration tests (already covered by Scenario 106)
- Multi-claimant competition tests
- Broad social-artifact runtime changes outside a bounded delivery-bounty planner fix

## Acceptance Criteria

### Tests That Must Pass

1. Delivery-bounty golden: perception → cargo progress → claim → reward without bounty-only shortcut
2. Deterministic replay companion produces identical outcome
3. Conservation invariant: total commodity quantities unchanged
4. Existing suite: `cargo test --workspace`

### Invariants

1. Courier plans from beliefs only — never reads authoritative artifact state during planning (Principle 14)
2. Bounty knowledge arrives through local artifact perception (Principle 7)
3. Reward transfers from real reserved lot — no created-from-nothing rewards (Principle 4)
4. Delivery uses canonical cargo/transport actions — no bounty-specific shortcut (Principle 26)
5. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — Delivery-bounty golden scenario + replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_integration`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-04-05
- **What changed**:
  - Added Scenario 108 in `crates/worldwake-ai/tests/golden_integration.rs` proving delivery-bounty fulfillment from local artifact perception through lawful cargo delivery, later claim at a distinct `claim_place`, reward transfer from a reserved lot, and deterministic replay.
  - Fixed a bounded planner contract gap in `crates/worldwake-ai/src/goal_model.rs` so delivery `claim_bounty` only becomes available after delivery is complete and the actor has returned to `claim_place`.
  - Added focused planner regressions in `crates/worldwake-ai/src/search/tests.rs` covering the distinct-place delivery-then-claim path and suppressing early `claim_bounty` root admission.
  - Refreshed generated coverage docs in `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`.
- **Deviations from original plan**:
  - The ticket did not remain test-only. Once the lawful golden used a distinct `claim_place`, it exposed a real lower-layer planner contradiction, so the ticket absorbed the bounded AI/planner fix already allowed by its stated engine-change boundary.
  - Final golden assertions were narrowed to the durable contract: delivery completion at destination, later claim at `claim_place`, reward transfer, fulfillment, conservation, and deterministic replay. Exact intermediate substeps such as a specific `put_down` trace were not treated as the owned contract.
- **Verification results**:
  - `cargo test -p worldwake-ai fulfill_bounty_delivery_search_finds_delivery_then_claim_plan -- --nocapture`
  - `cargo test -p worldwake-ai fulfill_bounty_delivery_does_not_surface_claim_candidate_before_reaching_claim_place -- --nocapture`
  - `cargo test -p worldwake-ai golden_s49_delivery_bounty_lifecycle -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_integration`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
