# S133SOUCOMTIE-006: Goldens for source composite tiebreaker

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Narrow fix — no-record source composite now returns neutral factors instead of omitting the rank; primary deliverable remains golden coverage.
**Deps**: archive/tickets/S133SOUCOMTIE-005.md

## Problem

D6 closes the verification loop. Without it: (a) the new same-commodity tiebreaker has no E2E coverage; (b) the four pre-existing survival goldens are the only protection against re-introducing the rolled-back motive-additive composite, and a new contributor with no familiarity with the 2026-05-03 incident could regress cross-category neutrality without the failure mode being attributed to that exact contract. Spec D6 calls out six scenarios that together exercise the firing path, the no-fire path, and every factor neutrality and demotion contract.

## Assumption Reassessment (2026-05-03)

<!-- Apply all docs/precision-rules.md domain rules. -->

1. The four pre-existing failing survival goldens still live in-tree: `survival_drive_escalation_lands_row_four` (`crates/worldwake-ai/tests/golden_survival_drive_escalation.rs:590`), `survival_offices_proves_force_law_uptake` (`golden_survival_offices.rs:467`), `survival_preferences_keeps_proactive_diversification_alive_under_survival` (`golden_survival_preferences.rs:379`), `survival_tell_lands_row_five` (`golden_survival_tell.rs:425`). They re-assert their original survival contracts post-S133 implementation. The S131 data-path goldens at `golden_source_reliability.rs` (`resource_extraction_wait_observation_records_when_promoted`, `capacity_observation_records_from_perception`) verify the wait/capacity write paths and remain in place per spec D6.
2. Spec D6 dictates the six new scenario contracts. Each scenario's planner surface: `GoalKind::AcquireCommodity { commodity: CommodityKind::Apple, purpose: CommodityPurpose::SelfConsume, .. }`. The delivered D6 scenarios are harness-authored goldens using the shared `golden_harness` infrastructure.
3. Shared abstraction boundary under audit: decision-trace ranked summaries and comparator attribution. The truthful proof surface is the ranked source order plus `RankedGoalComparisonDimension`; these goldens do not overclaim full action-commit source selection when the comparator invariant is already proven at the planning rank.
4. Failing-golden invariants: the four pre-existing goldens encode "drink", "force-law uptake", "diversification alive under survival", and "wash relief lands by row five" — all of which depend on cross-category goal ranking (Wash/Drink vs AcquireCommodity) being unaffected by source-of-acquisition perturbation. Live comparator ordering resolves the D6 cross-category fixture at `PriorityClass`, not `MotiveScore`, before `SourceComposite` is eligible.
5. Live `GoalKind` and operator surfaces: AcquireCommodity (resource extraction), Wash (place-dirtiness consumption), Drink (water acquisition + consume), Sleep (recovery). All confirmed live by the existing survival golden harness.
6. D6 no-record neutrality exposed a narrow production contradiction: a missing `ReliabilityRecord` previously omitted `SourceCompositeRank`, while comparator defaults could treat the missing source as `0` instead of neutral. This ticket fixes that by emitting neutral 1000-permille factors for no-record sources.
7. Positive low-capacity observations are smaller bonuses above neutral, not `[500, 1000)` penalties. Only empty-but-fresh observations demote into `[500, 1000)`, and the fixture keeps the authoritative source quantity positive so candidate generation still lawfully includes it.
8. Scenario isolation choice: the new file's six scenarios deliberately exclude political/social/combat affordances that could lawfully compete for the agent's tick budget. Scenarios use minimal worlds so the only contended dimension is the source choice.

## Architecture Check

1. A dedicated golden file (`golden_source_composite.rs`) keeps the new contracts grouped; reviewers can read them as a single coherent set. Alternatives considered: (i) folding into `golden_source_reliability.rs` — rejected because that file's contract is the write paths (S131); the composite is the consumer surface (S133), and conflating them obscures responsibility. (ii) Folding into the four survival goldens — rejected because the survival goldens already serve as cross-category-neutrality regression guards; the composite-firing scenarios deserve their own crisp contract.
2. No backward-compat shim. The file is net-new.

## Verification Layers

1. Same-commodity wait reranking → golden E2E ranked trace (far orchard top-ranked); decision trace attributes `RankedGoalComparisonDimension::SourceComposite`.
2. Cross-category neutrality → golden E2E ranked trace (Wash beats AcquireCommodity when priority/motive rank is higher); decision trace attributes `PriorityClass` for this live fixture and never reaches `SourceComposite`.
3. Fresh capacity bonus → golden E2E with two equal-distance orchards differing only in `last_observed_capacity` (18 vs 4, both fresh); higher-capacity wins via `SourceComposite`.
4. Stale capacity neutrality → golden E2E; trace records `capacity_factor_permille = 1000` for the stale source.
5. Empty-but-fresh demotion → golden E2E; trace records `capacity_factor_permille` between 500 and 1000 for the depleted source.
6. No-record neutrality → golden E2E with a fresh agent; both sources resolve to `composite_permille = 1000`; comparator either records no decisive top comparison or falls through to `PlaceKey`/`EntityKey` for deterministic ordering.

## What to Change

### 1. Create `crates/worldwake-ai/tests/golden_source_composite.rs`

Six `#[test] fn ...` scenarios per spec D6:

1. `same_commodity_wait_reranking_picks_far_orchard_when_close_orchard_has_observed_waits`
   - Two orchards (close and far) with equal travel cost (or accept that travel cost does NOT enter the composite per Design Goal 2 — only `(commodity, purpose, desired_target)` peer-key match gates).
   - Close orchard: three `observe_wait(30)` events recorded on agent's `SourceReliability`.
   - Far orchard: no wait observations.
   - `wait_sensitivity_weight = pm(800)`.
   - Expect: FAR orchard is top-ranked for AcquireCommodity; trace attributes `RankedGoalComparisonDimension::SourceComposite`; close orchard's `wait_factor_permille` < 1000.

2. `cross_category_neutrality_keeps_wash_above_acquire_apple_when_motive_higher`
   - One orchard at the agent's place (high capacity), one critical-dirtiness wash basin.
   - Wash motive > AcquireCommodity motive.
   - Expect: agent selects Wash; trace attributes `PriorityClass` in the live fixture; the comparator never reaches `SourceComposite`.

3. `fresh_capacity_bonus_picks_higher_capacity_orchard_when_motive_tied`
   - Two orchards at equal travel cost, identical `successful_acquisitions`, `last_observed_capacity` 18 vs 4 both fresh.
   - Expect: the 18-capacity orchard is top-ranked for AcquireCommodity; trace attributes `SourceComposite`; both positive-capacity factors are above 1000, with the low-capacity factor below the high-capacity factor.

4. `stale_capacity_observation_returns_neutral_factor`
   - Same as (3) but the high-capacity observation is older than `memory_retention_ticks`.
   - Expect: trace records `capacity_factor_permille = 1000` for the stale source.

5. `empty_but_fresh_observation_demotes_depleted_source`
   - Two orchards at equal cost and both authoritative candidates; one's most recent memory is `last_observed_capacity = 0` and within freshness window.
   - Expect: empty-observed source ranks lower; trace records its `capacity_factor_permille` in `[500, 1000)`.

6. `no_record_neutrality_falls_through_to_lower_tiebreakers`
   - Fresh agent with no `SourceReliability` for either source.
   - Expect: both sources resolve to `composite_permille = 1000`; comparator either records no decisive top comparison or falls through to `PlaceKey`/`EntityKey` for deterministic ordering. Trace never attributes `SourceComposite` as the decisive dimension on the final compare.

Each scenario uses the existing `golden_harness` test infrastructure (`crates/worldwake-ai/tests/golden_harness/`), follows the same fixture style as `golden_source_reliability.rs`, and asserts via the decision-trace summary path.

## Files to Touch

- `crates/worldwake-ai/tests/golden_source_composite.rs` (new)
- `crates/worldwake-ai/src/source_composite.rs`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-scenario-details/source-composite.md` (new)
- Other generated golden-scenario detail pages updated by `scripts/golden_inventory.py`
- `archive/specs/S133-source-composite-tiebreaker.md`
- `specs/IMPLEMENTATION-ORDER.md`
- `archive/tickets/S133SOUCOMTIE-006.md`

## Out of Scope

- Modifying any of the four pre-existing failing survival goldens — they re-assert their original contracts and must stay green untouched.
- Modifying S131 data-path goldens.
- Broad engine changes — covered by tickets 001–005. The only in-scope engine fix is no-record neutral `SourceCompositeRank` emission, discovered by D6.

## Acceptance Criteria

### Tests That Must Pass

1. All six new scenarios pass.
2. Pre-existing survival regression contract: `survival_drive_escalation_lands_row_four`, `survival_offices_proves_force_law_uptake`, `survival_preferences_keeps_proactive_diversification_alive_under_survival`, `survival_tell_lands_row_five` all pass without modification.
3. S131 data-path goldens pass: `resource_extraction_wait_observation_records_when_promoted` and `capacity_observation_records_from_perception` (both in `golden_source_reliability.rs`).
4. Existing suite: `cargo test --workspace`.

### Invariants

1. Cross-category goal ranking (Wash vs AcquireCommodity vs Sleep) is unaffected by source perturbation (Design Goal 1 / spec Non-Goal: cross-goal-kind comparison).
2. The composite never drops candidates from the rank — all sibling AcquireCommodity opportunities remain in the ordered vec (spec Non-Goal: pre-rank candidate deduplication).
3. Determinism: scenario seed is fixed; outputs are reproducible.
4. Information locality (FND-15): all reads are agent-local `SourceReliability` and `PreferenceProfile`; no global state queried in scenario assertions.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_source_composite.rs` — new file, six scenarios above.

### Commands

1. `cargo test -p worldwake-ai --test golden_source_composite` (focused, new file).
2. `cargo test -p worldwake-ai --test golden_survival_drive_escalation` (regression contract).
3. `cargo test -p worldwake-ai --test golden_survival_offices` (regression contract).
4. `cargo test -p worldwake-ai --test golden_survival_preferences` (regression contract).
5. `cargo test -p worldwake-ai --test golden_survival_tell` (regression contract).
6. `cargo test -p worldwake-ai --test golden_source_reliability` (S131 data-path goldens stay green).
7. `cargo test --workspace` (full).
8. `./scripts/verify.sh` (final pre-PR gate per CLAUDE.md).

## Implementation Notes

- Added `crates/worldwake-ai/tests/golden_source_composite.rs` with scenarios 375-380.
- Fixed `source_composite_rank` so missing source reliability records produce neutral `SourceCompositeRank` factors (`trust=wait=capacity=composite=1000`) instead of omitting the rank.
- Regenerated golden inventory docs. The generator also corrected `golden_source_reliability.rs` inventory counts to the two live S131 data-path tests.

## Verification Result

- `cargo test -p worldwake-ai --test golden_source_composite -- --list`
- `cargo test -p worldwake-ai --test golden_source_composite`
- `cargo test -p worldwake-ai --lib source_composite::tests`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai --test golden_source_reliability`
- `cargo test -p worldwake-ai --test golden_survival_drive_escalation`
- `cargo test -p worldwake-ai --test golden_survival_drive_escalation survival_drive_escalation_lands_row_four -- --ignored --exact`
- `cargo test -p worldwake-ai --test golden_survival_offices survival_offices_proves_force_law_uptake -- --ignored --exact`
- `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact`
- `cargo test -p worldwake-ai --test golden_survival_tell survival_tell_lands_row_five -- --ignored --exact`
- `./scripts/verify.sh`

## Outcome

Completed: 2026-05-04.

Delivered S133 D6 by adding `crates/worldwake-ai/tests/golden_source_composite.rs` with scenarios 375-380 and regenerating the golden inventory, index, matrix, and per-file detail pages. The delivered proof surface is the decision-trace ranked summary and comparator attribution, not a stronger action-commit source claim.

Post-review seam resumed: Scenario 380 metadata now matches the executable assertion that neutral source composites do not decide the final ordering, and generated scenario detail/index prose was regenerated from that corrected metadata.

The ticket also absorbed one narrow production fix exposed by D6: `source_composite_rank` now emits neutral `SourceCompositeRank` factors for missing per-source reliability records, so no-record sources compare neutrally instead of falling through to a missing-rank default.

Deviations from the draft:

- The cross-category neutrality fixture resolves at `PriorityClass` in the live comparator before `SourceComposite` is eligible, not at `MotiveScore`.
- Positive low-capacity observations are smaller bonuses above neutral; only empty-but-fresh observations demote below neutral.
- Empty-but-fresh remains a candidate by keeping authoritative source quantity positive while the agent memory says the source was empty.
- No-record neutrality may produce no decisive top comparison or a lower deterministic tiebreaker; the invariant is that `SourceComposite` is not decisive when both composites are neutral.

Verification:

- `cargo test -p worldwake-ai --test golden_source_composite -- --list`
- `cargo test -p worldwake-ai --test golden_source_composite`
- `cargo test -p worldwake-ai --lib source_composite::tests`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `python3 scripts/golden_inventory.py --write --check-docs` (post-review Scenario 380 metadata correction)
- `cargo test -p worldwake-ai --test golden_source_reliability`
- `cargo test -p worldwake-ai --test golden_survival_drive_escalation`
- `cargo test -p worldwake-ai --test golden_survival_drive_escalation survival_drive_escalation_lands_row_four -- --ignored --exact`
- `cargo test -p worldwake-ai --test golden_survival_offices survival_offices_proves_force_law_uptake -- --ignored --exact`
- `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact`
- `cargo test -p worldwake-ai --test golden_survival_tell survival_tell_lands_row_five -- --ignored --exact`
- `./scripts/verify.sh`
