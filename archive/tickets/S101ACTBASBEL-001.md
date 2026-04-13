# S101ACTBASBEL-001: Activation computation helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new functions in worldwake-core belief module, new method on HomeostaticNeeds
**Deps**: None

## Problem

The activation-based belief decay system (S101) needs three pure computational helpers before the pruning migration can proceed: activation computation from a tick buffer, need-gated salience boost, and a max-value accessor on HomeostaticNeeds. These are standalone additions with no removals.

## Assumption Reassessment (2026-04-13)

1. `HomeostaticNeeds` exists at `crates/worldwake-core/src/needs.rs:9` with 5 `Permille` fields (hunger, thirst, fatigue, bladder, dirtiness). No `max_value()` method exists yet. Confirmed by grep.
2. `Permille::value(self) -> u16` exists at `crates/worldwake-core/src/numerics.rs:50`. The activation formula uses `u16` arithmetic.
3. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs:1322` has `believed_kind: Option<EntityKind>` (line 1324). `EntityKind::ItemLot` exists at `crates/worldwake-core/src/entity.rs:10`.
4. `PerceptionProfile` at `crates/worldwake-core/src/belief.rs:2181` — the new fields (`need_salience_boost`, `need_salience_urgency_threshold`) do not exist yet. The `salience_boost` function's signature references these future fields from ticket 003. For this ticket, the function takes the raw `Permille` values as parameters rather than a `&PerceptionProfile` reference, so it compiles before the profile migration.
5. Rust toolchain is 1.93.0 (`rust-toolchain.toml`), so `u64::isqrt()` is available (stabilized in 1.84).
6. Focused proof against the spec reference table exposed a mismatch in the original ticket snippet: `1000 / isqrt(age)` does not produce the documented values for age 48 (`166`, not `144`) or the 5/15/25/35/45 accumulation (`1399`, not `1223`). The live ticket now uses scaled integer square-root math to compute `floor(1000 / sqrt(age))` without floats, which matches the spec table and accumulation example.

## Architecture Check

1. All three functions are pure computations over concrete state (tick buffers, need values, entity kind) — no side effects, no global state access. FND-3 compliant: activation is derived on-demand, never stored.
2. No backward-compatibility shims. These are new additions only.

## Verification Layers

1. `compute_activation` formula correctness → focused unit tests with known reference values from spec
2. `salience_boost` graduated scaling → focused unit tests with boundary conditions
3. `HomeostaticNeeds::max_value()` correctness → focused unit test comparing all 5 fields
4. Single-layer ticket (worldwake-core computation) — no cross-system mapping needed.

## What to Change

### 1. Add `HomeostaticNeeds::max_value()`

In `crates/worldwake-core/src/needs.rs`, add to the `impl HomeostaticNeeds` block:

```rust
#[must_use]
pub fn max_value(&self) -> u16 {
    self.hunger.value()
        .max(self.thirst.value())
        .max(self.fatigue.value())
        .max(self.bladder.value())
        .max(self.dirtiness.value())
}
```

### 2. Add `compute_activation()` function

In `crates/worldwake-core/src/belief.rs`, add a public free function:

```rust
/// ACT-R base-level activation (d=0.5) scaled to Permille.
/// Computes Σ floor(1000 / sqrt(max(1, T - t_j))) via scaled integer sqrt.
pub fn compute_activation(current_tick: Tick, presentation_ticks: &[Tick], count: u8) -> u16 {
    let mut total: u32 = 0;
    for i in 0..count as usize {
        let age = current_tick.0.saturating_sub(presentation_ticks[i].0).max(1) as u64;
        let scaled_root = age.saturating_mul(1_000_000).isqrt().max(1);
        total += 1_000_000 / scaled_root as u32;
    }
    total.min(u16::MAX as u32) as u16
}
```

### 3. Add `salience_boost()` function

In `crates/worldwake-core/src/belief.rs`, add a public free function:

```rust
/// Need-gated activation bonus for item-kind entities during survival crises.
pub fn salience_boost(
    max_need: u16,
    believed_kind: Option<EntityKind>,
    urgency_threshold: Permille,
    boost: Permille,
) -> u16 {
    if believed_kind != Some(EntityKind::ItemLot) {
        return 0;
    }
    if max_need < urgency_threshold.value() {
        return 0;
    }
    (max_need as u32 * boost.value() as u32 / 1000) as u16
}
```

Note: This takes raw parameters rather than `&PerceptionProfile` because the new profile fields don't exist until ticket 003. Ticket 003 will wrap this or inline the call with profile field access.

### 4. Unit tests

In the `#[cfg(test)]` block of `crates/worldwake-core/src/belief.rs`:

- `test_activation_computation_single_observation` — verify reference values from spec table (age 1→1000, 4→500, 16→250, 48→144, 100→100, 400→50)
- `test_activation_computation_multiple_observations` — verify 5 observations at ages 5,15,25,35,45 sums to 1223
- `test_activation_computation_empty_buffer` — count=0 returns 0

In the `#[cfg(test)]` block of `crates/worldwake-core/src/needs.rs`:

- `test_homeostatic_needs_max_value` — verify max_value returns the highest of all 5 fields
- `test_homeostatic_needs_max_value_all_zero` — sated needs return 0

In the `#[cfg(test)]` block of `crates/worldwake-core/src/belief.rs`:

- `test_salience_boost_scales_with_need_urgency` — max_need=500, threshold=500, boost=500 → 250; max_need=1000 → 500
- `test_salience_boost_zero_below_threshold` — max_need=499, threshold=500 → 0
- `test_salience_boost_zero_for_non_items` — believed_kind=Agent → 0 regardless of need

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify) — add `max_value()` method + tests
- `crates/worldwake-core/src/belief.rs` (modify) — add `compute_activation()`, `salience_boost()` functions + tests

## Out of Scope

- Modifying BelievedEntityState or PerceptionProfile fields (ticket 002, 003)
- Replacing enforce_capacity or any existing pruning logic (ticket 003)
- Call site updates in worldwake-systems (ticket 003)
- Golden tests (ticket 004)
- Commodity-specific salience mapping (spec non-goal)
- Variable decay exponent (spec non-goal, d=0.5 is fixed)

## Acceptance Criteria

### Tests That Must Pass

1. `test_activation_computation_single_observation` — formula matches spec reference table
2. `test_activation_computation_multiple_observations` — accumulation matches spec example
3. `test_salience_boost_scales_with_need_urgency` — graduated boost formula correct
4. `test_salience_boost_zero_below_threshold` — no boost when needs are low
5. `test_homeostatic_needs_max_value` — returns highest of 5 fields
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `compute_activation` returns 0 for empty buffer (count=0)
2. `compute_activation` returns values matching ACT-R d=0.5 reference table within integer rounding
3. `salience_boost` returns 0 for non-ItemLot entities regardless of need values
4. All computation uses integer arithmetic only — no floats

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` — `test_homeostatic_needs_max_value`, `test_homeostatic_needs_max_value_all_zero`
2. `crates/worldwake-core/src/belief.rs` — `test_activation_computation_single_observation`, `test_activation_computation_multiple_observations`, `test_activation_computation_empty_buffer`, `test_salience_boost_scales_with_need_urgency`, `test_salience_boost_zero_below_threshold`, `test_salience_boost_zero_for_non_items`

### Commands

1. `cargo test -p worldwake-core -- test_activation_computation`
2. `cargo test -p worldwake-core -- test_salience_boost`
3. `cargo test -p worldwake-core -- test_homeostatic_needs_max_value`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

Completed on 2026-04-13.

- Added `HomeostaticNeeds::max_value()` in `crates/worldwake-core/src/needs.rs` plus focused unit coverage for non-zero and all-zero cases.
- Added public `compute_activation()` and `salience_boost()` helpers in `crates/worldwake-core/src/belief.rs` plus focused unit coverage for spec reference values, multi-observation accumulation, empty buffers, threshold gating, and non-item suppression.
- Kept the implementation scoped to the ticket's owned `worldwake-core` helper surface; no ring-buffer or `PerceptionProfile` migration work from tickets 002/003 was pulled forward.

## Deviations

- The original ticket snippet used `1000 / isqrt(age)`, but that contradicts the S101 reference table and accumulation example. The landed helper uses scaled integer square-root math to compute `floor(1000 / sqrt(age))` without floats so the implementation matches the spec values.
- `HomeostaticNeeds::max_value()` landed as a normal pure method rather than `const fn` because the current toolchain does not permit `u16::max()` inside `const fn` on this path. The ticket's behavioral contract is unchanged.

## Verification Result

- Passed `cargo test -p worldwake-core --lib test_activation_computation`
- Passed `cargo test -p worldwake-core --lib test_salience_boost`
- Passed `cargo test -p worldwake-core --lib homeostatic_needs_max_value`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
