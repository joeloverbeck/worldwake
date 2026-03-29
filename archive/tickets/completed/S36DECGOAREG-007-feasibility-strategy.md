# S36DECGOAREG-007: Introduce FeasibilityStrategy and migrate feasibility dispatch

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` declaration-owned feasibility dispatch and focused regression coverage
**Deps**: S36DECGOAREG-002

## Problem

`goal_specific_feasibility()` in `feasibility.rs` still owns a monolithic exhaustive `GoalKind` match even though the rest of S36's dispatch architecture already routes static metadata and invalidation through `GoalDispatchKey` -> `GoalDispatchDeclaration`. Feasibility is now the remaining live dispatch surface that should be declaration-owned, following the same pattern as `InvalidationStrategy` (006).

## Assumption Reassessment (2026-03-29)

1. Shared abstraction boundary under audit: the S36 declaration routing contract between [`crates/worldwake-ai/src/goal_dispatch_key.rs`](crates/worldwake-ai/src/goal_dispatch_key.rs), [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](crates/worldwake-ai/src/goal_dispatch_decl.rs), and [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs). The ticket is not changing authoritative goal identity in `worldwake-core`; it is migrating one remaining AI-local dispatch surface onto the existing declaration table.
2. `goal_specific_feasibility()` is still the live per-goal dispatch point in [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs). It is called from `feasibility_hint()` after the shared Phase 1 checks (`check_exhausted_frame`, `check_blocker_memory`) and still returns `Option<FeasibilityHint>`.
3. The current `GoalDispatchDeclaration` in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](crates/worldwake-ai/src/goal_dispatch_decl.rs) already owns `trace_label`, `provenance_family`, `relevant_ops`, and `invalidation_strategy`. The original ticket text was stale in treating the declaration struct as not yet extended. This ticket must add a fifth field, `feasibility_strategy`, rather than introducing the declaration pattern from scratch.
4. `GoalDispatchKey` already exists and is exhaustive over 24 live dispatch-distinguishing goal shapes in [`crates/worldwake-ai/src/goal_dispatch_key.rs`](crates/worldwake-ai/src/goal_dispatch_key.rs). The feasibility migration should route from `GoalDispatchKey::from_goal_kind(&goal.grounded.key.kind).declaration().feasibility_strategy`, not add a second identity surface.
5. The current `goal_specific_feasibility()` match body groups goals by real strategy families. Live families validated against [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs):
   - **OwnedCommodityCheck**: `ConsumeOwnedCommodity` — checks `commodity_quantity > 0`.
   - **EvidencePlaceLocal**: `AcquireCommodity`, `ProduceCommodity`, `RestockCommodity`, `LootCorpse`, `ClaimOffice` — checks `check_evidence_places_local()`.
   - **AlwaysLikely**: `Sleep`, `Relieve` — returns `Likely` unconditionally.
   - **CommodityPresenceCheck**: `Wash` — checks water quantity.
   - **ColocationOrDead**: `EngageHostile`, `TreatWounds`, `ShareBelief`, `SupportCandidateForOffice`, `Accuse`, `PunishAccused` — checks `check_colocated_or_dead()`.
   - **NoOpinion**: `ReduceDanger`, `StealItem` — returns `None` (falls through to `Uncertain`).
   - **SellCheck**: `SellCommodity` — checks commodity presence, then evidence places.
   - **CargoDestinationCheck**: `MoveCargo` — checks commodity + destination adjacency.
   - **CorpseBurialCheck**: `BuryCorpse` — checks corpse co-location + burial site adjacency.
   - **PlaceMatch**: `InvestigateViolation` — checks agent at violation place.
6. Existing helper functions in [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs) (`check_evidence_places_local`, `check_colocated_or_dead`) are already the correct concrete strategy implementations and should be reused.
7. Existing focused feasibility coverage is broader than the original ticket assumed. `cargo test -p worldwake-ai -- --list` already shows individual tests for exhausted frame, blocker memory, local evidence, commodity presence, co-location/dead checks, sell behavior, investigate behavior, deferred crime defaults, and ordering. The missing proof surface is declaration completeness plus direct strategy-routing coverage, not basic feasibility behavior from scratch.
8. Scope correction: `Engine Changes: None` was incorrect. This is a production AI-architecture refactor inside `worldwake-ai`, even though it should preserve behavior.

## Architecture Check

1. This change is more beneficial than keeping the current architecture because feasibility is the last major dispatch surface still hard-coded outside the S36 declaration table. Leaving it as a free-floating `match GoalKind` preserves real drift risk: adding a new `GoalDispatchKey` today requires reviewing declarations for provenance, relevant ops, and invalidation, but feasibility can still silently diverge in a separate file.
2. Declaration-owned `FeasibilityStrategy` is the cleaner long-term architecture than either the current monolithic match or a helper that reconstructs dispatch from `GoalKindTag`. It keeps one AI-local dispatch identity (`GoalDispatchKey`), one declaration table (`GoalDispatchDeclaration`), and one dynamic strategy-routing pattern for both invalidation and feasibility. That aligns with S36, preserves P3/P25, and avoids introducing aliases or backward-compatibility shims.
3. The strategy enum must remain a compile-time selector only. It should not encode belief data, cached feasibility state, or new abstract summaries. Concrete runtime checks stay in `feasibility.rs` helper functions that consume `GoalBeliefView`, agent identity, and payload-specific goal data.

## Verification Layers

1. Declaration completeness / compile-time routing review -> focused declaration tests in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](crates/worldwake-ai/src/goal_dispatch_decl.rs).
2. Strategy routing equivalence -> focused feasibility unit tests in [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs) that compare declaration-routed dispatch outcomes across representative goal shapes and payload-sensitive splits.
3. Existing behavior preservation at the AI crate layer -> `cargo test -p worldwake-ai`.
4. Workspace regression guard for refactor fallout -> `cargo test --workspace` and `cargo clippy --workspace`.

## What to Change

### 1. Define `FeasibilityStrategy` enum in `goal_dispatch_decl.rs`

One variant per identified feasibility family (see list above).

### 2. Add `feasibility_strategy` field to `GoalDispatchDeclaration`

Extend the existing struct and populate the field in every declaration.

### 3. Refactor `goal_specific_feasibility()` in `feasibility.rs`

Replace the monolithic match with:
1. Look up `GoalDispatchKey::from_goal_kind(&goal.grounded.key.kind).declaration().feasibility_strategy`.
2. Match on the strategy enum to call family-specific helper functions.
3. Helpers still take `(view, agent, goal)` and may inspect concrete payload fields.

### 4. Equivalence and completeness tests

Add focused tests that:
1. prove every declaration uses a `FeasibilityStrategy`,
2. prove payload-sensitive dispatch keys map to the intended shared-or-distinct strategies,
3. prove declaration-routed feasibility returns the same results as the current behavior for representative goal shapes and key edge cases.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add `FeasibilityStrategy` enum, add field to struct, populate in declarations)
- `crates/worldwake-ai/src/feasibility.rs` (modify — refactor main dispatch to strategy routing)
- `crates/worldwake-ai/src/lib.rs` (modify only if re-export of `FeasibilityStrategy` is needed for consistency with other declaration surfaces)

## Out of Scope

- Changing `FeasibilityHint` type or the `feasibility_hint()` function signature
- Modifying shared Phase 1 checks (`check_exhausted_frame`, `check_blocker_memory`)
- Invalidation strategy (ticket 006)
- Wildcard audit (ticket 008)
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. Focused declaration coverage proving every live `GoalDispatchKey` declaration has a `feasibility_strategy` and that payload-sensitive/shared families are assigned intentionally.
2. Focused feasibility coverage proving declaration-routed dispatch preserves current outcomes for representative goal shapes and edge cases.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. Zero intended behavioral change — feasibility hints remain identical for all live goal shapes.
2. Adding or editing a declaration without deliberate feasibility-strategy review fails at the declaration table boundary because the struct field is required.
3. The strategy enum routes to computations; it does not encode belief data directly (P3).
4. Shared Phase 1 checks (`check_exhausted_frame`, `check_blocker_memory`) remain unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — declaration completeness / feasibility-strategy assignment tests.
2. `crates/worldwake-ai/src/feasibility.rs` — strategy-routing equivalence tests and edge-case regressions.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai feasibility::tests`
3. `cargo test -p worldwake-ai goal_dispatch_decl::tests`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- Actual changes:
  - Added `FeasibilityStrategy` to [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](crates/worldwake-ai/src/goal_dispatch_decl.rs) and required every live `GoalDispatchDeclaration` to choose a feasibility routing family.
  - Refactored [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs) so `goal_specific_feasibility()` now dispatches through `GoalDispatchKey -> GoalDispatchDeclaration -> FeasibilityStrategy` while preserving the existing concrete helper checks and return surface.
  - Re-exported `FeasibilityStrategy` from [`crates/worldwake-ai/src/lib.rs`](crates/worldwake-ai/src/lib.rs) for consistency with the existing declaration surface exports.
  - Strengthened focused tests in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](crates/worldwake-ai/src/goal_dispatch_decl.rs) and [`crates/worldwake-ai/src/feasibility.rs`](crates/worldwake-ai/src/feasibility.rs) to cover declaration completeness, payload-sensitive/shared family assignments, and move-cargo edge routing.
- Deviations from original plan:
  - The original ticket assumed declaration infrastructure and focused feasibility coverage were largely absent. Reassessment showed both already existed, so the delivered work was narrower: add the missing declaration-owned feasibility selector and strengthen the proof surface instead of rebuilding tests from scratch.
  - `Engine Changes: None` was corrected to `Engine Changes: Yes — worldwake-ai declaration-owned feasibility dispatch and focused regression coverage`.
- Verification results:
  - `cargo test -p worldwake-ai feasibility::tests`
  - `cargo test -p worldwake-ai goal_dispatch_decl::tests`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
