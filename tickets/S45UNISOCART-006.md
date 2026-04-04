# S45UNISOCART-006: Golden E2E tests for bounty lifecycle, expiration, and notice discovery

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S45UNISOCART-004, S45UNISOCART-005

## Problem

The social artifact substrate (types, actions, perception, AI) is implemented in tickets 001-005 but has no end-to-end golden tests proving the full lifecycle works. This ticket adds 3 golden test scenarios validating Canonical Scenario A (bounty lifecycle), expiration, and notice-driven belief acquisition.

## Assumption Reassessment (2026-04-04)

1. Golden tests live in `crates/worldwake-ai/tests/` and use the `golden_*` naming convention. They test E2E behavior across all crates.
2. Golden test harness: scenarios set up a world with agents, items, places, and profiles. They then tick the simulation and assert on authoritative state, event log, and belief stores.
3. `PerceptionProfile` is required on agents that need to observe post-action output — tests without perception profiles silently fail to observe newly created entities.
4. All 3 scenarios require deterministic replay companions (seeded RNG, BTreeMap ordering).
5. Scenario A (bounty lifecycle) requires: office holder posts bounty → agent perceives → agent travels to target → eliminates target → travels to claim place → claims bounty → reward transfers. This exercises PostBounty (002), perception (004), AI candidate generation (005), ClaimBounty (003).
6. Scenario B (expiration) requires: bounty posted with expires_at → no claim → artifact_lifecycle_system transitions to Expired → agent perceives Expired and does not pursue.
7. Scenario C (notice discovery) requires: office posts ThreatWarning notice → agent perceives notice locally → believed artifact and route-threat consequence update → later travel/routing behavior reflects the warning.

## Architecture Check

1. Golden tests exercise the full cross-crate lifecycle without privileged shortcuts — agents plan from beliefs, act through standard affordances, and perceive through the perception system. This validates Principles 1 (emergence), 7 (locality), and 14 (belief-only planning).
2. No backward-compatibility shims. Tests use the current API surface only.

## Verification Layers

1. Bounty lifecycle (Scenario A): post → perceive → pursue → claim → reward → Fulfilled
   - PostBounty event emitted → event-log delta
   - Agent belief updated with bounty → belief store assertion
   - FulfillBounty goal chosen → decision trace
   - ClaimBounty committed → action trace
   - Reward transferred → authoritative world state (lot ownership)
   - Bounty state Fulfilled → authoritative world state (ArtifactHeader.state)
2. Expiration (Scenario B):
   - Artifact transitions to Expired → authoritative world state
   - Agent perceives Expired → belief store assertion
   - No FulfillBounty candidate emitted → decision trace absence
3. Notice discovery (Scenario C):
   - Notice perceived → belief store assertion (`believed_artifact`)
   - Threat warning increases remembered route threat / perceived travel cost for the warned place
4. Multi-layer ticket: each golden test maps invariants to specific proof surfaces as listed above.

## What to Change

### 1. Golden test A: Bounty lifecycle (Canonical Scenario A fragment)

Create test in `crates/worldwake-ai/tests/`:

**Setup**:
- 2 places: TownSquare, WildernessA (connected, travel_ticks: 2)
- 3 agents: OfficeHolder (at TownSquare, office-holding), Hunter (at TownSquare, CombatProfile, PerceptionProfile, enterprise_weight high), HostileTarget (at WildernessA, combatable)
- Treasury: ItemLot with 10 Coin on OfficeHolder or institutional entity
- OfficeHolder posts bounty: EliminateEntity(HostileTarget), reward 10 Coin, claim_place TownSquare

**Execution**: Tick until Hunter claims bounty (bounded tick limit with assertion).

**Assertions**:
- Hunter perceived bounty at TownSquare
- Hunter chose FulfillBounty goal
- Hunter traveled to WildernessA
- HostileTarget eliminated (dead or incapacitated)
- Hunter traveled back to TownSquare
- Hunter claimed bounty: 10 Coin transferred from treasury to Hunter
- Bounty state: Fulfilled
- Conservation: total Coin unchanged

### 2. Golden test B: Bounty expiration

**Setup**:
- 1 place: TownSquare
- 1 agent: Observer (at TownSquare, PerceptionProfile)
- Bounty posted with `expires_at: Tick(10)`

**Execution**: Tick to tick 11.

**Assertions**:
- Bounty state: Expired (after tick 10)
- Observer perceives Expired bounty
- No FulfillBounty candidate emitted by Observer after expiration
- Bounty entity still exists (not deleted — just Expired)

### 3. Golden test C: Notice discovery

**Setup**:
- 2 places: TownSquare, DangerousRoute (connected)
- 1 agent: Traveler (at DangerousRoute, PerceptionProfile, enterprise_weight moderate)
- ThreatWarning notice posted at TownSquare for DangerousRoute

**Execution**: Tick until Traveler perceives the notice, then prove warned-route behavior through the existing route-threat / travel-choice surface.

**Assertions**:
- Traveler perceived notice at TownSquare
- Traveler's believed_artifact includes NoticeTopic::ThreatWarning for DangerousRoute
- Traveler's remembered route threat or resulting travel preference reflects the warning

## Files to Touch

- `crates/worldwake-ai/tests/golden_social_artifacts.rs` (new)
- `crates/worldwake-ai/tests/mod.rs` or test registration (modify — if needed)

## Out of Scope

- Warrant, contract, debt golden tests (future artifact types)
- Bounty competition tests (two agents racing for same bounty — valuable but defer to follow-up)
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

1. `crates/worldwake-ai/tests/golden_social_artifacts.rs` — 3 golden scenarios (bounty lifecycle, expiration, notice discovery)

### Commands

1. `cargo test -p worldwake-ai -- golden_social_artifacts`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
