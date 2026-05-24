# S166OPPCMPSRCFID-004: Trace and diagnostics for derived status distribution

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `OpportunityCompilerLoad` gains a per-status count map; `ScenarioDiagnosticsReport::PerformanceMetrics` mirrors the per-tag aggregation. `OpportunityCompilerLoad` loses its `Copy` derive (storage-shape change forced by `BTreeMap` field).
**Deps**: `archive/tickets/S166OPPCMPSRCFID-003.md` (derived `source_belief.status` available in emission loop); spec `specs/S166-opportunity-compiler-source-fidelity.md` (D3)

## Problem

After ticket 003 lands, opportunity compilation produces a real per-status
distribution — but `OpportunityCompilerLoad` (`decision_trace.rs:1008`) carries
only `compiled_count`, `salience_floored`, `learned_memory_damped`,
`cap_truncated`. The "all compiled opportunities are Probable" regression that
S166 prevents by construction is invisible to the existing trace and to
`ScenarioDiagnosticsReport::PerformanceMetrics`. Without per-status visibility,
a future regression that re-collapses status (e.g., reverting D1's derivation
to a literal) would not be caught by diagnostics. This ticket adds the
per-status count map to both surfaces and seeds a focused mixed-freshness
assertion that proves at least two distinct tags can be observed.

## Assumption Reassessment (2026-05-24)

1. `OpportunityCompilerLoad` at `crates/worldwake-ai/src/decision_trace.rs:1006-1017` derives `Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`. Adding `compiled_by_status: BTreeMap<BeliefStatusTag, u32>` is incompatible with the `Copy` derive (`BTreeMap` is not `Copy`). This is a path-level storage-shape discrepancy: the spec proposed `BTreeMap` but did not enumerate the host struct's existing `Copy` derive. **Resolution**: drop `Copy` from the derive list. The one consumer relying on byval `Copy` semantics is the test at `decision_trace.rs:3300-3311`, which constructs `let load = OpportunityCompilerLoad { ... }; trace.opportunity_compiler_load = Some(load); ... assert_eq!(sink.opportunity_compiler_load(agent, tick), Some(&load))` — this needs `.clone()` after the rewrite (`Some(load.clone())` so the trailing `&load` reference compiles). No runtime hot path uses byval Copy on this struct; the change is a narrow test-fixture adjustment.
2. `OpportunityCompilerLoad` is **not** part of the `SAVE_FORMAT_VERSION`-versioned simulation payload (`crates/worldwake-sim/src/save_load.rs:7` currently at `SAVE_FORMAT_VERSION = 100`). It's a trace/observer surface only — no save-format bump is needed. Verified by grepping `save_load.rs` for `decision_trace` / `OpportunityCompilerLoad`: 0 matches.
3. Explicit struct-literal construction sites (no `..Default::default()` spread) that need the new field added:
   - `crates/worldwake-ai/src/decision_trace.rs:3300-3305` (test fixture)
   - `crates/worldwake-cli/src/bin/observer.rs:9136-9141` (observer test scaffolding)
   - `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:1317-1322` (aggregator test fixture)
   Plus the runtime construction at `crates/worldwake-ai/src/opportunity_compiler/compile.rs:22` (`OpportunityCompilerLoad::default()`) — this uses `Default` and is unaffected by the new field as long as `Default::default()` returns an empty map.
4. `ScenarioDiagnosticsReport::PerformanceMetrics` at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` carries `opportunity_compiled_count: PercentileBucket` and three sibling buckets. The per-status aggregation should follow the existing convention used elsewhere in `PerformanceMetrics` for per-tag distributions; the implementing engineer selects between `PercentileBucket`-per-tag, `BTreeMap<BeliefStatusTag, PercentileBucket>`, or a flat `BTreeMap<BeliefStatusTag, u64>` aggregate by reading the file's surrounding patterns. The spec's D3 explicitly delegates this shape to implementation.
5. The aggregator at `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` is the writer surface for `PerformanceMetrics`. Adding the new field requires extending the aggregator's per-tick reduction over `OpportunityCompilerLoad` instances to also reduce the per-status counts. The construction site at line 1317-1322 is a unit-test fixture and is included in Files to Touch.
6. The opportunity-compiler golden scenario at `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs:227, 329` exercises the live read-phase pipeline. The new mixed-freshness assertion (per spec D3's V&F point) should slot into this file's existing scenario set or a sibling scenario constructed to seed both a fresh and a stale claim.
7. Existing inline tests in `decision_trace.rs::tests` (the test at line 3300 already named in Assumption 1), `scenario_diagnostics/aggregator.rs::tests` (the fixture at line 1317), and `crates/worldwake-cli/src/bin/observer.rs::tests` (the scaffolding at line 9136) must continue to pass after the new field is wired through. All three are construction-site updates; no behavior assertion in those tests targets the new field directly.

## Architecture Check

1. The `OpportunityCompilerLoad` Copy-drop is a path-level storage adjustment forced by the spec's intent (per-status counts via `BTreeMap`) — the alternative (e.g., `[u32; 5]` indexed by tag ordinal) would preserve `Copy` but require introducing an `as_usize()` shim on `BeliefStatusTag` (a core change) or a fragile cast pattern. Dropping `Copy` is the smaller change with the smaller blast radius (1 test fixture update vs. core-type surface expansion). The struct remains `Clone` for any caller that previously relied on byval semantics — a `.clone()` site replaces an implicit move-with-Copy site.
2. Populating `compiled_by_status` inside `compile_opportunities`'s emission loop (after `source_belief()` returns the derived status) keeps the trace data self-consistent with the field that drives it — the same status value flows into the emitted `Opportunity.source_belief.status` and into the load counter. No second derivation path is introduced.
3. Mirroring on `PerformanceMetrics` follows the existing per-counter mirror precedent (every `OpportunityCompilerLoad` field surfaced today is already mirrored as a `PercentileBucket` on `PerformanceMetrics`). The per-status aggregation choice (selected per Assumption 4) preserves the file's local convention.

## Verification Layers

1. Per-status counts populated correctly during emission → focused unit test in `opportunity_compiler/compile.rs::tests` constructing a multi-status fixture, asserting `OpportunityCompilerLoad::compiled_by_status` matches the expected per-tag distribution.
2. Mixed-freshness diagnostics regression guard → integration test in `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` seeding one direct-observation claim + one decayed-past-threshold claim, asserting `ScenarioDiagnosticsReport::PerformanceMetrics` carries ≥2 distinct tags in the per-status aggregation. This is the "all-Probable cannot recur silently" check from spec D3.
3. Aggregator mirror correctness → focused unit test in `scenario_diagnostics/aggregator.rs::tests` constructing two `OpportunityCompilerLoad` instances with distinct per-status maps and asserting the aggregator reduces them correctly into `PerformanceMetrics`.
4. Existing tests pass unchanged after construction-site updates: `decision_trace.rs::tests` (line-3300 fixture), `scenario_diagnostics/aggregator.rs::tests` (line-1317 fixture), `observer.rs` tests touching the line-9136 scaffolding.

## What to Change

### 1. Extend `OpportunityCompilerLoad` in `crates/worldwake-ai/src/decision_trace.rs:1006-1017`

Drop `Copy` from the derive list; add field:

```rust
#[derive(
    Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct OpportunityCompilerLoad {
    pub compiled_count: u32,
    pub salience_floored: u32,
    pub learned_memory_damped: u32,
    pub cap_truncated: u32,
    /// Per-`BeliefStatusTag` count of emitted opportunities. Distribution should
    /// not collapse to a single tag once ticket 003's derivation lands.
    pub compiled_by_status: BTreeMap<BeliefStatusTag, u32>,
}
```

Update the existing inline test at `decision_trace.rs:3300-3311` to use `.clone()` instead of byval Copy:

```rust
let load = OpportunityCompilerLoad {
    compiled_count: 3,
    salience_floored: 1,
    learned_memory_damped: 2,
    cap_truncated: 1,
    compiled_by_status: BTreeMap::new(),
};
let mut trace = dead_trace(agent, tick);
trace.opportunity_compiler_load = Some(load.clone());

sink.record(trace);

assert_eq!(sink.opportunity_compiler_load(agent, tick), Some(&load));
```

### 2. Populate `compiled_by_status` in `crates/worldwake-ai/src/opportunity_compiler/compile.rs`

After `source_belief()` is called at line 121 (ticket 003 has rewritten it to derive the status), increment the load counter:

```rust
let source_belief_ref = source_belief(belief_view, agent, entity, commodity, &state, current_tick);
*load.compiled_by_status.entry(source_belief_ref.status).or_insert(0) += 1;
opportunities.push(Opportunity {
    key,
    perceived_at: current_tick,
    source_belief: source_belief_ref,
    // ... rest of fields
});
```

The increment happens *before* the cap truncation at `compile.rs:147-150`, so `compiled_by_status` reflects all emitted opportunities (including those that may be cap-truncated). This matches the existing `compiled_count` semantic. If the spec intends counts to reflect post-cap output only, a deduction loop would be needed — but per `OpportunityCompilerLoad`'s existing semantics (`compiled_count` is also pre-cap), pre-cap is the correct alignment.

### 3. Mirror on `ScenarioDiagnosticsReport::PerformanceMetrics`

In `crates/worldwake-ai/src/scenario_diagnostics/mod.rs`, add a per-status aggregation field to `PerformanceMetrics` alongside the existing `opportunity_compiled_count` / `opportunity_salience_floored` / etc. The exact shape is selected at implementation time by following the file's existing convention for per-tag distributions (e.g., `BTreeMap<BeliefStatusTag, PercentileBucket>` if other per-tag distributions exist; `BTreeMap<BeliefStatusTag, u64>` for a flat aggregate; or per-tag `PercentileBucket` fields if the file's convention favors named fields over maps).

Extend the aggregator at `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` to reduce `OpportunityCompilerLoad::compiled_by_status` instances across ticks into the chosen `PerformanceMetrics` field shape.

### 4. Update explicit struct-literal construction sites

Each of these sites currently constructs `OpportunityCompilerLoad` without `..Default::default()`; add `compiled_by_status: BTreeMap::new()` (or the small fixture-appropriate map) to each:

- `crates/worldwake-ai/src/decision_trace.rs:3300-3305` (test fixture — also gets the `.clone()` update per item 1).
- `crates/worldwake-cli/src/bin/observer.rs:9136-9141` (observer test scaffolding — `compiled_by_status: BTreeMap::new()` is sufficient).
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:1317-1322` (aggregator test fixture — may want a small populated map if the test asserts per-status aggregation; `BTreeMap::new()` otherwise).

### 5. Add focused test for compile-time per-status population

In `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests`, add `compile_opportunities_records_per_status_distribution`: constructs an `AgentBeliefStore` with multiple claims at mixed freshness (one direct-observation `Certain`, one decayed `Stale`), runs `compile_opportunities`, asserts `load.compiled_by_status.len() >= 2` and the distribution sums to `load.compiled_count`.

### 6. Add mixed-freshness golden-style assertion

In `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs`, add a scenario that:
- Seeds two known inventory entities — one with a fresh direct-observation claim, one with a decayed (or rumor-sourced) claim past the staleness threshold.
- Runs the agent-tick read phase.
- Asserts the resulting `ScenarioDiagnosticsReport::PerformanceMetrics` per-status aggregation contains ≥2 distinct `BeliefStatusTag` values.

This is the spec's D3 mixed-freshness regression guard.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — drop Copy, add field, update test fixture at line 3300)
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — populate `compiled_by_status` in emission loop, add focused test)
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — extend `PerformanceMetrics` with per-status aggregation)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — extend reducer, update fixture at line 1317)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — update scaffolding at line 9136)
- `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` (modify — add mixed-freshness scenario assertion)

## Out of Scope

- Adding `compiled_by_status` semantics for *what* gets counted beyond emission. The counter reflects pre-cap emission (matching the existing `compiled_count` semantic); a post-cap variant would require a separate decision.
- Promoting `compiled_by_status` to authoritative state. It remains a derived trace/diagnostics counter, not a planner input.
- Extending the observer's display surface to render per-status counts. The new field flows through the observer via `OpportunityCompilerLoad`'s existing rendering path; explicit per-status display is a separate observability concern outside this ticket.
- Changing `OpportunityCompilerLoad`'s remaining derives beyond the `Copy` drop. `Eq`, `Ord`, `Hash`, `Serialize`, `Deserialize` are preserved (all are `BTreeMap`-compatible).

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `compile_opportunities_records_per_status_distribution` asserts per-status counts populate correctly.
2. New integration test in `tests/scenarios/opportunity_compiler.rs` asserts mixed-freshness produces ≥2 distinct tags in the diagnostics aggregation.
3. New focused test in `scenario_diagnostics/aggregator.rs::tests` asserts the aggregator reducer combines per-status maps correctly across ticks.
4. The existing test at `decision_trace.rs:3300` passes after the `.clone()` update; the test at `aggregator.rs:1317` and observer scaffolding at `observer.rs:9136` pass after their construction-site updates.
5. `cargo test -p worldwake-ai` and `cargo test -p worldwake-cli` — both crates' suites pass.

### Invariants

1. `OpportunityCompilerLoad::compiled_by_status.values().sum::<u32>() == OpportunityCompilerLoad::compiled_count` for every emitted load (the per-status counts partition the total). Verify with a focused assertion in the new compile-time test.
2. `BTreeMap<BeliefStatusTag, _>` iteration is determinism-stable (no `HashMap` in authoritative or trace state).
3. `OpportunityCompilerLoad` is no longer `Copy` after this ticket. Any future code attempting byval Copy will fail to compile, forcing an explicit `.clone()` decision.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — `compile_opportunities_records_per_status_distribution`: mixed-freshness fixture, asserts per-status map populates and sums to `compiled_count`.
2. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (inline `tests` module) — focused reducer test for the new per-status aggregation field.
3. `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` — new scenario asserting ≥2 distinct tags in the diagnostics report under mixed-freshness input.
4. `crates/worldwake-ai/src/decision_trace.rs::tests` (line-3300 test) — updated to `.clone()` after Copy is dropped.
5. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (line-1317 fixture) — updated to include `compiled_by_status`.
6. `crates/worldwake-cli/src/bin/observer.rs` (line-9136 scaffolding) — updated to include `compiled_by_status: BTreeMap::new()`.

### Commands

1. `cargo test -p worldwake-ai opportunity_compiler` — targets the new compile-time per-status test and existing emission tests.
2. `cargo test -p worldwake-ai scenario_diagnostics` — targets the aggregator reducer test and existing fixtures.
3. `cargo test -p worldwake-ai --test golden_ai opportunity_compiler` — exercises the new mixed-freshness scenario alongside existing goldens.
4. `cargo test -p worldwake-cli` — confirms the observer scaffolding update compiles and passes.
5. `cargo test --workspace` — full workspace gate; this ticket touches multiple crates.
6. `cargo clippy --workspace --all-targets -- -D warnings` — clippy gate.
7. `./scripts/verify.sh` — full pre-PR gate before pushing.

Merge note: This ticket drops `Copy` from `OpportunityCompilerLoad` (one consumer update at `decision_trace.rs:3300`) but does not bump `SAVE_FORMAT_VERSION` — `OpportunityCompilerLoad` is a trace/diagnostics surface, not part of the saved simulation payload (verified by grepping `crates/worldwake-sim/src/save_load.rs` for `OpportunityCompilerLoad`: 0 matches).
