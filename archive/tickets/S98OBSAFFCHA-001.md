# S98OBSAFFCHA-001: Add affordance-change detection and report formatting

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None (uses existing S85 infrastructure)

## Problem

The observer binary has no visibility into when an agent's available action-type set changes between consecutive planning decisions. When debugging why an agent stopped performing an action (e.g., Guard Theron had no affordance data between tick 823 and 1342), the observer cannot answer "when did action X stop being available?" This ticket adds change-detection analysis over existing `AffordanceTrace` data and formats the results into the per-agent decision summary.

## Assumption Reassessment (2026-04-12)

1. `AffordanceTrace` exists at `crates/worldwake-ai/src/decision_trace.rs:218` with fields `available: Vec<AffordanceSummary>` and `place: Option<EntityId>`. `AffordanceSummary` at line 208 has fields `def_id: ActionDefId`, `action_name: String`, `target_count: usize`. Both derive `Clone, Debug`.
2. `planning_affordance_snapshots` exists at `crates/worldwake-cli/src/bin/observer.rs:742` returning `Vec<(Tick, &AffordanceTrace)>`. `post_travel_affordance_snapshots` at line 767 and `final_affordance_snapshot` at line 799 both consume the same `&[(Tick, &AffordanceTrace)]` slice.
3. Observer report formatting in `format_report` uses `**bold label**` convention for affordance entries within Section 7 (Per-Agent Decision Summary). Post-travel affordances are formatted at line 1838, final affordance at line 1856. The `#[cfg(test)]` boundary is at line 2209. Existing tests: `post_travel_affordance_snapshot_uses_first_new_place_after_travel` (line 2601), `final_affordances_use_last_planning_snapshot` (line 2620), `no_post_travel_affordance_snapshot_without_travel_commit` (line 2631), `affordance_summary_omits_target_count_when_zero` (line 2641).

## Architecture Check

1. Pure derived analysis over existing `AgentDecisionTrace` data — no new simulation state, no causal changes. The `AffordanceChangeEvent` struct is local to `observer.rs` and not exported. This follows the same pattern as `post_travel_affordance_snapshots` and `final_affordance_snapshot`.
2. No backwards-compatibility shims. New function slots into the existing affordance snapshot pipeline alongside its siblings.

## Verification Layers

1. Affordance-change detection correctness -> focused unit tests in `observer.rs` `#[cfg(test)]` block
2. Report formatting correctness -> focused unit test verifying output string contains expected `**Affordance changes**` lines
3. Single-layer ticket (observer-only tooling); additional layer mapping is not applicable — no simulation state or AI pipeline changes.

## What to Change

### 1. Add `AffordanceChangeEvent` struct and `affordance_change_snapshots` function

In `observer.rs`, add after `final_affordance_snapshot` (after line 802):

```rust
struct AffordanceChangeEvent<'a> {
    tick: Tick,
    affordances: &'a AffordanceTrace,
    appeared: Vec<String>,
    disappeared: Vec<String>,
    place_changed: bool,
}

fn affordance_change_snapshots<'a>(
    affordance_snapshots: &[(Tick, &'a AffordanceTrace)],
) -> Vec<AffordanceChangeEvent<'a>> { ... }
```

Implementation: iterate consecutive pairs from the input slice. For each pair, collect `action_name` strings into `BTreeSet`s, compute symmetric difference. If non-empty, push an `AffordanceChangeEvent` with the current tick's data, the appeared/disappeared sets, and `place_changed` set by comparing `AffordanceTrace.place` between the two snapshots.

Use `BTreeSet<&str>` for deterministic iteration order (project invariant: no `HashSet` in authoritative paths; observer is tooling but maintaining consistency).

### 2. Format affordance changes in `format_report`

In `format_report`, after the `post_travel_affordance_snapshots` loop (after line 1854) and before the `final_affordance_snapshot` block (line 1856), insert:

```rust
for event in affordance_change_snapshots(&affordance_snapshots) {
    let mut parts = Vec::new();
    for name in &event.appeared {
        parts.push(format!("+{name}"));
    }
    for name in &event.disappeared {
        parts.push(format!("-{name}"));
    }
    let hint = if event.place_changed {
        event.affordances.place.map_or_else(
            String::new,
            |place| format!(" (at {})", entity_display_name(world, place)),
        )
    } else {
        String::new()
    };
    writeln!(
        out,
        "**Affordance changes** (tick {}): {}{hint}",
        event.tick.0,
        parts.join(", ")
    )
    .unwrap();
}
```

### 3. Add unit tests

Add tests in the `#[cfg(test)]` block:

- `affordance_change_detects_appeared_action`: Two snapshots where the second has an additional action type. Verify `appeared` contains the new name and `disappeared` is empty.
- `affordance_change_detects_disappeared_action`: Two snapshots where the second is missing an action type. Verify `disappeared` contains the removed name.
- `affordance_change_ignores_target_count_changes`: Two snapshots with the same `action_name` but different `target_count`. Verify no change event is emitted.
- `affordance_change_detects_place_change`: Two snapshots with different `place` values and different action sets. Verify `place_changed` is true.
- `no_affordance_change_when_sets_identical`: Two snapshots with identical action-name sets. Verify empty result.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Per-tick affordance dumps (too verbose — spec non-goal TQ-1).
- Engine-side affordance tracking (observer-only analysis).
- Modifying the simulation tick or decision pipeline.
- New ECS components or SystemFn changes.

## Acceptance Criteria

### Tests That Must Pass

1. `affordance_change_detects_appeared_action` — verifies appeared set populated correctly
2. `affordance_change_detects_disappeared_action` — verifies disappeared set populated correctly
3. `affordance_change_ignores_target_count_changes` — verifies deduplication by `action_name`
4. `affordance_change_detects_place_change` — verifies `place_changed` flag
5. `no_affordance_change_when_sets_identical` — verifies no spurious events
6. Existing suite: `cargo test -p worldwake-cli --bin observer`

### Invariants

1. `AffordanceChangeEvent` is purely derived from existing `AgentDecisionTrace` data — no new authoritative state introduced.
2. Observer report formatting uses `**bold label**` convention consistently with existing affordance entries.
3. Action-type comparison uses `action_name` only, deduplicating across `target_count` variants.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (6 new tests in `#[cfg(test)]`) — validate change-detection logic, edge cases, and report formatting

### Commands

1. `cargo test -p worldwake-cli --bin observer -- affordance_change` (targeted new tests)
2. `cargo test -p worldwake-cli --bin observer` (full observer test suite)
3. `cargo clippy --workspace --all-targets -- -D warnings` (lint)

## Outcome

Completed on 2026-04-12.

- Added local `AffordanceChangeEvent` analysis in `crates/worldwake-cli/src/bin/observer.rs` to compare consecutive planning affordance snapshots by `action_name`, ignoring `target_count` churn and recording whether the believed place changed.
- Inserted `**Affordance changes**` lines into Section 7 of the observer report between post-travel affordances and final affordances, including `+`/`-` markers and a place hint when the affordance snapshot moved to a different place.
- Added focused observer-bin unit coverage for appeared/disappeared detection, target-count-only stability, place-change detection, identical-set suppression, and report rendering.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer -- affordance_change`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ticket was untracked before archival; archived file remains untracked in this worktree (`?? archive/tickets/S98OBSAFFCHA-001.md`)
