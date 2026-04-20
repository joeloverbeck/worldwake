# AIDECREG-002: Reassess and fix `golden_discrepancy_memory_with_ttl_expiry`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — reassessment shows this is a stale golden proof seam; direct local depletion never reaches a failed harvest boundary, so no blocker/discrepancy memory should be recorded at all.
**Deps**: None

## Problem

Broader verification after `S122FRAASSCOM-005` still fails in `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_discrepancy_memory_with_ttl_expiry` with `Agent should record a discrepancy when the depleted orchard source blocks harvest`. This is outside the S122 commodity-assumption probe ticket, but it blocks honest same-crate verification and indicates either a real regression in depleted-source handling or a stale golden contract in the older AI decisions scenario.

## Assumption Reassessment (2026-04-20)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai --test golden_ai_decisions golden_discrepancy_memory_with_ttl_expiry`; this is not a broad-suite artifact.
2. The failing scenario lives in `crates/worldwake-ai/tests/golden_ai_decisions.rs` and sets up a depleted orchard `ResourceSource` at `ORCHARD_FARM` with `available_quantity: Quantity(0)` and `regeneration_ticks_per_unit: Some(nz(5))`, then expects both (a) discrepancy recording while the source is depleted and (b) eventual harvest after regeneration.
3. Archived `archive/tickets/AIDECREG-001.md` previously repaired the neighboring `golden_blocked_intent_memory_with_ttl_expiry` scenario in the same file. That earlier ticket proves this family of tests can drift independently and should be reassessed at the live mixed-layer boundary rather than patched by analogy.
4. Shared abstraction boundary under audit: authoritative `ResourceSource` depletion/regeneration, AI-side failure classification for depleted harvest sources, and the golden’s proof surface for observing the canonical recorded carrier.
5. Live code in `crates/worldwake-ai/src/failure_handling.rs` would classify an actual failed harvest on a zero-quantity source as `BlockingFact::SourceDepleted`, and `handle_plan_failure()` would persist that via `BlockerMemory`, not `DiscrepancyMemory`.
6. However, the scenario seeds direct local observation of `available_quantity: 0`, and candidate generation only emits acquisition opportunities for resource sources whose believed quantity is already sufficient. In the live path, this means no harvest plan is attempted while the source is known-empty, so the scenario never reaches any failure-memory boundary at all.
7. The drafted discrepancy assertion is therefore stale at a deeper level: the honest invariant is “direct local depletion does not create spurious blocker/discrepancy memory, regeneration restores availability, and the agent eventually harvests once the source becomes actionable.”
8. Intended layer is mixed: authoritative world state must prove the source is depleted/regenerates; focused memory inspection must prove no spurious blocker/discrepancy is recorded before harvest becomes actionable; the golden must still prove the authored regeneration chain through to eventual harvest.

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than folding this unrelated failure into S122 or ignoring it as ambient suite noise.
2. The earliest honest contradiction is the golden’s stale proof seam, not the production path. The fix should retarget the scenario away from failure memory entirely and preserve the real local-regeneration contract.

## Verification Layers

1. Depleted orchard source exists and later regenerates -> authoritative world state / focused lower-layer proof
2. AI does not record spurious blocker/discrepancy memory while the locally observed source remains known-empty and no failed harvest step occurs -> focused memory inspection at the true boundary
3. Scenario still reaches eventual harvest after regeneration -> repaired `golden_ai_decisions` scenario
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`, with any newly exposed unrelated blocker isolated and handed off explicitly

## What to Change

### 1. Reassess the failing golden against live code

- Name the exact failure-classification symbols and the first live failure boundary for a depleted orchard source.
- Correct the ticket/golden from discrepancy-memory wording to the real local-regeneration path if reassessment proves no failure boundary is reached in this scenario.

### 2. Land the smallest honest fix

- Tighten the scenario so it observes the live no-spurious-memory behavior honestly while preserving the authored regeneration chain.
- Do not change production code unless reassessment proves local depletion is mishandled in the live path.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/src/...` (no changes expected after reassessment)

## Out of Scope

- S122 commodity-assumption probes
- Broad cleanup of unrelated `golden_ai_decisions` scenarios
- New discrepancy taxonomy work beyond the failing depleted-orchard path

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_local_depleted_source_regenerates_without_spurious_failure_memory`
2. Any new focused regression added at the true boundary
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p worldwake-ai` is rerun as a broader blocker sweep; any still-unrelated failing case must be isolated and handed off explicitly instead of silently absorbed

### Invariants

1. The final fix preserves an honest local-depletion contract for directly observed empty sources rather than papering over the failure with a weaker assertion.
2. The golden proves that no blocker/discrepancy memory is recorded before any failed harvest step exists.
3. Eventual harvest after regeneration remains part of the scenario contract unless reassessment proves the authored invariant itself was wrong.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_local_depleted_source_regenerates_without_spurious_failure_memory` — repaired local-regeneration golden with honest proof surface
2. Focused lower-layer regression at the true boundary — exact test to be chosen during reassessment if production code changes

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_local_depleted_source_regenerates_without_spurious_failure_memory`
2. `<exact focused regression command added during implementation if production code changes>`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

1. Reassessed the stale `golden_discrepancy_memory_with_ttl_expiry` contract against the live candidate-generation and failure-handling path.
2. Confirmed this scenario never reaches a failed harvest boundary because direct local observation of `available_quantity: 0` suppresses acquisition opportunity emission until regeneration makes the source actionable.
3. Replaced the stale memory assertion with an honest local-regeneration scenario in `crates/worldwake-ai/tests/golden_ai_decisions.rs`, renamed to `golden_local_depleted_source_regenerates_without_spurious_failure_memory`.

## Deviations

1. Reassessment first narrowed the scenario from discrepancy memory to blocker memory, then corrected it a second time once the live candidate-generation path proved the scenario never reaches any failure-memory boundary at all.
2. No production code changes landed; the honest fix was a golden-contract rewrite and test rename only.

## Verification Result

1. Passed: `cargo test -p worldwake-ai --test golden_ai_decisions golden_local_depleted_source_regenerates_without_spurious_failure_memory`
2. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
3. Broader same-crate rerun still fails in unrelated existing pathology coverage: `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals`
