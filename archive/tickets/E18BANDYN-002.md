# E18BANDYN-002: Add RegroupWithFaction and RaidTarget goal kinds

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` goal identity, `worldwake-ai` goal dispatch + goal model
**Deps**: E13 (decision architecture — completed), [specs/E18-bandit-dynamics.md](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md)

## Problem

E18 requires two new authoritative goal identities:

- `RegroupWithFaction { faction: EntityId }` for survivor regrouping via rally-point beliefs after camp loss
- `RaidTarget { target: EntityId }` for bandit-initiated predation that is semantically distinct from generic hostility

These goal kinds must exist in shared goal identity before later tickets wire candidate generation, rally-point planning, and raid-specific action semantics.

## Assumption Reassessment (2026-03-29)

1. The shared abstraction boundary under audit is: `worldwake_core::GoalKind` -> `worldwake_core::GoalKey` -> `worldwake_ai::GoalDispatchKey` / `GoalDispatchDeclaration` -> `worldwake_ai::GoalKindPlannerExt`. Current AI dispatch does not use a `GoalKindTag`; the live dispatch layer is `crates/worldwake-ai/src/goal_dispatch_key.rs` plus `crates/worldwake-ai/src/goal_dispatch_decl.rs`.
2. `GoalKind` currently lives in `crates/worldwake-core/src/goal.rs` and is consumed exhaustively both by `GoalKey::from` and by `GoalKindPlannerExt for GoalKind` in `crates/worldwake-ai/src/goal_model.rs`. Adding new variants requires updating those exact exhaustive surfaces; a build-only ticket that mentions only `goal.rs` and a removed tag layer is incomplete.
3. The current planner op surface does not contain a generic `Combat` op. The live ops are `PlannerOpKind::Attack` and `PlannerOpKind::Defend` in `crates/worldwake-ai/src/planner_ops.rs`. Any ticket text that claims `RaidTarget -> Combat` is stale and must be corrected to the current operator vocabulary.
4. The current AI test surface already proves dispatch completeness in focused tests: `goal_dispatch_key::tests::test_goal_dispatch_key_exhaustive_coverage`, `goal_dispatch_decl::tests::test_declaration_*`, and `goal_model::tests::all_goal_kind_variants_have_*_impl`. This ticket should extend those focused tests rather than rely on a nonexistent `GoalKindTag` mapping test.
5. The spec still justifies distinct goal kinds. [specs/E18-bandit-dynamics.md](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md) Section 4 makes raid semantically distinct from generic attack, and Section 8 makes regroup depend on faction-specific rally-point beliefs. Reusing `EngageHostile` or `ReduceDanger` would collapse distinct motivations and make later ranking/suppression logic harder to express cleanly.
6. Current `GoalBeliefView` in `crates/worldwake-sim/src/belief_view.rs` does not yet expose a dedicated rally-point belief query. That means this ticket should not invent speculative regroup-search behavior. It should add the new goal identities and lawful dispatch placeholders, while deferring rally-place resolution and raid-specific planner search/action payload semantics to later E18 tickets.
7. Current targeted commands are real and dry-run checked: `cargo test -p worldwake-core goal::tests -- --nocapture` and `cargo test -p worldwake-ai goal_model -- --nocapture` both run today. `cargo build --workspace` and `cargo clippy --workspace` remain valid broader checks.
8. Mismatch + correction: the original ticket referenced a removed `GoalKindTag` layer, a nonexistent `PlannerOpKind::Combat`, and additive behavior that implied full raid/regroup planning semantics in this ticket. Correct scope: add authoritative goal kinds plus the current dispatch/model plumbing needed for compilation and focused coverage, while deferring full candidate-generation, rally-destination lookup, and raid action/planner integration to E18BANDYN-003, E18BANDYN-006, and E18BANDYN-007.

## Architecture Check

1. Distinct `GoalKind` variants are still the cleaner architecture than overloading `EngageHostile` or `ReduceDanger`. Raid and regroup are not mere aliases; they need independent ranking, suppression, tracing, and planner policy later.
2. The AI-specific distinction should live in `GoalDispatchKey` / `GoalDispatchDeclaration`, not in a duplicate tag enum. That keeps one authoritative goal identity in `worldwake-core` and one payload-aware dispatch identity in `worldwake-ai`, which is the current architecture and the right extensibility point.
3. This ticket should not add backwards-compatibility aliases or premature rally-point helper APIs. If regroup needs new belief-view substrate, that belongs in the follow-up ticket that actually consumes it.

## Verification Layers

1. Shared goal identity remains serializable and canonically keyed -> focused `worldwake-core` unit tests in `crates/worldwake-core/src/goal.rs`
2. AI dispatch coverage remains exhaustive for all goal kinds -> focused `worldwake-ai` unit tests in `crates/worldwake-ai/src/goal_dispatch_key.rs` and `crates/worldwake-ai/src/goal_dispatch_decl.rs`
3. Goal-model exhaustive matches remain updated and coherent for new variants -> focused `worldwake-ai` unit tests in `crates/worldwake-ai/src/goal_model.rs`
4. No hidden compile holes remain in downstream exhaustive matches -> `cargo build --workspace`
5. This ticket intentionally does not prove candidate generation or executable raid/regroup plans; those proof surfaces belong to later tickets once the belief/action substrate exists

## What to Change

### 1. Add authoritative goal kinds and canonical goal-key support

In `crates/worldwake-core/src/goal.rs`:

- add `GoalKind::RegroupWithFaction { faction: EntityId }`
- add `GoalKind::RaidTarget { target: EntityId }`
- update `GoalKey::from` so the new goals retain their canonical faction/target identity
- add focused roundtrip and canonical-field tests for the new variants

### 2. Extend AI dispatch to recognize the new goals

In `crates/worldwake-ai/src/goal_dispatch_key.rs` and `crates/worldwake-ai/src/goal_dispatch_decl.rs`:

- add distinct `GoalDispatchKey` variants for regroup and raid
- add declarations for both keys with current op-family placeholders that match the live planner vocabulary:
  - `RegroupWithFaction` -> `Travel`
  - `RaidTarget` -> `Attack`
- extend representative-goal and completeness tests

### 3. Update goal-model exhaustive matches without inventing future substrate

In `crates/worldwake-ai/src/goal_model.rs`:

- update exhaustive matches so the file compiles with the new `GoalKind` variants
- keep behavior minimal and lawful:
  - `RaidTarget` may use the existing target-oriented combat shape where needed
  - `RegroupWithFaction` should stay distinct but must not fabricate a rally destination from omniscient state
- add or adjust focused tests that prove the new variants participate in the current dispatch/model surface

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- any additional file revealed by exhaustive compile failures (modify only if required)

## Out of Scope

- raid action payload/handler mechanics (`E18BANDYN-003`)
- bandit candidate generation and regroup candidate generation (`E18BANDYN-006`)
- rally-point destination lookup and executable raid/regroup planner search semantics (`E18BANDYN-007`)
- camp establishment / abandonment systems (`E18BANDYN-004`, `E18BANDYN-005`)
- route threat estimation (`E18BANDYN-008`)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::RegroupWithFaction` and `GoalKind::RaidTarget` serialize, deserialize, and expose stable canonical `GoalKey` identity
2. `GoalDispatchKey::from_goal_kind` and `GoalDispatchDeclaration` cover the new variants exhaustively
3. `GoalKindPlannerExt` exhaustive matches compile and focused tests cover the new variants on the live dispatch/model surface
4. Existing suite: `cargo test -p worldwake-core goal::tests -- --nocapture`
5. Existing suite: `cargo test -p worldwake-ai goal_model -- --nocapture`
6. Existing suite: `cargo build --workspace`
7. Existing suite: `cargo clippy --workspace`

### Invariants

1. Shared authoritative goal identity remains the single source of truth; no aliasing or compatibility shim goal variants are introduced
2. AI-specific dispatch remains payload-aware through `GoalDispatchKey`, not through a duplicated tag layer
3. This ticket does not grant omniscient rally-point knowledge to planners; regroup-destination resolution stays belief-driven work for the follow-up ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — prove canonical key extraction and bincode roundtrip for `RegroupWithFaction` and `RaidTarget`
2. `crates/worldwake-ai/src/goal_dispatch_key.rs` — prove new goal kinds map to distinct dispatch keys and extend exhaustive coverage
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — prove declaration completeness and op-family mapping for the new dispatch keys
4. `crates/worldwake-ai/src/goal_model.rs` — prove the new variants participate in the live goal-model surface without relying on future raid/regroup planner substrate

### Commands

1. `cargo test -p worldwake-core goal::tests -- --nocapture`
2. `cargo test -p worldwake-ai goal_dispatch_key -- --nocapture`
3. `cargo test -p worldwake-ai goal_dispatch_decl -- --nocapture`
4. `cargo test -p worldwake-ai goal_model -- --nocapture`
5. `cargo build --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- What actually changed:
  - Added `GoalKind::RaidTarget` and `GoalKind::RegroupWithFaction` in `worldwake-core`
  - Extended canonical `GoalKey` extraction and shared-goal serialization coverage
  - Added distinct `GoalDispatchKey` and `GoalDispatchDeclaration` entries for both goals in `worldwake-ai`
  - Updated the live AI surfaces that exhaustively match on `GoalKind`: `goal_model`, `feasibility`, `exhaustion`, `goal_policy`, and `ranking`
  - Added focused tests for the new goal kinds across core identity, AI dispatch, and goal-model coverage
- Deviations from original plan:
  - The original ticket referenced a removed `GoalKindTag` layer and a nonexistent generic `Combat` planner op. The implementation followed the live `GoalDispatchKey` / `GoalDispatchDeclaration` architecture instead.
  - `RegroupWithFaction` was kept architecturally distinct but intentionally does not synthesize a rally destination yet because the current belief-view surface still lacks a dedicated rally-point query. That planner substrate remains for the follow-up ticket.
  - `RaidTarget` now uses the existing attack-shaped planner surface as the current lawful placeholder while preserving a distinct goal and dispatch identity for later raid-specific planning and action wiring.
- Verification results:
  - `cargo test -p worldwake-core goal::tests -- --nocapture` passed
  - `cargo test -p worldwake-ai goal_dispatch_key -- --nocapture` passed
  - `cargo test -p worldwake-ai goal_dispatch_decl -- --nocapture` passed
  - `cargo test -p worldwake-ai goal_model -- --nocapture` passed
  - `cargo test -p worldwake-core` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo build --workspace` passed
  - `cargo clippy --workspace` passed
