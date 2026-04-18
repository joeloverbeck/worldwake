# S118STUAGEDET-002: Stuck-agent detector guardrail tests

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only additions.
**Deps**: archive/tickets/S118STUAGEDET-001.md, specs/S118-stuck-agent-detector-active-frame-exclusion.md

## Problem

After S118STUAGEDET-001 lands the `had_action || in_open_frame` extension in the stuck-agent detector, two failure modes become newly plausible and unguarded: (a) the fix could mistakenly suppress a legitimate idle window if a prior-run open frame were to leak across agents or runs; (b) the StartFailed arm could be extended incorrectly in the future to open a frame, hiding agents that are genuinely stuck in a failing-request loop. Neither mode is covered by the regression test in 001 (which only asserts absence during active work). The spec's D3 and D4 deliverables exist to lock these invariants with explicit fixtures so a future refactor of the open-frame tracker cannot silently regress them.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Current code state (post-001): `had_action = had_event || in_open_frame` at the per-agent inner loop inside the outer scan loop in `crates/worldwake-cli/src/bin/observer.rs`. `open_frame: BTreeMap<EntityId, bool>` is an outer-loop local — not a global, not persisted — so cross-run leakage is architecturally impossible. The guardrail value is behavioral: future refactors that promote `open_frame` to a longer-lived struct must not break the invariants below. `refine_stuck_agents` at `observer.rs:1930-1960` strips STUCK_AGENT anomalies whose window has all five needs ≤ 300 permille at window start; fixtures here must clear that filter.
2. Spec reference: `specs/S118-stuck-agent-detector-active-frame-exclusion.md` D3 (genuine idle) and D4 (StartFailed idle). The spec-level fixture + binary-invocation pattern was locked in during the 2026-04-18 reassessment.
3. Shared boundary under audit: the `STUCK_AGENT` anomaly emission gate at `observer.rs:836-875` (threshold check on `stats.max_consecutive_idle >= 20`) combined with the `refine_stuck_agents` post-filter. Both tests assert the emission-and-survival path: an anomaly fires AND survives the low-need refinement.
4. Mismatch found during live reassessment: D3 remains honestly provable through the observer binary with a one-agent no-input scenario, but D4's drafted pure-`.ron` observer fixture is not. The live scenario schema has no authored input-queue/request surface, and probes of static single-agent harvest/workstation mismatches produced lawful silence (`Harvest Water | 0`) rather than AI-driven `ActionTraceKind::StartFailed`. Current lower-layer `tick_step.rs` tests already prove the `StartFailed` carrier exists; the missing slice is observer-local confirmation that `StartFailed` does **not** count as activity. Safe correction: keep D3 as binary fixture coverage, but prove D4 at the strongest honest local seam in `crates/worldwake-cli/src/bin/observer.rs` by driving the same `had_event` filter against synthetic `ActionTraceKind::StartFailed` events and asserting the detector still emits `StuckAgent`.

## Architecture Check

1. **Pure test-only additions**: no production code changes; all invariants are already established by S118STUAGEDET-001. These fixtures pin the invariants to concrete scenarios so future observer refactors cannot drift.
2. **Fixture-based harness keeps observer-boundary discipline**: tests run the binary and parse the text report just like the S117 suite. The detector stays inline in the binary; tests do not require exposing internals.
3. **Mixed proof surface is the cleaner D4 boundary**: D3 remains a binary-level observer regression because the authored no-input idle shape is available through `.ron`. D4 moves to a same-file observer unit test because the live observer scenario seam cannot author a `StartFailed`-only span without extra runtime request substrate the ticket does not own. This keeps the proof at the earliest honest detector boundary instead of inventing a misleading fixture.
4. **Invariant coverage is still disjoint from 001**: 001 proves absence-of-anomaly when an active frame covers the window; this ticket proves presence-of-anomaly in the two remaining lawful idle shapes (no lifecycle activity; StartFailed-only). Together they form a three-way partition of the detector's input space.

## Verification Layers

1. Genuine idle still triggers STUCK_AGENT -> observer binary invocation + text report parse via `count_anomalies_of_kind(&report, "STUCK_AGENT") == 1` and `anomaly_block(&report, "STUCK_AGENT").contains("consecutive ticks")`.
2. StartFailed-only spans still trigger STUCK_AGENT -> focused `observer.rs` unit coverage that feeds synthetic `ActionTraceKind::StartFailed` events through the same `had_event` filter and `record_idle_tick` path, then asserts `detect_anomalies` still emits `AnomalyKind::StuckAgent`.
3. Single-layer ticket (observer-only text-report assertions); additional layer mapping is not applicable.

## What to Change

### 1. Genuine-idle fixture

Create `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron`. Design constraints:

- Exactly one agent in a configuration that produces **zero** ActionTrace events across a span comfortably exceeding 20 ticks. Two viable designs:
  - AI-controlled agent whose affordances are all gated (e.g., placed in a place with no matching workstations and no reachable resource sources for any of its drives).
  - Human-controlled agent (`AgentDef.control_source: Human`) with no queued input — the observer runs it but no actions are requested.

  Pick the design that is cheapest to author; cross-check against `scenarios/*.ron` for a precedent.
- At least one need rising above the 300-permille `NEEDS_LOW_CEILING` used by `refine_stuck_agents` so the anomaly survives the low-need strip (otherwise the test would pass trivially against a broken detector).
- Simulated tick budget just past the span — shorter is better.

### 2. Genuine-idle test

Append to `crates/worldwake-cli/tests/golden_observer_anomalies.rs`:

```rust
#[test]
fn stuck_detector_still_fires_on_genuine_idle() {
    let report = run_observer(
        "tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron",
        /* ticks: span length + small buffer */,
    );
    assert_eq!(count_anomalies_of_kind(&report, "STUCK_AGENT"), 1);
    let block = anomaly_block(&report, "STUCK_AGENT");
    assert!(block.contains("consecutive ticks"));
}
```

### 3. StartFailed-idle focused unit proof

Add a focused unit test to the existing `#[cfg(test)]` module in `crates/worldwake-cli/src/bin/observer.rs` that:

- creates one synthetic agent stats entry with elevated needs
- records one `ActionTraceKind::StartFailed` event per tick in an `ActionTraceSink`
- computes `had_event` with the same live filter used by the observer loop (`!matches!(e.kind, ActionTraceKind::StartFailed { .. })`)
- feeds the resulting `had_action` values into `AgentStats::record_idle_tick`
- flushes the final idle window and asserts `detect_anomalies(...)` still emits exactly one `AnomalyKind::StuckAgent`

This keeps D4 at the strongest honest observer-local boundary now that reassessment has shown the authored observer `.ron` seam does not lawfully produce a `StartFailed`-only span on the current branch.

### 4. StartFailed-idle unit test

```rust
#[test]
fn stuck_detector_does_not_treat_startfailed_as_active_frame() {
    // synthetic StartFailed-only action trace must still accumulate idle
    // and survive the STUCK_AGENT emission/refinement path
}
```

## Files to Touch

- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (modify — append genuine-idle observer-binary test)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron` (new)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — append focused StartFailed-only detector test in `#[cfg(test)]`)

## Out of Scope

- Detector runtime changes (owned by S118STUAGEDET-001).
- Threshold calibration, other detector refinements, new CLI flags (spec Non-Goals).
- Skill documentation simplification (owned by S118STUAGEDET-003).
- Additional guardrail shapes beyond genuine-idle and StartFailed-only. If more shapes warrant coverage in future, open a follow-up ticket.

## Acceptance Criteria

### Tests That Must Pass

1. `stuck_detector_still_fires_on_genuine_idle` — new test; passes against a codebase where S118STUAGEDET-001 has landed.
2. `stuck_detector_does_not_treat_startfailed_as_active_frame` — new test; passes against the same codebase.
3. `stuck_detector_excludes_wash_travel_cycle` (landed by 001) — must continue passing.
4. Existing `golden_observer_anomalies` tests — must continue passing.
5. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. An agent with zero action-trace events across a span exceeding the 20-tick threshold still triggers `AnomalyKind::StuckAgent` when at least one need is elevated (survives `refine_stuck_agents`).
2. An agent whose only action-trace events are `ActionTraceKind::StartFailed` still triggers `AnomalyKind::StuckAgent` under the same need conditions — StartFailed is not treated as an open frame.
3. The detector's three-way input partition (active frame / no events / StartFailed-only) is fully covered by fixtures 001 + 002.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` — append `stuck_detector_still_fires_on_genuine_idle`.
2. `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron` — fixture producing a zero-event span.
3. `crates/worldwake-cli/src/bin/observer.rs` `#[cfg(test)]` module — add `stuck_detector_does_not_treat_startfailed_as_active_frame`, proving StartFailed-only lifecycle activity still accumulates idle and survives anomaly emission.

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies stuck_detector_still_fires_on_genuine_idle -- --exact` — targeted proof for the genuine-idle observer-binary test.
2. `cargo test -p worldwake-cli --bin observer tests::stuck_detector_does_not_treat_startfailed_as_active_frame -- --exact` — targeted proof for the focused StartFailed-only detector test.
3. `cargo test -p worldwake-cli --test golden_observer_anomalies` — full observer-anomaly suite.
4. `cargo test -p worldwake-cli` — crate-wide regression guard.
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint parity with CI.

## Outcome

Completed on 2026-04-18.

- Added `stuck_detector_still_fires_on_genuine_idle` to `crates/worldwake-cli/tests/golden_observer_anomalies.rs` plus the new fixture `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron`, proving the observer still reports `STUCK_AGENT` for a genuine no-action idle span that survives `refine_stuck_agents`.
- Added `tests::stuck_detector_does_not_treat_startfailed_as_active_frame` to `crates/worldwake-cli/src/bin/observer.rs`, proving `ActionTraceKind::StartFailed` does not get counted as active work by the observer's stuck-detector path.
- Verified that the 001 regression `stuck_detector_excludes_wash_travel_cycle` still passes, so the detector now has explicit coverage for all three intended partitions: active frame, zero events, and StartFailed-only.

## Deviations

- D4 was corrected during reassessment from a pure observer-binary `.ron` fixture to a focused `observer.rs` unit proof. The live scenario seam does not currently expose an authored input/request surface that truthfully produces a StartFailed-only span; probe scenarios on the current branch produced lawful silence instead. The unit test keeps the proof at the strongest honest observer-local boundary without inventing a misleading fixture.
- The exact passing focused command for the unit proof is `cargo test -p worldwake-cli --bin observer tests::stuck_detector_does_not_treat_startfailed_as_active_frame -- --exact`; the unqualified filter recorded earlier in the ticket compiles the test but matches zero cases under `--exact`.

## Verification Result

- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies stuck_detector_still_fires_on_genuine_idle -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer tests::stuck_detector_does_not_treat_startfailed_as_active_frame -- --exact`
- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
