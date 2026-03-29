# S37COOBASEXH-007: Save/load version bump for ExhaustionEntry schema change

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — SAVE_FORMAT_VERSION bump in save_load.rs
**Deps**: S37COOBASEXH-002 (ExhaustionEntry field changes)

## Problem

`ExhaustionEntry` gains `next_retry_tick: Option<Tick>` and `consecutive_failures: u8`, and loses `consecutive_budget_exhaustions: u8`. This changes the serialized representation of `AgentDecisionRuntime` (which contains the exhaustion cache). `SAVE_FORMAT_VERSION` must be bumped so pre-change saves fail with an explicit version error instead of opaque deserialization failures.

## Assumption Reassessment (2026-03-29)

1. `SAVE_FORMAT_VERSION` is currently `11` in `crates/worldwake-sim/src/save_load.rs:6`. `LEGACY_SAVE_FORMAT_VERSION` is `5` at line 7. Version check at line 130-134. The repo uses explicit version gating — no backward-compatibility shims.
2. Spec S37 Section 9 specifies bumping `SAVE_FORMAT_VERSION` and using `#[serde(default)]` on new fields for forward-compatible deserialization of old saves. However, per the project convention (no backward-compatibility layers), old saves should fail cleanly at the version gate — `#[serde(default)]` on the `ExhaustionEntry` fields is for resilience within the same version, not cross-version migration.
3. `ExhaustionEntry` is serialized as part of `AgentDecisionRuntime` → `AgentTickDriver` → the AI payload section of saves. The serialization boundary is in `worldwake-sim/src/save_load.rs`.
4. N/A — no golden scenario.
5. N/A — not planner-driven.
6. N/A — not an AI regression.
7. N/A — no ordering.
8. N/A — no heuristic removal.
9-12. N/A.
13. The `#[serde(default)]` on `ExhaustionEntry`'s new fields (added in S37COOBASEXH-002) provides resilience for the current format version. The version bump ensures old saves don't accidentally deserialize with wrong field layout.
14. Spec says to keep `#[serde(default)]` on new fields. This is fine — it protects against partial deserialization within the same version. The version gate prevents cross-version confusion.
15. N/A.

## Architecture Check

1. Bumping `SAVE_FORMAT_VERSION` from 11 to 12 follows the established pattern (bumped 6 times previously: 4→5→6→7→8→9→10→11). This is the standard mechanism for signaling incompatible wire format changes.
2. No backward-compatibility shims. The existing `LEGACY_SAVE_FORMAT_VERSION` (5) path is unaffected.

## Verification Layers

1. Save output writes version 12 → `save_load_roundtrip` test (existing, uses `SAVE_FORMAT_VERSION`)
2. Load rejects version 11 (now old) → `load_rejects_wrong_version` test (existing, uses relative `SAVE_FORMAT_VERSION ± 1`)
3. Round-trip preserves cooldown state → new focused test
4. Single-layer: save/load boundary only.

## What to Change

### 1. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`:

```rust
pub const SAVE_FORMAT_VERSION: u32 = 12;
```

### 2. Add round-trip test for cooldown state

Add a focused test that creates an `AgentDecisionRuntime` with exhaustion entries containing `next_retry_tick` and `consecutive_failures`, saves and loads, and verifies the cooldown state survives round-trip.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — version constant)
- `crates/worldwake-ai/src/agent_tick/planning.rs` or a save/load test module (modify — add round-trip test)

## Out of Scope

- `ExhaustionEntry` struct changes (S37COOBASEXH-002)
- Any planning logic changes (S37COOBASEXH-003, -004, -005)
- Decision trace changes (S37COOBASEXH-006)
- `PlanningBudget` changes (S37COOBASEXH-001)
- `LEGACY_SAVE_FORMAT_VERSION` changes (remains at 5)
- Legacy save migration path (old saves fail at version gate — this is correct behavior)

## Acceptance Criteria

### Tests That Must Pass

1. Save/load round-trip preserves cooldown state (`next_retry_tick`, `consecutive_failures`)
2. `load_rejects_wrong_version` — rejects saves with version 11 (now stale)
3. Save output header contains version 12
4. Existing suite: `cargo test -p worldwake-sim -- save_load`

### Invariants

1. `SAVE_FORMAT_VERSION == 12`
2. `LEGACY_SAVE_FORMAT_VERSION` unchanged at 5
3. Old saves (version ≤ 11, except legacy 5) rejected with explicit error message

## Test Plan

### New/Modified Tests

1. New test in save/load module — `cooldown_state_survives_roundtrip` (creates runtime with cooldown entries, saves, loads, verifies)
2. Existing `load_rejects_wrong_version` test passes without modification (uses relative offsets)
3. Existing `save_load_roundtrip` test passes (includes AI payload with exhaustion cache)

### Commands

1. `cargo test -p worldwake-sim -- save_load`
2. `cargo test -p worldwake-ai -- save_load`
3. `cargo clippy --workspace && cargo test --workspace`
