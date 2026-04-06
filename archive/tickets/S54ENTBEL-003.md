# S54ENTBEL-003: Golden test: contradictory claims coexist

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: `tell` contradictory entity-belief intake
**Deps**: S54ENTBEL-002

## Problem

Claims infrastructure (001) and perception migration (002) are implemented, but no golden test proves the core claim-substrate value proposition: contradictory reports coexisting with confidence-based resolution and DirectObservation superseding heard claims. This is the distinctive behavior that the claim substrate enables (P16 — contradiction is first-class).

## Assumption Reassessment (2026-04-05)

1. After ticket 002, perception emits `EntityBeliefClaim` entries and derives `known_entities` from claims via `derive_entity_summary`. Confirmed by design.
2. Tell acceptance creates claims with `Report` or `Rumor` source and incremented chain_len. Confidence reduced by `report_chain_penalty` or `rumor_chain_penalty`. Confirmed by design.
3. `PerceptionSource::DirectObservation` uses `direct_observation_base` confidence (typically 900‰). Reports use `report_base` (typically 700‰). Rumors use `rumor_base` (typically 400‰). DirectObservation should consistently win over heard claims. Confirmed from `BeliefConfidencePolicy` defaults.
4. `golden_social.rs` at `crates/worldwake-ai/tests/golden_social.rs` is the strongest existing owner for entity-belief tell relay, report/rumor degradation, and direct-observation correction. `golden_integration.rs` is broader and does not currently own this narrower social-claim proof surface.
5. Existing social goldens already prove the surrounding infrastructure: `golden_rumor_chain_degrades_through_three_agents`, `golden_rumor_leads_to_wasted_trip_then_discovery`, and `golden_skeptical_listener_rejects_told_belief` cover relay degradation, accepted rumor intake, and direct-observation correction. The remaining gap is explicit coexistence of contradictory entity claims on one listener for the same subject/aspect.
6. Live reassessment exposed a production contradiction in `tell_actions.rs`: entity tell intake still gated updates on summary recency alone, so same-tick contradictory hearsay could be dropped as `AlreadyHeldEqualOrNewer` before a second claim was recorded. The ticket must correct that intake rule in addition to adding the golden.

## Architecture Check

1. The primary deliverable remains golden coverage, but the live codebase also needs a small production correction in `tell` entity-belief intake so contradictory same-tick hearsay actually reaches the claim store. The golden still exercises the cross-crate contract: Tell (systems) → claim emission (core) → confidence resolution (core) → planner reads derived summary (AI). All interaction through state per P26.
2. The test proves P16 (contradiction coexistence) and P27 (derived summaries are caches) at golden E2E level.
3. No backward-compatibility shims.

## Verification Layers

1. Two agents tell contradictory location claims for the same subject to one listener → authoritative belief-store assertion (both location claims present in `entity_claims`)
2. `known_entities` reflects highest-confidence claim → authoritative belief state (derived summary check)
3. Third agent later perceives entity directly → DirectObservation claim emitted → supersedes both heard claims in summary
4. Both heard claims still exist in `entity_claims` after direct observation (not deleted, just outranked) → claim store assertion
5. Contradictory beliefs arrive through the live `tell` action path rather than test-only store mutation → action trace

## What to Change

### 1. Add golden scenario: contradictory claims with resolution

In `crates/worldwake-ai/tests/golden_social.rs`:

**Setup**:
- 3 places: Market, Farm, TownSquare (all connected).
- 1 target entity (e.g., a merchant) at Farm.
- 2 AI speakers at TownSquare with focused Tell profiles and high social weight.
  - Speaker A has a claim-backed DirectObservation belief that Target is at the wrong place.
  - Speaker B has a claim-backed Report belief that Target is at the correct place, so the listener internalizes it as a lower-confidence Rumor.
- 1 listener at TownSquare with accepting Tell/Communication profiles and no prior belief about Target's location.

**Execution phase 1**: Tick until both informants Tell the receiver about Target's location.

**Assertions phase 1**:
- Listener's `entity_claims` for Target has 2 contradictory location claims: one from A (Location → wrong place, Report source), one from B (Location → correct place, Rumor source).
- Listener's `known_entities` for Target shows the wrong place because Report confidence outranks Rumor confidence.

**Execution phase 2**: Move Receiver to Farm. Tick until Receiver directly perceives Target.

**Assertions phase 2**:
- Listener's `entity_claims` for Target now has 3 contradictory location claims: A's Report, B's Rumor, and Listener's DirectObservation (Location → correct place).
- Listener's `known_entities` for Target shows the correct place (DirectObservation wins).
- A's and B's claims still exist in `entity_claims` — not deleted, just outranked.

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical world hash and event-log hash.

### 3. Correct contradictory entity-belief intake in tell commit

In `crates/worldwake-systems/src/tell_actions.rs`:

- accept a told entity belief when the incoming shared content differs from the listener's current summary, even if the observed tick is equal
- keep provenance-only no-op behavior for genuinely equivalent content
- add a focused systems regression proving a same-tick contradictory hearsay tell records the second claim instead of being dropped as `AlreadyHeldEqualOrNewer`

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- Claim lifecycle management (disputed/retracted) — deferred per spec
- Explicit contradiction detection — deferred per spec
- Institutional claim changes — unchanged
- broader tell/planner refactors

## Acceptance Criteria

### Tests That Must Pass

1. Golden: two contradictory location claims coexist in receiver's entity_claims
2. Golden: highest-confidence claim wins for known_entities summary
3. Golden: DirectObservation supersedes both heard claims
4. Golden: old claims persist in entity_claims after being outranked (not deleted)
5. Deterministic replay companion produces identical world hash and event-log hash
6. Existing suite: `cargo test --workspace`

### Invariants

1. Multiple contradictory claims coexist without being a system error (P16)
2. `known_entities` is always consistent with highest-confidence claim in `entity_claims` (P27)
3. DirectObservation beats Report beats Rumor (confidence policy ordering)
4. Claims are never deleted by resolution — only by capacity eviction
5. Deterministic: same seed → same world hash and event-log hash

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — contradictory entity-location claims golden scenario + replay companion
2. `crates/worldwake-systems/src/tell_actions.rs` — focused regression for same-tick contradictory entity-belief intake

### Commands

1. `cargo test -p worldwake-ai --test golden_social golden_contradictory_location_claims_coexist_and_direct_observation_wins -- --nocapture`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Added `golden_contradictory_location_claims_coexist_and_direct_observation_wins` in `crates/worldwake-ai/tests/golden_social.rs`
  - Added golden-local helpers to seed claim-backed entity beliefs and drive a scripted live `tell` setup without autonomous interference
  - Corrected `crates/worldwake-systems/src/tell_actions.rs` so changed same-tick entity hearsay is accepted into the listener claim store instead of being dropped as `AlreadyHeldEqualOrNewer`
  - Added a focused systems regression proving same-tick contradictory entity tells preserve both location claims while the higher-confidence heard claim wins the derived summary
- **Deviations from original plan**:
  - The ticket did not remain test-only; focused golden verification exposed a real production contradiction in `tell` entity-belief intake, so the ticket scope was corrected to include that bounded systems fix
  - The final golden uses explicit external `tell` requests with speakers set to `ControlSource::None` to prove the live tell path deterministically without autonomous duplicate tells obscuring the contradiction contract
- **Verification**:
  - `cargo test -p worldwake-systems tell_commit_accepts_same_tick_contradictory_entity_belief_and_preserves_both_claims -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_social golden_contradictory_location_claims_coexist_and_direct_observation_wins -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
