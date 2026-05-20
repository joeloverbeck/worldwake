# S149PARPLASEG-001: Typed PlanTerminalKind, discriminant key, and ProgressBarrier removal

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner terminal taxonomy (`worldwake-ai`), repair-kind enum mirror (`worldwake-core`), diagnostics histogram key, observer test data (`worldwake-cli`), save format
**Deps**: None (foundation ticket for S149)

## Problem

Every non-success plan path terminates as the generic `PlanTerminalKind::ProgressBarrier` (`crates/worldwake-ai/src/planner_ops.rs:388`), so the planner, observer, and S144 diagnostics cannot distinguish "missing fact" from "contested resource" from "depleted stock" from "out of jurisdiction" from "out of search budget." S149 D1–D3 replace the three-variant terminal with seven typed terminals and remove `ProgressBarrier` entirely (FND-28: no alias). This ticket is the compile-atomic foundation: redefining the enum breaks every exhaustive match at once, so the variant reshape, the payload-free histogram-key discriminant (D2), and the full cross-crate `ProgressBarrier`/`DowngradeToTypedBarrier` migration (D3) must land together.

## Assumption Reassessment (2026-05-20)

1. `PlanTerminalKind` is at `crates/worldwake-ai/src/planner_ops.rs:388` with exactly three variants (`GoalSatisfied`, `ProgressBarrier`, `CombatCommitment`) and derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. The new payload types (`TellTopic`, `EntityId`, `CommodityKind`, `u16`) are all `Copy`, so the derive set is preserved. The terminal producer is `terminal_kind(...)` at `crates/worldwake-ai/src/search/transition.rs:323`; creation sites are in `search/transition.rs` (4) and `search/mod.rs` fallback (2).
2. `RepairKind` is defined in `crates/worldwake-core/src/decision_event_payload.rs` (NOT in ai), with a `DowngradeToProgressBarrier` variant; it is consumed by `crates/worldwake-ai/src/plan_repair.rs`, `crates/worldwake-ai/src/agent_tick/execution.rs`, and `crates/worldwake-ai/tests/scenarios/plan_repair.rs`. Renaming `DowngradeToProgressBarrier` → `DowngradeToTypedBarrier` is therefore a cross-crate enum-variant rename (core definition + ai consumers + cli/tests).
3. Shared boundary under audit: the `PlanTerminalKind` enum (ai) and the `RepairKind` enum (core) — both are cross-crate exhaustive-match surfaces. `PlanTerminalKind` blast radius (per-file `ProgressBarrier` counts): `search/tests.rs` (24, test), `plan_repair.rs` (8), `goal_model.rs` (6, test), `agent_tick/planning.rs` (5), `search/transition.rs` (4), `agent_tick/tests.rs` (4, test), `agent_tick/observation.rs` (4), `planner_ops.rs` (3), `agent_tick/execution.rs` (3), `search/mod.rs` (2), `candidate_generation.rs` (2, test), `agent_tick/active_action.rs` (2), `plan_selection.rs` (1), `failure_handling.rs` (1), `decision_trace.rs` (1), plus `worldwake-cli/src/bin/observer.rs` (1, test data).
4. Existing inline tests that exercise the changed surfaces and must be updated: `plan_repair.rs::downgrade_to_typed_barrier_preserves_committed_prefix_only` (line 705), `plan_repair.rs::discrepancy_clearing_dispatch_covers_all_variants` (484), `plan_repair.rs::repair_kind_attempt_order_is_deterministic` (420); `scenario_diagnostics/mod.rs` test fixture at line 229 (`PlanTerminalKind::GoalSatisfied`); the `search/tests.rs` assertions (24 `ProgressBarrier` references).
5. Live planner surface: typed terminals are search outcomes produced by `terminal_kind` in `search/transition.rs`; the search layer knows its termination reason at each site, so each former `ProgressBarrier` construction maps to the typed terminal matching its cause. Direct no-plan search-budget exhaustion remains the existing `PlanSearchResult::BudgetExhausted` outcome; `SearchBudgetExhausted` exists for typed repair/discrepancy terminal contexts where a terminal record is actually emitted. Richer barrier-fact derivation (commodity/place/authority specifics) is deferred to ticket 004 — this ticket maps each site to the closest typed terminal the search layer can determine locally.
6. Adjacent contradiction: `terminal_kind_distribution` is keyed `BTreeMap<PlanTerminalKind, u64>` (`scenario_diagnostics/mod.rs:43`). Keying on the now-payload-bearing enum would fragment the histogram per payload value. This ticket adds the payload-free `PlanTerminalKindDiscriminant` (D2) and re-keys the distribution — required consequence of the enum reshape, not a separate bug.
7. Save format: `SAVE_FORMAT_VERSION = 90` (`crates/worldwake-sim/src/save_load.rs:7`). `PlanTerminalKind` persists in decision-event payloads, so removing `ProgressBarrier` is an incompatible serialized-format change. This ticket bumps `SAVE_FORMAT_VERSION` 90→91 (the spec attributed the bump to D10/ticket 003; relocated here because this is the larger format break — ticket 003's additive `Option` field rides `#[serde(default)]` without a second bump).

## Architecture Check

1. Merging D1+D2+D3 is mandated by the workspace-builds-after-each-ticket constraint: redefining the enum variants atomically changes every exhaustive match arm and the `RepairKind` variant arity, so the reshape, discriminant, and consumer migration cannot land in separate compiling states. Doing the discriminant re-key in the same ticket avoids double-churning the diagnostics fixture.
2. No backward-compat alias: `ProgressBarrier` is removed outright, not deprecated. `DowngradeToTypedBarrier` is renamed in place (single live representation), not shimmed. The discriminant is a derived projection (`From<&PlanTerminalKind>`), not a parallel authoritative type — it is a histogram key only (FND-27).

## Verification Layers

Result: verified as completed; direct no-plan search-budget exhaustion remains outcome-level as noted in `## Outcome`.

1. Typed terminal produced for each termination cause → focused unit test on `terminal_kind` (`search/transition.rs`) asserting the correct typed variant per cause.
2. Histogram does not fragment on payloads → focused unit test on the `PlanTerminalKindDiscriminant` keying in `scenario_diagnostics` (two terminals with different payloads but same kind aggregate to one bucket).
3. Repair downgrade path intact after rename → existing `plan_repair.rs::downgrade_to_typed_barrier_preserves_committed_prefix_only` (renamed) passes against `RepairKind::DowngradeToTypedBarrier`.
4. Save-format compatibility boundary → `SAVE_FORMAT_VERSION` bump verified by save/load roundtrip test (existing save_load coverage) — old `ProgressBarrier` payloads are rejected at the version boundary, not silently mis-decoded (FND-12).

## What to Change

Result: completed with the search-budget terminal boundary described in `## Outcome`.

### 1. Redefine `PlanTerminalKind` (D1)

In `crates/worldwake-ai/src/planner_ops.rs`, replace the three-variant enum with the seven typed variants: `GoalSatisfied`, `CombatCommitment`, `InformationBarrier { topic: TellTopic }`, `CoordinationBarrier { contested_resource: EntityId }`, `ResourceBarrier { commodity: CommodityKind, place: EntityId }`, `JurisdictionBarrier { authority: EntityId, jurisdiction: EntityId }`, `SearchBudgetExhausted { budget_consumed: u16, budget_total: u16 }`. Preserve the `Copy` + ordering + serde derives. Import `TellTopic` from `worldwake_core::belief`.

### 2. Add `PlanTerminalKindDiscriminant` (D2)

Define a payload-free discriminant enum alongside `PlanTerminalKind` (derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`) with a 1:1 `impl From<&PlanTerminalKind> for PlanTerminalKindDiscriminant`. Re-key `PlanningMetrics.terminal_kind_distribution` (`scenario_diagnostics/mod.rs:43`) to `BTreeMap<PlanTerminalKindDiscriminant, u64>`, updating the increment sites and the test fixture at line 229. Update `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` to the discriminant-keyed form.

### 3. Migrate `ProgressBarrier` removal + `DowngradeToTypedBarrier` rename (D3)

- Replace each `ProgressBarrier` construction in `search/transition.rs` and `search/mod.rs` with the typed terminal matching its local cause. Direct no-plan budget-out remains `PlanSearchResult::BudgetExhausted`; typed `SearchBudgetExhausted` is used in typed repair/discrepancy terminal contexts.
- Replace `ProgressBarrier` match arms in `agent_tick/planning.rs`, `agent_tick/observation.rs`, `agent_tick/execution.rs`, `agent_tick/active_action.rs`, `failure_handling.rs`, `plan_selection.rs`, `goal_model.rs`, `decision_trace.rs` with arms over the typed terminals (use a barrier-bearing catch-all where handling is uniform).
- Rename `RepairKind::DowngradeToProgressBarrier` → `DowngradeToTypedBarrier` in `crates/worldwake-core/src/decision_event_payload.rs`, and update consumers in `plan_repair.rs` (incl. `downgrade_to_progress_barrier` fn name → `downgrade_to_typed_barrier`), `agent_tick/execution.rs`, and tests.
- Update observer test data in `crates/worldwake-cli/src/bin/observer.rs` and all `#[cfg(test)]` assertions referencing `ProgressBarrier`.

### 4. Bump save format

Bump `SAVE_FORMAT_VERSION` 90→91 in `crates/worldwake-sim/src/save_load.rs:7`.

## Files to Touch

Result: completed; additional generated golden docs and visualizer/diagnostics fallout were updated as listed in `## Outcome`.

- `crates/worldwake-ai/src/planner_ops.rs` (modify) — enum reshape + discriminant
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify) — distribution re-key
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify) — terminal_kind construction sites + inline tests
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify) — 24 assertions
- `crates/worldwake-ai/src/agent_tick/planning.rs`, `observation.rs`, `execution.rs`, `active_action.rs`, `tests.rs` (modify)
- `crates/worldwake-ai/src/plan_repair.rs` (modify) — repair-kind rename + fn rename + tests
- `crates/worldwake-ai/src/goal_model.rs`, `failure_handling.rs`, `plan_selection.rs`, `decision_trace.rs`, `candidate_generation.rs` (modify)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify) — `RepairKind` variant rename
- `crates/worldwake-cli/src/bin/observer.rs` (modify) — test data + any terminal rendering
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify) — assertions
- `crates/worldwake-sim/src/save_load.rs` (modify) — version bump

## Out of Scope

- `PartialPlanSegment` and partial-plan storage (tickets 002, 003).
- Barrier→`Discrepancy`/`BlockingFact` mapping and resume-condition derivation (ticket 004) — this ticket only maps search sites to the closest typed terminal locally determinable.
- Resumption logic, companion subgoals, observer barrier-detail rendering (tickets 005–008).

## Acceptance Criteria

### Tests That Must Pass

Result: completed with the search-budget terminal boundary described in `## Outcome`.

1. Verified: focused typed-terminal tests cover information, coordination, and resource barriers; `SearchBudgetExhausted { .. }` is produced for typed repair/discrepancy terminal contexts, while direct no-plan budget exhaustion remains the existing `PlanSearchResult::BudgetExhausted`.
2. Verified: two `PlanTerminalKind` values differing only in payload map to the same `PlanTerminalKindDiscriminant` and aggregate to one `terminal_kind_distribution` bucket.
3. Renamed: `plan_repair` downgrade test passes against `RepairKind::DowngradeToTypedBarrier`.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `PlanTerminalKind` has no `ProgressBarrier` variant anywhere in the workspace; `RepairKind` has no `DowngradeToProgressBarrier` variant (FND-28: single live representation).
2. `terminal_kind_distribution` is keyed by the payload-free discriminant; no payload value can create a distinct bucket.
3. `PlanTerminalKind` remains `Copy` and its serde representation roundtrips under the bumped save format.

## Test Plan

Result: completed; all required commands are mirrored in `## Verification Result`.

### Modified Tests

Result: completed.

1. `crates/worldwake-ai/src/search/transition.rs` (inline) — per-cause typed-terminal assertions on `terminal_kind`.
2. `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (inline) — discriminant-keyed aggregation test.
3. `crates/worldwake-ai/src/plan_repair.rs` (inline) — rename existing downgrade test; assert behavior preserved.

### Commands

Result: completed.

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-core -p worldwake-sim -p worldwake-cli`
3. `scripts/verify.sh`

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 90→91 (PlanTerminalKind serialized-format break); ticket 003 deliberately avoids a second bump via `#[serde(default)]` on the additive `AgendaEntry.partial_plan_segment` field.

## Verification Result

1. Passed: `cargo test -p worldwake-ai`
2. Passed: `cargo test -p worldwake-core -p worldwake-sim -p worldwake-cli`
3. Passed: `cargo test --workspace --no-run`
4. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
5. Passed: `scripts/verify.sh`
6. Passed: `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`
7. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
8. Passed: removed-symbol scan found no live code references to `PlanTerminalKind::ProgressBarrier`, `RepairKind::DowngradeToProgressBarrier`, or `downgrade_to_progress_barrier`.

## Outcome

Completed: 2026-05-20.

S149 D1-D3 landed as the compile-atomic typed terminal foundation. `PlanTerminalKind` now carries typed barrier variants plus `PlanTerminalKindDiscriminant`; scenario diagnostics aggregate by the payload-free discriminant; the observer, visualizer, generated scenario diagnostics fixture, generated golden docs, and search/repair tests were updated to the new terminal taxonomy. `RepairKind::DowngradeToProgressBarrier` was renamed to `DowngradeToTypedBarrier`, and the downgrade path now maps repair context to typed barriers. `SAVE_FORMAT_VERSION` is now 91.

Deviation from draft scope: direct no-plan search budget exhaustion remains the existing `PlanSearchResult::BudgetExhausted` outcome rather than being converted into a found terminal plan. `PlanTerminalKind::SearchBudgetExhausted` exists and is used for typed repair/discrepancy terminal contexts; richer barrier-fact derivation and resume mapping remain in later S149 tickets.
