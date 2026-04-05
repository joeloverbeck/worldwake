# S45UNISOCART-006: Golden E2E tests for bounty lifecycle, expiration, and notice discovery

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S45UNISOCART-002, S45UNISOCART-003, S45UNISOCART-004, S45UNISOCART-005

## Problem

The social artifact substrate is implemented through the S45 ticket chain, but it still lacks the golden closeout proving the live cross-system lifecycle works. This ticket adds 3 golden test scenarios validating the elimination-bounty lifecycle, authoritative expiration before pursuit, and notice-driven route-threat behavior.

## Assumption Reassessment (2026-04-04)

1. Golden tests live in `crates/worldwake-ai/tests/` and use the existing owning suite files. There is no `golden_social_artifacts.rs`; the live owning boundary for these S45 end-to-end scenarios is `golden_integration.rs`.
2. Golden test harness: scenarios set up a world with agents, items, places, and profiles. They then tick the simulation and assert on authoritative state, event log, and belief stores.
3. `PerceptionProfile` is required on agents that need to observe post-action output — tests without perception profiles silently fail to observe newly created entities.
4. All 3 scenarios require deterministic replay companions (seeded RNG, BTreeMap ordering).
5. Scenario A (bounty lifecycle) requires the elimination-bounty path specifically: issuer posts an `EliminateEntity` bounty → hunter perceives it → hunter travels to the believed target place → hunter eliminates the target → hunter returns to the claim place → hunter claims the reward. Under the live `ClaimBounty` contract, this scenario must use `ProofRequirement::SelfReport` rather than `PhysicalEvidence`; the current implementation requires the dead target to be present at the claim place for physical-evidence claims, and no S45 ticket owns corpse transport as part of the bounty loop.
6. Scenario B (expiration) requires: bounty posted with `expires_at` → no claim starts before that tick → the pre-action artifact-expiry phase transitions the artifact to `Expired` before later action admission → an agent that perceives the expired artifact does not generate `FulfillBounty`.
7. Scenario C (notice discovery) requires: issuer posts a `ThreatWarning` notice → an agent perceives the notice locally → `believed_artifact` carries the active warning → the next AI travel plan uses the existing route-threat surface and prefers the safer route over the shorter warned route.

## Architecture Check

1. Golden tests exercise the full cross-crate lifecycle without privileged shortcuts — agents plan from beliefs, act through standard affordances, and perceive through the perception system. This validates Principles 1 (emergence), 7 (locality), and 14 (belief-only planning).
2. No backward-compatibility shims. Tests use the current API surface only.

## Verification Layers

1. Bounty lifecycle (Scenario A): post → perceive → pursue → claim → reward → Fulfilled
   - PostBounty event emitted → event-log delta
   - Agent belief updated with bounty → belief store assertion
   - Elimination-bounty `FulfillBounty` goal chosen → decision trace
   - ClaimBounty committed → action trace
   - Reward transferred → authoritative world state (lot ownership)
   - Bounty state Fulfilled → authoritative world state (ArtifactHeader.state)
2. Expiration (Scenario B):
   - Artifact transitions to Expired → authoritative world state
   - Agent perceives Expired → belief store assertion
   - No `FulfillBounty` candidate emitted → decision trace absence
3. Notice discovery (Scenario C):
   - Notice perceived → belief store assertion (`believed_artifact`)
   - Threat warning changes the next search-selected travel branch away from the warned route
4. Multi-layer ticket: each golden test maps invariants to specific proof surfaces as listed above.

## What to Change

### 1. Golden test A: Bounty lifecycle (Canonical Scenario A fragment)

Add scenarios to `crates/worldwake-ai/tests/golden_integration.rs`.

**Setup**:
- 2 places: TownSquare, WildernessA (connected, travel_ticks: 2)
- 3 agents: Issuer (Human), Hunter (AI, CombatProfile, PerceptionProfile, enterprise_weight high), HostileTarget (non-moving combat target)
- Reward source: real ItemLot with 10 Coin on Issuer at TownSquare
- Issuer posts bounty: `EliminateEntity(HostileTarget)`, reward 10 Coin, claim_place TownSquare, `ProofRequirement::SelfReport`
- Hunter starts with a lawful prior belief about the target's wilderness location so the route proof is about bounty pursuit, not missing target-location knowledge

**Execution**: Tick until Hunter claims bounty (bounded tick limit with assertion).

**Assertions**:
- Hunter perceived bounty at TownSquare
- Hunter chose the elimination-bounty `FulfillBounty` goal
- Hunter traveled to WildernessA
- HostileTarget eliminated (dead or incapacitated)
- Hunter traveled back to TownSquare
- Hunter claimed bounty: 10 Coin transferred from treasury to Hunter
- Bounty state: Fulfilled
- Conservation: total Coin unchanged

### 2. Golden test B: Bounty expiration

**Setup**:
- 1 place: TownSquare
- 2 agents: Issuer (Human) and Observer (initially non-AI, then AI after expiration)
- Bounty posted with `expires_at`

**Execution**: Tick to tick 11.

**Assertions**:
- Bounty state: Expired (after tick 10)
- Observer perceives Expired bounty
- No `FulfillBounty` candidate emitted by Observer after expiration
- Bounty entity still exists (not deleted — just Expired)

### 3. Golden test C: Notice discovery

**Setup**:
- 4 places with two routes to food:
  - Market → WarnedRoad → Orchard (shorter)
  - Market → SafeRoute → Orchard (longer base path, lower perceived threat)
- 1 agent: Traveler (at Market, AI after perception, hungry, PerceptionProfile)
- ThreatWarning notice posted at Market for WarnedRoad
- Orchard workstation/source is already known to the traveler so the route-choice proof is about warning uptake, not missing source knowledge

**Execution**: Tick until Traveler perceives the notice, then prove warned-route behavior through the existing route-threat / travel-choice surface.

**Assertions**:
- Traveler perceived the notice
- Traveler's `believed_artifact` includes `NoticeTopic::ThreatWarning` for the warned route place
- The first search-selected travel step for apple acquisition routes through `SafeRoute`, not the warned shorter branch

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs`
- generated golden docs refreshed via `scripts/golden_inventory.py`

## Out of Scope

- Warrant, contract, debt golden tests (future artifact types)
- Bounty competition tests (two agents racing for same bounty — valuable but defer to follow-up)
- Delivery-bounty-specific golden closure beyond these three canonical S45 scenarios
- Tell-based bounty knowledge sharing tests (works via existing Tell infrastructure)
- CLI display golden tests (CLI display tested via cli-improvement pipeline)

## Acceptance Criteria

### Tests That Must Pass

1. Golden A: Full bounty lifecycle from post to claim to reward transfer
2. Golden B: Bounty expiration transitions correctly, expired bounty not pursued
3. Golden C: Notice perceived and internalized as belief
4. All golden tests deterministic: same seed → same outcome
5. Conservation invariant holds in Scenario A: total commodity unchanged
6. Existing suite: `cargo test --workspace`

### Invariants

1. Agents plan from beliefs only — no authoritative artifact state read during planning (Principle 14)
2. Information reaches agents through perception at co-located places — no global artifact registry (Principle 7)
3. Reward transfer uses real commodity lots from real sources — no created-from-nothing rewards (Principle 4)
4. Deterministic replay: same seed reproduces identical outcomes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — 3 golden scenarios (bounty lifecycle, expiration, notice discovery)

### Commands

1. `cargo test -p worldwake-ai --test golden_integration`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed: 2026-04-05

- Added Scenario 105 `golden_s45_bounty_lifecycle`, Scenario 106 `golden_s45_bounty_expiration_blocks_pursuit`, and Scenario 107 `golden_s45_notice_warning_flips_route_choice` to `crates/worldwake-ai/tests/golden_integration.rs`, each with deterministic replay companions.
- Refreshed the generated golden inventory/docs in `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`.
- Absorbed bounded planner fallout exposed by Scenario 105 in `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/src/search/tests.rs` so elimination-bounty `FulfillBounty` no longer leaks stale `Attack` branches after the target is dead and only surfaces `claim_bounty` once the elimination contract is satisfied.

Deviations from original plan:

- Scenario A landed with a lawful reserved-lot reward setup rather than the ticket's original personal-funds wording, because the scenario needed issuer isolation after posting without weakening the real reward-transfer contract.
- The golden closeout exposed a real lower-layer planner contradiction, so this ticket absorbed the bounded production fix and focused regression proof instead of treating the failure as golden-only fixture churn.

Verification:

- `cargo test -p worldwake-ai --test golden_integration`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
