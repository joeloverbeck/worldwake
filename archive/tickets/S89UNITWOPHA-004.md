# S89UNITWOPHA-004: Give exploration fallback a lawful tactical contract

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — strategic/tactical planner boundary for exploration fallback
**Deps**: S89UNITWOPHA-001

## Problem

`S89UNITWOPHA-001` removed the two-phase whitelist and added `TravelToGoal` for real strategic `SatisfyGoal` stages, but it intentionally left `strategic::TacticalSubGoal::Explore` unscoped. The live strategic planner still emits `Explore` steps from `strategic::exploration_plan()` as adjacent probe waypoints, not as durable tactical destinations. During `001`, treating those steps like ordinary tactical barriers caused existing `worldwake-ai` search coverage to loop/regress, so the implementation was narrowed and the active S89 spec was corrected. The remaining architectural gap is that exploration fallback still has no lawful stable tactical contract.

## Assumption Reassessment (2026-04-11)

1. `crates/worldwake-ai/src/search/strategic.rs::exploration_plan()` currently builds a `StrategicPlan` by enumerating every adjacent place from the actor's current place and emitting one `StrategicStep` per adjacent destination with `sub_goal: TacticalSubGoal::Explore`, sorted by travel ticks.
2. `crates/worldwake-ai/src/search/mod.rs::TacticalGoal::from_strategic_step()` now maps `TacticalSubGoal::SatisfyGoal` to `Some(TacticalGoal::TravelToGoal { ... })`, but still maps `TacticalSubGoal::Explore` to `None`, so exploration fallback receives no tactical scoping at all.
3. `crates/worldwake-ai/src/search/mod.rs::search_plan_with_trace_metadata()` only consumes the first strategic step when constructing the tactical goal (`strategic_plan.as_ref().and_then(|plan| plan.steps.first())`). The current multi-step exploration fallback is therefore not a lawful itinerary contract; only the first adjacent probe can affect search behavior.
4. `crates/worldwake-ai/src/search/transition.rs::terminal_kind()` does not treat arrival at an exploration fallback destination as `PlanTerminalKind::ProgressBarrier`. Even if exploration were scoped tactically, the search layer would still need an explicit arrival-as-barrier contract to return a lawful travel-only exploration plan.
5. Shared abstraction boundary under audit: `strategic::StrategicStep` / `TacticalSubGoal` as the strategic output contract, and `search::TacticalGoal`, `apply_tactical_candidate_filter`, and `transition::terminal_kind()` as the tactical-consumption and planner-terminal boundary.
6. The archived `archive/tickets/S89UNITWOPHA-001.md` outcome records the live mismatch: the old exploration steps acted like probe waypoints rather than stable tactical destinations, and naively scoping generic goal search to them caused regressions in existing `cargo test -p worldwake-ai` coverage.
7. Information-path analysis: no new belief source is required. The open design problem is destination semantics and planner-stage ownership, not missing information.

## Architecture Check

1. The clean fix is to give exploration fallback a dedicated barrier contract rather than treating `Explore` as interchangeable with `SatisfyGoal`. A single chosen exploration destination plus an explicit arrival-as-`ProgressBarrier` contract matches the actual planner boundary: travel to probe, then replan with new beliefs.
2. No backward-compatibility shim should be introduced. `Explore` should either become a real single-destination tactical barrier stage or remain intentionally unscoped; the current \"multi-step strategic list but only first step consumed\" shape is the architectural contradiction and should be removed.
3. Reassessment during implementation showed this barrier cannot be global. It is lawful only when the grounded goal has no explicit evidence carriers and the goal family actually uses probe travel as a fallback (`AcquireCommodity`, `SearchForMissing`). Evidence-backed or exact-bound goal families must keep their existing search paths.

## Verification Layers

1. Strategic fallback output is a single chosen exploration destination, not an adjacent-place list -> focused strategic planner test over `strategic::plan()` / `exploration_plan()`
2. Tactical consumption of exploration fallback returns a lawful arrival barrier instead of looping or remaining unscoped -> focused `worldwake-ai` search test at the tactical-filter/search boundary
3. Terminal semantics are explicit -> focused proof that reaching the exploration destination yields `PlanTerminalKind::ProgressBarrier`
4. Mixed-layer ticket: strategic output contract and tactical search behavior must be proved separately rather than with one generic crate-pass command

## What to Change

### 1. Reassess exploration fallback semantics

Replace the current \"all adjacent probe steps\" fallback with one chosen exploration destination that the tactical layer can lawfully consume. The live search boundary only consumes the first strategic step, so the fallback contract must become explicitly single-destination rather than a pseudo-itinerary.

Make that decision against the live `strategic::exploration_plan()` output and the tactical-search barrier/filter behavior, not against the older S89 draft narrative.

### 2. Align the strategic/tactical boundary

Land a dedicated exploration barrier contract:
- `strategic::exploration_plan()` chooses one deterministic adjacent destination
- `TacticalSubGoal::Explore` maps to a tactical exploration barrier with that destination only for no-evidence probe families
- arrival at that destination terminates as `PlanTerminalKind::ProgressBarrier`

### 3. Lock the contract with focused proof

Add focused tests and any needed planner/search trace assertions so future work cannot silently reintroduce the \"scope transient probe waypoints as destinations\" failure mode.

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify tests only)
- `specs/S89-universal-two-phase-planning.md` (modify if the final contract differs again)

## Out of Scope

- Search-trace metadata recording for already lawful tactical goals (`S89UNITWOPHA-002`)
- The representative `TravelToGoal` regression tests already planned in `S89UNITWOPHA-003`
- Raising search budgets or broad heuristic tuning as a substitute for a lawful planner contract

## Acceptance Criteria

### Tests That Must Pass

1. Focused strategic-planner coverage proving the intended exploration fallback contract
2. Focused tactical-search coverage proving that the chosen contract returns a one-step travel `ProgressBarrier` plan for exploration fallback and does not reintroduce the exploration-loop regression seen during `S89UNITWOPHA-001`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Exploration fallback is a single chosen tactical barrier destination, not an adjacent-place list whose later steps are ignored
2. Tactical search does not scope itself to transient exploration probes unless the strategic layer lawfully guarantees that exact destination as the current exploration barrier
3. Exploration fallback tactical scoping stays limited to no-evidence probe families; grounded goals with explicit evidence carriers do not get overridden by generic exploration
4. Active S89 specs/tickets describe the landed exploration contract accurately

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — focused coverage for the chosen exploration fallback contract at the strategic/tactical boundary

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-11.

- Changed `strategic::exploration_plan()` to choose one deterministic adjacent exploration destination instead of emitting an adjacent-place list that only the first step could ever influence.
- Reintroduced a dedicated tactical `Explore` barrier in planner search, added heuristic support for its destination, and made arrival at that destination commit as `PlanTerminalKind::ProgressBarrier`.
- Scoped that exploration barrier narrowly: it activates only for no-evidence `AcquireCommodity` and `SearchForMissing` fallback, so evidence-backed or exact-bound goals keep their existing search paths.
- Added focused strategic and search tests for the new exploration contract at both the strategic output and tactical search layers.

## Deviations

- The final landed contract is narrower than the initial ticket draft. Exploration fallback did not become a blanket tactical barrier for every goal family with empty strategic stages; doing so overrode lawful evidence-backed paths and route-choice behavior in existing planner coverage.

## Verification Result

- Passed `cargo test -p worldwake-ai test_empty_beliefs_exploration_fallback_chooses_single_nearest_probe`
- Passed `cargo test -p worldwake-ai search_empty_beliefs_exploration_fallback_returns_nearest_travel_barrier`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
