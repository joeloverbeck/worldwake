# S103BELCLADED-004: Canonicalize claim-backed semantic entity belief updates

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief update boundary (worldwake-core, worldwake-systems)
**Deps**: S101 (completed), S103BELCLADED-001 (completed)

## Problem

`AgentBeliefStore` currently allows the same semantic entity fact to travel through two storage paths:

1. `entity_claims` with provenance and timing
2. direct `known_entities` writes through helpers like `update_entity`, `update_believed_activity`, and `update_departure_projection`, plus direct evidence mutation in investigation code

That duplication makes `known_entities` more than a derived summary cache. It also makes `prune_decayed_beliefs` semantically load-bearing, because unconditional refresh is currently reconciling summary-only writes back against claims. The result violates the intended `entity_claims` -> summary architecture and blocks `S103BELCLADED-002` from being a lawful FND-12 optimization.

## Assumption Reassessment (2026-04-14)

1. `derive_entity_summary` at `crates/worldwake-core/src/belief.rs:1876` already derives `Location`, `Inventory`, `Alive`, `Wounded`, `Activity`, `WorkstationPresent`, `ResourceAvailable`, `ContentionState`, `ArtifactState`, `Courage`, and `Evidence` from `EntityBeliefClaim` winners — verified.
2. `EntityBeliefAspect` and `ClaimValue` in `crates/worldwake-core/src/entity_belief_claim.rs:15` already have explicit variants for `Location`, `Activity`, `ContentionState`, `ArtifactState`, and `Evidence` — verified.
3. Production code still mutates `known_entities` semantics outside claims:
   - `update_believed_activity` and `update_departure_projection` in `crates/worldwake-core/src/belief.rs:692` and `:717`
   - witness/report snapshot imports via `update_entity` call sites such as `crates/worldwake-systems/src/epistemic_actions.rs:357`
   - direct observation place refresh via `update_entity` in `crates/worldwake-systems/src/investigate_actions.rs:164`
4. The earlier cited `belief.believed_evidence = ...` write in `crates/worldwake-systems/src/investigate_actions.rs:898` is test-only (`#[cfg(test)] mod tests`), not a live production mutation path. Correction applied: keep the ticket focused on the real production semantic writers above and prove evidence carriage at the `AgentBeliefStore` layer instead of over-claiming an `investigate_actions` production evidence rewrite.
5. The motivating mixed-layer failure is `guard_theron_water_at_thornwall_finds_harvest_plan` in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`. The intended invariant is not “that golden must pass for any implementation,” but “unchanged claims must imply unchanged planner-visible summaries.” The failed optimization proved that invariant is currently false.
6. The exact shared abstraction boundary under audit is `AgentBeliefStore::{entity_claims, known_entities}`. After this ticket, claim-backed aspects must have one canonical semantic transport path: explicit `EntityBeliefClaim` storage followed by `refresh_entity_summary_from_claims`.
7. Direct presentation/history reinforcement is still lawful on `known_entities`, but semantic fields that already have `EntityBeliefAspect` coverage are not. The likely clean seam is: claim-writing helpers for semantic updates plus a separate presentation-only reinforcement helper for departure/local-refresh paths.
8. Adjacent contradiction classification:
   - required consequence of this ticket: remove duplicate semantic summary mutation paths for claim-backed aspects
   - future cleanup, out of scope here: making `known_entities` private or broadly refactoring all non-semantic helper surfaces

## Architecture Check

1. Canonicalizing semantic updates through claims is cleaner than adding dirty bits or “also refresh when summary helpers ran.” Dirty tracking would preserve two competing sources of truth instead of removing the contradiction.
2. No backward-compatibility shims. Call sites should stop transporting semantic facts through direct summary mutation for claim-backed fields, rather than adding mirrored writes to both paths.

## Verification Layers

1. Activity, departure projection, evidence, and snapshot-import semantics survive pruning through claims -> focused `worldwake-core` unit tests on `AgentBeliefStore`
2. Perception/investigation/witness call sites still populate planner-visible beliefs after moving to claims -> mixed focused proof (`worldwake-core` storage coverage plus the existing Guard Theron golden)
3. Planner-visible regression is removed -> `guard_theron_water_at_thornwall_finds_harvest_plan`
4. Full AI behavior remains stable after the boundary cleanup -> `cargo test -p worldwake-ai`
5. Strongest proof surface is mixed: unit coverage for the storage contract and golden coverage for planner-visible stability. No additional traceability ticket is required because the storage boundary itself is inspectable in `worldwake-core`

## What to Change

### 1. Add claim-backed helpers for semantic entity belief updates

In `AgentBeliefStore`, add or repurpose helpers that record semantic updates as `EntityBeliefClaim`s with explicit:

- subject
- aspect/value
- `PerceptionSource`
- learned tick
- claimed event tick when needed
- confidence derived from `BeliefConfidencePolicy`

These helpers should refresh the summary from claims after writing.

### 2. Move production semantic call sites onto claim-backed helpers

Update production callers that currently mutate summary-only semantics:

- co-located activity observation in `crates/worldwake-systems/src/perception.rs`
- departure projection in `crates/worldwake-systems/src/perception.rs`
- witness/report snapshot transfer paths that currently use `update_entity` (currently `crates/worldwake-systems/src/epistemic_actions.rs`; broader tell/report-style action files remain out of scope unless verification proves they are live production fallout for this invariant)
- direct observation place refresh in `crates/worldwake-systems/src/investigate_actions.rs`

### 3. Leave `known_entities` for derived summaries and presentation-only state

After the call-site migration, `known_entities` should only be changed for:

- summary derivation / refresh
- activation-based eviction
- presentation/history preservation
- non-semantic fallback metadata that still intentionally sits outside claims

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/entity_belief_claim.rs` (modify only if helper surface requires it)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)

## Out of Scope

- The changed-entity pruning optimization itself (`S103BELCLADED-002`)
- Social observation deduplication (`S103BELCLADED-003`)
- Broad encapsulation refactors unrelated to claim-backed semantics

## Acceptance Criteria

### Tests That Must Pass

1. New focused core tests prove claim-backed storage for activity, departure projection, evidence carriage, and imported snapshot semantics
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. For fields already represented by `EntityBeliefAspect`, `entity_claims` is the only semantic storage path
2. After pruning with unchanged claims, semantic `known_entities` content is unchanged because it is claim-derived

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — focused tests for claim-backed activity, departure projection, evidence carriage, and imported snapshot stability across prune
2. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — existing Guard Theron golden named as the mixed-layer regression proof surface
3. `None — existing Guard Theron golden plus `worldwake-core` storage tests are the planned proof surfaces unless implementation fallout proves a dedicated systems test is necessary.`

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::`
2. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

**Completion date**: 2026-04-14

1. `AgentBeliefStore` now has claim-backed helper paths for semantic updates:
   - activity changes and clears now record `EntityBeliefAspect::Activity` claims
   - departure projection now records `EntityBeliefAspect::Location` claims and only reinforces presentation history separately
   - imported snapshots now record claims and then restore presentation history metadata instead of reintroducing summary-only semantic drift
2. Production call sites in `perception.rs`, `epistemic_actions.rs`, and `investigate_actions.rs` were moved onto the claim-backed helper surface for the in-scope semantic update paths.
3. Focused `worldwake-core` tests now prove that activity, departure projection, and imported snapshot semantics survive pruning through claims.
4. **Deviation from original plan**: broadened verification exposed two adjacent correctness issues that were fixed instead of masked:
   - `ConsumeOwnedCommodity` candidate generation was narrowed to locally owned or directly possessed stock, leaving unowned local lots on the `AcquireCommodity` path
   - the Guard Theron golden snapshot helper was only clearing ownership, not direct possession, so the test setup now explicitly drops possession when it intends to place carried water on the ground

## Verification

1. `cargo test -p worldwake-core --lib belief::tests::`
2. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
