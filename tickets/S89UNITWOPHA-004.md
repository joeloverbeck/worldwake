# S89UNITWOPHA-004: Give exploration fallback a lawful tactical contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — strategic/tactical planner boundary for exploration fallback
**Deps**: S89UNITWOPHA-001

## Problem

`S89UNITWOPHA-001` removed the two-phase whitelist and added `TravelToGoal` for real strategic `SatisfyGoal` stages, but it intentionally left `strategic::TacticalSubGoal::Explore` unscoped. The live strategic planner still emits `Explore` steps from `strategic::exploration_plan()` as adjacent probe waypoints, not as durable tactical destinations. During `001`, treating those steps like ordinary tactical barriers caused existing `worldwake-ai` search coverage to loop/regress, so the implementation was narrowed and the active S89 spec was corrected. The remaining architectural gap is that exploration fallback still has no lawful stable tactical contract.

## Assumption Reassessment (2026-04-11)

1. `crates/worldwake-ai/src/search/strategic.rs::exploration_plan()` currently builds a `StrategicPlan` by enumerating adjacent places from the actor's current place and emitting one `StrategicStep` per adjacent destination with `sub_goal: TacticalSubGoal::Explore`.
2. `crates/worldwake-ai/src/search/mod.rs::TacticalGoal::from_strategic_step()` now maps `TacticalSubGoal::SatisfyGoal` to `Some(TacticalGoal::TravelToGoal { ... })`, but still maps `TacticalSubGoal::Explore` to `None`.
3. Shared abstraction boundary under audit: `strategic::StrategicStep` / `TacticalSubGoal` as the strategic output contract, and `search::TacticalGoal` plus `apply_tactical_candidate_filter` as the tactical-consumption boundary.
4. The archived `archive/tickets/S89UNITWOPHA-001.md` outcome records the live mismatch: `Explore` steps are probe waypoints rather than stable terminal destinations, and scoping generic goal search to them caused loops/regressions in existing `cargo test -p worldwake-ai` coverage.
5. This is not a trace-only or tests-only gap. The unresolved question is production architecture: what exact stable destination or barrier contract should exploration fallback expose to tactical search, if any?
6. Information-path analysis: no new belief source is required. The open design problem is destination semantics and planner-stage ownership, not missing information.

## Architecture Check

1. The clean fix is to define a lawful contract for exploration fallback at the strategic/tactical boundary instead of reusing `Explore` as if it were already equivalent to `SatisfyGoal` or `AcquirePrerequisite`.
2. No backward-compatibility shim should be introduced. Either `Explore` becomes a real stable tactical stage with explicit semantics, or exploration fallback remains intentionally unscoped and the broader S89 draft is narrowed permanently.

## Verification Layers

1. Strategic fallback output has stable semantics -> focused strategic planner test over `strategic::plan()` / `exploration_plan()`
2. Tactical consumption of exploration fallback does not loop or over-constrain lawful plans -> focused `worldwake-ai` search tests at the tactical-filter/search boundary
3. Narrowed or expanded contract remains debuggable -> decision-trace or search-trace assertion at the strongest available planner-layer surface
4. Mixed-layer ticket: strategic output contract and tactical search behavior must be proved separately rather than with one generic crate-pass command

## What to Change

### 1. Reassess exploration fallback semantics

Decide whether `TacticalSubGoal::Explore` should represent:
- a durable tactical destination
- a transient probe waypoint that must stay unscoped
- or a different staged contract entirely

Make that decision against the live `strategic::exploration_plan()` output and the tactical-search barrier/filter behavior, not against the older S89 draft narrative.

### 2. Align the strategic/tactical boundary

If exploration fallback should become scoped, change the strategic output and tactical-consumption path together so the tactical layer receives a lawful stable barrier target rather than an arbitrary adjacent probe. If it should remain unscoped, codify that contract clearly and remove any remaining roadmap ambiguity.

### 3. Lock the contract with focused proof

Add focused tests and any needed planner/search trace assertions so future work cannot silently reintroduce the \"scope transient probe waypoints as destinations\" failure mode.

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `specs/S89-universal-two-phase-planning.md` (modify if the final contract differs again)

## Out of Scope

- Search-trace metadata recording for already lawful tactical goals (`S89UNITWOPHA-002`)
- The representative `TravelToGoal` regression tests already planned in `S89UNITWOPHA-003`
- Raising search budgets or broad heuristic tuning as a substitute for a lawful planner contract

## Acceptance Criteria

### Tests That Must Pass

1. Focused strategic-planner coverage proving the intended exploration fallback contract
2. Focused tactical-search coverage proving that the chosen contract does not reintroduce the exploration-loop regression seen during `S89UNITWOPHA-001`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Exploration fallback is either explicitly a stable tactical barrier or explicitly not one; the planner boundary no longer relies on ambiguous semantics
2. Tactical search does not scope itself to transient exploration probes unless the strategic layer now lawfully guarantees they are durable destinations
3. Active S89 specs/tickets describe the landed exploration contract accurately

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — focused coverage for the chosen exploration fallback contract at the strategic/tactical boundary

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
