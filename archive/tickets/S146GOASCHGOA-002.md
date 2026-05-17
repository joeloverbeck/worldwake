# S146GOASCHGOA-002: Add `GoalPlanningBudget` core type

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — new type, no consumer wiring in this ticket
**Deps**: None

## Problem

S146 PR-17 introduces per-goal planning budgets so `Eat`'s search differs from `BakeBread`'s without per-agent profile tuning. The budget type and its 5 presets (SELF_CARE, TRAVEL_PURCHASE, PRODUCTION, INVESTIGATION, BOUNTY_ESCORT) land first as a standalone core type; ticket 004 places it on `GoalSchema` and ticket 006 applies it in the search dispatch.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Permille` exists at `crates/worldwake-core/src/numerics.rs:25` with `Permille::new(value: u16) -> Result<Self, &'static str>` (`:31`) and `Permille::new_unchecked(value: u16) -> Self` (`:43`). Since the 5 preset constants supply statically-known `<= 1000` values, `new_unchecked` is the const-friendly path. The `Permille` wrapper is `pub struct Permille(u16)` — small, copy-derive-friendly, no float dependencies.
2. Per `archive/specs/S146-goal-schema-and-per-goal-budgets.md` D2: `GoalPlanningBudget` derives `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`. Fields are `max_depth: u8`, `max_node_expansions: u16`, `repair_budget_fraction: Permille`, `max_strategic_expansions: u16`. All five preset constants are `pub const` associated items.
3. No shared abstraction boundary under audit — this ticket adds an isolated type. No existing consumer references `GoalPlanningBudget` (workspace-wide grep confirms 0 sites prior to this ticket).

## Architecture Check

1. Concrete typed budget per FND-3: fields express bounded resource limits as plain integers + `Permille`, not abstract scores. Each preset is a named const that future tickets read by name.
2. No backward-compat layer (FND-28): the type is net-new; no shim required.
3. Serialization-friendly without floats per `AGENTS.md` determinism invariant: all fields are integer or `Permille` (u16-backed).

## Verified Layers

1. New core type compiles + serializes round-trip → focused unit test in `crates/worldwake-core/src/goal_planning_budget.rs`'s `#[cfg(test)]` block
2. Preset constants satisfy invariants (depth ordering, `Permille` validity) → const-evaluated assertions in the unit test
3. Single-layer ticket — no cross-system invariants apply.

## Landed Changes

### 1. New file: `crates/worldwake-core/src/goal_planning_budget.rs`

Added `GoalPlanningBudget` with the four D2 fields and the five requested associated preset constants: `SELF_CARE`, `TRAVEL_PURCHASE`, `PRODUCTION`, `INVESTIGATION`, and `BOUNTY_ESCORT`.

### 2. Module declaration and re-export

Added `pub mod goal_planning_budget;` and `pub use goal_planning_budget::GoalPlanningBudget;` in `crates/worldwake-core/src/lib.rs`.

### 3. Focused unit tests

Inside `goal_planning_budget.rs`'s `#[cfg(test)] mod tests`:
- `presets_have_monotone_depth_ordering()` — SELF_CARE ≤ TRAVEL_PURCHASE ≤ PRODUCTION ≤ INVESTIGATION ≤ BOUNTY_ESCORT for `max_depth`
- `presets_have_monotone_expansion_ordering()` — same for `max_node_expansions`
- `budget_roundtrips_through_bincode()` — Serialize/Deserialize round-trip preserves all field values for `BOUNTY_ESCORT` (the largest preset)

## Landed Files

- `crates/worldwake-core/src/goal_planning_budget.rs` (added)
- `crates/worldwake-core/src/lib.rs` (module decl + re-export)

## Out of Scope

- `GoalPlanningBudget::preset_name()` reverse-lookup helper — owned by ticket 008 (observer rendering is its only consumer per FND-28 "no dead paths").
- Placement on `GoalSchema` — ticket 004 owns the schema-field addition.
- Application in search dispatch — ticket 006 owns the `effective_budget` computation.
- Trace provenance field — ticket 006 owns the `PlanAttemptTrace.goal_budget` addition.

## Acceptance Result

### Passed Gates

1. `cargo test -p worldwake-core goal_planning_budget` — new unit tests pass
2. `cargo build --workspace` — type compiles and re-exports resolve
3. `cargo clippy --workspace --all-targets -- -D warnings` — no lints

### Invariants

1. Preset constants are `const`-constructible (no runtime construction required).
2. `Permille::new_unchecked` is used only for statically-known `<= 1000` values (all 5 presets supply ≤ 500).
3. No `HashMap`, `HashSet`, float, or `std::time` dependency (`AGENTS.md` determinism).

## Verification Plan Result

### Added Tests

1. `crates/worldwake-core/src/goal_planning_budget.rs` `#[cfg(test)]` block — focused unit tests for preset ordering and serde round-trip (3 tests per "Focused unit tests" above).

### Command Results

1. `cargo test -p worldwake-core goal_planning_budget`
2. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh` — deferred to the final pre-PR harness gate for the whole S146 family.

## Outcome

Completed on 2026-05-17.

- Added the standalone core `GoalPlanningBudget` type with integer-only planning budget limits and `Permille` repair-budget fractions.
- Added the five S146 preset constants using `Permille::new_unchecked` only for statically-known in-range values.
- Re-exported the type from `worldwake-core` for later S146 tickets to place it on `GoalSchema` and apply it during search.
- Added focused unit coverage for preset ordering and bincode round-trip serialization.

## Verification Result

- Passed `cargo test -p worldwake-core goal_planning_budget`.
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`.
- Passed `cargo build --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Waived `scripts/verify.sh` for this ticket iteration because the harness runs the pre-PR wrapper once the full S146 ticket family is complete.
