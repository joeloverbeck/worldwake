# HARPREE14-008: Decouple enterprise module from candidate generation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- API change between enterprise and candidate_generation
**Deps**: HARPREE14-006 (HARDEN-A02 must be done first)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A03

## Problem

`candidate_generation.rs` imports `crate::enterprise::restock_gap` and `crate::enterprise::opportunity_signal` directly. The enterprise module's internal API becomes a coupling point -- changes to enterprise internals force changes in candidate generation.

## Assumption Reassessment (2026-03-11)

1. `candidate_generation.rs` currently imports `crate::enterprise::restock_gap`, but it does not import `opportunity_signal` -- corrected
2. HARPREE14-006 is already completed; the relevant domain seam is `emit_enterprise_candidates()` plus the production emitter path that still consults enterprise restock analysis -- corrected
3. Candidate generation still depends on enterprise restock analysis in two places:
   - `emit_restock_goals()` decides whether to emit `RestockCommodity`
   - `emit_produce_goals()` decides whether a recipe output serves restocking
4. `opportunity_signal()` and `market_signal_for_place()` are still used by `ranking.rs`, not by candidate generation -- corrected
5. Candidate-generation unit coverage already exists in `crates/worldwake-ai/src/candidate_generation.rs`, including restock coverage, plus golden e2e coverage in `crates/worldwake-ai/tests/golden_e2e.rs` -- corrected

## Architecture Check

1. The hardening goal is still valid, but the boundary should be narrower than the original ticket implied: decouple candidate generation from enterprise restock analysis, not from all enterprise helpers.
2. The cleanest boundary is an explicit enterprise-analysis data object computed once in `generate_candidates()` and then passed through `GenerationContext` to the enterprise and production emitters.
3. This improves architecture beyond mere import cleanup:
   - candidate generation stops reaching into enterprise internals from lower-level emitters
   - enterprise analysis is computed once per tick instead of being re-derived ad hoc in multiple emitters
   - future enterprise signals can extend a single data contract without re-coupling emitter internals
4. `ranking.rs` should remain out of scope. It still legitimately consumes enterprise opportunity signals for ranking, and this ticket should not broaden into ranking refactors.

## What to Change

### 1. Define enterprise analysis data for candidate generation

Add a small data type in `enterprise.rs` representing the candidate-generation-relevant enterprise outputs for one agent tick. The current required payload is restock-gap information keyed by commodity. Do not pull ranking-only opportunity signals into this ticket unless they are needed by the final design.

### 2. Move enterprise analysis to the top-level orchestrator

Compute the enterprise analysis once in `generate_candidates()` and thread it through `GenerationContext` so lower-level emitters consume data instead of calling `restock_gap()` directly.

### 3. Refactor both restock and production emitters to consume the data boundary

Update:
- `emit_restock_goals()` to use precomputed enterprise restock data
- `emit_produce_goals()` to use the same precomputed restock data when deciding whether recipe outputs serve restocking

### 4. Remove direct enterprise-analysis calls from candidate-generation internals

After the change, lower-level emitters in `candidate_generation.rs` must not call `restock_gap()` directly. The top-level orchestrator may call a single enterprise-analysis entrypoint.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/enterprise.rs` (modify -- add signal type or adjust exports)

## Out of Scope

- Changing enterprise ranking behavior in `ranking.rs`
- Changing `opportunity_signal()` semantics
- Adding new enterprise goal kinds
- Modifying `generate_candidates()` public signature
- Broad refactors outside `candidate_generation.rs` and `enterprise.rs` unless a focused test update is required

## Acceptance Criteria

### Tests That Must Pass

1. Existing candidate-generation tests pass
2. Existing enterprise/ranking tests that exercise enterprise signals pass
3. Golden e2e hashes remain identical
4. `cargo test --workspace` passes
5. `cargo clippy --workspace --all-targets -- -D warnings` passes

### Invariants

1. `generate_candidates()` public signature unchanged
2. Same candidates generated for identical inputs
3. Enterprise restock analysis logic unchanged
4. Golden e2e state hashes identical

## Test Plan

### New/Modified Tests

1. Add a focused unit test proving candidate generation can emit enterprise-driven goals from precomputed enterprise analysis data without lower-level emitters consulting `restock_gap()` directly.
2. Strengthen or adjust the existing restock/production candidate tests if needed so they cover the shared precomputed data path.

### Commands

1. `cargo test -p worldwake-ai candidate` (targeted)
2. `cargo test -p worldwake-ai enterprise` (targeted coverage for enterprise helpers still in scope)
3. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-11
- What actually changed:
  - Corrected the ticket scope to match the current codebase: only enterprise restock analysis remained coupled to candidate generation, and `opportunity_signal()` stayed correctly scoped to ranking.
  - Added `EnterpriseSignals` plus `analyze_candidate_enterprise()` in `crates/worldwake-ai/src/enterprise.rs` as the explicit enterprise-to-candidate-generation data boundary.
  - Updated `generate_candidates()` to compute enterprise analysis once and store it in `GenerationContext`.
  - Updated `emit_restock_goals()` and `emit_produce_goals()` to consume precomputed enterprise signals instead of calling enterprise analysis helpers directly.
  - Removed the now-unused `restock_gap()` helper instead of keeping a dead compatibility surface.
  - Added a focused unit test proving the enterprise emitters depend on precomputed signals rather than direct enterprise calls.
- Deviations from original plan:
  - Did not introduce opportunity-signal data into the candidate-generation boundary because ranking, not candidate generation, is the remaining consumer.
  - Removed the obsolete helper instead of preserving it, consistent with the repo's no-backward-compatibility rule.
- Verification results:
  - `cargo test -p worldwake-ai candidate -- --nocapture` passed
  - `cargo test -p worldwake-ai enterprise -- --nocapture` passed
  - `cargo test -p worldwake-ai --test golden_e2e -- --nocapture` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
