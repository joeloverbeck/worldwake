# S109TYPDISTAX-003: Belief-view accessors, CognitiveProfile TTL fields, and discrepancy_ttl function

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new `GoalBeliefView` accessor methods, new `CognitiveProfile` fields (additive), new `discrepancy_ttl` helper
**Deps**: S109TYPDISTAX-001, S109TYPDISTAX-002

## Problem

S109's emission migration (T004) will write typed discrepancies into `DiscrepancyMemory` and bucket their TTLs by `Discrepancy` class. Before that migration can land, three additive pieces must be in place: (1) belief-view accessors that let the AI crate read the new memories through the `GoalBeliefView` surface; (2) per-class TTL fields on `CognitiveProfile` so each `Discrepancy` variant has a profile-driven backoff; (3) a `discrepancy_ttl(&Discrepancy, &CognitiveProfile) -> u32` helper that mirrors the existing `blocking_fact_ttl` function.

The existing `CognitiveProfile::unknown_block_ticks` field stays for now — T006 removes it together with the `BlockingFact::Unknown` variant. This ticket's additions are strictly additive, so the workspace keeps building and every existing scenario still deserializes.

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `CognitiveProfile` is defined at `crates/worldwake-core/src/cognitive_profile.rs:6` and currently has three TTL fields: `transient_block_ticks` (default 20), `unknown_block_ticks` (default 5), `structural_block_ticks` (default 200). `blocking_fact_ttl` lives at `crates/worldwake-ai/src/failure_handling.rs:992–1011` and buckets `BlockingFact` variants by those three fields. Existing focused unit tests on `CognitiveProfile` at `cognitive_profile.rs:81–223`: `cognitive_profile_component_bounds`, `cognitive_profile_default_matches_split_defaults`, `cognitive_profile_roundtrips_through_bincode`, `cognitive_profile_deserialization_defaults_use_ff_heuristic`, `cognitive_profile_deserialization_defaults_travel_candidate_cap_to_none`, `cognitive_profile_registers_for_agents`. Existing `blocking_fact_ttl` tests at `failure_handling.rs:2517–2543`: `blocking_fact_ttl_uses_budget_classification`, `unknown_blocker_uses_dedicated_ttl`, `transient_blockers_unchanged_ttl`.
2. `GoalBeliefView` trait is defined at `crates/worldwake-sim/src/belief_view.rs:70`. No `blocked_intent_memory` or `blocker_memory` accessor exists on the trait today — the 12 AI-crate consumers of `BlockerMemory` (post-T001 rename) access it directly via `get_component_blocker_memory`. `RuntimeBeliefView` impl lives in the same file (lines ~230–520) and `PerAgentBeliefView` impl lives at `crates/worldwake-sim/src/per_agent_belief_view.rs`. Spec D7 prescribes 4 read-only accessors on the trait, each defaulting to `None`.
3. Shared abstraction boundary: the `GoalBeliefView` trait surface used by the AI crate's candidate generation, ranking, and runtime observers. The boundary under audit is purely additive — we add 4 new methods with `None` defaults, so existing implementors that don't override them compile unchanged. The new methods are read-only by contract; mutable access continues through direct component accessors.
8. No heuristic removed or bypassed. The new `discrepancy_ttl` function is additive alongside the unchanged `blocking_fact_ttl`. T004's emission migration will choose which function to call based on the `FailureClassification` the classifier returns.
13. No adjacent contradictions. The 9 new `CognitiveProfile` TTL fields all carry `#[serde(default = "...")]` so existing scenario RONs and existing `CognitiveProfile { ... }` literal sites continue to deserialize and compile. The 32 explicit construction sites + 12 spread sites surveyed during reassessment do not need updating in this ticket — the spread sites inherit the new defaults, and the explicit sites remain valid because they list a subset of fields (Rust requires all fields at construction, which means every explicit site already lists `unknown_block_ticks` — those sites stay compilable until T006 removes the field).

Correction from reassessment: the 9 new fields are not added with `#[serde(default)]` alone because `CognitiveProfile` is constructed in Rust literal form (not just deserialized) at ~20 sites. Rust field-init syntax requires every field at each explicit literal site. Therefore the 20 explicit sites must either (a) use `..CognitiveProfile::default()` spread, or (b) list all 9 new fields. Decision: all 20 explicit sites already exist (surveyed 2026-04-19); T003 updates those sites to include the 9 new fields at their default values, OR converts them to spread syntax where appropriate. This is mechanical and adds ~9 lines per site. This is necessary in T003 rather than deferred because otherwise the workspace won't compile after T003's CognitiveProfile field additions.

## Architecture Check

1. Landing belief-view accessors and TTL infrastructure together keeps the additive surface small and self-contained. T004 can then consume both without introducing new trait methods or profile fields of its own. Splitting T003 further would create three one-file tickets (belief-view alone, CognitiveProfile alone, `discrepancy_ttl` alone); bundling them preserves reviewability because all three are small and interdependent.
2. No backwards-compatibility aliasing. New accessor methods on `GoalBeliefView` have `None` defaults, which is the standard lazy-extension pattern for this trait (see `agent_belief_store` at line 91 using the same pattern). `discrepancy_ttl` is a new function, not an alias. FND-28 compliant.

## Verification Layers

1. `CognitiveProfile` new-field defaults → focused unit test: `cognitive_profile_default_matches_split_defaults` extended with the 9 new fields at their spec defaults (30, 60, 2, 20, 120, 40, 200, 100, 4).
2. `CognitiveProfile` serde-default behavior → focused unit test: deserialize a minimal RON without the new fields and assert each surfaces at its default value. Mirrors the existing `cognitive_profile_deserialization_defaults_use_ff_heuristic` pattern.
3. `discrepancy_ttl` correctness → focused unit test: for each `Discrepancy` variant, `discrepancy_ttl(variant, &CognitiveProfile::default())` returns the documented default (spec D6 table).
4. `GoalBeliefView` default accessors return `None` on the trait default impl → focused unit test at `belief_view.rs`.
5. `PerAgentBeliefView` accessors return `Some(&DiscrepancyMemory)` / etc. when the component is registered on the agent → focused unit test at `per_agent_belief_view.rs`.
6. Single-layer ticket for each addition: trait default, `CognitiveProfile` field, `discrepancy_ttl` function are each provable at their own unit layer; no mixed-layer invariant asserted here.

## What to Change

### 1. Add belief-view accessors

In `crates/worldwake-sim/src/belief_view.rs` `trait GoalBeliefView`, add 4 read-only accessors with `None` defaults (mirroring the `agent_belief_store` pattern at line 91):

```rust
fn discrepancy_memory(&self, agent: EntityId) -> Option<&DiscrepancyMemory> {
    let _ = agent;
    None
}
fn blocker_memory(&self, agent: EntityId) -> Option<&BlockerMemory> {
    let _ = agent;
    None
}
fn repair_memory(&self, agent: EntityId) -> Option<&RepairMemory> {
    let _ = agent;
    None
}
fn learned_opportunity_memory(&self, agent: EntityId) -> Option<&LearnedOpportunityMemory> {
    let _ = agent;
    None
}
```

In the `RuntimeBeliefView` impl (same file) and `PerAgentBeliefView` impl (`crates/worldwake-sim/src/per_agent_belief_view.rs`), override each accessor to return `world.get_component_<memory_name>(agent)`.

Add imports for `BlockerMemory`, `DiscrepancyMemory`, `RepairMemory`, `LearnedOpportunityMemory` from `worldwake-core` at the top of each file.

### 2. Add `CognitiveProfile` TTL fields

In `crates/worldwake-core/src/cognitive_profile.rs:6` struct, add 9 new fields each with `#[serde(default = "...")]`:

```rust
#[serde(default = "default_stale_belief_backoff_ticks")]
pub stale_belief_backoff_ticks: u32,
#[serde(default = "default_contradicted_belief_backoff_ticks")]
pub contradicted_belief_backoff_ticks: u32,
#[serde(default = "default_improper_state_backoff_ticks")]
pub improper_state_backoff_ticks: u32,
#[serde(default = "default_missing_observation_backoff_ticks")]
pub missing_observation_backoff_ticks: u32,
#[serde(default = "default_no_legal_binding_backoff_ticks")]
pub no_legal_binding_backoff_ticks: u32,
#[serde(default = "default_counterparty_refusal_backoff_ticks")]
pub counterparty_refusal_backoff_ticks: u32,
#[serde(default = "default_route_unknown_backoff_ticks")]
pub route_unknown_backoff_ticks: u32,
#[serde(default = "default_search_exhaustion_backoff_ticks")]
pub search_exhaustion_backoff_ticks: u32,
#[serde(default = "default_partial_drift_backoff_ticks")]
pub partial_drift_backoff_ticks: u32,
```

Add `const fn default_*` functions returning the spec D6 defaults (30, 60, 2, 20, 120, 40, 200, 100, 4). Update `impl Default for CognitiveProfile` to include the new fields at those defaults. Keep `unknown_block_ticks` unchanged — it is removed in T006.

### 3. Add `discrepancy_ttl` function

In `crates/worldwake-ai/src/failure_handling.rs`, alongside `blocking_fact_ttl` at line 992:

```rust
fn discrepancy_ttl(discrepancy: Discrepancy, cognitive: &CognitiveProfile) -> u32 {
    match discrepancy {
        Discrepancy::BeliefStale => cognitive.stale_belief_backoff_ticks,
        Discrepancy::BeliefContradicted => cognitive.contradicted_belief_backoff_ticks,
        Discrepancy::ImproperPlanningState => cognitive.improper_state_backoff_ticks,
        Discrepancy::MissingObservation => cognitive.missing_observation_backoff_ticks,
        Discrepancy::NoLegalBinding => cognitive.no_legal_binding_backoff_ticks,
        Discrepancy::NoWillingCounterparty => cognitive.counterparty_refusal_backoff_ticks,
        Discrepancy::RouteUnknown => cognitive.route_unknown_backoff_ticks,
        Discrepancy::SearchBudgetExhausted => cognitive.search_exhaustion_backoff_ticks,
        Discrepancy::PartialExecutionDrift => cognitive.partial_drift_backoff_ticks,
    }
}
```

The exhaustive match is the compile-time cover for spec Validation item 7. Add `Discrepancy` import.

### 4. Update explicit `CognitiveProfile { ... }` construction sites

The ~20 Rust literal sites that explicitly enumerate all `CognitiveProfile` fields (without `..Default::default()` spread) must be updated to include the 9 new fields at their default values. Sites surveyed 2026-04-19:

- `crates/worldwake-core/src/cognitive_profile.rs` test module (default-match + roundtrip tests).
- `crates/worldwake-core/src/delta.rs:582` — within a constructor helper.
- `crates/worldwake-ai/src/failure_handling.rs:1375` — `ProfileFixture`-derived.
- `crates/worldwake-ai/src/decision_runtime.rs:358` — same.
- `crates/worldwake-ai/src/agent_tick/planning.rs:1382` — same.
- `crates/worldwake-ai/src/agent_tick/tests.rs:105` — same.
- `crates/worldwake-ai/src/goal_model.rs:2590` — same.
- `crates/worldwake-ai/src/search/tests.rs:60` — same.
- `crates/worldwake-ai/src/lib.rs:132, 150` — `PlanningBudget` or `ProfileFixture` default.
- `crates/worldwake-cli/src/scenario/types.rs:939` — fallback default in scenario loader.

Each site adds the 9 new fields at their defaults (or converts to `..CognitiveProfile::default()` spread when cleaner — the choice is per-site, guided by whether the surrounding construction needs explicit control over other fields).

### 5. Update CognitiveProfile tests

- Extend `cognitive_profile_default_matches_split_defaults` (line 102) to assert the 9 new field defaults.
- Extend `cognitive_profile_roundtrips_through_bincode` (line 127) with the 9 new fields in the sample profile.
- Add a new test `cognitive_profile_deserialization_defaults_ttl_fields` that deserializes a RON profile missing all 9 new fields and asserts each falls back to its serde default. Mirrors `cognitive_profile_deserialization_defaults_use_ff_heuristic` at line 153.

### 6. Add `discrepancy_ttl` tests

In `crates/worldwake-ai/src/failure_handling.rs` `#[cfg(test)]`:

- `discrepancy_ttl_uses_class_specific_defaults` — iterates all 9 `Discrepancy` variants and asserts each returns the spec D6 default.
- `discrepancy_ttl_respects_profile_override` — construct a `CognitiveProfile` with non-default TTL values and assert `discrepancy_ttl` reads them.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add 4 accessors to trait + `RuntimeBeliefView` impl)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — override 4 accessors)
- `crates/worldwake-core/src/cognitive_profile.rs` (modify — 9 new fields + defaults + existing-test extensions + new serde-default test)
- `crates/worldwake-core/src/delta.rs` (modify — `CognitiveProfile` literal at line 582)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — add `discrepancy_ttl` + tests; update `ProfileFixture`-derived literal at line 1375)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — `CognitiveProfile` literal at line 358)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `CognitiveProfile` literal at line 1382)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — `CognitiveProfile` literal at line 105)
- `crates/worldwake-ai/src/goal_model.rs` (modify — `CognitiveProfile` literal at line 2590)
- `crates/worldwake-ai/src/search/tests.rs` (modify — `CognitiveProfile` literal at line 60)
- `crates/worldwake-ai/src/lib.rs` (modify — `PlanningBudget`/`ProfileFixture` default at lines 132, 150)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `CognitiveProfile` fallback default at line 939)

## Out of Scope

- No change to `blocking_fact_ttl` — it keeps all existing arms including `Unknown => unknown_block_ticks` (T006 removes both).
- No change to `unknown_block_ticks` field — survives this ticket, removed in T006.
- No migration of `BlockingFact::Unknown`/`AssumptionFailed` emission sites (T004).
- No replacement of `UnknownBlockerTrace` (T005).
- No scenario RON changes (the new fields use `#[serde(default)]` so existing RONs deserialize unchanged).
- No observer binary / CLI rendering of the new accessors — observer uses direct component access.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core cognitive_profile` — existing + new default/roundtrip/serde-default tests.
2. `cargo test -p worldwake-ai failure_handling::tests::discrepancy_ttl` — new `discrepancy_ttl` tests.
3. `cargo test -p worldwake-sim belief_view per_agent_belief_view` — new accessor tests.
4. Existing focused tests still pass: `cognitive_profile_default_matches_split_defaults`, `cognitive_profile_roundtrips_through_bincode`, `cognitive_profile_deserialization_defaults_use_ff_heuristic`, `cognitive_profile_deserialization_defaults_travel_candidate_cap_to_none`, `blocking_fact_ttl_uses_budget_classification`, `unknown_blocker_uses_dedicated_ttl`, `transient_blockers_unchanged_ttl`.
5. Existing full suite: `cargo test --workspace`.

### Invariants

1. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
2. Existing scenario RONs in `scenarios/*.ron` and `crates/worldwake-cli/tests/fixtures/observer_anomalies/*.ron` continue to deserialize without modification (new fields are `#[serde(default)]`).
3. `blocking_fact_ttl` behavior is unchanged for every input.
4. `GoalBeliefView` implementors that do not override the 4 new accessors continue to compile and return `None`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` — extend default-match + bincode-roundtrip tests; add `cognitive_profile_deserialization_defaults_ttl_fields`.
2. `crates/worldwake-ai/src/failure_handling.rs` `#[cfg(test)]` — add `discrepancy_ttl_uses_class_specific_defaults`, `discrepancy_ttl_respects_profile_override`.
3. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — add default-returns-None test for each of the 4 new accessors.
4. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — add returns-Some-when-registered test for each of the 4 new accessors.

### Commands

1. `cargo test -p worldwake-core cognitive_profile`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test -p worldwake-sim belief_view`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
