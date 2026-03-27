**Status**: DEFERRED

# S31-006: Remove EXHAUSTION_SKIP_TTL and Update Skip Predicate

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No — ticket scope corrected before implementation
**Deps**: S31-004, S31-005, S31-008

## Problem

The intended S31 end-state is still directionally correct: the planner should not keep a separate time-based retry authority once exhaustion invalidation is complete. Live reassessment against current code and tests shows that this ticket is not implementation-ready. Removing `EXHAUSTION_SKIP_TTL` today regresses live AI behavior because the current invalidation model does not yet cover every case where a budget-exhausted goal must become searchable again.

## Assumption Reassessment (2026-03-27)

1. The shared abstraction boundary under audit is still the planner-side exhaustion retry contract across [`planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs): `invalidate_exhausted_goals()` decides when an `AgentDecisionRuntime.exhaustion_cache` entry is removed, while `build_candidate_plans()` still applies a second TTL-based retry gate.
2. `EXHAUSTION_SKIP_TTL = 20` and `exhaustion_skip_active()` are still live in [`planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). The current skip predicate is still `exhaustion_cache.get(&c.grounded.key).is_some_and(|entry| exhaustion_skip_active(entry, current_tick))`.
3. The goal-aware invalidation substrate already exists in production code. `derive_invalidation_conditions()`, `condition_changed()`, and `invalidate_exhausted_goals()` are live in [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs). The needs-driven branch now uses `NeedChangedBands`, not `NeedCrossedThreshold`, and persisted runtime shape already includes `invalidation_conditions` plus `baseline` in [`decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs). `SAVE_FORMAT_VERSION` is 8 in [`save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs).
4. The live TTL-focused planner coverage remains `agent_tick::planning::tests::exhausted_goal_skip_window_remains_active_until_20_tick_boundary` and `agent_tick::planning::tests::exhausted_goal_without_ttl_marker_is_not_skipped` in [`planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). The previously written combined `cargo test` command for both unit tests was invalid. The verified runnable forms require `--lib` and one exact test filter per command.
5. The live motivating invariant is not merely "remove the constant." It is planner retry correctness for exhausted goals: a goal should retry when concrete local planner facts changed enough to justify a new search, but it must also not become permanently suppressed when the prior failure was a bounded search-budget artifact rather than a durable impossibility proof.
6. A direct TTL-removal experiment on current code, implemented by replacing the skip predicate with `entry.exhausted_at.is_some()`, still regresses live golden behavior. The current failure set is:
   - `golden_goal_invalidation_by_another_agent`
   - `golden_wash_action` passes
   - `golden_three_way_need_competition`
   - `golden_utility_weight_diversity_in_need_selection`
7. That result corrects the prior ticket narrative. The remaining contradiction is broader than "non-wash planner behavior" in the abstract. It still includes local self-care and competition scenarios whose relevant facts have not changed enough to fire current invalidation signals, but which still need eventual retry because the prior `BudgetExhausted` result was not a durable impossibility proof.
8. The adjacent contradiction is a required architectural consequence of the intended change, not a separate incidental bug. This ticket cannot honestly stay scoped as a tiny planner cleanup until the retry model distinguishes "wait for local fact change" from "same-world retry after bounded search truncation."

## Architecture Check

1. Removing TTL is not more beneficial than the current live architecture if done in isolation. It would replace an admittedly coarse retry path with indefinite suppression for goals that are still lawful and still relevant under the same local world facts.
2. The clean end-state is still a single retry authority, but the durable abstraction is not "delete TTL and treat every exhaustion entry as invalid until world change." The durable abstraction is "record why search failed." A goal that is impossible pending new local facts should stay invalidated by conditions; a goal that merely hit `PlanSearchResult::BudgetExhausted` under the current world needs a different retry path.
3. The cleaner long-term design is to separate semantic impossibility from bounded search truncation. Examples of architecturally cleaner follow-up directions include resumable frontier state, deterministic staged-budget retries, or a distinct retry class derived from search provenance. Any of those is cleaner than keeping a time constant forever, but they are broader than the small deletion this ticket originally described.
4. No backward-compatibility aliasing belongs in that follow-up. When the retry contract is corrected, the TTL path should be removed outright rather than preserved beside the new authority.

## Verification Layers

1. live TTL gate remains present and behaves as currently documented -> `agent_tick::planning::tests::exhausted_goal_skip_window_remains_active_until_20_tick_boundary`
2. TTL gate does not suppress entries without an active TTL marker -> `agent_tick::planning::tests::exhausted_goal_without_ttl_marker_is_not_skipped`
3. unchanged code still satisfies the current golden behavior surface -> `golden_goal_invalidation_by_another_agent`, `golden_wash_action`, `golden_three_way_need_competition`, `golden_utility_weight_diversity_in_need_selection`
4. broader AI regression surface on the unchanged branch -> `cargo test -p worldwake-ai`
5. broader lint surface on the unchanged branch -> `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## What To Change

1. Do not remove `EXHAUSTION_SKIP_TTL` in this ticket.
2. Defer this ticket and move the real implementation work into a follow-up that audits the retry semantics around `PlanSearchResult::BudgetExhausted`, `record_exhausted_goals()`, and `build_candidate_plans()`.
3. When that follow-up proves a concrete replacement for same-world retries, remove:
   - `EXHAUSTION_SKIP_TTL`
   - `exhaustion_skip_active()`
   - the TTL-based skip predicate in `build_candidate_plans()`

## Files To Touch

- [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) in the future follow-up, once retry semantics are corrected
- this ticket record during reassessment and archival

## Out Of Scope

- changing production planner behavior in this ticket
- adding workaround retries or a second alias path beside the existing TTL gate
- revisiting the already-landed needs-band substrate from `S31-008`

## Acceptance Criteria

1. This ticket is archived as deferred with corrected assumptions, scope, and runnable verification commands.
2. No production code changes ship under the false premise that TTL removal is already safe.
3. The archived record clearly states that the remaining gap is retry semantics after `BudgetExhausted`, not save-format compatibility or missing needs-band invalidation.

## Tests

### New/Modified Tests

1. None.
   Rationale: live reassessment showed the ticket is still blocked on an architectural retry-semantics gap, so the correct action was to fix the ticket record rather than land code or test changes under the wrong scope.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::exhausted_goal_skip_window_remains_active_until_20_tick_boundary -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::exhausted_goal_without_ttl_marker_is_not_skipped -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
6. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
7. `cargo test -p worldwake-ai`
8. `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Reassessed the ticket against live planner code, live `worldwake-ai` test inventory, and the current S31 implementation state.
  - Corrected the ticket’s assumptions by verifying that direct TTL removal still regresses current behavior.
  - Archived the ticket as deferred instead of implementing the planned code removal.
- Deviations from original plan:
  - The original plan was to remove `EXHAUSTION_SKIP_TTL` and simplify the skip predicate.
  - Live verification showed that this would still break current goldens, including `golden_goal_invalidation_by_another_agent`, so no production code change was justified.
  - The recommended next step is a follow-up on retry semantics for `BudgetExhausted`, not this small cleanup.
- Verification results:
  - Unchanged branch passes the targeted unit and golden commands listed above.
  - `cargo test -p worldwake-ai` passed.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
  - Temporary uncommitted TTL-removal experiment:
    - `golden_wash_action` passed.
    - `golden_goal_invalidation_by_another_agent`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection` failed.
