# T01DEBVIS-011: Derived Pain/Danger tooltip rows

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: [T01DEBVIS-006](T01DEBVIS-006.md)

## Problem

Spec T01 §D6 says the agent hover tooltip includes `Pain` and `Danger` rows when those derived pressures are non-zero. T01DEBVIS-006 landed the tooltip and reusable `need_bar` widget, but truthfully rendered only the five embodied `HomeostaticNeeds` because `FrameSnapshot::AgentView` does not yet carry derived pain/danger pressure values. This ticket lands that missing read-side surface and renders the optional rows without changing engine semantics.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `archive/tickets/T01DEBVIS-006.md` records the completed tooltip seam and explicitly excludes derived Pain/Danger rows because `AgentView` currently has `needs: HomeostaticNeeds` and `drive_thresholds: DriveThresholds`, but no derived pressure values.
2. `crates/worldwake-visualizer/src/snapshot.rs` currently builds `AgentView` from the world, scheduler, and `AgentTickDriver`; the snapshot carries `needs` and thresholds only.
3. `crates/worldwake-ai/src/pressure.rs` exposes `derive_pain_pressure` and `derive_danger_pressure`, but both operate through the `worldwake_sim::GoalBeliefView` contract. Implementation must use a truthful existing read surface or add a narrow visualizer-local adapter; it must not fake danger from threshold bands alone.
4. Spec T01 §D6 remains valid: `Pain` and `Danger` rows are conditional, not always rendered. Zero derived pressure values should omit those rows.
5. Mixed-layer boundary: this is a read-only visualization contract between the visualizer `FrameSnapshot::AgentView` and AI pressure derivation through `GoalBeliefView`. It must not mutate authoritative world state, planner state, action validation, or pressure semantics.
6. Per template item 6: visualizer snapshot/rendering ticket; decision/action/event-log assertions are not the primary proof surface.
7. Adjacent contradiction classified as this ticket's scope: the completed tooltip has the widget and hover integration, while the active spec still requires optional derived rows. No broader modal tab or manual-QA implementation is implied here.

## Architecture Check

1. Store derived pressures on the snapshot/read model as cacheable UI values, not as authoritative state.
2. Reuse the `need_bar` widget from T01DEBVIS-006 for `Pain` and `Danger`; do not introduce a parallel bar implementation.
3. If the live visualizer cannot lawfully construct the pressure read view at snapshot time, reassess the ticket before coding rather than deriving a misleading value from unrelated fields.
4. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Derived pressure population -> focused snapshot/helper test proving non-zero pain and danger values are carried into `AgentView`.
2. Conditional tooltip rendering -> focused tooltip/helper test proving zero derived pressures omit `Pain`/`Danger` rows and non-zero values render those rows through the same need-bar path.
3. Per template item 6: single visualizer read/render layer; no AI action, authoritative resolution, or golden E2E layer mapping applies.

## What to Change

### 1. Extend the snapshot read model

Add a small visualizer-local derived pressure surface to `AgentView` (for example `derived_pain: Permille` and `derived_danger: Permille`, or a named `DerivedDrivePressures` struct).

Populate the values from the strongest truthful existing read surface. Prefer reusing `derive_pain_pressure` / `derive_danger_pressure` through a lawful `GoalBeliefView`; if an adapter is needed, keep it local to the visualizer snapshot path and document why it is read-only.

### 2. Render optional tooltip rows

Update `tooltip.rs` so the need-row stack adds:

- `Pain` with `agent.drive_thresholds.pain` when derived pain is non-zero.
- `Danger` with `agent.drive_thresholds.danger` when derived danger is non-zero.

Both rows must call the existing `need_bar` helper and use the same tooltip width contract as the five embodied needs.

## Files to Touch

- `crates/worldwake-visualizer/src/snapshot.rs` (modify)
- `crates/worldwake-visualizer/src/tooltip.rs` (modify)
- `crates/worldwake-visualizer/src/app.rs` (modify — pass action definitions into snapshot pressure derivation)
- `crates/worldwake-visualizer/src/canvas.rs` (test call-site update only)
- `crates/worldwake-visualizer/src/lib.rs` (modify only if a new helper module is added)
- `specs/T01-debug-visualizer.md` (modify — sync live snapshot read model)
- `tickets/T01DEBVIS-010.md` (no change needed; manual QA wording already names Pain/Danger when non-zero)

## Out of Scope

- Changing AI pressure semantics in `worldwake-ai`.
- Changing authoritative wound, combat, danger, or planner behavior.
- Detail modal Needs tab expansion unless it has already landed and shares the same derived pressure read surface.
- Manual QA execution; T01DEBVIS-010 owns the terminal checklist pass.

## Acceptance Criteria

### Tests That Must Pass

1. Focused snapshot/helper test proves non-zero derived pain and danger pressure values are present in `AgentView`.
2. Focused tooltip/helper test proves zero derived pressure rows are omitted and non-zero rows are rendered.
3. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. Derived pressure values are read-only snapshot/UI values, not new authoritative stored state.
2. The same `need_bar` widget renders embodied and derived pressure rows.
3. `Pain` and `Danger` rows are conditional on non-zero derived values, matching spec T01 §D6.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/snapshot.rs` or a local helper module — derived pressure population for a controlled non-zero pain/danger fixture.
2. `crates/worldwake-visualizer/src/tooltip.rs` — conditional row rendering helper test for zero and non-zero derived pressures.

### Commands

1. `cargo test -p worldwake-visualizer --lib -- --list`
2. `cargo test -p worldwake-visualizer`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added `DerivedDrivePressures { pain, danger }` to `FrameSnapshot::AgentView` as a read-only UI snapshot surface.
- Populated derived pressures through the existing AI pressure helpers (`derive_pain_pressure` / `derive_danger_pressure`) over `PerAgentBeliefView`, with the visualizer app passing its live `ActionDefRegistry` into snapshot construction so active attack actions can contribute to danger pressure.
- Updated the tooltip need-row path so non-zero `Pain` and `Danger` rows are appended to the same `need_bar` rendering stack as the five embodied needs; zero derived pressures are omitted.
- Added focused snapshot proof for non-zero pain/danger carriage and focused tooltip row-selection proof for omitted vs rendered derived rows.
- Updated spec T01's snapshot read-model section to include `DerivedDrivePressures` and the AI pressure helper read surface.

## Deviations

- `build_snapshot` now accepts `Option<&ActionDefRegistry>` so production visualizer snapshots can include active-action danger while unit tests that do not exercise current attackers can keep using `None`.
- The focused danger fixture proves the live pressure semantics exactly: an active attacker against an already wounded agent reaches the `critical` danger band, not merely `high`.
- `crates/worldwake-visualizer/src/lib.rs` was unchanged because no new module was needed.
- `tickets/T01DEBVIS-010.md` was unchanged because its manual QA checklist already says to verify Pain/Danger rows when non-zero.

## Verification Result

- Passed `cargo test -p worldwake-visualizer --lib -- --list`.
- Passed `cargo test -p worldwake-visualizer --lib snapshot::tests::snapshot_carries_derived_pain_and_danger_pressures -- --exact`.
- Passed `cargo test -p worldwake-visualizer --lib tooltip::tests::need_row_specs_include_non_zero_derived_pressures -- --exact`.
- Passed `cargo test -p worldwake-visualizer`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` before final ticket/spec closeout documentation sync; no Rust source changed afterward.
- Passed `git diff --check` after final ticket/spec closeout documentation sync.
