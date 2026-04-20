# S110DECHISEVE-005: Event-log replay invariance for decision events

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only; exercises existing `replay_execution.rs` / `replay_state.rs` harness
**Deps**: S110DECHISEVE-004 (emission must be wired before a live run can produce decision events for replay)

## Problem

S110 adds new `EventTag` variants and `decision_payload` data to the authoritative append-only log. Worldwake already has a replay invariant: given an event log, replay reproduces the same agenda transitions as the live run. This ticket adds a focused integration test that proves replay still satisfies this invariant after decision events are added — specifically, that `EventPayload::decision_payload` bincode-round-trips without loss and replay-decoded events have byte-identical payloads to the live-run events. Since S110 does not mutate any world state on decision emission (events are record-only), the invariant reduces to "decoding must not fail and payloads must round-trip." The test is a safety net covering the schema change, not a semantic change.

## Assumption Reassessment (2026-04-20)

1. Replay harness lives at `crates/worldwake-sim/src/replay_execution.rs` and `crates/worldwake-sim/src/replay_state.rs`. `save_load.rs` tests at `crates/worldwake-sim/src/save_load.rs:654` onward already exercise full-save round-trip with `SAVE_FORMAT_VERSION`. The new test piggybacks on the same round-trip surface — serialize a full `EventLog` containing decision events, deserialize, and assert equality.
2. Ticket 004 lands the emission that produces real decision events for a scenario. After 004, running `survival-baseline.ron` for a small tick window emits populated `GoalCommitted`, `PlanAdopted`, `BlockerRecorded`, etc. events that the replay test can exercise. Pre-004 the log would contain only `decision_payload: None` events, which is not a useful replay test — hence the hard dependency on 004.
3. Shared abstraction boundary under audit: the `EventLog` wire format including `decision_payload`. Replay invariance here means: `bincode::serialize(&event_log) → bincode::deserialize::<EventLog>(…)` produces a byte-equal `EventLog` after ticket 002's `SAVE_FORMAT_VERSION` bump lands, and every `decision_payload: Some(…)` field survives the round trip with field equality. No world-state re-simulation is required (S110 adds no world-state mutations).
4. No failing golden motivates this ticket. It is a safety net for the schema change.

## Architecture Check

1. Replay-invariance testing that piggybacks on the existing save/load round-trip surface is cleaner than building a new replay oracle. The save/load path is the canonical serialization contract for `EventLog`; replay that decodes through the same path is guaranteed to be consistent with persisted state.
2. No new abstractions — the test uses the existing `save_to_bytes` / `load_from_bytes` surface and the existing scenario loader. FND-28 preserved (no backwards-compat); the test exercises the current single-format path.

## Verification Layers

1. Bincode round-trip of `EventLog` with populated decision events → the new test asserts `event_log == deserialize(serialize(event_log))` byte-for-byte.
2. Decision-event preservation across round-trip → assert that after round-trip, for every event with `decision_payload: Some(…)`, the payload variant and every field match the pre-round-trip value.
6. Single-layer ticket (serialization invariance only) — no decision-trace, action-trace, or belief-view mapping. The invariant is a pure wire-format check.

## What to Change

### 1. Add replay-invariance test

Add to `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` block (or a new test file `crates/worldwake-sim/tests/replay_decision_events.rs` if test scope warrants it — implementer chooses based on test-harness boundary preference at implementation time):

```rust
#[test]
fn event_log_with_decision_events_roundtrips_through_save_load() {
    // 1. Run survival-baseline.ron for N ticks (or construct a synthetic EventLog
    //    with at least one event per DecisionEventPayload variant).
    // 2. Serialize the full SaveableRuntime via `save_to_bytes`.
    // 3. Deserialize via `load_from_bytes`.
    // 4. Assert round-trip equality on the EventLog and on every decision_payload.
}
```

The test uses the scenario-loader path already exercised by existing tests. A synthetic `EventLog` constructed in-test is an acceptable alternative if scenario execution in a sim-crate test is harness-heavy — the invariant under test is serialization equality, not simulation correctness.

### 2. Assert per-variant decision-payload survival

Within the test, iterate `event_log.events()` (or equivalent accessor), filter to events where `decision_payload.is_some()`, and compare payload-for-payload between pre- and post-round-trip. Use a helper that asserts bincode-equality rather than structural-equality via `Eq` alone — this catches any ordering issues in collections inside payloads (e.g., `EvidenceSummary::evidence_kind_counts: BTreeMap<…>` is already `Ord`-stable but the assertion should still exercise the bytes path).

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — add `#[cfg(test)]` test) or `crates/worldwake-sim/tests/replay_decision_events.rs` (new — if extracted to a dedicated integration test file)

## Out of Scope

- Full-simulation replay equivalence (re-simulating from the event log and comparing end state). S110 does not change any world-state mutation, so re-simulation equivalence is already guaranteed by existing replay infrastructure and is not a claim this ticket reopens.
- Cross-version migration testing. Ticket 002 bumps `SAVE_FORMAT_VERSION`; old saves fail with `SaveError::VersionMismatch`. No migration path is in scope.
- Agenda-transition comparison across live vs. replayed runs. S110 is record-only; agenda transitions are driven by authoritative state, not by decision events. The invariant is weaker than S110's broad design-goal statement allowed for (the spec notes this: "reduces to decoding must not fail").

## Acceptance Criteria

### Tests That Must Pass

1. New test `event_log_with_decision_events_roundtrips_through_save_load` passes.
2. All existing `save_load` tests continue to pass at `SAVE_FORMAT_VERSION = 34`.
3. `cargo test -p worldwake-sim save_load` — targeted.
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `bincode::serialize(&event_log) → bincode::deserialize::<EventLog>(…)` is the identity function for any `EventLog` containing any mix of `decision_payload: Some(…)` and `decision_payload: None` events.
2. No decision-event variant produces a serialization error or decoding error under the current format version.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` or `crates/worldwake-sim/tests/replay_decision_events.rs` — new round-trip test covering all 11 `DecisionEventPayload` variants at least once.

### Commands

1. `cargo test -p worldwake-sim replay_decision_events` (or `save_load` if in-file) — targeted.
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
