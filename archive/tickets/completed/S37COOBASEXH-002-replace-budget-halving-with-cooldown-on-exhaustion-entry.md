# S37COOBASEXH-002: Replace budget-halving with cooldown-based retry for exhaustion

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `ExhaustionEntry` schema plus the immediately coupled planning-path consumers
**Deps**: None (`PlanningBudget` cooldown fields already exist in current code)

## Problem

`ExhaustionEntry` still encodes repeated budget exhaustion as search-budget halving via `consecutive_budget_exhaustions` and `effective_max_expansions()`. That degrades search quality on repeated failure. The cleaner architecture is full-depth retry with deterministic cooldown gating: preserve planner capability, space retries out in time, and keep the retry contract explicit.

## Assumption Reassessment (2026-03-29)

1. `ExhaustionEntry` is currently defined in `crates/worldwake-ai/src/decision_runtime.rs` with `consecutive_budget_exhaustions: u8`, `effective_max_expansions()`, `suppresses_planning()`, and `is_budget_retry_pending()`. Factory constructors still build the old shape.
2. `PlanningBudget` already has `initial_cooldown_ticks` and `max_cooldown_ticks` in `crates/worldwake-ai/src/budget.rs`. The original dependency on S37COOBASEXH-001 is stale and removed from this ticket.
3. The live consumers are in `crates/worldwake-ai/src/agent_tick/planning.rs`, not `crates/worldwake-ai/src/planning.rs`. The relevant symbols are `build_candidate_plans()`, `record_exhausted_goals()`, `has_pending_budget_retry()`, and the two `should_plan` call sites.
4. The original split that deferred planning-path adaptations to S37COOBASEXH-003/004/005 is no longer a clean boundary for the current code. Removing the old `ExhaustionEntry` helpers without updating those planning consumers would intentionally break compilation. This ticket therefore absorbs the directly coupled planning-path updates required to keep `worldwake-ai` coherent.
5. `crates/worldwake-ai/src/exhaustion.rs` does not depend on budget-halving behavior, but it does contain `ExhaustionEntry` struct literals in tests that must be updated to the new schema.
6. `cargo test -p worldwake-ai -- --list` confirms focused verification surfaces already exist under `decision_runtime` and `agent_tick::planning`.
7. This is planner-path work even though it is not driven by a golden scenario. The exact boundary under audit is the retry contract between `ExhaustionEntry` in `decision_runtime.rs` and the planning pipeline in `agent_tick/planning.rs`.
8. Removing `effective_max_expansions()` and `is_budget_retry_pending()` is justified. They are the old architecture directly, and there should be no compatibility alias once cooldown eligibility becomes canonical.
9. The live S37 spec is internally inconsistent about `suppresses_planning()`: one section says to remove it, but the replacement code and candidate-filter design still rely on it for `FrontierExhausted`. The cleaner architecture is to keep `suppresses_planning()` as the narrow permanent-suppression helper and add `is_retry_eligible()` for cooldown gating.
10. Cooldown arithmetic belongs inside `ExhaustionEntry::record_budget_exhaustion()`: `initial_cooldown_ticks << (consecutive_failures - 1)`, capped at `max_cooldown_ticks`, using saturating arithmetic and a bounded shift. With defaults `(4, 64)`, progression is `4 -> 8 -> 16 -> 32 -> 64 -> 64`.
11. No golden scenario, political surface, stale-request path, or `ControlSource` behavior is involved.
12. Adjacent contradiction exposed during reassessment: the original file list and out-of-scope section no longer matched the real contract boundary. That contradiction belongs in this ticket and is corrected here instead of deferred.

## Architecture Check

1. Cooldown-based retry is better than budget halving. Retrying less often preserves bounded cost without degrading planner competence, which is more robust and extensible than encoding failure state as a hidden reduction in search depth.
2. Updating the data model and its only meaningful planning consumers together is cleaner than preserving the old split. The retry contract remains singular and the tree stays compiling throughout the change.
3. `suppresses_planning()` should stay. It expresses a distinct semantic state: `FrontierExhausted` means "do not retry until invalidated," which is different from cooldown timing on `BudgetRetryPending`.
4. No backward-compatibility shims. Remove the old field and helper methods outright. Save-format handling remains an explicit separate ticket.

## Verification Layers

1. `record_budget_exhaustion()` computes first-failure cooldown correctly -> focused unit test in `crates/worldwake-ai/src/decision_runtime.rs`
2. Consecutive failures double and then cap cooldown deterministically -> focused unit test in `crates/worldwake-ai/src/decision_runtime.rs`
3. `is_retry_eligible(current_tick)` admits only elapsed cooldown entries and never admits `FrontierExhausted` -> focused unit test in `crates/worldwake-ai/src/decision_runtime.rs`
4. Planning loop wakes only for retry-eligible cooldown entries -> focused unit test in `crates/worldwake-ai/src/agent_tick/planning.rs`
5. Candidate admission skips frontier-suppressed and cooldown-ineligible entries -> focused unit test in `crates/worldwake-ai/src/agent_tick/planning.rs`
6. Exhaustion recording delegates cooldown arithmetic to `ExhaustionEntry` and clears entries on success -> focused unit test in `crates/worldwake-ai/src/agent_tick/planning.rs`

## What to Change

### 1. Replace `consecutive_budget_exhaustions` on `ExhaustionEntry`

In `crates/worldwake-ai/src/decision_runtime.rs`:

- Remove `consecutive_budget_exhaustions: u8`
- Add `next_retry_tick: Option<Tick>` with `#[serde(default)]`
- Add `consecutive_failures: u8` with `#[serde(default)]`

### 2. Remove budget-halving helpers

Delete `effective_max_expansions()` and `is_budget_retry_pending()`.

### 3. Keep `suppresses_planning()` and add cooldown helpers

- Keep `suppresses_planning()` as the explicit `FrontierExhausted` helper
- Add `is_retry_eligible(&self, current_tick: Tick) -> bool`
- Add `record_budget_exhaustion(&mut self, current_tick: Tick, budget: &PlanningBudget)`

### 4. Update factory methods

- `frontier_exhausted()`: initialize `next_retry_tick: None` and `consecutive_failures: 0`
- `budget_retry_pending()`: accept `current_tick: Tick` and `budget: &PlanningBudget`, then call `record_budget_exhaustion()`

### 5. Update the immediately coupled planning-path consumers

In `crates/worldwake-ai/src/agent_tick/planning.rs`:

- `build_candidate_plans()`: stop reducing `max_node_expansions`; retries use the full planning budget
- `build_candidate_plans()`: filter out `BudgetRetryPending` entries that are not retry-eligible at `current_tick`
- `has_pending_budget_retry()`: accept `current_tick` and return true only for retry-eligible entries
- `record_exhausted_goals()`: accept `budget: &PlanningBudget`, replace manual counter handling with `record_budget_exhaustion()`, and use the updated `budget_retry_pending()` factory
- Update both planning-loop call sites to pass `current_tick` and `budget`

### 6. Update focused tests

Add or update focused tests for cooldown progression, cap, eligibility, planning-trigger gating, candidate admission, and exhaustion recording.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/exhaustion.rs` (schema/test updates only if needed)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (schema/test updates only if needed)

## Out of Scope

- Decision trace changes (`S37COOBASEXH-006`)
- Save/load version bump (`S37COOBASEXH-007`)
- Golden test changes
- Any changes to invalidation strategy semantics in `crates/worldwake-ai/src/exhaustion.rs`

## Acceptance Criteria

### Tests That Must Pass

1. First budget exhaustion sets `next_retry_tick = current_tick + initial_cooldown_ticks`
2. Consecutive budget exhaustions double cooldown until capped by `max_cooldown_ticks`
3. `is_retry_eligible()` returns false before cooldown expiry and true at or after expiry
4. `is_retry_eligible()` always returns false for `FrontierExhausted`
5. `has_pending_budget_retry(runtime, current_tick)` returns false when all budget-retry entries are still cooling down
6. `has_pending_budget_retry(runtime, current_tick)` returns true when at least one budget-retry entry is eligible
7. `build_candidate_plans()` does not reduce `max_node_expansions` for retry entries
8. `record_exhausted_goals()` stores cooldown state through the canonical helper rather than manual counter mutation
9. Custom `PlanningBudget` cooldown values are respected
10. `frontier_exhausted()` initializes `consecutive_failures: 0` and `next_retry_tick: None`

### Invariants

1. `ExhaustionEntry` remains `Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Serialize + Deserialize`
2. `FrontierExhausted` entries are never retry-eligible and remain invalidation-driven
3. Cooldown is deterministic tick arithmetic only
4. Budget-retry attempts always use full `PlanningBudget::max_node_expansions`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — cooldown progression, cap, eligibility, and factory tests
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — retry-trigger gating, candidate-admission, and exhaustion-recording tests
3. Existing tests using `consecutive_budget_exhaustions` or `effective_max_expansions` must be updated to the new schema/contract

### Commands

1. `cargo test -p worldwake-ai -- decision_runtime`
2. `cargo test -p worldwake-ai -- planning`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-29
- Completion date: 2026-03-29
- What changed: `ExhaustionEntry` now stores deterministic cooldown state (`next_retry_tick`, `consecutive_failures`) instead of budget-halving state; the planning pipeline now retries at full budget, only wakes for retry-eligible entries, and records repeated budget exhaustion through the canonical helper; affected runtime/golden test literals were updated to the new schema; `SAVE_FORMAT_VERSION` was bumped to `12` so pre-change saves fail cleanly at the version gate instead of misreporting the wire format.
- Deviations from original plan: the ticket was broadened during reassessment to absorb the directly coupled planning-path updates originally split into `S37COOBASEXH-003/004/005`, because the original split would have left `worldwake-ai` intentionally uncompilable after removing the old helpers. `suppresses_planning()` was intentionally retained as the explicit `FrontierExhausted` semantic helper because that is cleaner than overloading cooldown eligibility. The save-format bump was also pulled in because the schema change was already real and leaving the old version number in place would have been an architectural correctness bug.
- Verification results: `cargo test -p worldwake-ai -- decision_runtime`, `cargo test -p worldwake-ai -- planning`, `cargo test -p worldwake-ai --lib from_saved_runtime_restores_and_validates_driver_state`, `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim -- save_load`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
