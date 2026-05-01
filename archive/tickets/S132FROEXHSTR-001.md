# S132FROEXHSTR-001: Declare frontier-exhaustion strategy on goal dispatch metadata

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - `worldwake-ai` goal dispatch metadata and exhaustion strategy types
**Deps**: `specs/S132-frontier-exhaustion-strategy.md`

## Problem

`frontier_exhaustion_entry` currently decides which `GoalKind` variants retry after `FrontierExhausted` through a local allow-list in `crates/worldwake-ai/src/agent_tick/planning.rs`. That keeps recurring-goal retry behavior away from the goal metadata surface and makes each new recurring goal class vulnerable to permanent suppression until someone patches the planning helper.

## Assumption Reassessment (2026-05-01)

1. The live allow-list is in `crates/worldwake-ai/src/agent_tick/planning.rs::frontier_exhaustion_entry`, where `GoalKind::Sleep`, `GoalKind::AcquireCommodity { purpose: CommodityPurpose::SelfConsume, .. }`, and `GoalKind::Patrol { .. }` use `ExhaustionEntry::budget_retry_pending`; all other variants fall through to `ExhaustionEntry::frontier_exhausted`.
2. The live metadata boundary for AI goal-kind properties is `crates/worldwake-ai/src/goal_dispatch_decl.rs::GoalDispatchDeclaration`, reached through the exhaustive `crates/worldwake-ai/src/goal_dispatch_key.rs::GoalDispatchKey::from_goal_kind`.
3. `GoalDispatchKey::from_goal_kind` already has no `_` arm and matches every active `worldwake_core::GoalKind` variant in `crates/worldwake-core/src/goal.rs`, including payload-sensitive splits for `AcquireCommodity`, `PostNotice`, `ShareBelief`, and `PunishAccused`.
4. Existing focused coverage names the current behavioral exceptions in `crates/worldwake-ai/src/agent_tick/planning.rs`: `record_exhausted_goals_records_sleep_frontier_exhaustion_as_budget_retry`, `record_exhausted_goals_records_self_consume_acquire_frontier_exhaustion_as_retry`, and `record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry`.
5. This is a planner-internal metadata refactor, not an authoritative validation change. The shared abstraction boundary under audit is `GoalDispatchKey -> GoalDispatchDeclaration`, not `get_affordances`, `generate_candidates`, `search_plan`, or `tick_step` action start.
6. The live `GoalKind` under test is every active variant reachable through `GoalDispatchKey::from_goal_kind`; the exact current operator surface is `GoalDispatchDeclaration.relevant_ops`, which this ticket must not change.
7. The testing gap is focused/unit coverage for the new declaration field. Existing runtime `record_exhausted_goals` tests cover the current exceptions, and no new golden/E2E coverage is required for introducing the metadata field alone.
8. No timing or ordering contract changes. The strategy is a compile-time declaration read during exhaustion recording; `ExhaustionEntry` cooldown arithmetic and invalidation semantics remain unchanged.
9. No information-path refactor. Frontier exhaustion remains agent-local planner state in `AgentDecisionRuntime.exhaustion_cache`; no belief, perception, witness, or report transport path is added or removed.
10. No save-format change. `ExhaustionEntry` and `AgentDecisionRuntime.exhaustion_cache` serialization are unchanged.
11. Adjacent contradiction classification: if adding the field exposes a `GoalDispatchKey` without a declaration, that is a required consequence of this ticket and must be fixed here. If a goal's current retry behavior proves semantically wrong, that is separate behavior work unless it contradicts the preserved S132 strategy table.

## Architecture Check

1. Extending `GoalDispatchDeclaration` keeps frontier-exhaustion strategy co-located with existing goal metadata: trace label, relevant planner ops, invalidation strategy, feasibility strategy, family policy, and progress barriers.
2. The design avoids a second switch over `GoalKind` in `planning.rs` while preserving the existing exhaustive `GoalDispatchKey::from_goal_kind` compile-time guard.
3. No backwards-compatibility aliasing or shim path is introduced. The new declaration field becomes the sole metadata source for this strategy.

## Verification Layers

1. Every dispatch declaration has an explicit frontier-exhaustion strategy -> focused unit test over `GoalDispatchKey::all()` in `goal_dispatch_decl.rs`.
2. `GoalKind` payload splits retain their current dispatch identity before behavior changes -> existing `goal_dispatch_key.rs` tests plus targeted `goal_dispatch_decl.rs` coverage.
3. Additional layer mapping is not applicable for this ticket because it only introduces declaration metadata and does not route runtime exhaustion recording through the metadata yet.

## What to Change

### 1. Add strategy type

Add `FrontierExhaustionStrategy` with `PermanentUntilInvalidator` and `CooldownRetry`. Place it in `crates/worldwake-ai/src/exhaustion.rs` or `crates/worldwake-ai/src/goal_dispatch_decl.rs` based on import hygiene, and re-export it from `crates/worldwake-ai/src/lib.rs` if tests or sibling modules need it.

### 2. Extend goal dispatch declarations

Add a `frontier_exhaustion_strategy: FrontierExhaustionStrategy` field to `GoalDispatchDeclaration`.

Assign:

- `CooldownRetry`: `DECL_SLEEP`, `DECL_ACQUIRE_SELF_CONSUME`, `DECL_PATROL`.
- `PermanentUntilInvalidator`: every other existing declaration, including `DECL_WASH`, `DECL_RELIEVE`, `DECL_ACQUIRE_RECIPE_INPUT`, `DECL_ACQUIRE_RESTOCK`, production, social, political, combat, theft, and punishment declarations.

### 3. Add declaration coverage

Add focused tests proving all dispatch declarations expose the field and the three existing recurring retry declarations are `CooldownRetry` while representative preserved-default declarations are `PermanentUntilInvalidator`.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `specs/S132-frontier-exhaustion-strategy.md` (truth-sync declaration boundary)

## Out of Scope

- Refactoring `frontier_exhaustion_entry`.
- Changing `ExhaustionEntry::budget_retry_pending` cooldown decay.
- Changing `ExhaustionEntry::frontier_exhausted` invalidation behavior.
- Changing `GoalDispatchKey` identities, `GoalKind` variants, planner operators, action validation, or golden scenarios.

## Acceptance Criteria

### Tests That Must Pass

1. New focused declaration test proves `GoalDispatchKey::Sleep`, `GoalDispatchKey::AcquireSelfConsume`, and `GoalDispatchKey::Patrol` declare `FrontierExhaustionStrategy::CooldownRetry`.
2. New focused declaration test proves representative preserved-default declarations, including `GoalDispatchKey::Wash`, `GoalDispatchKey::Relieve`, `GoalDispatchKey::AcquireRestock`, and `GoalDispatchKey::ProduceCommodity`, declare `FrontierExhaustionStrategy::PermanentUntilInvalidator`.
3. Existing suite: `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests::`

### Invariants

1. Every `GoalDispatchDeclaration` has an explicit frontier-exhaustion strategy.
2. The new field does not alter planner operator relevance, invalidation strategy, feasibility strategy, family policy, or progress-barrier declarations.
3. Future `GoalKind` additions still pass through the existing exhaustive `GoalDispatchKey::from_goal_kind` and declaration mapping before they can compile cleanly.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` - focused unit coverage for declared frontier-exhaustion strategies.

### Commands

1. `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests::frontier_exhaustion_strategy`
2. `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests::`
3. `cargo test -p worldwake-ai -- --list`

## Outcome

Completed on 2026-05-01.

- Added `FrontierExhaustionStrategy` to the goal dispatch declaration metadata surface and re-exported it from `worldwake-ai`.
- Added an explicit `frontier_exhaustion_strategy` field to every `GoalDispatchDeclaration`.
- Declared `CooldownRetry` for `GoalDispatchKey::Sleep`, `GoalDispatchKey::AcquireSelfConsume`, and `GoalDispatchKey::Patrol`.
- Declared `PermanentUntilInvalidator` for all other existing dispatch declarations, preserving current runtime behavior.
- Added focused declaration coverage over `GoalDispatchKey::all()` plus explicit positive coverage for the three recurring retry declarations and representative preserved-default declarations.
- No save-format bump was required because no serialized runtime state, save carrier, or `ExhaustionEntry` shape changed.

## Deviations

- The enum lives in `goal_dispatch_decl.rs`, not `exhaustion.rs`, because this ticket's landed boundary is declaration metadata. Runtime routing remains out of scope for `S132FROEXHSTR-001` and is still owned by `S132FROEXHSTR-002`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests:: -- --list`.
- Passed `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests::frontier_exhaustion_strategy`.
- Passed `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests::`.
- Passed `cargo test -p worldwake-ai -- --list`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `git diff --check` after final Markdown closeout.
