# S146GOASCHGOA-008: Observer Section 7 per-goal budget rendering + `GoalPlanningBudget::preset_name()`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — observer-only rendering extension; `preset_name()` helper added to existing core type
**Deps**: 006

## Problem

S146's debuggability story (FND-29) hinges on the observer surfacing which budget bounded each plan attempt. After ticket 006 lands `PlanAttemptTrace.goal_budget`, the observer's Section 7 (Planning) extension renders the preset name (SELF_CARE / TRAVEL_PURCHASE / PRODUCTION / INVESTIGATION / BOUNTY_ESCORT / CUSTOM) and the actual `max_depth` / `max_node_expansions` applied per plan attempt. A small `GoalPlanningBudget::preset_name() -> Option<&'static str>` helper on the core type does the reverse-lookup; observers without it would need to compare structures field-by-field.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Observer Section 7 lives at `crates/worldwake-cli/src/bin/observer.rs:4580-4581` (existing `## Section 7 — End-State Inventory & Resources` heading per the spec's D10 description). The format convention is `## Section N — Title\n` for major sections and `### Title\n` for subsections (verified via grep; example at line 3675 uses `### Planning\n`). After ticket 006, `PlanAttemptTrace.goal_budget: GoalPlanningBudget` is available at every trace site.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D10: the observer reads the budget field and renders preset name plus effective depth/expansions. The `preset_name() -> Option<&'static str>` helper on `GoalPlanningBudget` returns `Some("SELF_CARE")` etc. for preset-equivalent budgets and `None` for `CUSTOM` overrides — observer renders `CUSTOM` when `preset_name()` returns `None`.
3. Shared abstraction boundary under audit: `GoalPlanningBudget` (defined in ticket 002, populated on `GoalSchema` in ticket 004, traced in ticket 006) and the observer's Section 7 rendering pipeline. The `preset_name()` helper lives on `GoalPlanningBudget` itself because it's a property of the type, not of the observer — but its only consumer is this ticket's observer code per FND-28 "no dead paths".
4. No mixed-layer concern: this is an observer-only rendering ticket. No engine changes, no simulation state mutation, no decision-trace mutation (ticket 006 already added the field).

## Architecture Check

1. FND-29 (debuggability): the rendering makes "which budget bounded this attempt" trivially observable at a glance, without requiring readers to compare numeric field values mentally.
2. FND-28 (no dead paths): `preset_name()` lives on `GoalPlanningBudget` because it's a property of the type, but its only caller is the observer. Defining it now (rather than in ticket 002) ensures it lands with its consumer — no period where the helper exists without a use site.
3. Tooling-only scope: observer enhancements compress diagnostic output; world meaning is unchanged. CLAUDE.md determinism preserved.

## Verification Layers

1. `preset_name()` returns the correct name for each of 5 presets and `None` for a custom override → focused unit test in `crates/worldwake-core/src/goal_planning_budget.rs::#[cfg(test)]`
2. Observer Section 7 renders the preset name and effective depth/expansions in the expected format → focused unit test in `crates/worldwake-cli/src/bin/observer.rs::#[cfg(test)]` (or a sibling test module if the binary doesn't carry inline tests — use an integration test under `crates/worldwake-cli/tests/`)
3. Tooling-only ticket — no further verification layer applies; render output is the contract.

## What to Change

### 1. `preset_name()` helper on `GoalPlanningBudget`

In `crates/worldwake-core/src/goal_planning_budget.rs`:

```rust
impl GoalPlanningBudget {
    pub fn preset_name(&self) -> Option<&'static str> {
        if *self == Self::SELF_CARE { Some("SELF_CARE") }
        else if *self == Self::TRAVEL_PURCHASE { Some("TRAVEL_PURCHASE") }
        else if *self == Self::PRODUCTION { Some("PRODUCTION") }
        else if *self == Self::INVESTIGATION { Some("INVESTIGATION") }
        else if *self == Self::BOUNTY_ESCORT { Some("BOUNTY_ESCORT") }
        else { None }
    }
}
```

Add unit test in same module:

```rust
#[test]
fn preset_name_returns_canonical_names() {
    assert_eq!(GoalPlanningBudget::SELF_CARE.preset_name(), Some("SELF_CARE"));
    assert_eq!(GoalPlanningBudget::PRODUCTION.preset_name(), Some("PRODUCTION"));
    let custom = GoalPlanningBudget { max_depth: 99, max_node_expansions: 999,
        repair_budget_fraction: Permille::new_unchecked(123), max_strategic_expansions: 17 };
    assert_eq!(custom.preset_name(), None);
}
```

### 2. Observer Section 7 rendering extension

In `crates/worldwake-cli/src/bin/observer.rs`, locate the existing Section 7 (planning) per-attempt rendering loop and extend it to include the budget. Approximate insertion (exact site varies):

```rust
for attempt in &agent_decision_trace.plan_attempts {
    writeln!(out, "  Goal: {:?}", attempt.goal)?;
    writeln!(out, "    Budget: {} (depth {}, expansions {})",
        attempt.goal_budget.preset_name().unwrap_or("CUSTOM"),
        attempt.goal_budget.max_depth,
        attempt.goal_budget.max_node_expansions,
    )?;
    writeln!(out, "    Outcome: {:?}", attempt.outcome)?;
    // ... existing per-attempt rendering ...
}
```

Format convention: 4-space indent under the goal name, matching surrounding observer style (verify against neighboring renderings during implementation).

### 3. Observer rendering test

Add (or extend existing) integration test at `crates/worldwake-cli/tests/observer_section_7.rs` (or similar — confirm path during implementation):

```rust
#[test]
fn observer_renders_per_goal_budget_preset_name() {
    let report = run_observer_on_per_goal_budget_golden_scenario();
    assert!(report.contains("Budget: PRODUCTION (depth 16, expansions 384)"));
    assert!(report.contains("Budget: SELF_CARE (depth 6, expansions 96)"));
}
```

The test reuses `scenarios/per-goal-budget-golden.ron` from ticket 007.

## Files to Touch

- `crates/worldwake-core/src/goal_planning_budget.rs` (modify — `preset_name()` helper + unit test)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — Section 7 per-attempt budget rendering)
- Likely: `crates/worldwake-cli/tests/observer_section_7.rs` (new or modify; confirm path during implementation — `grep -rln "observer" crates/worldwake-cli/tests/` to find existing integration tests for observer output)

## Out of Scope

- S144's `PlanningMetrics` exhaustion-by-preset aggregation — owned by S144 (already archived; the spec just mentions S144 reads `goal_budget`, no S146 ticket implements aggregation).
- Adding new observer sections — only extends existing Section 7 per-attempt rendering.
- Changing the budget computation or trace recording — ticket 006 owns both.
- Rendering per-agent `budget_overrides` from `AgentSchemaContextProfile` — deferred along with the override-read path (see ticket 006 Out of Scope).

## Acceptance Criteria

### Tests That Must Pass

1. `preset_name_returns_canonical_names` — new core unit test
2. `observer_renders_per_goal_budget_preset_name` — new observer integration test
3. Existing observer test suite: `cargo test -p worldwake-cli`
4. `cargo test --workspace`
5. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Observer Section 7 renders the preset name for every plan attempt whose `goal_budget` matches a defined preset; renders `CUSTOM` for budgets that don't.
2. Observer rendering preserves CLAUDE.md determinism — same scenario, same seed, same observer output.
3. `preset_name()` is a pure read-only helper on `GoalPlanningBudget` (no side effects, no allocation).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal_planning_budget.rs` `#[cfg(test)]` — `preset_name_returns_canonical_names`
2. `crates/worldwake-cli/tests/observer_section_7.rs` (path subject to discovery) — `observer_renders_per_goal_budget_preset_name`

### Commands

1. `cargo test -p worldwake-core preset_name`
2. `cargo test -p worldwake-cli observer_renders_per_goal_budget`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`
