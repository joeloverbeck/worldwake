# E09NEEMET-006: Deprivation wounds and involuntary relief

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends the current metabolism system with deprivation consequence logic
**Deps**: Current `crates/worldwake-systems/src/needs.rs` metabolism tick, current `worldwake-core` wound schema, and later follow-up work in E09NEEMET-007 for actual forced sleep behavior

## Problem

Critical unmet needs must create concrete downstream effects. Without consequences, the survival loop has no teeth and agents can remain at max deprivation indefinitely without meaningful impact. The current code already has the physiology carriers and critical-exposure counters, but it still lacks concrete starvation/dehydration harm and bladder-accident behavior. This ticket implements the deprivation consequences that can be expressed cleanly through authoritative world state today.

## Assumption Reassessment (2026-03-10)

1. `HomeostaticNeeds`, `DeprivationExposure`, `MetabolismProfile`, `DriveThresholds`, `WoundList`, `WoundCause::Deprivation`, and `CommodityKind::Waste` already exist in `worldwake-core`.
2. `crates/worldwake-systems/src/needs.rs` already applies basal progression, active-action body costs, and critical-exposure updates, then commits via `WorldTxn`.
3. The current metabolism tick does **not** advance `bladder` from `MetabolismProfile.bladder_rate`; this is a live gap in the current implementation, not a future dependency.
4. There is currently no system-facing scheduler hook for “cancel current action and start Sleep now.” `InputKind` is external-input oriented, and no `Sleep` action exists yet.
5. Because of item creation and placement APIs already present in `WorldTxn`, involuntary relief can create a real `Waste` lot at the agent’s effective place without introducing aliases, wrappers, or compatibility paths.

## Architecture Check

1. Consequence logic should run inside the existing needs-system tick, after updated needs values and critical-exposure counters are known.
2. Starvation, dehydration, and bladder accidents are clean state-mediated consequences today: they mutate `WoundList`, `HomeostaticNeeds`, `DeprivationExposure`, and create/place a real waste item lot through `WorldTxn`.
3. Fatigue collapse should **not** be implemented here by inventing a scheduler shortcut or input alias. A clean forced-sleep path belongs with the real Sleep action and scheduler integration work in E09NEEMET-007 or an explicit follow-up ticket.
4. Existing `EventLog` causality is sufficient through normal `WorldTxn` commits tagged with `EventTag::System` and `EventTag::WorldMutation`; this ticket does not need new deprivation-specific event tags.

## What to Change

### 1. Extend `needs_system` in `crates/worldwake-systems/src/needs.rs`

After basal progression and critical-exposure updates, add deprivation consequences for the cases the current architecture can express directly.

Also correct basal bladder progression so metabolism actually applies `MetabolismProfile.bladder_rate` each tick before exposure/consequence evaluation.

**Starvation consequence**:
- If `hunger_critical_ticks >= metabolism.starvation_tolerance_ticks.get()`:
  - add `Wound { cause: WoundCause::Deprivation(Starvation), .. }` to the agent's `WoundList`
  - reset `hunger_critical_ticks` to 0 so the next wound requires another full tolerance period

**Dehydration consequence**:
- Same pattern as starvation but with thirst/dehydration thresholds and `DeprivationKind::Dehydration`

**Bladder accident**:
- If `bladder_critical_ticks >= metabolism.bladder_accident_tolerance_ticks.get()`:
  - set `bladder` to `Permille(0)`
  - increase `dirtiness` deterministically while staying in `Permille` range
  - create a `CommodityKind::Waste` item lot at the agent's effective place
  - reset `bladder_critical_ticks` to 0

### 2. Add a local deprivation-wound helper

Add a local helper in `needs.rs` that appends a deprivation wound to an existing `WoundList` or creates one if absent. Keep the representation explicit and avoid scattering wound construction across multiple branches.

### 3. Use existing world placement APIs for waste

Use `WorldTxn::create_item_lot` and `WorldTxn::set_ground_location` so a bladder accident produces a real waste entity at a real place through the same authoritative path as other item creation.

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify — add bladder progression and deprivation consequence logic)
- `crates/worldwake-systems/src/needs.rs` tests (modify — add bladder progression, deprivation wound, bladder accident, counter-reset, and event-log coverage)

## Out of Scope

- Forced collapse / forced sleep action-start behavior
- Scheduler-driven interruption for system-requested sleep
- Wound healing / progression (E12)
- Pain derivation from wounds (E13)
- AI awareness of deprivation (E13)
- Disease from dirtiness (future phase)

## Acceptance Criteria

### Tests That Must Pass

1. Agent at critical hunger for `starvation_tolerance_ticks` receives a wound with `WoundCause::Deprivation(Starvation)`.
2. Agent at critical thirst for `dehydration_tolerance_ticks` receives a wound with `WoundCause::Deprivation(Dehydration)`.
3. Agent with non-zero `bladder_rate` actually accumulates bladder pressure through the metabolism tick.
4. Agent at critical bladder for `bladder_accident_tolerance_ticks` has bladder reset to 0, dirtiness increased, and a waste item created at the agent’s location.
5. Deprivation counters reset after consequence fires — a second wound requires another full tolerance period.
6. Consequence application is captured through normal committed system/world-mutation events.
7. Non-critical agents receive no deprivation wounds or bladder accidents regardless of tick count.
8. Existing suite: `cargo test --workspace`

### Invariants

1. Deprivation wounds use the shared `WoundList` / `WoundCause` types — same carrier as combat wounds.
2. Consequences propagate through shared state only (Principle 12).
3. Conservation: waste item created by bladder accident is a real entity at a real location.
4. All `Permille` values stay in valid range after consequence application.
5. No stored fear or wellness scores are created by this system.
6. No scheduler/input compatibility shim is introduced just to approximate forced sleep before the proper Sleep action exists.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` (tests) — bladder basal progression, starvation wound, dehydration wound, bladder accident, counter reset, event emission

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-10
- Actual changes:
  - extended `crates/worldwake-systems/src/needs.rs` so basal metabolism now advances `bladder`
  - added starvation and dehydration deprivation wounds through the shared `WoundList` carrier
  - added bladder accidents that reset bladder pressure, increase dirtiness from the current bladder pressure, and create a real `Waste` item lot at the agent’s place
  - strengthened unit tests for bladder progression, deprivation wounds, bladder accidents, counter reset behavior, and event-log coverage
- Deviations from original plan:
  - did **not** add a `CollapseRequest` component or an input/scheduler shim for forced sleep
  - did **not** add new deprivation-specific `EventTag` variants
  - fatigue collapse remains deferred until the proper Sleep action and system-driven interruption path are implemented cleanly
- Verification results:
  - `cargo test -p worldwake-systems` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
