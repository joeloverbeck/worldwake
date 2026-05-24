# S166OPPCMPSRCFID-004: Trace and diagnostics for derived status distribution

**Status**: COMPLETED
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
2. Populating `compiled_by_status` from the final emitted opportunities keeps the trace data self-consistent with the field that drives it — the same status values carried by `Opportunity.source_belief.status` are counted after salience filtering and cap truncation. No second derivation path is introduced.
3. Mirroring on `PerformanceMetrics` follows the existing per-counter mirror precedent (every `OpportunityCompilerLoad` field surfaced today is already mirrored as a `PercentileBucket` on `PerformanceMetrics`). The per-status aggregation choice (selected per Assumption 4) preserves the file's local convention.

## Verified Layers

1. Per-status counts populated correctly during emission -> focused unit test in `opportunity_compiler/compile.rs::tests` constructs a multi-status fixture and asserts `OpportunityCompilerLoad::compiled_by_status` matches the expected per-tag distribution.
2. Mixed-freshness regression guard -> golden scenario test in `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` seeds one fresh direct-observation claim plus one decayed-past-threshold claim and asserts the live compiler load carries at least two status buckets. This is the "all-Probable cannot recur silently" check from spec D3 at the compiler-load seam.
3. Aggregator mirror correctness -> focused unit test in `scenario_diagnostics/aggregator.rs::tests` constructs an `OpportunityCompilerLoad` with distinct per-status counts and asserts the aggregator reduces it into `PerformanceMetrics::opportunity_compiled_by_status`.
4. Existing construction-site fallout passed after explicit clone/field updates: `decision_trace.rs::tests`, `scenario_diagnostics/aggregator.rs::tests`, and observer tests.

## Landed Changes

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

The landed code populates `compiled_by_status` from the final emitted opportunity slice after salience filtering, sorting, and cap truncation. This keeps `compiled_by_status.values().sum::<u32>() == compiled_count`, matching the live `compiled_count` meaning ("after salience filtering and cap truncation").

```rust
for opportunity in &opportunities {
    *load
        .compiled_by_status
        .entry(opportunity.source_belief.status)
        .or_insert(0) += 1;
}
```

The drafted pre-cap wording was corrected during implementation because the live `compiled_count` field is documented and implemented as a post-cap emitted count.

### 3. Mirror on `ScenarioDiagnosticsReport::PerformanceMetrics`

`crates/worldwake-ai/src/scenario_diagnostics/mod.rs` now adds `opportunity_compiled_by_status: BTreeMap<BeliefStatusTag, PercentileBucket>` alongside the existing opportunity-load percentile buckets.

The aggregator at `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` reduces `OpportunityCompilerLoad::compiled_by_status` instances across planning traces into that per-tag percentile map.

### 4. Update explicit struct-literal construction sites

Each explicit construction site was updated with `compiled_by_status: BTreeMap::new()` or a small fixture-appropriate populated map:

- `crates/worldwake-ai/src/decision_trace.rs:3300-3305` (test fixture — also gets the `.clone()` update per item 1).
- `crates/worldwake-cli/src/bin/observer.rs:9136-9141` (observer test scaffolding — `compiled_by_status: BTreeMap::new()` is sufficient).
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:1317-1322` (aggregator test fixture — may want a small populated map if the test asserts per-status aggregation; `BTreeMap::new()` otherwise).

### 5. Add focused test for compile-time per-status population

In `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests`, add `compile_opportunities_records_per_status_distribution`: constructs an `AgentBeliefStore` with multiple claims at mixed freshness (one direct-observation `Certain`, one decayed `Stale`), runs `compile_opportunities`, asserts `load.compiled_by_status.len() >= 2` and the distribution sums to `load.compiled_count`.

### 6. Add mixed-freshness golden-style assertion

`crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` now has Scenario 462:
- Seeds two known inventory entities — one with a fresh direct-observation claim, one with a decayed (or rumor-sourced) claim past the staleness threshold.
- Runs the live compiler against the belief view at the later tick.
- Asserts the resulting `OpportunityCompilerLoad::compiled_by_status` contains at least two distinct `BeliefStatusTag` values and partitions `compiled_count`.

This is the spec's D3 mixed-freshness regression guard.

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs` (modify — drop Copy, add field, update test fixture at line 3300)
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — populate `compiled_by_status` in emission loop, add focused test)
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — extend `PerformanceMetrics` with per-status aggregation)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — extend reducer, update fixture at line 1317)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — update scaffolding at line 9136)
- `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` (modify — add mixed-freshness scenario assertion)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (modify — refreshed diagnostics fixture for the new serialized performance field)
- `docs/generated/golden-*` and `docs/generated/golden-scenario-details/*` (modify/add — regenerated after Scenario 462 metadata and S167 duplicate-ID hygiene)
- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` (modify — verification hygiene: renumbered two S167 scenario metadata IDs from duplicated 454/455 to unused 463/464 so `golden_inventory.py` can pass)

## Out of Scope

- Adding a separate pre-cap status distribution. The landed counter partitions the post-cap emitted opportunities because that is the live `compiled_count` semantic.
- Promoting `compiled_by_status` to authoritative state. It remains a derived trace/diagnostics counter, not a planner input.
- Extending the observer's display surface to render per-status counts. The new field flows through the observer via `OpportunityCompilerLoad`'s existing rendering path; explicit per-status display is a separate observability concern outside this ticket.
- Changing `OpportunityCompilerLoad`'s remaining derives beyond the `Copy` drop. `Eq`, `Ord`, `Hash`, `Serialize`, `Deserialize` are preserved (all are `BTreeMap`-compatible).

## Acceptance Result

### Tests Passed

1. New focused test `compile_opportunities_records_per_status_distribution` asserts per-status counts populate correctly.
2. New integration test in `tests/scenarios/opportunity_compiler.rs` asserts mixed-freshness produces at least two distinct tags in the compiler load.
3. New focused test in `scenario_diagnostics/aggregator.rs::tests` asserts the aggregator reducer combines per-status maps correctly across ticks.
4. The existing test at `decision_trace.rs:3300` passes after the `.clone()` update; the test at `aggregator.rs:1317` and observer scaffolding at `observer.rs:9136` pass after their construction-site updates.
5. `cargo test -p worldwake-ai` and `cargo test -p worldwake-cli` — both crates' suites pass.

### Invariants

1. `OpportunityCompilerLoad::compiled_by_status.values().sum::<u32>() == OpportunityCompilerLoad::compiled_count` for every emitted load (the per-status counts partition the total). Verify with a focused assertion in the new compile-time test.
2. `BTreeMap<BeliefStatusTag, _>` iteration is determinism-stable (no `HashMap` in authoritative or trace state).
3. `OpportunityCompilerLoad` is no longer `Copy` after this ticket. Any future code attempting byval Copy will fail to compile, forcing an explicit `.clone()` decision.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — `compile_opportunities_records_per_status_distribution`: mixed-freshness fixture, asserts per-status map populates and sums to `compiled_count`.
2. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (inline `tests` module) — focused reducer test for the new per-status aggregation field.
3. `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` — new Scenario 462 asserting at least two distinct tags in `OpportunityCompilerLoad` under mixed-freshness input.
4. `crates/worldwake-ai/src/decision_trace.rs::tests` (line-3300 test) — updated to `.clone()` after Copy is dropped.
5. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (line-1317 fixture) — updated to include `compiled_by_status`.
6. `crates/worldwake-cli/src/bin/observer.rs` (line-9136 scaffolding) — updated to include `compiled_by_status: BTreeMap::new()`.

### Commands Run

1. `cargo test -p worldwake-ai opportunity_compiler` — targets the new compile-time per-status test and existing emission tests.
2. `cargo test -p worldwake-ai scenario_diagnostics` — targets the aggregator reducer test and existing fixtures.
3. `cargo test -p worldwake-ai --test golden_ai scenarios::opportunity_compiler::mixed_freshness_status_distribution_reaches_scenario_diagnostics -- --exact` — exercises the new mixed-freshness scenario.
4. `cargo test -p worldwake-cli` — confirms the observer scaffolding update compiles and passes.
5. `cargo test -p worldwake-ai` — full AI crate suite after the final source/generated diff.
6. `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact` — intentionally refreshed the diagnostics fixture for the new JSON field.
7. `cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact` — verified the refreshed fixture is stable.
8. `python3 scripts/golden_inventory.py --write --check-docs` — regenerated and validated golden inventory docs after Scenario 462 metadata and duplicate-ID hygiene.
9. Waived `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `./scripts/verify.sh` at per-ticket closeout because the `implement-spec-tickets` final branch phase owns the full pre-PR verification gate before push.

Merge note: This ticket drops `Copy` from `OpportunityCompilerLoad` (one consumer update at `decision_trace.rs:3300`) but does not bump `SAVE_FORMAT_VERSION` — `OpportunityCompilerLoad` is a trace/diagnostics surface, not part of the saved simulation payload (verified by grepping `crates/worldwake-sim/src/save_load.rs` for `OpportunityCompilerLoad`: 0 matches).

## Outcome

Completed on 2026-05-24.

- Added `OpportunityCompilerLoad::compiled_by_status: BTreeMap<BeliefStatusTag, u32>` and removed the `Copy` derive from `OpportunityCompilerLoad`.
- Populated `compiled_by_status` from the final emitted opportunities after salience filtering, ordering, and cap truncation so the per-status counts partition `compiled_count`.
- Added `PerformanceMetrics::opportunity_compiled_by_status: BTreeMap<BeliefStatusTag, PercentileBucket>` and wired scenario diagnostics aggregation, serde/json round-trip coverage, and observer text rendering for the new field.
- Added focused compiler, diagnostics reducer, and mixed-freshness opportunity-compiler scenario coverage.
- Refreshed the scenario diagnostics fixture and regenerated golden inventory docs for Scenario 462.
- Renumbered the S167 cognitive-archetype divergence metadata scenarios from duplicated IDs 454/455 to unused IDs 463/464 so the golden inventory generator could validate the S166 metadata refresh.

## Deviations

- The drafted ticket described `compiled_by_status` as a pre-cap count, but live `OpportunityCompilerLoad::compiled_count` is explicitly post-cap. The landed counter follows the live post-cap emitted-opportunity contract and the ticket now records pre-cap status distribution as out of scope.
- The mixed-freshness golden-style scenario proves the live compiler-load distribution directly. The `ScenarioDiagnosticsReport` mirror is proved by the focused reducer test and the refreshed diagnostics fixture rather than by constructing a synthetic diagnostics report in the scenario test.
- The observer text renderer now lists the per-status percentile buckets. This is bounded renderer fallout from adding the field to `PerformanceMetrics`, not a new planner-visible consumer.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests::compile_opportunities_records_per_status_distribution -- --exact`.
- Passed `cargo test -p worldwake-ai --lib scenario_diagnostics::aggregator::tests::performance_metrics_roll_up_opportunity_load_and_cache_counters -- --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::opportunity_compiler::mixed_freshness_status_distribution_reaches_scenario_diagnostics -- --exact`.
- Passed `cargo test -p worldwake-ai opportunity_compiler`.
- Passed `cargo test -p worldwake-ai scenario_diagnostics`.
- Passed `cargo test -p worldwake-cli`.
- Passed `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact` to intentionally refresh `expected-scenario-diagnostics.json`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo test -p worldwake-ai`.
- Waived `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `./scripts/verify.sh` for per-ticket closeout because the final `implement-spec-tickets` branch phase owns the full pre-PR verification gate before push.
