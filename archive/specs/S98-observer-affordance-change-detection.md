# S98: Observer Affordance-Change Detection

## Summary

Enhances the observer binary to emit affordance snapshots whenever an agent's available action-type set changes between consecutive planning decisions, filling the diagnostic gap identified in the simulation observer report (no affordance data between tick 823 and 1342 for Guard Theron).

## Phase and Status

Phase 7 adjunct. Status: COMPLETED.

## Crates

- `worldwake-cli` — observer binary enhancement (`observer.rs`)

## Dependencies

- None. Uses existing `AffordanceTrace` and `AgentDecisionTrace` infrastructure from S85.

## Design Goals

- Answer the diagnostic question: "When did action X stop being available to this agent, and why?"
- Minimal observer overhead (<10% increase on 1440-tick CLI evaluation scenario).
- Build on S85's affordance snapshot infrastructure (no new trace types).

## Non-Goals

- Per-tick affordance dumps (TQ-1 acceptable trade-off — too verbose).
- Engine-side affordance tracking (this is observer-binary report-time analysis only).
- Modifying the simulation tick or decision pipeline.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-29 (Debuggability Is a Product Feature) | Directly addresses: "Why did this agent stop doing X?" The observer can now show when an action type appeared or disappeared from the affordance set. |
| FND-12 (Performance May Compress Computation, Never Causality) | Observer-only analysis does not change simulation causality. Affordance-change detection is a derived view over existing decision traces. |

## Deliverables

### D1: Affordance-change detection function

New function in `observer.rs`:

```rust
/// Identifies ticks where an agent's available action-type set changed.
/// Compares consecutive AffordanceTrace entries and emits a snapshot
/// whenever an action type appears or disappears.
fn affordance_change_snapshots<'a>(
    affordance_snapshots: &[(Tick, &'a AffordanceTrace)],
) -> Vec<AffordanceChangeEvent<'a>> { ... }

struct AffordanceChangeEvent<'a> {
    tick: Tick,
    affordances: &'a AffordanceTrace,
    appeared: Vec<String>,    // action types newly available
    disappeared: Vec<String>, // action types no longer available
    place_changed: bool,      // true if AffordanceTrace.place differs from previous snapshot
}
```

The function iterates consecutive `(Tick, &AffordanceTrace)` pairs from `planning_affordance_snapshots`. For each pair, it computes the symmetric difference of action-type sets. If the difference is non-empty, it records an `AffordanceChangeEvent`.

**Action-type comparison**: Comparison uses `AffordanceSummary::action_name` as the set element. Multiple affordances with the same `action_name` but different `target_count` are deduplicated — the diagnostic concern is whether an action *type* is available, not how many targets it has. The `appeared` and `disappeared` vectors contain cloned `action_name` strings from the respective `AffordanceSummary` entries.

The `place_changed` field is set by comparing `AffordanceTrace.place` between consecutive snapshots. This allows D2's formatting to include a place-change hint without separate detection logic.

### D2: Observer report formatting

Add affordance-change entries to the per-agent Section 7 (Decision Summary) output, between the existing post-travel affordance snapshots and the final affordance snapshot, using the same `**bold label**` formatting convention:

```
**Affordance changes** (tick 450): +harvest_resource (arrived at Eldergrove Forest)
**Affordance changes** (tick 823): -post_notice (no longer at posting place)
**Affordance changes** (tick 1342): +post_notice (returned to posting place)
```

Each entry shows the tick, the action types that appeared (+) or disappeared (-), and optionally a parenthetical hint when `AffordanceChangeEvent.place_changed` is true — derived from the affordance trace's place field.

### D3: Integration with existing snapshot pipeline

The `affordance_change_snapshots` function operates on the same `planning_affordance_snapshots` vector used by `post_travel_affordance_snapshots` and `final_affordance_snapshot`. It is called after those functions and its output is formatted into the report.

### D4: Performance guard

The change-detection is report-time analysis over already-collected `AgentDecisionTrace` data. No additional simulation overhead. The only cost is iterating the affordance trace vector once per agent during report generation. For N agents with M planning decisions each, cost is O(N*M) string set comparisons — negligible for the CLI evaluation scenario's 4 agents over 1440 ticks.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: N/A — this is observer-binary diagnostic tooling, not a simulation system.

2. **Positive-feedback analysis**: None. Read-only analysis over existing traces.

3. **Concrete dampeners**: N/A.

4. **Stored state vs. derived**: The affordance-change events are purely derived from existing `AgentDecisionTrace` data. No new authoritative state.

## SystemFn Integration

None. Observer-only enhancement. No simulation SystemFn changes.

## Component Registration

None. No new ECS components.

## Cross-System Interactions

None. The observer binary reads simulation traces after the run completes. No interaction with live simulation systems.

## Profile-Driven Parameters

None. This is diagnostic tooling, not agent behavior.

## Outcome

Completed on 2026-04-12.

- Added local `AffordanceChangeEvent` analysis in `crates/worldwake-cli/src/bin/observer.rs` to compare consecutive planning affordance snapshots by `action_name`, ignore `target_count` churn, and record whether the believed place changed.
- Inserted `**Affordance changes**` lines into Section 7 of the observer report between post-travel affordances and final affordances, including `+`/`-` markers and a place hint when the affordance snapshot moved.
- Added focused observer-bin unit coverage for appeared/disappeared detection, target-count-only stability, place-change detection, identical-set suppression, and final report rendering.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer -- affordance_change`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
