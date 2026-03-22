# TRACEVIEW-001: Add a Cross-Layer Timeline View for Emergent Scenario Debugging

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — debug/test-support reporting surface over existing trace sinks and event log
**Deps**: `archive/specs/2026-03-17-action-execution-trace.md`, existing decision trace system in `crates/worldwake-ai/src/decision_trace.rs`, existing action trace system in `crates/worldwake-sim/src/action_trace.rs`, existing political trace system in `crates/worldwake-sim/src/politics_trace.rs`

## Problem

Mixed emergent scenarios currently require jumping between multiple debug surfaces:

- decision trace for AI reasoning
- action trace for lifecycle execution
- raw event-log deltas for authoritative mutation
- political trace for system-level office decisions

That is workable but slow. For cross-system debugging, the missing artifact is one compact timeline view that aligns these layers by tick.

## Assumption Reassessment (2026-03-18)

1. The repo already has all three trace sinks this ticket needs:
   - AI decision traces via `worldwake_ai::AgentTickDriver::enable_tracing()` and `DecisionTraceSink` in `crates/worldwake-ai/src/agent_tick.rs` and `crates/worldwake-ai/src/decision_trace.rs`
   - action lifecycle traces via `ActionTraceSink` in `crates/worldwake-sim/src/action_trace.rs`
   - political/system traces via `PoliticalTraceSink` in `crates/worldwake-sim/src/politics_trace.rs`
2. The original dependency on a future `POLTRAC-001` ticket is stale. Political tracing already exists and is already wired into the golden harness through `GoldenHarness::enable_politics_tracing()` in `crates/worldwake-ai/tests/golden_harness/mod.rs`.
3. Cross-layer golden coverage already exists, but it is manually correlated today rather than unified. The clearest current example is `run_combat_death_force_succession()` in `crates/worldwake-ai/tests/golden_emergent.rs`, which already checks:
   - action lifecycle facts through `ActionTraceSink`
   - political succession facts through `PoliticalTraceSink`
   - authoritative mutation ordering through event-log helpers
4. Coverage gap reassessment:
   - focused/unit coverage for the existing sinks already exists in `crates/worldwake-ai/src/decision_trace.rs`, `crates/worldwake-sim/src/action_trace.rs`, and `crates/worldwake-sim/src/politics_trace.rs`
   - there is no existing focused coverage for a merged cross-layer timeline helper; `rg -n "timeline" crates/worldwake-* docs tickets archive/specs` only finds this ticket and one archived spec note, not implementation or tests
   - there is no existing golden helper that aligns decision, action, political, and selected event-log records into one derived view
5. This remains a debug/reporting-surface problem, not an authoritative-model problem. No production behavior or source-of-truth state should move into the timeline.
6. Architectural constraint reassessment:
   - a reusable production timeline type cannot cleanly live in `worldwake-sim` because it would need to depend on `worldwake-ai` decision-trace types, creating an unwanted layer inversion
   - the first implementation should therefore live in golden/test support, where all relevant layers are already intentionally visible for debugging

## Architecture Check

1. The cleanest approach is a read-only, derived timeline builder over existing sinks plus explicitly selected event-log records. It must not become a new cache or a new authority path.
2. The first implementation belongs in golden/debug test support rather than a runtime crate. That preserves current crate boundaries:
   - `worldwake-ai` already depends on `worldwake-sim`
   - `worldwake-sim` must not grow a dependency back onto AI trace types
3. The timeline should preserve layer boundaries instead of flattening them into one vague event stream. Each entry should carry its source layer explicitly.
4. Authoritative event-log inclusion should be explicit rather than heuristic. The timeline helper should let tests provide the authoritative event filter for the invariant under inspection, instead of guessing which event-log records belong to an agent or office.
5. This is more robust than the current manual-debugging workflow because it centralizes ordering and rendering logic without introducing aliases, caches, or cross-system coupling.
6. Example target output:
   - Tick 7: decision trace selected `EngageHostile`
   - Tick 8: action trace committed `attack`
   - Tick 8: authoritative mutation set `DeadAt`
   - Tick 8: political trace marked office vacant
   - Tick 13: political trace installed new holder

## What to Change

### 1. Define timeline model

Create a compact read-only timeline representation in golden/test support that can merge:

- decision trace entries
- action trace entries
- explicitly selected authoritative event-log entries
- political trace entries

### 2. Add builder/helpers for tests

Expose helpers that let golden tests build and render a cross-layer timeline for one agent, one office, or one scenario window, with an explicit event-log predicate for authoritative entries.

### 3. Add at least one golden/debug usage

Use the timeline in one existing cross-system golden scenario so the merged output shape is proven against a real mixed-layer chain.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/` test-support files (new or modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs`
- `crates/worldwake-ai/tests/golden_emergent.rs`
- `docs/golden-e2e-testing.md`

## Out of Scope

- New authoritative behavior
- Replacing existing decision/action/system trace sinks
- Moving the timeline into the live simulation path
- Automatic event-log causality inference heuristics
- UI work outside textual/debug helper output

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession -- --exact`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`

### Invariants

1. The cross-layer timeline must remain a derived debug view, never the source of truth.
2. Timeline output must preserve actual tick ordering and avoid inventing causal links not present in the underlying traces or selected event-log records.
3. Layer boundaries must remain explicit in the merged output: decision reasoning, action lifecycle, political/system trace, and authoritative event-log entries must stay distinguishable.

## Verification Layers

1. AI reasoning presence and per-tick selection ordering -> `DecisionTraceSink` entries in the merged timeline
2. Action lifecycle facts -> `ActionTraceSink` entries in the merged timeline
3. Political/system succession facts -> `PoliticalTraceSink` entries in the merged timeline
4. Authoritative mutation provenance and append order -> explicit event-log entries selected into the merged timeline
5. Timeline merge ordering -> focused/unit coverage over synthetic per-layer entries plus one real golden scenario

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/timeline.rs` -> `merges_entries_by_tick_and_layer`  
   Rationale: proves the merged helper preserves per-tick ordering and keeps decision, action, event-log, and politics entries distinct.
2. `crates/worldwake-ai/tests/golden_harness/timeline.rs` -> `build_requires_explicit_event_filter_for_authoritative_entries`  
   Rationale: proves authoritative event-log inclusion stays explicit instead of silently inferring causality.
3. `crates/worldwake-ai/tests/golden_emergent.rs` -> `golden_combat_death_triggers_force_succession`  
   Rationale: proves the helper is useful on a real mixed-layer emergent chain that already spans AI reasoning, action execution, political succession, and authoritative mutations.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession -- --exact`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-18
- What actually changed:
  - added a derived `CrossLayerTimelineBuilder` and timeline model under `crates/worldwake-ai/tests/golden_harness/timeline.rs`
  - exposed the helper through `crates/worldwake-ai/tests/golden_harness/mod.rs`
  - enabled decision tracing and asserted merged timeline rendering in `crates/worldwake-ai/tests/golden_emergent.rs`
  - documented explicit event-log selection guidance in `docs/golden-e2e-testing.md`
- Deviations from original plan:
  - the original ticket assumed political tracing was still pending; that dependency was removed because `PoliticalTraceSink` already exists
  - instead of introducing a new runtime cross-crate debug surface, the implementation lives in golden/test support to preserve crate layering and avoid a `worldwake-sim` -> `worldwake-ai` dependency inversion
  - authoritative event-log entries are caller-selected rather than heuristically inferred
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
