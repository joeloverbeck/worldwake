# S11WOULIFAUD-002: Wound pruning hardening and investigation tests

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — debug assertion in `progress_wounds()`
**Deps**: None

## Problem

An anomaly was observed where wounds with `natural_recovery_rate: pm(0)` vanished. The root cause is likely a since-removed test workaround (`no_recovery_combat_profile()`), but no focused tests exist to prevent regression. The pruning contract (wounds pruned iff severity reaches 0) is enforced only by implicit code structure, not by explicit tests or assertions.

## Assumption Reassessment (2026-03-21)

1. `progress_wounds()` in `crates/worldwake-systems/src/combat.rs` (line 192) is private. Pruning at line 224: `next.wounds.retain(|w| w.severity.value() > 0)`. Recovery path (line ~218): `severity -= recovery_rate` only when `can_recover` is true. With `recovery_rate = pm(0)`, `saturating_sub` is a no-op — severity cannot reach 0 via recovery.
2. `recovery_conditions_met()` (line 230) checks: not in combat AND hunger < high AND thirst < high AND fatigue < high.
3. Not an AI ticket. Pure systems-layer test hardening.
4. No ordering dependency.
5. N/A — no heuristic removal.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. No mismatch. The `no_recovery_combat_profile()` workaround cited in the original spec no longer exists, confirming the anomaly source has been removed. Tests still needed to prevent regression.

## Architecture Check

1. Adding focused tests to `progress_wounds()` and a `#[cfg(debug_assertions)]` contract check is the minimal, non-invasive approach. No alternative (e.g., type-level enforcement) is justified for this simple invariant.
2. No backwards-compatibility shims.

## Verification Layers

1. Zero-recovery wound persists → focused unit test on `progress_wounds()`
2. Bleed/clot arithmetic is exact → focused unit test with known parameters
3. Only severity-0 wounds are pruned → focused unit test with mixed-severity list
4. No-change returns None → focused unit test
5. Contract assertion fires on violation → `#[cfg(debug_assertions)]` check (tested implicitly by all debug-mode runs)

## What to Change

### 1. Add `#[cfg(debug_assertions)]` pruning contract check

Before the `retain` line in `progress_wounds()`, add a debug assertion that documents and enforces the pruning contract:

```rust
#[cfg(debug_assertions)]
{
    // Contract: only wounds with severity == 0 should be pruned.
    // If a wound with recovery_rate pm(0) somehow reached severity 0
    // without the recovery path, something is wrong.
    for w in &next.wounds {
        if w.severity.value() == 0 {
            debug_assert!(
                w.bleed_rate_per_tick.value() == 0,
                "Wound {} reached severity 0 while still bleeding — pruning contract violated",
                w.id.0,
            );
        }
    }
}
```

### 2. Add four focused tests in `combat.rs` `mod tests`

All tests call `progress_wounds()` directly with controlled inputs.

- `zero_recovery_rate_wound_persists`: Clotted wound (bleed_rate 0), severity pm(200), recovery_rate pm(0). Tick 50 times. Assert severity unchanged at pm(200) and wound not pruned.
- `wound_bleed_clot_arithmetic_exact`: Wound with severity pm(100), bleed_rate pm(50), clot_resistance pm(25). Tick until clotted. Assert final severity = initial + sum of all bleed increments.
- `pruning_only_at_severity_zero`: List with wounds at severity pm(500), pm(0), pm(100). After one `progress_wounds` call (no bleeding, no recovery), assert only the pm(0) wound is pruned.
- `progress_wounds_returns_none_when_no_change`: Non-bleeding wound, recovery_rate pm(0), recovery conditions met. Assert `progress_wounds()` returns `None`.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — add debug assertion + tests)

## Out of Scope

- Changing the pruning logic itself (retain behavior is correct)
- Changing `recovery_conditions_met()` logic
- Any changes to wounds.rs, needs.rs, or ranking.rs
- Golden test changes
- Any AI/ranking changes

## Acceptance Criteria

### Tests That Must Pass

1. `zero_recovery_rate_wound_persists` — wound with pm(0) recovery stays at original severity after 50 ticks
2. `wound_bleed_clot_arithmetic_exact` — final severity matches manual arithmetic
3. `pruning_only_at_severity_zero` — only severity-0 wound removed from list
4. `progress_wounds_returns_none_when_no_change` — returns `None` for static wound state
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. A wound is pruned if and only if `severity.value() == 0`
2. A wound with `natural_recovery_rate: pm(0)` cannot have its severity reduced by the recovery path
3. `progress_wounds()` returns `None` when no wound field changes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs::tests::zero_recovery_rate_wound_persists` — H1 regression guard
2. `crates/worldwake-systems/src/combat.rs::tests::wound_bleed_clot_arithmetic_exact` — H2 regression guard
3. `crates/worldwake-systems/src/combat.rs::tests::pruning_only_at_severity_zero` — pruning contract
4. `crates/worldwake-systems/src/combat.rs::tests::progress_wounds_returns_none_when_no_change` — None-return contract

### Commands

1. `cargo test -p worldwake-systems -- zero_recovery_rate_wound_persists wound_bleed_clot_arithmetic pruning_only progress_wounds_returns_none`
2. `cargo clippy -p worldwake-systems`
3. `cargo test -p worldwake-systems`
