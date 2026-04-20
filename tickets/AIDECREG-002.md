# AIDECREG-002: Reassess and fix `golden_discrepancy_memory_with_ttl_expiry`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — mixed-layer reassessment likely touching `worldwake-ai` discrepancy recording and/or the golden proof surface
**Deps**: None

## Problem

Broader verification after `S122FRAASSCOM-005` still fails in `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_discrepancy_memory_with_ttl_expiry` with `Agent should record a discrepancy when the depleted orchard source blocks harvest`. This is outside the S122 commodity-assumption probe ticket, but it blocks honest same-crate verification and indicates either a real regression in discrepancy recording for depleted resource sources or a stale golden contract in the older AI decisions scenario.

## Assumption Reassessment (2026-04-20)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai --test golden_ai_decisions golden_discrepancy_memory_with_ttl_expiry`; this is not a broad-suite artifact.
2. The failing scenario lives in `crates/worldwake-ai/tests/golden_ai_decisions.rs` and sets up a depleted orchard `ResourceSource` at `ORCHARD_FARM` with `available_quantity: Quantity(0)` and `regeneration_ticks_per_unit: Some(nz(5))`, then expects both (a) discrepancy recording while the source is depleted and (b) eventual harvest after regeneration.
3. Archived `archive/tickets/AIDECREG-001.md` previously repaired the neighboring `golden_blocked_intent_memory_with_ttl_expiry` scenario in the same file. That earlier ticket proves this family of tests can drift independently and should be reassessed at the live mixed-layer boundary rather than patched by analogy.
4. Shared abstraction boundary under audit: authoritative `ResourceSource` depletion/regeneration, AI-side opportunity/discrepancy recording when a depleted local source blocks harvest, and the golden’s proof surface for observing that discrepancy.
5. Intended layer is mixed: authoritative world state must prove the source is depleted/regenerates; focused runtime or decision/discrepancy inspection must prove the AI records the right typed discrepancy; the golden must prove the authored causal chain honestly rather than only eventual harvest.
6. Reassessment must explicitly determine whether the missing discrepancy is caused by a production regression in the current discrepancy-memory path, a lawful change in the first failure boundary, or stale golden setup/assertions that no longer observe the live canonical carrier.

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than folding this unrelated failure into S122 or ignoring it as ambient suite noise.
2. The fix should target the earliest honest contradiction: either the production discrepancy recording path if it regressed, or the golden’s proof seam if the scenario no longer observes the canonical discrepancy carrier.

## Verification Layers

1. Depleted orchard source exists and later regenerates -> authoritative world state / focused lower-layer proof
2. AI records a typed discrepancy when the depleted local source blocks harvest -> focused runtime/discrepancy-memory proof or decision trace at the true failure boundary
3. Scenario still reaches eventual harvest after regeneration -> `golden_discrepancy_memory_with_ttl_expiry`
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`, with any newly exposed unrelated blocker isolated and handed off explicitly

## What to Change

### 1. Reassess the failing golden against live code

- Name the exact discrepancy-recording symbols and the first live failure boundary when a depleted orchard source blocks the local harvest path.
- Determine whether the missing discrepancy is a production regression, a moved failure boundary, or a stale golden proof seam.

### 2. Land the smallest honest fix

- If production no longer records the intended discrepancy at the canonical boundary, fix that path narrowly.
- If the golden is asserting against the wrong proof surface, tighten the scenario so it observes the live discrepancy carrier honestly while preserving the authored regeneration chain.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/src/...` (modify only if reassessment proves the production discrepancy path is wrong)

## Out of Scope

- S122 commodity-assumption probes
- Broad cleanup of unrelated `golden_ai_decisions` scenarios
- New discrepancy taxonomy work beyond the failing depleted-orchard path

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_discrepancy_memory_with_ttl_expiry`
2. Any new focused regression added at the true discrepancy boundary
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p worldwake-ai` is rerun as a broader blocker sweep; any still-unrelated failing case must be isolated and handed off explicitly instead of silently absorbed

### Invariants

1. The final fix preserves an honest discrepancy-recording contract for depleted local harvest sources rather than papering over the failure with a weaker assertion.
2. The golden’s discrepancy assertion observes the live canonical carrier for that contradiction.
3. Eventual harvest after regeneration remains part of the scenario contract unless reassessment proves the authored invariant itself was wrong.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_discrepancy_memory_with_ttl_expiry` — repaired discrepancy/regeneration golden with honest proof surface
2. Focused lower-layer regression at the true discrepancy boundary — exact test to be chosen during reassessment if production code changes

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_discrepancy_memory_with_ttl_expiry`
2. `<exact focused regression command added during implementation if production code changes>`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
