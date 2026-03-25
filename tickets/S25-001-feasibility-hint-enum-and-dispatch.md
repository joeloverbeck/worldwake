# S25-001: Add FeasibilityHint enum, dispatch table, and feasibility_hint() function

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-ai, new field on RankedGoal
**Deps**: S20 (completed), S23 (completed), S22 (completed)

## Problem

The GOAP planner wastes all `max_candidates_to_plan` slots on infeasible high-motive goals (e.g., food at an unreachable place) while directly-actionable lower-motive goals sit unsearched. A cheap pre-check can estimate local actionability before committing full search budget.

## Assumption Reassessment (2026-03-25)

1. `RankedGoal` in `crates/worldwake-ai/src/goal_model.rs:1478-1483` has fields: `grounded`, `priority_class`, `motive_score`, `provenance`. No `feasibility` field exists yet. Adding one with `Default`-like initialization (`Uncertain`) is backward-compatible.
2. `GoalKind` in `crates/worldwake-core/src/goal.rs:14-63` has exactly 17 variants (ConsumeOwnedCommodity through SupportCandidateForOffice). The spec's dispatch table covers all 17.
3. `GoalBeliefView` trait in `crates/worldwake-sim/src/belief_view.rs` provides `effective_place()`, `commodity_quantity()`, `is_dead()`, `is_alive()` — all methods needed by the feasibility checks.
4. `BlockedIntentMemory` in `crates/worldwake-core/src/blocked_intent.rs` uses `BTreeMap<BlockerKey, BlockedIntent>` (S23 refactored). The `intents` field is `pub`. Iterating `.values()` and checking `blocker_key.goal_key` and `expires_tick` is straightforward.
5. `IntentionFrame` in `crates/worldwake-core/src/intention_frame.rs:131-152` has `goal: GoalKey`, `state: FrameState`. `FrameState::Exhausted` is the variant we check.
6. This is a new AI-layer module — no existing heuristic is being weakened or removed. The only insertion point is a new field on `RankedGoal` initialized to `Uncertain`.
7. Not applicable — no stale-request or start-failure surface.

## Architecture Check

1. A dedicated `feasibility.rs` module is cleaner than inlining checks into `ranking.rs` or `agent_tick/mod.rs`. It isolates the heuristic and makes it independently testable with mock `GoalBeliefView`.
2. No backward-compatibility shims. The new `feasibility` field on `RankedGoal` is initialized to `Uncertain` everywhere `RankedGoal` is constructed, preserving existing sort order until annotation runs.

## Verification Layers

1. Exhausted frame → Unlikely: focused unit test with synthetic IntentionFrame
2. Active blocker → Unlikely: focused unit test with populated BlockedIntentMemory
3. Each GoalKind check path → {Likely, Unlikely, Uncertain}: focused unit tests with mock GoalBeliefView
4. New field on RankedGoal compiles cleanly with all existing code: `cargo test -p worldwake-ai`
5. Single-layer ticket — all verification is at the focused unit test level for the new module

## What to Change

### 1. Create `crates/worldwake-ai/src/feasibility.rs`

- Define `FeasibilityHint` enum: `Likely`, `Uncertain`, `Unlikely` with derive `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. Variant order gives `Likely < Uncertain < Unlikely` via derived `Ord`.
- Implement `feasibility_hint()` function with signature from spec (takes `&dyn GoalBeliefView`, `EntityId`, `&RankedGoal`, `&BlockedIntentMemory`, `Option<&IntentionFrame>`, `Tick`).
- Phase 1 shared checks: `check_exhausted_frame()` and `check_blocker_memory()`.
- Phase 2: `goal_specific_feasibility()` match dispatch on all 17 `GoalKind` variants per spec table.
- Default: `FeasibilityHint::Uncertain`.
- Add `#[cfg(test)] mod tests` with focused unit tests.

### 2. Add `feasibility` field to `RankedGoal` in `goal_model.rs`

- Add `pub feasibility: FeasibilityHint` field to `RankedGoal`.
- Initialize to `FeasibilityHint::Uncertain` in all existing construction sites (`rank_candidates()` in `ranking.rs`, any test helpers).

### 3. Register module and re-export in `lib.rs`

- Add `mod feasibility;` declaration.
- Add `pub use feasibility::{feasibility_hint, FeasibilityHint};` to the public API.

## Files to Touch

- `crates/worldwake-ai/src/feasibility.rs` (new)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add field to `RankedGoal`)
- `crates/worldwake-ai/src/ranking.rs` (modify — initialize `feasibility: FeasibilityHint::Uncertain` in `RankedGoal` construction)
- `crates/worldwake-ai/src/lib.rs` (modify — add module declaration and re-export)

## Out of Scope

- Integration into `process_agent()` (S25-002)
- Updating `compare_ranked_goals()` sort order (S25-002)
- Decision trace changes (S25-003)
- Budget allocation changes for Unlikely goals (explicitly out of scope per spec)
- Any changes to `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-cli`

## Acceptance Criteria

### Tests That Must Pass

1. `test_exhausted_frame_returns_unlikely` — IntentionFrame with `FrameState::Exhausted` matching goal key → `Unlikely`
2. `test_suspended_frame_not_unlikely` — IntentionFrame with `FrameState::Suspended` → does NOT return `Unlikely`
3. `test_active_blocker_returns_unlikely` — BlockedIntentMemory with live blocker for goal → `Unlikely`
4. `test_expired_blocker_not_unlikely` — BlockedIntentMemory with expired blocker → no effect
5. `test_consume_owned_with_commodity_likely` — ConsumeOwnedCommodity when agent possesses commodity → `Likely`
6. `test_consume_owned_without_commodity_uncertain` — ConsumeOwnedCommodity when no commodity → `Uncertain` (not Unlikely — might acquire)
7. `test_sleep_always_likely` — Sleep → `Likely`
8. `test_relieve_always_likely` — Relieve → `Likely`
9. `test_wash_with_water_likely` — Wash when agent has Water → `Likely`
10. `test_wash_without_water_uncertain` — Wash when no Water → `Uncertain`
11. `test_engage_hostile_colocated_likely` — EngageHostile with co-located target → `Likely`
12. `test_engage_hostile_target_dead_unlikely` — EngageHostile with dead target → `Unlikely`
13. `test_treat_wounds_colocated_likely` — TreatWounds with co-located patient → `Likely`
14. `test_treat_wounds_patient_dead_unlikely` — TreatWounds with dead patient → `Unlikely`
15. `test_share_belief_colocated_likely` — ShareBelief with co-located listener → `Likely`
16. `test_share_belief_listener_dead_unlikely` — ShareBelief with dead listener → `Unlikely`
17. `test_claim_office_evidence_local_likely` — ClaimOffice with evidence_places containing current place → `Likely`
18. `test_sell_commodity_no_commodity_unlikely` — SellCommodity when agent has no commodity → `Unlikely`
19. `test_sell_commodity_with_stock_and_local_evidence_likely` — SellCommodity with stock AND local evidence → `Likely`
20. `test_reduce_danger_returns_none` — ReduceDanger → `Uncertain` (no specific check)
21. `test_default_uncertain` — GoalKind with no specific opinion → `Uncertain`
22. Existing suite: `cargo test -p worldwake-ai` — all existing tests pass unchanged

### Invariants

1. `FeasibilityHint::Likely < FeasibilityHint::Uncertain < FeasibilityHint::Unlikely` (Ord derivation from enum variant order)
2. `feasibility_hint()` never reads authoritative world state — only `GoalBeliefView` + `BlockedIntentMemory` + `IntentionFrame`
3. All existing `RankedGoal` construction sites initialize `feasibility` to `Uncertain`
4. No GoalKind variant is unhandled in the dispatch match (exhaustive match enforced by compiler)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/feasibility.rs` (inline `#[cfg(test)] mod tests`) — 21 focused unit tests covering each check path with mock GoalBeliefView implementations

### Commands

1. `cargo test -p worldwake-ai feasibility` — run new focused tests
2. `cargo test -p worldwake-ai` — verify no regressions
3. `cargo clippy -p worldwake-ai` — no new warnings
