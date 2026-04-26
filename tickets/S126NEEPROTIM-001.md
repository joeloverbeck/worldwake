# S126NEEPROTIM-001: Variants and projection helpers (foundation)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `FrameAssumption::NeedSafeUntilTick` variant, `Discrepancy::NeedHorizonExceeded` variant, `HomeostaticNeeds::projected_tick_of` derived helper, keyed `MetabolismProfile::rate(need)` and `DriveThresholds::high(need)` accessors. Bumps `SAVE_FORMAT_VERSION` from 47 to 48 to mark the schema change.
**Deps**: specs/S126-need-projection-time-budget.md

## Problem

Three downstream tickets (002, 003, 004) all need new types and small helpers in `worldwake-core` and a single rendering arm in `worldwake-ai/decision_trace.rs`. Landing them as separate variant-only tickets would force a long chain of single-line PRs; bundling them as a foundation ticket lets each downstream ticket import a complete type surface from day one.

The bundle is compile-safe because `FrameAssumption` and `Discrepancy` are matched exhaustively in only two places — the trace formatter (`decision_trace.rs:2042-2047`) and `evaluate_assumptions` (`agent_tick/frame.rs:339-392`). The trace formatter gets the real arm here; `evaluate_assumptions` gets a no-op placeholder that ticket 003 replaces with the real evaluation logic. Without the placeholder, adding `NeedSafeUntilTick` to the enum would break the workspace build.

## Assumption Reassessment (2026-04-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `FrameAssumption` enum at `crates/worldwake-core/src/intention_frame.rs:62-74` has 4 variants today (`TargetAlive`, `RouteExists`, `NoCriticalThreat`, `CommodityAvailableAt`) and derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. The new `NeedSafeUntilTick { need: HomeostaticNeedId, until_tick: Tick }` payload is `Copy`-compatible (both fields are `Copy`). `Discrepancy` enum at `crates/worldwake-core/src/discrepancy.rs:6-27` has 10 unit variants and derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`; the new `NeedHorizonExceeded { need: HomeostaticNeedId, projected_breach_tick: Tick }` payload is also Copy-compatible. `HomeostaticNeeds` (`needs.rs:8-88`) exposes `value(need: HomeostaticNeedId) -> Permille` (lines 71-79). `HomeostaticNeedId::ALL: [Self; 5]` (lines 28-34). `MetabolismProfile` (lines 116-217) carries per-need `*_rate: Permille` fields at lines 119-127. `DriveThresholds` (`drives.rs:56-117`) already exposes `critical(need)` per-need keyed accessor at lines 92-100; new `high(need)` mirrors that pattern over `ThresholdBand::high()` at `drives.rs:46-48`. `Permille` is `u16`-backed (`numerics.rs:24`) bounded `0..=1000`. `Tick(pub u64)` (`ids.rs:55-77`).
2. Spec authority: `specs/S126-need-projection-time-budget.md` D1, D2, D3, D6 (variant addition only — recording-side body lands in ticket 003), D7. The reassessment confirmed `SAVE_FORMAT_VERSION` lives at `crates/worldwake-sim/src/save_load.rs:6` and is currently 47.
3. Shared abstraction boundary: this ticket touches the type surface (`FrameAssumption`, `Discrepancy`, projection helper, keyed accessors) without changing any function's runtime contract beyond adding a no-op placeholder match arm. The placeholder arm in `evaluate_assumptions` returns the existing `AllPass`-equivalent fall-through (no-op match body); ticket 003 replaces it with the real evaluation. The placeholder is named in §1 of this ticket's What to Change so reviewers don't misread the no-op as the final contract.
4. `decision_trace.rs:2042-2047` is the only exhaustive `FrameAssumption` formatter site in production code; `agent_tick/frame.rs:339-392` is the only exhaustive match in evaluation logic. `Discrepancy` is rendered via Debug derive (`observer.rs:476` uses `{discrepancy:?}`), so `NeedHorizonExceeded` requires no manual rendering arm.
5. SAVE_FORMAT_VERSION bump rationale: `FrameAssumption` is stored inside `IntentionFrame.assumptions: Vec<FrameAssumption>` (an ECS component, bincode-serialized). `Discrepancy` is stored inside `DiscrepancyEntry.discrepancy` inside `DiscrepancyMemory.entries` (also an ECS component, bincode-serialized). Adding new enum variants is forward-compatible for new code reading old saves (no old save contains the new tag) but is a schema change by project convention; bump from 47 to 48.

## Architecture Check

1. Bundling type-surface changes (variants + helpers) in one ticket lets downstream tickets import the complete surface immediately rather than chaining single-line variant additions across multiple tickets. The placeholder arm in `evaluate_assumptions` is a transparent compile-safety device, not a fossilized branch — ticket 003 replaces it with the real logic in the same release cycle. No backward-compatibility shims survive.
2. The new keyed accessors (`MetabolismProfile::rate(need)`, `DriveThresholds::high(need)`) mirror the existing `DriveThresholds::critical(need)` precedent (`drives.rs:92-100`); they are not novel architecture.
3. `projected_tick_of` lives on `HomeostaticNeeds` (the data owner) rather than on a free function, matching `HomeostaticNeeds::value(need)` (`needs.rs:71-79`) which already keys per-need reads by the same enum.

## Verification Layers

1. `FrameAssumption::NeedSafeUntilTick` exists and round-trips bincode → focused unit test in `intention_frame.rs::tests`.
2. `Discrepancy::NeedHorizonExceeded` exists and round-trips bincode → focused unit test in `discrepancy.rs::tests` (alongside the existing `discrepancy_roundtrips_through_bincode` pattern at `discrepancy.rs:139-146`).
3. `HomeostaticNeeds::projected_tick_of` arithmetic correctness (current ≥ target → returns current_tick; rate == 0 → returns None; otherwise returns `current_tick + ⌈(target − current) / rate⌉`) → focused unit tests in `needs.rs::tests`.
4. `MetabolismProfile::rate(need)` and `DriveThresholds::high(need)` return the same value as direct field access for every variant of `HomeostaticNeedId::ALL` → focused unit tests in `needs.rs::tests` and `drives.rs::tests` respectively.
5. Decision-trace formatter renders `NeedSafeUntilTick { need: Hunger, until_tick: 412 }` as `"NeedSafeUntilTick { need: Hunger, until_tick: 412 }"` → focused unit test in `decision_trace.rs::tests` (single-layer rendering test; no agent_tick scenario needed).
6. Workspace-wide compile after the placeholder arm is added → `cargo build --workspace` (no other layer mapping is needed because this ticket is purely type-surface + one no-op arm).

## What to Change

### 1. Add `FrameAssumption::NeedSafeUntilTick` variant

In `crates/worldwake-core/src/intention_frame.rs`, extend the `FrameAssumption` enum with the new variant per spec D1:

```rust
NeedSafeUntilTick {
    need: HomeostaticNeedId,
    until_tick: Tick,
},
```

Add `HomeostaticNeedId` to the existing `use crate::{...}` import line. Confirm the enum's `Copy` derive remains valid (both payload fields are Copy).

### 2. Add `Discrepancy::NeedHorizonExceeded` variant

In `crates/worldwake-core/src/discrepancy.rs`, extend the flat `Discrepancy` enum with the new variant per spec D6 part 1:

```rust
NeedHorizonExceeded {
    need: HomeostaticNeedId,
    projected_breach_tick: Tick,
},
```

Add `HomeostaticNeedId` to the existing `use crate::{...}` import line. Confirm the enum's `Copy` derive remains valid.

### 3. Add `HomeostaticNeeds::projected_tick_of` derived helper

In `crates/worldwake-core/src/needs.rs`, add the helper method on `HomeostaticNeeds` per spec D2:

```rust
#[must_use]
pub fn projected_tick_of(
    &self,
    need: HomeostaticNeedId,
    target_level: Permille,
    base_rate: Permille,
    current_tick: Tick,
) -> Option<Tick> {
    let current = self.value(need).value();
    let target = target_level.value();
    if current >= target {
        return Some(current_tick);
    }
    let rate = base_rate.value();
    if rate == 0 {
        return None;
    }
    let delta_ticks = u64::from(target - current).div_ceil(u64::from(rate));
    Some(Tick(current_tick.0.saturating_add(delta_ticks)))
}
```

The early `current >= target` return prevents `u16` underflow in the subtraction.

### 4. Add `MetabolismProfile::rate(need)` keyed accessor

In `crates/worldwake-core/src/needs.rs`, add an `impl MetabolismProfile { pub const fn rate(...) }` block per spec D3:

```rust
#[must_use]
pub const fn rate(&self, need: HomeostaticNeedId) -> Permille {
    match need {
        HomeostaticNeedId::Hunger => self.hunger_rate,
        HomeostaticNeedId::Thirst => self.thirst_rate,
        HomeostaticNeedId::Fatigue => self.fatigue_rate,
        HomeostaticNeedId::Bladder => self.bladder_rate,
        HomeostaticNeedId::Dirtiness => self.dirtiness_rate,
    }
}
```

### 5. Add `DriveThresholds::high(need)` keyed accessor

In `crates/worldwake-core/src/drives.rs`, add `pub const fn high(&self, need: HomeostaticNeedId) -> Permille` to the existing `impl DriveThresholds` block (the same impl that holds `critical(need)` at lines 92-100). Mirror the structure of `critical(need)` exactly:

```rust
#[must_use]
pub const fn high(&self, need: HomeostaticNeedId) -> Permille {
    match need {
        HomeostaticNeedId::Hunger => self.hunger.high(),
        HomeostaticNeedId::Thirst => self.thirst.high(),
        HomeostaticNeedId::Fatigue => self.fatigue.high(),
        HomeostaticNeedId::Bladder => self.bladder.high(),
        HomeostaticNeedId::Dirtiness => self.dirtiness.high(),
    }
}
```

Scope is the 5 `HomeostaticNeedId` variants only; `pain` and `danger` bands remain accessible via direct field access and are not in scope.

### 6. Decision-trace formatter arm for `NeedSafeUntilTick`

In `crates/worldwake-ai/src/decision_trace.rs`, locate the exhaustive `FrameAssumption` formatter at lines 2042-2047 and add the new arm per spec D7:

```rust
FrameAssumption::NeedSafeUntilTick { need, until_tick } => {
    format!("NeedSafeUntilTick {{ need: {need:?}, until_tick: {until_tick:?} }}")
}
```

Match the formatting style of the existing arms (the `CommodityAvailableAt` arm at line 2047 is the nearest analog).

### 7. Placeholder arm in `evaluate_assumptions` (replaced by ticket 003)

In `crates/worldwake-ai/src/agent_tick/frame.rs`, locate the exhaustive match in `evaluate_assumptions` (lines 339-392) and add a no-op placeholder arm to maintain compile-safety:

```rust
FrameAssumption::NeedSafeUntilTick { .. } => {
    // PLACEHOLDER: ticket S126NEEPROTIM-003 replaces this arm with the real
    // projection re-evaluation logic. Today the assumption is never produced
    // (ticket 002 ships population), so the no-op match body is unreachable
    // at runtime and is present only to keep the workspace compiling.
}
```

The placeholder is documented in-source so that anyone reading the file before ticket 003 lands understands the deferred contract. Ticket 003's What to Change explicitly references replacing this arm.

### 8. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs`, change `pub const SAVE_FORMAT_VERSION: u32 = 47;` to `pub const SAVE_FORMAT_VERSION: u32 = 48;` to mark the schema change for the new enum variants.

## Files to Touch

- `crates/worldwake-core/src/intention_frame.rs` (modify) — add `NeedSafeUntilTick` variant + `HomeostaticNeedId` import
- `crates/worldwake-core/src/discrepancy.rs` (modify) — add `NeedHorizonExceeded` variant + `HomeostaticNeedId` import
- `crates/worldwake-core/src/needs.rs` (modify) — add `projected_tick_of` helper + `MetabolismProfile::rate(need)` keyed accessor + tests
- `crates/worldwake-core/src/drives.rs` (modify) — add `DriveThresholds::high(need)` keyed accessor + tests
- `crates/worldwake-ai/src/decision_trace.rs` (modify) — add `NeedSafeUntilTick` formatter arm + test
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify) — add placeholder arm in `evaluate_assumptions`
- `crates/worldwake-sim/src/save_load.rs` (modify) — bump `SAVE_FORMAT_VERSION` 47 → 48

## Out of Scope

- `populate_assumptions` extension and its caller updates — ticket 002 (D4)
- `evaluate_assumptions` real arm logic (replaces the placeholder) and `record_assumption_failure` extension and their caller updates — ticket 003 (D5 + D6 part 2)
- Golden test coverage — ticket 004 (D8)
- Any save migration logic for save format 47 → 48 (the change is forward-additive; old saves don't contain the new variants, new code loads them as-is)

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `FrameAssumption::NeedSafeUntilTick { need: Hunger, until_tick: Tick(100) }` round-trips through bincode.
2. New unit test: `Discrepancy::NeedHorizonExceeded { need: Hunger, projected_breach_tick: Tick(50) }` round-trips through bincode.
3. New unit test: `HomeostaticNeeds::projected_tick_of` returns `Some(current_tick)` when `current >= target_level`.
4. New unit test: `HomeostaticNeeds::projected_tick_of` returns `None` when `base_rate == Permille(0)` and `current < target`.
5. New unit test: `HomeostaticNeeds::projected_tick_of` returns `Some(current_tick + ⌈(target − current) / rate⌉)` for a representative non-trivial case (e.g., current=400, target=700, rate=50, current_tick=10 → expected `Some(Tick(16))`).
6. New unit test: for each `HomeostaticNeedId::ALL` variant, `MetabolismProfile::rate(need)` returns the same value as direct field access on a populated profile.
7. New unit test: for each `HomeostaticNeedId::ALL` variant, `DriveThresholds::high(need)` returns the same value as `thresholds.<field>.high()` on a populated `DriveThresholds`.
8. New unit test: decision-trace formatter renders `NeedSafeUntilTick { need: Hunger, until_tick: Tick(412) }` as the expected human-readable string.
9. Existing suite: `cargo test --workspace` passes after the placeholder arm is added.
10. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. `FrameAssumption` and `Discrepancy` retain their `Copy` derive — both new payloads are Copy.
2. `HomeostaticNeeds::projected_tick_of` never panics on any combination of valid `Permille` inputs (the early-return guard prevents underflow; `Tick.0.saturating_add` prevents tick overflow).
3. `SAVE_FORMAT_VERSION` increments monotonically by 1 per schema change.
4. The placeholder arm in `evaluate_assumptions` is unreachable at runtime in this ticket because no caller produces `NeedSafeUntilTick` until ticket 002 lands.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/intention_frame.rs` — add a bincode round-trip test for `NeedSafeUntilTick`. Sibling pattern: the `intention_frame_satisfies_component_bounds` test at lines 184-187.
2. `crates/worldwake-core/src/discrepancy.rs` — add a bincode round-trip test for `NeedHorizonExceeded`, mirroring `discrepancy_roundtrips_through_bincode` at lines 138-146.
3. `crates/worldwake-core/src/needs.rs` — add 3 unit tests for `projected_tick_of` (the three branches: current ≥ target, rate == 0, normal case) and 1 keyed-accessor test for `MetabolismProfile::rate(need)` covering all 5 variants.
4. `crates/worldwake-core/src/drives.rs` — add 1 keyed-accessor test for `DriveThresholds::high(need)` covering all 5 `HomeostaticNeedId` variants.
5. `crates/worldwake-ai/src/decision_trace.rs` — add 1 formatter test for `NeedSafeUntilTick` rendering. Sibling pattern: nearby focused tests in the same `mod tests` block (search for `format_frame_assumption` in the test module).

### Commands

1. `cargo test -p worldwake-core --lib needs::tests::projected_tick_of`
2. `cargo test -p worldwake-core --lib`
3. `cargo test -p worldwake-ai --lib decision_trace`
4. `cargo build --workspace`
5. `./scripts/verify.sh`
