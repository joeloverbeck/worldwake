# S83BELCANDPR-003: Belief-gated place filtering in AcquireCommodity candidates

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation filtering, diagnostics
**Deps**: S83BELCANDPR-001, S83BELCANDPR-002

## Problem

`acquisition_path_opportunities_inner()` currently scans every topologically reachable place (potentially 1000-7000+) before deciding whether that place has lawful acquisition evidence for `AcquireCommodity` and `RestockCommodity` goals. The live helper already drops places whose evidence is empty, but it still pays the full reachability sweep first and does not distinguish "places with any remembered topology presence" from "places with concrete acquisition support" until the late evidence pass. This ticket adds an explicit belief-gating layer ahead of the final evidence assembly so candidate generation only expands places with lawful acquisition support, while optionally allowing speculative agents to keep known-but-currently-unsupported places in the search set.

## Assumption Reassessment (2026-04-10)

1. `acquisition_path_opportunities_inner` at `crates/worldwake-ai/src/candidate_generation.rs:4058` currently enumerates `reachable_places_within_horizon(...)` and only later calls `acquisition_path_evidence_at_place(...)` to decide whether a place is worth keeping.
2. The lawful place-evidence surface for `AcquireCommodity` is broader than `resource_sources_at(place, commodity)` plus `controlled_commodity_quantity_at_place(...)`: `acquisition_path_evidence_at_place(...)` also recognizes seller lots, loose lots, corpse inventory, and recipe-backed acquisition paths. The prefilter must preserve that live acquisition contract rather than narrowing to only resource-source places.
3. `known_place_observations(view, agent)` at `candidate_generation.rs:4147` returns `BTreeMap<EntityId, Tick>` of places the agent has beliefs about. It is the correct existing helper for the speculative path.
4. `CandidateGenerationDiagnostics` at `candidate_generation.rs:159` is only an internal generation carrier today. If the filtering ratio is meant to be debuggable, the new counters must also cross `agent_tick/observation.rs`, `agent_tick/mod.rs`, and `decision_trace.rs` into `CandidateTrace`.
5. `acquisition_path_opportunities` and `direct_acquisition_path_opportunities` both delegate to `_inner`, and `emit_restock_goals` also consumes the same shared helper. Counter recording must therefore reflect the shared `AcquireCommodity` / `RestockCommodity` place-search boundary rather than only the self-consume path.
6. Existing domain goldens already exercise `AcquireCommodity` and S80 exploration fallback, especially `golden_trade_*` and `golden_exploration_*`; they remain the owning golden surfaces for broadened verification.

## Architecture Check

1. Belief-gated filtering remains the minimal intervention, but the gate must mirror the current lawful acquisition-evidence surface instead of introducing a narrower "resource-source-only" contract. One private place-filter helper plus a shared opportunity return-carrier is cleaner than patching each emitter separately.
2. No backward-compatibility shims. The change keeps the existing `GoalBeliefView` acquisition semantics, adds speculative widening only through `CognitiveProfile.speculative_acquisition`, and extends the existing trace carrier honestly so the filter ratio is visible where candidate-generation diagnostics already live.

## Verification Layers

1. Shared place filter preserves lawful acquisition support (seller, source, loose lot, corpse, recipe path) and removes unsupported places -> focused unit tests with `TestBeliefView`
2. Agents with no remote acquisition evidence generate zero remote acquisition candidates and can fall through to exploration -> focused unit test plus existing exploration goldens
3. Speculative mode includes known-but-currently-unsupported places only when `CognitiveProfile.speculative_acquisition` is true -> focused unit test
4. Filtering counters record the reachable-vs-kept ratio and surface it through `CandidateTrace` -> focused unit test plus trace-carrier unit coverage
5. Existing `AcquireCommodity` / exploration goldens continue passing -> golden E2E suites
6. Cross-file traceability addition: candidate generation -> read phase -> `CandidateTrace` -> focused trace tests

## What to Change

### 1. Add shared `belief_gated_places` helper and opportunity carrier

In `crates/worldwake-ai/src/candidate_generation.rs`, add a private place-filter helper plus a small shared search carrier (`AcquisitionPathSearchResult`) so the acquisition helper can return both opportunities and filter counts.

Logic:
- Always include the agent's current place (local acquisition)
- Include places with any existing lawful direct acquisition support at that place: seller lots, loose lots, live resource sources, matching corpse inventory, or other current acquisition evidence already recognized by `acquisition_path_evidence_at_place`
- Preserve recipe-backed acquisition places when the existing evidence surface says the place can lawfully produce the target commodity
- If `include_speculative`: include places in `known_place_observations(view, agent)`

Add a small shared return carrier for `_inner` so the helper can return both the kept opportunities and the `reachable` / `after_filter` counts without recording diagnostics at only one caller.

### 2. Integrate into `acquisition_path_opportunities_inner`

Replace the direct `.into_iter().filter_map()` chain with a shared search path that:

```rust
let cognitive = view.cognitive_profile(agent);
let include_speculative = cognitive
    .map(|p| p.speculative_acquisition)
    .unwrap_or(false);
let reachable = reachable_places_within_horizon(view, origin, travel_horizon);
let belief_filtered = belief_gated_places(
    view, agent, &reachable, commodity, gate_options,
);
belief_filtered
    .into_iter()
    .filter_map(|filtered_place| { ... })
    .collect()
```

### 3. Add diagnostic fields and carry them through decision traces

Add two fields:

```rust
pub places_reachable: u32,
pub places_after_belief_filter: u32,
```

Record these at the shared acquisition-place search boundary, then carry them through:

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/agent_tick/observation.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/decision_trace.rs`

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)

## Out of Scope

- Hierarchical plan decomposition (TravelTo + AcquireLocal subgoals)
- Dynamic expansion budget scaling per-agent
- Modifying `reachable_places_within_horizon()` itself
- Filtering for goal kinds other than `AcquireCommodity`

## Acceptance Criteria

### Tests That Must Pass

1. `belief_gated_places` returns only the current place when no remote lawful acquisition evidence exists
2. `belief_gated_places` preserves remote places supported by seller lots, resource sources, corpse inventory, and recipe-backed acquisition paths
3. `belief_gated_places` includes known-but-no-current-support places only when `include_speculative` is true
4. `acquisition_path_opportunities_inner` records fewer kept places than reachable places when beliefs are sparse
5. Diagnostic counters correctly reflect pre- and post-filter counts and are visible on `CandidateTrace`
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Local acquisition candidates (at the agent's current place) are always generated regardless of speculative mode
2. The place gate never removes a place that the existing lawful acquisition evidence surface would keep
3. The belief view is the sole source of filtering data — no authoritative world state is read directly by candidate generation (FND-14)
4. `known_place_observations` is reused, not duplicated (DRY)
5. All existing golden tests pass: `cargo test -p worldwake-ai -- golden`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_returns_current_place_with_no_remote_support` — verifies zero remote places when no lawful acquisition evidence exists
2. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_preserves_direct_acquisition_support` — verifies seller/source/corpse style evidence passes the filter
3. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_preserves_recipe_backed_support` — verifies recipe-backed acquisition places are not pruned
4. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_speculative_includes_known_places` — verifies speculative mode includes visited places without current support
5. `crates/worldwake-ai/src/candidate_generation.rs::acquisition_path_diagnostics_record_filtering_ratio` — verifies internal counters
6. `crates/worldwake-ai/src/decision_trace.rs` — trace fixture updates and focused coverage for surfaced counters

### Commands

1. `cargo test -p worldwake-ai belief_gated`
2. `cargo test -p worldwake-ai acquisition_path_diagnostics_record_filtering_ratio`
3. `cargo test -p worldwake-ai -- golden_exploration`
4. `cargo test -p worldwake-ai -- golden_trade`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added a shared belief-gated acquisition-place search in `crates/worldwake-ai/src/candidate_generation.rs` that preserves the full lawful acquisition surface: sellers, loose lots, live resource sources, corpse inventory, recipe-backed acquisition, and speculative known-place widening via `CognitiveProfile.speculative_acquisition`.
- Replaced the old direct reachable-place scan in the shared acquisition helper with a return carrier that records both reachable-place count and post-filter count, then aggregated those counters onto `CandidateGenerationDiagnostics`.
- Carried the new counters through `crates/worldwake-ai/src/agent_tick/observation.rs`, `crates/worldwake-ai/src/agent_tick/mod.rs`, and `crates/worldwake-ai/src/decision_trace.rs` so the filter ratio is visible on `CandidateTrace`.
- Added focused unit coverage for direct support, recipe-backed support, speculative widening, diagnostics aggregation, and the surfaced trace counters.

## Verification Result

- Passed `cargo test -p worldwake-ai belief_gated`
- Passed `cargo test -p worldwake-ai acquisition_path_diagnostics_record_filtering_ratio`
- Passed `cargo test -p worldwake-ai candidate_trace_retains_place_filter_counters`
- Passed `cargo test -p worldwake-ai -- golden_exploration`
- Passed `cargo test -p worldwake-ai -- golden_trade`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
