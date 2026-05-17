# S146GOASCHGOA-008: Observer failed-plan budget rendering + `GoalPlanningBudget::preset_name()`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — pure diagnostic helper on `GoalPlanningBudget`; observer-only rendering extension
**Deps**: archive/tickets/S146GOASCHGOA-006.md

## Problem

S146's debuggability story (FND-29) hinges on the observer surfacing which budget bounded failed plan attempts. After ticket 006 landed `PlanAttemptTrace.goal_budget`, the observer's Section 8 failed-plan attempt table needed to render the preset name (SELF_CARE / TRAVEL_PURCHASE / PRODUCTION / INVESTIGATION / BOUNTY_ESCORT / CUSTOM) and the actual `max_depth` / `max_node_expansions` applied to each rendered failed attempt. A small `GoalPlanningBudget::preset_name() -> Option<&'static str>` helper on the core type now does the reverse-lookup; observers no longer need to compare structures field-by-field.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Live observer numbering differs from the drafted spec prose: `crates/worldwake-cli/src/bin/observer.rs` Section 7 is `## Section 7 — End-State Inventory & Resources`, while per-agent plan attempt details are rendered under `## Section 8 — Per-Agent Decision Summary` in the `**Failed plan attempts**` table. This ticket's live rendering seam is therefore the existing Section 8 failed-attempt table, not a new Section 7 planning loop.
2. After ticket 006, `PlanAttemptTrace.goal_budget: GoalPlanningBudget` is available at every trace site. The failed-attempt table already iterates `planning.planning.attempts` filtered to `FrontierExhausted` / `BudgetExhausted`, so it is the stable observer row surface for exposing the applied budget on actionable failures.
3. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D10, now truthed to Section 8, the observer reads the budget field and renders preset name plus effective depth/expansions. The `preset_name() -> Option<&'static str>` helper on `GoalPlanningBudget` returns `Some("SELF_CARE")` etc. for preset-equivalent budgets and `None` for `CUSTOM` overrides — observer renders `CUSTOM` when `preset_name()` returns `None`.
4. Shared abstraction boundary under audit: `GoalPlanningBudget` (defined in ticket 002, populated on `GoalSchema` in ticket 004, traced in ticket 006) and the observer's failed-plan attempt rendering pipeline. The `preset_name()` helper lives on `GoalPlanningBudget` itself because it's a property of the type, not of the observer — but its only consumer is this ticket's observer code per FND-28 "no dead paths".
5. No mixed-layer concern: this is an observer-only rendering ticket. No simulation state mutation and no decision-trace mutation (ticket 006 already added the field).

## Architecture Check

1. FND-29 (debuggability): the rendering makes "which budget bounded this attempt" trivially observable at a glance, without requiring readers to compare numeric field values mentally.
2. FND-28 (no dead paths): `preset_name()` lives on `GoalPlanningBudget` because it's a property of the type, but its only caller is the observer. Defining it now (rather than in ticket 002) ensures it lands with its consumer — no period where the helper exists without a use site.
3. Tooling-only scope: observer enhancements compress diagnostic output; world meaning is unchanged. `AGENTS.md` determinism preserved.

## Verified Layers

1. `preset_name()` returns the correct name for each of 5 presets and `None` for a custom override → focused unit test in `crates/worldwake-core/src/goal_planning_budget.rs::#[cfg(test)]`
2. Observer Section 8 failed-plan attempts table renders the preset name and effective depth/expansions in the expected format → focused unit test in `crates/worldwake-cli/src/bin/observer.rs::#[cfg(test)]`
3. Tooling-only ticket — no further verification layer applies; render output is the contract.

## Landed Changes

### 1. `preset_name()` helper on `GoalPlanningBudget`

`crates/worldwake-core/src/goal_planning_budget.rs` now exposes `GoalPlanningBudget::preset_name() -> Option<&'static str>`, returning canonical names for all five presets and `None` for custom budgets.

### 2. Observer Section 8 failed-plan rendering extension

`crates/worldwake-cli/src/bin/observer.rs` now includes a `Budget` column in Section 8's failed-plan attempts table. Rows render `<PRESET> (depth <max_depth>, expansions <max_node_expansions>)`, with `CUSTOM` for non-preset-equivalent budgets.

### 3. Observer rendering test

Added an inline observer unit test that builds a synthetic decision trace with a failed plan attempt and verifies the rendered table includes `PRODUCTION (depth 16, expansions 384)`. This avoided depending on the stale drafted `scenarios/per-goal-budget-golden.ron` path; ticket 007 intentionally replaced that autonomous golden with focused search-trace validation.

## Landed Files

- `crates/worldwake-core/src/goal_planning_budget.rs`
- `crates/worldwake-cli/src/bin/observer.rs`
- `specs/S146-goal-schema-and-per-goal-budgets.md`

## Out of Scope

- S144's `PlanningMetrics` exhaustion-by-preset aggregation — owned by S144 (already archived; the spec just mentions S144 reads `goal_budget`, no S146 ticket implements aggregation).
- Adding new observer sections — only extends the existing Section 8 failed-plan attempt rendering.
- Changing the budget computation or trace recording — ticket 006 owns both.
- Rendering per-agent `budget_overrides` from `AgentSchemaContextProfile` — deferred along with the override-read path (see ticket 006 Out of Scope).

## Acceptance Result

### Tests Passed

1. `preset_name_returns_canonical_names` — new core unit test
2. `format_report_renders_goal_budget_for_failed_plan_attempts` — new observer binary unit test
3. `cargo test -p worldwake-cli`
4. `cargo test --workspace --quiet`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Observer Section 8 renders the preset name for every failed plan attempt whose `goal_budget` matches a defined preset; renders `CUSTOM` for budgets that don't.
2. Observer rendering preserves `AGENTS.md` determinism — same scenario, same seed, same observer output.
3. `preset_name()` is a pure read-only helper on `GoalPlanningBudget` (no side effects, no allocation).

## Test Plan Result

### Added Tests

1. `crates/worldwake-core/src/goal_planning_budget.rs` `#[cfg(test)]` — `preset_name_returns_canonical_names`
2. `crates/worldwake-cli/src/bin/observer.rs` — `format_report_renders_goal_budget_for_failed_plan_attempts`

## Outcome

Completed on 2026-05-17.

- Added a canonical preset-name reverse lookup on `GoalPlanningBudget`.
- Rendered the applied goal budget in the observer's Section 8 failed-plan attempts table.
- Truthed the active S146 spec and this ticket from the stale "Section 7 planning" wording to the live Section 8 failed-plan rendering seam.

## Deviations

- The landed observer proof is an inline `observer.rs` unit test over a synthetic decision trace, not a new integration test or autonomous golden. Ticket 007 already replaced the drafted per-goal-budget golden with focused search-trace validation, and the observer contract here is rendering `PlanAttemptTrace.goal_budget`.
- The observer renders budget provenance for failed plan attempts, because that is the existing per-attempt row surface in Section 8. This ticket did not add a new all-attempt observer section.
- `scripts/verify.sh` was not run; the live required gates it wraps for this diff were covered directly by `cargo test --workspace --quiet` and `cargo clippy --workspace --all-targets -- -D warnings`, with `cargo fmt --all` run before broad verification.

## Verification Result

- Passed `cargo test -p worldwake-core preset_name_returns_canonical_names`
- Passed `cargo test -p worldwake-cli format_report_renders_goal_budget_for_failed_plan_attempts`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test --workspace --quiet`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `git diff --check`
