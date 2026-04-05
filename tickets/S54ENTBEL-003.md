# S54ENTBEL-003: Golden test: contradictory claims coexist

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S54ENTBEL-002

## Problem

Claims infrastructure (001) and perception migration (002) are implemented, but no golden test proves the core claim-substrate value proposition: contradictory reports coexisting with confidence-based resolution and DirectObservation superseding heard claims. This is the distinctive behavior that the claim substrate enables (P16 — contradiction is first-class).

## Assumption Reassessment (2026-04-05)

1. After ticket 002, perception emits `EntityBeliefClaim` entries and derives `known_entities` from claims via `derive_entity_summary`. Confirmed by design.
2. Tell acceptance creates claims with `Report` or `Rumor` source and incremented chain_len. Confidence reduced by `report_chain_penalty` or `rumor_chain_penalty`. Confirmed by design.
3. `PerceptionSource::DirectObservation` uses `direct_observation_base` confidence (typically 900‰). Reports use `report_base` (typically 700‰). Rumors use `rumor_base` (typically 400‰). DirectObservation should consistently win over heard claims. Confirmed from `BeliefConfidencePolicy` defaults.
4. `golden_integration.rs` at `crates/worldwake-ai/tests/golden_integration.rs` — appropriate file for cross-system belief tests.
5. Tell action exists and is used in existing golden tests (e.g., social Tell scenarios). Infrastructure for multi-agent Tell chains is proven.

## Architecture Check

1. Test-only ticket. No production code changes. The golden test exercises the cross-crate contract: Tell (systems) → claim emission (core) → confidence resolution (core) → planner reads derived summary (AI). All interaction through state per P26.
2. The test proves P16 (contradiction coexistence) and P27 (derived summaries are caches) at golden E2E level.
3. No backward-compatibility shims.

## Verification Layers

1. Two agents tell contradictory location claims to third agent → belief store assertion (both claims present in `entity_claims`)
2. `known_entities` reflects highest-confidence claim → authoritative belief state (derived summary check)
3. Third agent later perceives entity directly → DirectObservation claim emitted → supersedes both heard claims in summary
4. Both heard claims still exist in `entity_claims` after direct observation (not deleted, just outranked) → claim store assertion
5. Planner behavior matches derived summary → decision trace (agent acts on derived summary, not on stale claims)

## What to Change

### 1. Add golden scenario: contradictory claims with resolution

In `crates/worldwake-ai/tests/golden_integration.rs`:

**Setup**:
- 3 places: Market, Farm, TownSquare (all connected).
- 1 target entity (e.g., a merchant) at Farm.
- 2 AI informants: Informant A at TownSquare, Informant B at TownSquare. Both have PerceptionProfile and TellProfile.
  - Informant A has a belief (via seeded claim or prior perception) that Target is at Market (stale/wrong).
  - Informant B has a belief that Target is at Farm (correct but via rumor — lower confidence).
- 1 AI receiver at TownSquare. Has PerceptionProfile, ReasoningProfile, UtilityProfile.
  - Receiver has NO prior belief about Target's location.

**Execution phase 1**: Tick until both informants Tell the receiver about Target's location.

**Assertions phase 1**:
- Receiver's `entity_claims` for Target has 2 claims: one from A (Location → Market, Report source), one from B (Location → Farm, Rumor source).
- Receiver's `known_entities` for Target shows `last_known_place` = Market (A's claim wins: Report confidence > Rumor confidence).

**Execution phase 2**: Move Receiver to Farm. Tick until Receiver directly perceives Target.

**Assertions phase 2**:
- Receiver's `entity_claims` for Target now has 3 claims: A's Report, B's Rumor, and Receiver's DirectObservation (Location → Farm).
- Receiver's `known_entities` for Target shows `last_known_place` = Farm (DirectObservation wins).
- A's and B's claims still exist in `entity_claims` — not deleted, just outranked.

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical world hash and event-log hash.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Claim lifecycle management (disputed/retracted) — deferred per spec
- Explicit contradiction detection — deferred per spec
- Institutional claim changes — unchanged
- Production code changes

## Acceptance Criteria

### Tests That Must Pass

1. Golden: two contradictory location claims coexist in receiver's entity_claims
2. Golden: highest-confidence claim wins for known_entities summary
3. Golden: DirectObservation supersedes both heard claims
4. Golden: old claims persist in entity_claims after being outranked (not deleted)
5. Deterministic replay companion produces identical outcome
6. Existing suite: `cargo test --workspace`

### Invariants

1. Multiple contradictory claims coexist without being a system error (P16)
2. `known_entities` is always consistent with highest-confidence claim in `entity_claims` (P27)
3. DirectObservation beats Report beats Rumor (confidence policy ordering)
4. Claims are never deleted by resolution — only by capacity eviction
5. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — Contradictory claims golden scenario + replay companion

### Commands

1. `cargo test -p worldwake-ai -- golden_integration`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
