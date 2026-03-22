# E16DPOLPLAN-018: Affordance payload enumeration verification tests

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`enumerate_bribe_payloads` and `enumerate_threaten_payloads` in `crates/worldwake-systems/src/office_actions.rs` still have no direct focused unit tests for their payload emission contract. Broader political coverage already exists at the planning and golden-E2E layers, but this specific action-framework seam is only exercised indirectly today.

## Assumption Reassessment (2026-03-18)

1. `enumerate_bribe_payloads` is in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) and currently emits one `ActionPayload::Bribe` per positive-quantity commodity in `CommodityKind::ALL`, using the actor's full currently visible quantity as `offered_quantity` — confirmed from the implementation.
2. `enumerate_threaten_payloads` is in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) and currently does not enumerate candidate targets itself. It emits a single `ActionPayload::Threaten` for `targets.first()` after target binding has already happened, and only if `target != actor` and `view.combat_profile(actor).is_some()` — corrected scope.
3. The previous ticket text incorrectly treated these helpers as target enumerators ("agents at same location"). Co-location and target selection are enforced one layer up by `bribe_action_def` / `threaten_action_def` through `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }` and matching action preconditions in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs).
4. The repository already contains higher-layer coverage for the surrounding political architecture:
   - authoritative office-action commit coverage in `office_actions::tests::*`
   - planning-state and planner-selection coverage in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - golden political scenarios in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs)
   This ticket therefore remains a focused unit-test gap, not a planner or system gap.
5. `cargo test -p worldwake-systems -- --list` confirms there are currently no direct tests named for bribe/threaten payload enumeration in `office_actions::tests` — confirmed.

## Architecture Check

1. Focused unit tests here are worthwhile because payload generation is a distinct contract between the action framework's target binding and the authoritative/social action handlers. Existing planner and golden tests prove the broader architecture works, but they do not isolate regressions in payload shape or gating.
2. The clean architecture is to keep target binding in `TargetSpec`/preconditions and keep payload helpers limited to payload emission for already-bound targets. The ticket should verify that contract rather than re-test target discovery in the wrong layer.
3. No production-code change is justified here. The current architecture is already cleaner than moving target discovery into these payload helpers, because that would duplicate binding logic, blur responsibilities, and make extensibility worse.

## What to Change

### 1. Bribe payload tests in office_actions.rs

1. Verify full commodity quantity is offered per payload (5 bread → `offered_quantity: Quantity(5)`)
2. Verify no self-bribe (actor == target → empty Vec)
3. Verify empty when agent has no commodities
4. Verify multiple payloads for agent with multiple commodity types

### 2. Threaten payload tests in office_actions.rs

1. Verify payload is emitted for the already-bound first target when actor has a combat profile
2. Verify no self-threaten (actor excluded from payload emission)
3. Verify empty when no valid bound target is present
4. Verify empty when actor lacks a combat profile

## Files to Touch

- `tickets/E16DPOLPLAN-018.md` (modify — reassess and correct scope before implementation)
- `crates/worldwake-systems/src/office_actions.rs` (modify — test module)

## Out of Scope

- Bribe/Threaten action execution
- Target binding / co-location enumeration by the action framework
- Planning semantics already covered in `crates/worldwake-ai/src/goal_model.rs`
- Golden tests already covered in `crates/worldwake-ai/tests/golden_offices.rs`
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `bribe_payload_offers_full_stock` — 5 bread → Quantity(5)
2. `bribe_payload_no_self_bribe` — empty Vec for self
3. `bribe_payload_empty_without_commodities` — empty Vec
4. `bribe_payload_multiple_commodity_types` — one payload per type
5. `threaten_payload_emits_for_bound_target_with_combat_profile` — first bound target becomes `ThreatenActionPayload`
6. `threaten_payload_no_self_threaten` — actor excluded
7. `threaten_payload_empty_without_targets` — empty Vec
8. `threaten_payload_requires_combat_profile` — empty Vec without combat profile
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Full stock offered per bribe payload (not partial)
2. Self-targeting never produced by payload helpers
3. Threaten payload emission is gated on the actor having a combat profile
4. Target co-location remains enforced by action target binding/preconditions, not by these helpers

## Tests

### New/Modified Tests

1. `office_actions.rs::tests::bribe_payload_offers_full_stock`
2. `office_actions.rs::tests::bribe_payload_no_self_bribe`
3. `office_actions.rs::tests::bribe_payload_empty_without_commodities`
4. `office_actions.rs::tests::bribe_payload_multiple_commodity_types`
5. `office_actions.rs::tests::threaten_payload_emits_for_bound_target_with_combat_profile`
6. `office_actions.rs::tests::threaten_payload_no_self_threaten`
7. `office_actions.rs::tests::threaten_payload_empty_without_targets`
8. `office_actions.rs::tests::threaten_payload_requires_combat_profile`

### Rationale

1. `bribe_payload_offers_full_stock` locks the current "full visible stock per commodity" contract that both planning and authoritative validation implicitly depend on.
2. `bribe_payload_no_self_bribe` prevents a degenerate self-target payload from leaking past the helper even if a caller binds the actor incorrectly.
3. `bribe_payload_empty_without_commodities` verifies the helper stays data-driven and does not invent zero-quantity offers.
4. `bribe_payload_multiple_commodity_types` protects the one-payload-per-positive-commodity contract rather than a single best-offer shortcut.
5. `threaten_payload_emits_for_bound_target_with_combat_profile` verifies the helper's real responsibility: payload emission for an already-bound target.
6. `threaten_payload_no_self_threaten` preserves the self-target guard at the payload layer.
7. `threaten_payload_empty_without_targets` covers the no-bound-target edge case directly.
8. `threaten_payload_requires_combat_profile` captures the current architectural gate that keeps non-combat actors from receiving threaten affordances.

### Commands

1. `cargo test -p worldwake-systems bribe_payload_`
2. `cargo test -p worldwake-systems threaten_payload_`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-18
- Actual changes:
  - Corrected the ticket's assumptions and scope to match the current architecture.
  - Added 8 focused unit tests in `crates/worldwake-systems/src/office_actions.rs` covering bribe/threaten payload emission semantics.
  - Left production code unchanged.
- Deviations from original plan:
  - Removed the incorrect "target enumeration" framing for `enumerate_threaten_payloads`; target binding is handled by `TargetSpec` and action preconditions, not these helpers.
  - Explicitly documented that planner-state and golden political coverage already exist in `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/tests/golden_offices.rs`, so this ticket remained a systems-layer focused test gap rather than an AI or golden gap.
- Verification results:
  - `cargo test -p worldwake-systems bribe_payload_` ✅
  - `cargo test -p worldwake-systems threaten_payload_` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
