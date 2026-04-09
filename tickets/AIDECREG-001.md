# AIDECREG-001: Reassess and fix `golden_blocked_intent_memory_with_ttl_expiry`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` and/or golden contract reassessment depending on live root cause
**Deps**: None

## Problem

The broader `cargo test -p worldwake-ai` suite currently fails in `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_blocked_intent_memory_with_ttl_expiry` with `Agent should eventually harvest apples after resource regeneration`. This is not owned by S76, but it blocks honest same-crate full-suite verification and indicates either a real regression in the blocked-intent/resource-regeneration path or a stale golden contract.

## Assumption Reassessment (2026-04-09)

1. The failing scenario lives in `crates/worldwake-ai/tests/golden_ai_decisions.rs` around the “Goal Invalidation / blocked intent TTL” coverage and currently expects a depleted orchard source with `regeneration_ticks_per_unit: Some(nz(5))` to regenerate and produce apple lots within 200 ticks.
2. The failure reproduces in isolation with `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`, so this is not an artifact of broad-suite contention.
3. Archived ticket `archive/tickets/E16DPOLPLAN-019.md` previously listed this test as passing, so the current failure represents either later runtime drift or a stale scenario assumption.
4. The live boundary under audit is mixed-layer: resource-source regeneration in authoritative state, candidate/planner behavior around depleted sources, and the golden’s proof surface about eventual harvest after regeneration.
5. The ticket must reassess first whether the intended invariant is still “TTL expiry enables eventual harvest after source regeneration” or whether the current architecture lawfully requires a different proof surface or setup math.

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than treating the failing test as incidental verification noise.
2. The ticket should fix the earliest concrete contradiction: stale golden setup if the invariant is still true, or production behavior if the regeneration/blocked-intent path has actually regressed.

## Verification Layers

1. Resource regeneration occurs authoritatively -> authoritative world state / focused lower-layer proof
2. AI revisits the opportunity after blocker expiry or condition change -> decision trace and/or focused runtime coverage
3. Golden eventual-harvest contract remains valid -> `golden_blocked_intent_memory_with_ttl_expiry`
4. Same-crate broader verification recovers -> `cargo test -p worldwake-ai`

## What to Change

### 1. Reassess the failing golden against live code

- Name the exact authoritative regeneration symbols and blocked-intent invalidation symbols under audit.
- Determine whether the current failure is stale setup, stale assertion surface, or a production regression.

### 2. Land the smallest honest fix

- If the golden setup is stale, update it to match live arithmetic/affordance behavior.
- If production behavior regressed, fix the earliest concrete layer and keep the golden honest.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify) and/or the exact owning production files revealed by reassessment

## Out of Scope

- S76 simulation-gap scenarios
- Broad golden-suite cleanup unrelated to this failing path
- Documentation-only alignment work for S76

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves the honest contract for resource regeneration and blocked-intent expiry rather than papering over the failure
2. If the golden changes, its scenario prose and assertion surface match the live causal boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_blocked_intent_memory_with_ttl_expiry` — repaired broader-suite blocker
2. Additional focused lower-layer tests only if reassessment proves the current golden lacks enough provenance

### Commands

1. `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
