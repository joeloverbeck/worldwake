# S05MERSTOSTALL-008: Align facility stock theft surfaces with the ordinary theft path

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — transport theft validation, AI theft candidate generation, E17 crime path reuse
**Deps**: S05MERSTOSTALL-005

## Problem

The system must distinguish lawful facility access (by the facility controller) from theft of displayed or stored goods (by unauthorized agents). Without this distinction, any agent could manipulate facility containers without consequence.

## Assumption Reassessment (2026-04-01)

1. Stock actions already enforce lawful facility control through `can_exercise_control`; this ticket should not re-do that work.
2. E17 already produces `SuspectedTheft` from committed theft transfer events and owner investigation; do not add a parallel precondition-failure crime path unless the live architecture requires it.
3. Displayed and stored facility lots are still containerized world entities; check whether the ordinary `steal` action can target them.
4. Candidate generation for theft still excludes containerized lots; verify the exact read-model boundary before widening it.
5. Keep theft, discovery, and justice on one concrete stock model with no stock-specific theft alias or compatibility shim.

## Architecture Check

1. Lawful stock handling stays in `stock_actions.rs`; unlawful removal of the same displayed/stored lots must go through the ordinary `steal` path rather than a stock-specific theft action or a failed-precondition side channel.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Unauthorized outsider can `steal` a displayed or stored facility lot → action trace (focused test)
2. Lawful controller still cannot use `steal` on controllable stock → action trace (focused test)
3. Displayed or stored facility lots appear as theft targets for non-controllers → candidate generation test
4. Committed theft of facility stock still yields `SuspectedTheft` through the existing E17 event/perception pipeline → focused event-log/perception proof

## What to Change

### 1. Keep stock actions as the lawful access path

Do not add a second authorization layer here. Preserve the existing lawful-control checks in `stock_actions.rs` and treat them as the sanctioned manipulation path.

### 2. Evolve `steal` so facility stock can be stolen through the ordinary theft path

In `transport_actions.rs`: allow `steal` to target eligible contained item lots so displayed/stored facility stock is not trapped behind a lawful-only container rule. When a containerized facility lot is stolen, clear any storage/display assignment and listing state so the lot leaves facility custody cleanly.

### 3. Update theft candidate generation to match the new theft surface

In `candidate_generation.rs`: non-controller agents should consider locally observed facility stock as theft targets when the ordinary `steal` path would be lawful for them. Do not leave AI stuck on the older “contained means not stealable” assumption.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` or equivalent proof surface (verify existing path, touch only if live evidence shows a gap)

## Out of Scope

- Audit hooks for stock inspection (009)
- AI planning for stock actions (007)
- Golden tests (010)
- Adding a new crime classification path for failed unauthorized stock-action attempts

## Acceptance Criteria

### Tests That Must Pass

1. Authorized facility controller succeeds at all stock actions
2. Lawful controller cannot use `steal` on controllable facility stock
3. Unauthorized outsider can steal eligible displayed/stored facility stock through the ordinary theft action
4. Facility stock appears as a theft target for non-controllers in candidate generation
5. `SuspectedTheft` still comes from the existing committed-theft / investigation pipeline, not from failed stock-action preconditions
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Only the facility controller can lawfully manipulate facility containers
2. Unauthorized removal of displayed/stored facility stock requires theft, not lawful pickup
3. `SuspectedTheft` continues to reuse the E17 event / investigation pipeline — no parallel enforcement
3. System decoupling — crime classification reuses existing pipeline

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` — contained facility lot can be stolen by an unauthorized outsider
2. `crates/worldwake-systems/src/transport_actions.rs` — lawful controller still cannot steal controllable facility stock
3. `crates/worldwake-ai/src/candidate_generation.rs` — contained facility stock appears as theft targets for outsiders
4. `crates/worldwake-systems/src/perception.rs` or equivalent focused proof — committed facility-stock theft still records `SuspectedTheft`

### Commands

1. `cargo test -p worldwake-systems -- steal`
2. `cargo test -p worldwake-systems -- theft`
3. `cargo test -p worldwake-ai -- theft`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - widened the authoritative `steal` action in `crates/worldwake-systems/src/transport_actions.rs` so eligible contained facility stock can be stolen through the ordinary theft path instead of being blocked by a container-only precondition
  - cleared `StockAssignment` and `SaleListing` when stolen facility stock leaves display/storage custody so the stolen lot does not retain stale facility-state markers
  - updated `crates/worldwake-ai/src/candidate_generation.rs` so non-controller agents can emit theft goals for locally observed contained owned lots instead of treating containment alone as a theft blocker
  - kept `stock_actions.rs` as the lawful facility-control path and relied on the existing committed-theft / investigation pipeline for `SuspectedTheft`
- **Deviations from original plan**:
  - the original ticket was stale and was corrected before implementation: stock-action authorization was already complete, and no new precondition-failure crime path was added
  - `crates/worldwake-systems/src/perception.rs` needed no code change because the existing `Crime + Transfer` observation path already produced the required `SuspectedTheft` evidence
- **Verification results**:
  - `cargo test -p worldwake-systems steal_happy_path_removes_facility_stock_markers_from_displayed_lot -- --nocapture`
  - `cargo test -p worldwake-systems steal_rejects_lawfully_controllable_displayed_lot -- --nocapture`
  - `cargo test -p worldwake-systems -- steal`
  - `cargo test -p worldwake-systems -- theft`
  - `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate -- --nocapture`
  - `cargo test -p worldwake-ai -- theft`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
