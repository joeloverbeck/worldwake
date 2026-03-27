# S31-006: Remove EXHAUSTION_SKIP_TTL and Update Skip Predicate

**Status**: BLOCKED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planning pipeline
**Deps**: S31-004, S31-005, S31-008

## Problem

The intended S31 end-state is still correct: the planner should not keep a separate time-based retry authority once exhaustion invalidation is complete. But live reassessment shows that the current invalidation substrate is not yet complete enough to support removing `EXHAUSTION_SKIP_TTL` without regressions.

## Assumption Reassessment (2026-03-27)

1. The shared abstraction boundary under audit is the planner-side exhaustion-cache contract: `crates/worldwake-ai/src/exhaustion.rs::invalidate_exhausted_goals()` determines whether an `AgentDecisionRuntime.exhaustion_cache` entry remains live, while `crates/worldwake-ai/src/agent_tick/planning.rs::build_candidate_plans()` still applies a TTL-based skip gate on top of that cache.
2. `EXHAUSTION_SKIP_TTL = 20` and `exhaustion_skip_active()` are still live in `crates/worldwake-ai/src/agent_tick/planning.rs`. The current skip predicate is `exhaustion_cache.get(&c.grounded.key).is_some_and(|entry| exhaustion_skip_active(entry, current_tick))`.
3. The live goal-aware invalidation substrate already exists in production code: `derive_invalidation_conditions()`, `condition_changed()`, and `invalidate_exhausted_goals()` are implemented in `crates/worldwake-ai/src/exhaustion.rs`, including facility- and blocker-driven invalidation branches. Focused coverage already exists in `exhaustion::tests::condition_changed_facility_and_blocker_branches_forward_runtime_flags` and `exhaustion::tests::derive_invalidation_conditions_covers_every_live_goalkind_variant`.
4. The live TTL-focused planner coverage is in `crates/worldwake-ai/src/agent_tick/planning.rs`: `agent_tick::planning::tests::exhausted_goal_skip_window_remains_active_until_20_tick_boundary` and `agent_tick::planning::tests::exhausted_goal_without_ttl_marker_is_not_skipped`.
5. The motivating invariant is planner retry correctness for exhausted goals: a previously exhausted goal should become searchable again when, and only when, concrete planner-relevant local state has changed enough to make the search space materially different. The planner must not retry only because abstract time passed, but it also must not stay indefinitely stale when the local decision surface changed.
6. The live `GoalKind` families exposed by the failing scenarios are needs-driven goals such as `GoalKind::ConsumeOwnedCommodity`, `GoalKind::Sleep`, and `GoalKind::Wash`. These scenarios rely on the current candidate-generation thresholds in `crates/worldwake-ai/src/candidate_generation.rs`, where `emit_sleep_goal()`, `emit_relieve_goal()`, and `emit_wash_goal()` are driven by `DriveThresholds::* .low()` rather than by a generic fixed delta.
7. Live verification after removing TTL showed that the replacement substrate is incomplete. `cargo test -p worldwake-ai` failed these existing goldens: `golden_goal_invalidation_by_another_agent`, `golden_wash_action`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection` in [golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs). This reproduces the exact “indefinite caching” failure class called out in `specs/S31-goal-aware-exhaustion-invalidation.md`.
8. Those failures show that removing TTL now would not merely simplify dead code; it would reopen a production contradiction in the live AI retry contract. Under `tickets/README.md` and `docs/FOUNDATIONS.md`, that means the ticket is blocked on missing substrate rather than ready for implementation.
9. Adjacent contradiction exposed during reassessment: `ExhaustionEntry` in `crates/worldwake-ai/src/decision_runtime.rs` still uses `#[serde(default)]` on `invalidation_conditions` and `baseline`, which preserves a compatibility path the S31 spec says should not survive. That contradiction is separate from TTL removal and should not be folded silently into this ticket.

## Architecture Check

1. The clean architecture is still “one authority over exhaustion invalidation.” TTL removal remains the correct eventual direction because time alone is not a concrete, local reason to retry search under `docs/FOUNDATIONS.md` Principles 2, 3, 25, and 26.
2. The wrong move is to remove TTL before the replacement substrate is complete. That would leave the planner unable to revise exhausted goals when concrete local conditions changed but the current invalidation signals did not fire, which conflicts with Principles 19 and 27.
3. This ticket therefore becomes a follow-through ticket, not a substrate ticket. `S31-008` must first complete the missing exhaustion invalidation contract. Only then can `S31-006` safely remove the temporary TTL fallback without reopening regressions.
4. No backward-compatibility aliasing or shim work belongs here. If `S31-008` makes TTL unnecessary, `S31-006` should remove it outright rather than layering a second live path beside the condition-driven one.

## Verification Layers

1. planner-side skip semantics after TTL removal -> focused `agent_tick::planning` unit coverage
2. needs-driven invalidation completeness precondition -> existing and new golden E2E coverage proved by `cargo test -p worldwake-ai`
3. facility/blocker invalidation substrate remains authoritative -> focused `exhaustion` unit coverage
4. broader regression surface after future TTL removal -> `cargo clippy --workspace` and `cargo test --workspace`

## What to Change

### 1. Do not remove TTL until `S31-008` completes

Keep the current planner-local TTL gate in place. Treat this ticket as blocked on the invalidation-completeness work rather than partially implementing it.

### 2. Re-run this ticket only after the substrate is complete

When `S31-008` lands and the needs-driven goldens pass without relying on periodic TTL refreshes, remove:
- `EXHAUSTION_SKIP_TTL`
- `exhaustion_skip_active()`
- the TTL-based skip predicate in `build_candidate_plans()`

### 3. Keep the architectural scope narrow when unblocked

When this ticket is resumed, it should remain a small planner cleanup. It should not absorb the broader invalidation redesign or the save-format cleanup.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (future modify, once unblocked)

## Out of Scope

- Completing needs-driven exhaustion invalidation semantics
- Golden proof for the missing substrate
- Removing the `ExhaustionEntry` save-compatibility defaults in `decision_runtime.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` passes after TTL removal
2. `golden_goal_invalidation_by_another_agent` still passes after TTL removal
3. `golden_wash_action` still passes after TTL removal
4. `golden_three_way_need_competition` still passes after TTL removal
5. `golden_utility_weight_diversity_in_need_selection` still passes after TTL removal
6. Existing suite: `cargo test --workspace`

### Invariants

1. No TTL-based retry remains in live planner code once this ticket is actually implemented
2. Removing TTL must not reintroduce indefinite caching for needs-driven goals
3. Condition-driven invalidation remains the single live authority over exhausted-goal retry behavior after implementation

## Test Plan

### New/Modified Tests

1. `None — blocked ticket; implementation and tests move forward only after S31-008 proves invalidation completeness.`

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
