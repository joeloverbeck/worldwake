# S48GOLGAP-001: Golden source-reliability reroute after trade rejection

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes
**Deps**: specs/S48-golden-gaps-S38.md, archive/specs/S38-learned-route-source-preferences.md

## Problem

S38's route-memory half is now golden-covered by Scenarios 91-93 in `golden_experience_preferences.rs`, but the suite still does not prove the matching source-learning emergence chain. The live code records failed trade attempts into `SourceReliability` and applies that memory in AI ranking, yet no golden currently shows that a real rejected trade against one seller later redirects `AcquireCommodity(SelfConsume)` to a lawful alternate seller for the same commodity.

## Assumption Reassessment (2026-04-02)

1. The claimed golden gap is still real. `docs/generated/golden-scenario-map.md` shows S38 route-memory coverage only in Scenarios `91`, `92`, and `93` from `crates/worldwake-ai/tests/golden_experience_preferences.rs`; there is no source-reliability golden scenario in the live inventory.
2. The original harvest-driven S48 narrative does not survive live-code reassessment. `crates/worldwake-ai/src/candidate_generation.rs` only emits harvest-source opportunities whose believed `resource_source.available_quantity > 0`, so a same-place depleted local orchard can lawfully disappear before ranking runs. That makes the old “local source still present but loses because of `SourceReliability`” proof surface too strong for the current harvest path.
3. The corrected shared abstraction boundary under audit is the S38 trade-side source-learning path: authoritative rejection recording in `crates/worldwake-systems/src/trade_actions.rs` plus belief-facing motive discounting in `apply_source_reliability_discount` / `source_reliability_discount_scope` in `crates/worldwake-ai/src/ranking.rs`.
4. The intended invariant is not generic “alternate seller succeeded.” It is narrower: a real rejected trade records `SourceReliability` on the rejecting seller, and on a later planning pass the same buyer prefers a lawful alternate seller for the same commodity because the prior seller is reliability-discounted rather than removed from candidacy.
5. The live `GoalKind` under test is `GoalKind::AcquireCommodity { commodity: Bread, purpose: CommodityPurpose::SelfConsume }`. The current operator surface is the trade acquire path already exercised in `crates/worldwake-ai/tests/golden_trade.rs`, where listed sellers remain lawfully discoverable after a rejected negotiation.
6. Golden implementation exposed a real production contradiction, so this ticket is no longer honestly "tests only." In the live trade-source scenario, planning reranks from seller A to seller B after seller A is reliability-discounted, but the executable next step still starts `trade` against seller A. The owned boundary is therefore mixed-layer: a bounded AI planning/search fix plus the new golden proof.
7. The ordering contract is mixed-layer: authoritative trade rejection and `SourceReliability` aftermath first, then next-planning-pass seller-choice ordering. The divergence is driven by motive-score discounting from `SourceReliability`, not by priority-class asymmetry.
8. The scenario depends on one specific ranking substrate rather than heuristic removal. `apply_source_reliability_discount` already exists and is covered in focused tests in `crates/worldwake-ai/src/ranking.rs`; this ticket only closes the missing cross-system golden proof.
9. The first failure boundary is post-start abort due to trade rejection, not request rejection or authoritative start failure. The lower-layer rejection aftermath is already focused-tested in `crates/worldwake-systems/src/trade_actions.rs`; this golden should prove the later planning consequence rather than restating earlier handler coverage.
10. Scenario isolation must be explicit. The intended branch is “learned seller rejection redirects later seller choice.” Lawful competing branches include local harvest/loose-lot food acquisition, sale-listing invalidation, and unrelated route-memory effects. Setup should remove those unrelated branches so the reroute remains attributable to learned seller unreliability.
11. The arithmetic that makes the branch reachable must be validated in the test setup: two sellers for the same commodity remain lawfully available after the rejection, the initially preferred seller should win absent learning, and `PreferenceProfile.source_trust_weight` plus the recorded failed attempt must be sufficient to make the alternate seller win under current `ranked_motive_score()` + `apply_source_reliability_discount()` arithmetic.
12. `cargo test -p worldwake-ai -- --list` confirms `golden_trade.rs` is a live test target, so ticket verification commands can name real test binaries and focused test names.
13. The contradiction lives in the AI root-candidate binding path, not in source-learning arithmetic. `search_candidates_from_affordance` in `crates/worldwake-ai/src/search/candidates.rs` can admit a `Trade` affordance for seller A while searching a sibling `GroundedGoal` opportunity whose single `evidence_entities` seller is seller B, because `GoalKind::matches_binding` deliberately treats `PlannerOpKind::Trade` as a flexible auxiliary operator. That leaves selected-opportunity traces and executable trade payloads out of sync.

## Architecture Check

1. Keeping this as one mixed-layer ticket is cleaner than splitting immediately because the new golden produced direct evidence of a specific AI planning/search contradiction. Under `AGENTS.md`, the ticket cannot stay `Engine Changes: None` once the requested invariant exposes a live production gap.
2. The canonical fact path after the fix is: grounded opportunity seller evidence -> admissible trade root candidate -> concrete `TradeActionPayload.counterparty` -> authoritative trade outcome. The bug is that the current root-candidate search lets a sibling seller's affordance satisfy the searched opportunity. The fix must remove that duplicate lawful path rather than documenting around it.
3. Reusing `crates/worldwake-ai/tests/golden_trade.rs` is still the right proof surface, but the implementation now also needs a focused search-layer regression so the seller-opportunity binding is proved at the strongest owned lower layer.
4. No backwards-compatibility aliasing or shim paths are introduced. The ticket adds one seller-scoped root-candidate rule for trade opportunities and the corresponding golden coverage for the current S38 architecture.

## Verification Layers

1. Focused search regression proves a trade affordance bound to seller A is rejected when the searched grounded opportunity is explicitly scoped to seller B -> `crates/worldwake-ai/src/search/tests.rs`
2. Initial trade starts and later aborts due to seller rejection rather than pre-start invalidation -> action trace
3. Durable learned memory exists on the exact rejected seller `SourceKey` -> authoritative world state
4. The next planning pass prefers the alternate seller because the prior seller now carries a `SourceReliabilityDiscount` -> decision trace
5. The alternate seller path is the one actually used after reranking -> action trace and authoritative world state (listed lot transfer and downstream consumption complete)
6. Deterministic replay reproduces the same causal chain -> golden replay companion plus world/event hash comparison

## What to Change

### 1. Fix trade root-candidate binding for seller-scoped opportunities

Update `crates/worldwake-ai/src/search/candidates.rs` so trade affordances cannot satisfy the wrong same-goal sibling opportunity. When the searched `GroundedGoal` is an S38-style trade-capable commodity goal with exactly one concrete seller in `evidence_entities`, a `Trade` affordance must only be admitted if its counterparty matches that seller. This keeps the selected-opportunity trace, root candidate, and executable `TradeActionPayload` on one canonical seller binding.

Add a focused regression in `crates/worldwake-ai/src/search/tests.rs` that proves a seller-A trade affordance is rejected while searching a grounded seller-B acquire opportunity, and that the correct seller-B path remains admissible.

### 2. Add the primary S48 golden scenario

Extend `crates/worldwake-ai/tests/golden_trade.rs` with a new scenario that sets up:
- one critically hungry buyer
- two lawful sellers for the same commodity
- an initially preferred seller that rejects a real trade attempt but remains lawfully available afterward
- an alternate seller that remains lawful for the same commodity
- beliefs/perception sufficient for the buyer to lawfully know both sellers and their listed lots
- `PreferenceProfile { source_trust_weight > 0, route_caution_weight = 0 }`

The scenario should prove the exact S48 chain after the binding fix:
- trade against seller A aborts due to rejection
- `SourceReliability` is recorded on seller A for the traded commodity
- a later planning pass selects seller B rather than seller A
- seller B is actually used and the hunger-relief chain completes

### 3. Add decision-trace and state assertions at the right boundaries

Use decision traces to prove the reroute reason at the planning boundary instead of inferring it only from eventual bread transfer or later eating. Use authoritative world state for the learned `SourceReliability` record and for final seller-B lot transfer / buyer hunger relief.

### 4. Add deterministic replay coverage and scenario metadata

Add the deterministic replay companion and the required `// Scenario` metadata block so `python3 scripts/golden_inventory.py --write --check-docs` continues to track the new scenario in the generated inventory and scenario map.

## Files to Touch

- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (modify, generated)
- `docs/generated/golden-scenario-map.md` (modify, generated)
- `docs/generated/golden-coverage-matrix.md` (modify, generated)

## Out of Scope

- Any production changes to `SourceReliability` recording or `apply_source_reliability_discount`
- Broader plan-selection rewrites beyond the bounded seller-opportunity binding fix in `search/candidates.rs`
- New route-memory goldens; S38 route coverage already exists in Scenarios `91`-`93`
- Harvest-source reroute goldens; the depleted-harvest narrative was corrected out of scope during reassessment because same-place depletion knowledge can lawfully remove the local source before ranking
- Coverage-dashboard or spec archival work beyond whatever the normal golden inventory regeneration updates mechanically

## Acceptance Criteria

### Tests That Must Pass

1. Focused search regression proves the searched seller opportunity cannot execute through a different seller's trade affordance.
2. New golden scenario proves that a real rejected trade records `SourceReliability` on seller A and later reroutes acquisition to lawful seller B for the same commodity.
3. New replay companion proves deterministic reproduction of the same source-learning reroute.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The reroute is proven at the planning boundary as a learned seller-choice change, not merely as eventual alternate trade success.
2. The searched opportunity's concrete seller evidence remains the canonical binding for the executable trade step after reranking; selected-opportunity and started-action seller identities must agree.
3. The first seller remains lawfully available after the rejection; absence or invalidation of seller A must not be the reason seller B wins.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — add focused regression for seller-scoped trade root-candidate binding
2. `crates/worldwake-ai/tests/golden_trade.rs` — add the primary S48 scenario proving learned seller-reliability reroute after a real rejected trade
3. `crates/worldwake-ai/tests/golden_trade.rs` — add deterministic replay companion for the same scenario

### Commands

1. `cargo test -p worldwake-ai search_candidates_from_affordance -- --nocapture`
2. `cargo test -p worldwake-ai --test golden_trade -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- Completed: 2026-04-02
- What changed:
  - fixed seller-scoped trade root-candidate binding in `crates/worldwake-ai/src/search/candidates.rs` so a grounded same-goal sibling opportunity cannot execute through the wrong seller's `Trade` affordance
  - added a focused regression in `crates/worldwake-ai/src/search/tests.rs` proving trade affordances bound to the wrong seller are rejected for seller-scoped acquire opportunities
  - added the S48 golden scenario and deterministic replay companion in `crates/worldwake-ai/tests/golden_trade.rs`, proving rejected local trade records `SourceReliability`, reranks to a lawful alternate seller, and actually executes through that alternate seller
  - refreshed the generated golden inventory docs after adding the new scenario metadata
  - repaired adjacent verification-gate fallout in `crates/worldwake-ai/tests/golden_soak.rs` by replacing unsafe environment-mutation test setup with a pure helper so `python3 scripts/golden_inventory.py --write --check-docs` could complete under the repo's no-unsafe lint gate
- Deviations from original plan:
  - the ticket started as a golden-only source-learning proof and was corrected twice during reassessment: first from a harvest-source narrative to a trade-source narrative, then from a tests-only ticket to a mixed-layer ticket when the new golden exposed a real AI search-binding contradiction
  - the final implementation therefore included one bounded production fix in `search/candidates.rs` in addition to the planned golden coverage
- Verification results:
  - `cargo test -p worldwake-ai search_candidates_from_affordance_rejects_trade_for_wrong_seller_opportunity -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_trade golden_trade_rejection_reroutes_to_reliable_seller -- --exact --nocapture`
  - `cargo test -p worldwake-ai --test golden_trade golden_trade_rejection_reroutes_to_reliable_seller_replays_deterministically -- --exact --nocapture`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
