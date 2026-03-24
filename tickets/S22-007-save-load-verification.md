# S22-007: Save/load verification for IntentionFrame and IntentionDispositionProfile

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — verification-only ticket
**Deps**: S22-002 (frame types active and registered as components)

## Problem

`IntentionFrame` and `IntentionDispositionProfile` must survive save/load round-trips. Since they are registered components via `component_tables.rs`, the macro-generated serialization should handle them automatically — but this needs explicit verification with golden-level assertions to prevent regressions.

## Assumption Reassessment (2026-03-24)

1. Component registration via `component_tables.rs` provides automatic `bincode` serialization via `serde`. The macro generates get/set/remove + serialize/deserialize for each registered component.
2. `golden_save_load_round_trip_under_ai` (or equivalent) exists in `golden_determinism.rs` for testing save/load fidelity.
3. `IntentionFrame` contains `Vec<FrameAssumption>` and nested enums (`IntentionDomain`, `FrameState`) — all derive `Serialize, Deserialize` from S22-001.
4. `IntentionDispositionProfile` contains `BTreeMap<IntentionDomainTag, NonZeroU32>` — both `BTreeMap` and `NonZeroU32` are serde-compatible.
5. This is a verification-only ticket. No production code changes, only test additions/modifications.

## Architecture Check

1. Verifying save/load is the standard approach for any new persisted component. No alternative needed.
2. No backward-compatibility concerns — new components, new test assertions.

## Verification Layers

1. Mid-journey `IntentionFrame` preserved after save/load → golden test assertion
2. Suspended frame remains suspended after save/load → golden test assertion
3. `IntentionDispositionProfile` per-domain patience preserved → golden test assertion
4. Frame assumptions list preserved → golden test assertion
5. Single-layer ticket: save/load fidelity is the proof surface.

## What to Change

### 1. Extend golden save/load test

In the existing `golden_save_load_round_trip_under_ai` test (or create a focused companion):
- Set up a scenario where an agent has an active `IntentionFrame` (e.g., mid-journey travel)
- Save state
- Load state
- Assert all `IntentionFrame` fields match: `goal`, `domain`, `state`, `established_at`, `last_progress_tick`, `stalled_ticks`, `patience_limit`, `assumptions`

### 2. Suspended frame round-trip

- Set up a scenario where an agent's frame is `Suspended { reason: PriorityInterrupt, suspended_at: Tick(N) }`
- Save/load
- Assert suspended state is preserved with correct reason and tick

### 3. IntentionDispositionProfile round-trip

- Set up an agent with `IntentionDispositionProfile` containing domain-specific patience entries
- Save/load
- Assert `domain_patience` map, `default_patience_ticks`, and `commitment_switch_margin` match

## Files to Touch

- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — add/extend save/load assertions)

## Out of Scope

- Production code changes (this is verification-only)
- Changes to serialization format or component registration
- Decision trace persistence (traces are ephemeral, not saved)
- Replay hash verification (covered in S22-008)
- Non-golden-test save/load verification

## Acceptance Criteria

### Tests That Must Pass

1. Golden test: active `IntentionFrame` with `IntentionDomain::Travel` survives save/load with all fields intact
2. Golden test: suspended `IntentionFrame` survives save/load with `FrameState::Suspended { reason, suspended_at }` intact
3. Golden test: `IntentionDispositionProfile` with 2+ domain_patience entries survives save/load
4. Golden test: `Vec<FrameAssumption>` with 2+ assumptions preserved after save/load
5. `cargo test -p worldwake-ai --test golden_determinism` — all pass
6. `cargo clippy --workspace` — no new warnings

### Invariants

1. Save/load is identity: loaded state produces identical component values to pre-save state
2. `BTreeMap` ordering in `IntentionDispositionProfile` is deterministic after deserialization
3. `Vec<FrameAssumption>` ordering is preserved (not re-sorted) after deserialization

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_determinism.rs` — save/load round-trip assertions for IntentionFrame (active, suspended) and IntentionDispositionProfile

### Commands

1. `cargo test -p worldwake-ai --test golden_determinism`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
