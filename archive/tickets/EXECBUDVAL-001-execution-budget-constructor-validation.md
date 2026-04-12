# EXECBUDVAL-001: Add constructor validation to ExecutionBudget

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `crates/worldwake-core/src/execution_budget.rs`
**Deps**: None

## Problem

`ExecutionBudget` exposes all fields as `pub` with no validation. Setting `beam_width = 0` causes a silent semantic failure: at `search/mod.rs:642`, `successors.len().min(usize::from(execution_budget.beam_width))` truncates to 0 retained successors, emptying the frontier. The search then returns `BudgetExhausted` instead of `FrontierExhausted`, producing the wrong exhaustion result type. This undermines the deterministic exhaustion contracts that golden tests pin (e.g., `BudgetExhausted(300)`, `FrontierExhausted(54)`).

The struct is constructed via direct field initialization from scenarios (`unwrap_or_default()` at `scenario/mod.rs:369`), golden test helpers, conformance tests, and the persistence handler — all paths that could pass zero values.

## Assumption Reassessment (2026-04-12)

1. `execution_budget.rs:6-13` defines `pub struct ExecutionBudget { pub beam_width: u8, pub max_prerequisite_locations: u8, pub preferred_operator_boost: u8 }`. All fields are public. No constructor with validation exists. `Default::default()` assigns `beam_width: 8, max_prerequisite_locations: 3, preferred_operator_boost: 2`.
2. `search/mod.rs:640-642` uses `successors.len().min(usize::from(execution_budget.beam_width))` for beam pruning. If `beam_width == 0`, `retained_len == 0`, all successors are pruned. The frontier eventually empties and search returns `BudgetExhausted` (wrong) instead of `FrontierExhausted` (correct).
3. This is a cross-crate ticket: `ExecutionBudget` lives in `worldwake-core`, consumed by `worldwake-ai/src/search/`. The shared boundary is the struct's public API.
4. N/A — no failing golden scenario motivates this ticket; this arises from architectural debt analysis.
5. N/A — not a planner- or golden-driven ticket.
6. N/A — not an AI regression ticket.
7. N/A — no ordering dependency.
8. N/A — no heuristic removal.
9. N/A — not a stale-request or contested-affordance ticket.
10. N/A — not a political office-claim ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no golden scenario isolation.
13. `max_prerequisite_locations` is already guarded downstream by `usize::max(1, ...)` at `search/strategic.rs:170`, but this is an ad-hoc defense rather than a validated contract at the struct level.
14. The underlying problem statement is correct, but the direct fallout is slightly broader than the original draft claimed. Making `ExecutionBudget` constructor-only also requires updating direct field readers in `worldwake-cli/src/handlers/inspect.rs` and `worldwake-cli/src/bin/observer.rs`, in addition to the existing full-literal construction sites.
15. N/A — no cumulative arithmetic dependency.

## Architecture Check

1. Adding validation at the struct boundary (constructor or `debug_assert!`) is cleaner than ad-hoc downstream guards because:
   - It enforces the invariant once, at construction time, rather than at every consumption site
   - It follows the "validate at system boundaries" principle — `ExecutionBudget` is a cross-crate boundary
   - `strategic.rs:110` already has an ad-hoc `usize::max(1, ...)` guard for `max_prerequisite_locations`, showing the need for upstream validation
   - The struct derives `Serialize`/`Deserialize`, so deserialization is also a construction path that needs validation
2. No backwards-compatibility aliasing or shims. Existing `ExecutionBudget { field: value, .. }` literals remain valid if fields stay `pub`. If fields become non-pub, a `new()` constructor replaces them — this is a clean API change, not a shim.

## Verification Layers

1. `beam_width >= 1` enforced at construction → focused unit test with `#[should_panic]` or `Result::Err` for zero values
2. `max_prerequisite_locations >= 1` enforced at construction → focused unit test
3. Existing golden exhaustion contracts unchanged → `cargo test -p worldwake-ai golden_budget_exhaustion`
4. Deserialization rejects invalid values → roundtrip test with zero values
5. Cross-crate ticket: core defines the contract, ai consumes it. Verification at the core level (construction) is sufficient because the invariant is structural, not behavioral.

## What to Change

### 1. Add validated constructors and accessors to ExecutionBudget

Make the fields non-public and add:

- `pub fn try_new(beam_width: u8, max_prerequisite_locations: u8, preferred_operator_boost: u8) -> Result<Self, &'static str>`
- `pub fn new(beam_width: u8, max_prerequisite_locations: u8, preferred_operator_boost: u8) -> Self`
- field accessors (`beam_width()`, `max_prerequisite_locations()`, `preferred_operator_boost()`)

`beam_width == 0` and `max_prerequisite_locations == 0` must be rejected. `preferred_operator_boost == 0` remains valid (disables boosting, per the doc comment).

### 2. Add deserialization validation

Implement a custom `Deserialize` or add a `#[serde(deserialize_with = "...")]` that rejects `beam_width == 0` and `max_prerequisite_locations == 0` during deserialization. This protects the scenario/RON loading path.

### 3. Update construction sites

Update all full `ExecutionBudget { ... }` construction sites to use `new()` and all downstream field readers to use accessors:
- `crates/worldwake-core/src/execution_budget.rs` (tests, lines 55, 73)
- `crates/worldwake-core/src/delta.rs` (test, line 579)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (lines 252, 281, 288)
- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (line 197)
- `crates/worldwake-ai/tests/golden_offices.rs` (lines 465, 725)
- `crates/worldwake-ai/src/search/strategic.rs` (test, line 915)
- `crates/worldwake-ai/src/search/tests.rs` (line 69)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (test, line 1346)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (line 115)
- `crates/worldwake-ai/src/goal_model.rs` (test, line 2535)
- `crates/worldwake-cli/src/handlers/persistence.rs` (line 191)
- `crates/worldwake-cli/src/handlers/inspect.rs` (line 321)
- `crates/worldwake-cli/src/bin/observer.rs` (lines 2142-2145)

## Files to Touch

- `crates/worldwake-core/src/execution_budget.rs` (modify — add constructor, validation, deserialization guard)
- `crates/worldwake-core/src/delta.rs` (modify — update test construction site, only if fields become non-pub)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify — update construction sites)
- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify — update construction site)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — update construction sites)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — update test construction site)
- `crates/worldwake-ai/src/search/tests.rs` (modify — update construction site)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — update test construction site)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update construction site)
- `crates/worldwake-ai/src/goal_model.rs` (modify — update test construction site)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — update construction site)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — use ExecutionBudget accessors)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — use ExecutionBudget accessors)

## Out of Scope

- Adding validation to `CognitiveProfile` (separate concern, not identified as problematic)
- Changing the semantics of `beam_width` or `max_prerequisite_locations`
- Refactoring the search pipeline's beam pruning logic

## Acceptance Criteria

### Tests That Must Pass

1. New test: `ExecutionBudget::new(0, 3, 2)` panics or returns error
2. New test: `ExecutionBudget::new(8, 0, 2)` panics or returns error
3. New test: `ExecutionBudget::new(8, 3, 0)` succeeds (zero boost is valid)
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `beam_width >= 1` at all construction and deserialization paths
2. `max_prerequisite_locations >= 1` at all construction and deserialization paths
3. All existing golden exhaustion contracts unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/execution_budget.rs` — validation tests for zero values (`try_new` returns `Err` for `beam_width=0` / `max_prerequisite_locations=0`; zero `preferred_operator_boost` still succeeds)
2. `crates/worldwake-core/src/execution_budget.rs` — deserialization rejection test for invalid values

### Commands

1. `cargo test -p worldwake-core -- execution_budget`
2. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots`
3. `cargo test -p worldwake-ai --test conformance_execution_budget`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo test -p worldwake-cli --bin observer --no-run`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Added validated `ExecutionBudget::try_new` / `ExecutionBudget::new`, made the fields private, and exposed accessor methods so zero `beam_width` / `max_prerequisite_locations` can no longer enter the live API.
- Replaced derived deserialization with validated serde construction so scenario/save-load style deserialize paths now reject invalid execution budgets instead of silently constructing them.
- Updated the direct construction and read sites across `worldwake-core`, `worldwake-ai`, and `worldwake-cli` to use the validated constructor/accessor surface.
- Updated `search::strategic::tests::strategic_search_budget_tracks_execution_budget_stage_cap` to assert the new lawful minimum (`max_prerequisite_locations == 1` -> strategic budget `2`) instead of the old illegal zero-value case.

## Deviations

- The ticket's original `cargo test -p worldwake-ai golden_budget_exhaustion` selector was not an honest proof surface on the current branch because it executed zero tests. Verification used exact integration-test selectors instead.

## Verification Result

- Passed `cargo test -p worldwake-core -- execution_budget`
- Passed `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots`
- Passed `cargo test -p worldwake-ai --test conformance_execution_budget`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo test -p worldwake-cli --bin observer --no-run`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
