# S36DECGOAREG-007: Introduce FeasibilityStrategy and migrate feasibility dispatch

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S36DECGOAREG-002

## Problem

`goal_specific_feasibility()` in `feasibility.rs` contains an exhaustive match on all `GoalKind` variants to produce feasibility hints. This dispatch should be mediated by a declaration-owned `FeasibilityStrategy` enum, following the same pattern as `InvalidationStrategy` (006).

## Assumption Reassessment (2026-03-29)

1. `goal_specific_feasibility()` is at `feasibility.rs:88-173`. Exhaustive match on `GoalKind`, no wildcard. Returns `Option<FeasibilityHint>`.
2. Called from `feasibility_hint()` at `feasibility.rs:32-55` as Phase 2 dispatch after shared checks (exhausted frame, blocker memory).
3. The match body groups goals by feasibility strategy — distinct families identified from live code:
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
4. The existing helper functions (`check_evidence_places_local`, `check_colocated_or_dead`) can be reused as strategy implementations.

## Architecture Check

1. Same rationale as 006: strategy selectors are static routing decisions in declarations; the computation consumes live belief state (P3). Adding a goal requires choosing a strategy (compile-time enforced).
2. No backwards-compatibility shims. `goal_specific_feasibility()` is refactored in-place with identical return type.

## Verification Layers

1. Strategy routing equivalence → focused unit test: for every `GoalKind` variant, the strategy-routed result matches the pre-migration result given the same mock belief view.
2. Behavioral equivalence → full AI test suite: all golden tests pass unchanged.
3. Single-layer ticket: feasibility dispatch only.

## What to Change

### 1. Define `FeasibilityStrategy` enum in `goal_dispatch_decl.rs`

One variant per identified feasibility family (see list above).

### 2. Add `feasibility_strategy` field to `GoalDispatchDeclaration`

Extend the struct and populate the field in every declaration.

### 3. Refactor `goal_specific_feasibility()` in `feasibility.rs`

Replace the monolithic match with:
1. Look up `GoalDispatchKey::from_goal_kind(&goal.grounded.key.kind).declaration().feasibility_strategy`.
2. Match on the strategy enum to call family-specific helper functions.
3. Helpers still take `(view, agent, goal)` and may inspect concrete payload fields.

### 4. Equivalence tests

Add tests comparing strategy-routed results against known expected outputs for representative goal shapes.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add `FeasibilityStrategy` enum, add field to struct, populate in declarations)
- `crates/worldwake-ai/src/feasibility.rs` (modify — refactor main dispatch to strategy routing)

## Out of Scope

- Changing `FeasibilityHint` type or the `feasibility_hint()` function signature
- Modifying shared Phase 1 checks (`check_exhausted_frame`, `check_blocker_memory`)
- Invalidation strategy (ticket 006)
- Wildcard audit (ticket 008)
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `test_feasibility_strategy_equivalence`: For every `GoalKind` variant (including payload-sensitive splits), the strategy-routed `goal_specific_feasibility()` produces identical results as the pre-migration implementation, given the same mock belief view.
2. `test_feasibility_strategy_completeness`: Every `FeasibilityStrategy` variant is used by at least one declaration.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`

### Invariants

1. Zero behavioral change — feasibility hints are identical for all goal shapes.
2. Adding a `GoalDispatchKey` without a `feasibility_strategy` fails compilation (struct field is required).
3. The strategy enum routes to computations; it does not encode belief data directly (P3).
4. Shared Phase 1 checks (exhausted frame, blocker memory) are unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/feasibility.rs` (test module) — strategy routing equivalence tests across all goal shapes with mock belief views.

### Commands

1. `cargo test -p worldwake-ai -- feasibility`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
