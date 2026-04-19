# S109TYPDISTAX-003: Belief-view accessors, CognitiveProfile TTL fields, and discrepancy_ttl function

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new `GoalBeliefView` accessor methods, new `CognitiveProfile` fields (additive), new `discrepancy_ttl` helper
**Deps**: archive/tickets/S109TYPDISTAX-001.md, archive/tickets/S109TYPDISTAX-002.md, archive/tickets/S109TYPDISTAX-007.md

## Problem

S109's emission migration (T004) will write typed discrepancies into `DiscrepancyMemory` and bucket their TTLs by `Discrepancy` class. Before that migration can land, three additive pieces must be in place: (1) belief-view accessors that let the AI crate read the new memories through the `GoalBeliefView` surface; (2) per-class TTL fields on `CognitiveProfile` so each `Discrepancy` variant has a profile-driven backoff; (3) a `discrepancy_ttl(&Discrepancy, &CognitiveProfile) -> u32` helper that mirrors the existing `blocking_fact_ttl` function.

The existing `CognitiveProfile::unknown_block_ticks` field stays for now — T006 removes it together with the `BlockingFact::Unknown` variant. This ticket's additions are strictly additive, so the workspace keeps building and every existing scenario still deserializes.

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `CognitiveProfile` is defined at `crates/worldwake-core/src/cognitive_profile.rs:6`. T007 already landed `repair_memory_ticks` and `learned_opportunity_memory_ticks`, so the still-live additive TTL work here is only the 9 discrepancy/backoff fields from S109 D6 alongside the existing `transient_block_ticks`, `unknown_block_ticks`, and `structural_block_ticks`. `blocking_fact_ttl` lives at `crates/worldwake-ai/src/failure_handling.rs:988` and still buckets `BlockingFact` variants by those three legacy fields. Existing focused unit tests on `CognitiveProfile` already include T007 memory-TTL coverage; T003 extends that proof surface only for the 9 discrepancy TTL additions.
2. `GoalBeliefView` is defined at `crates/worldwake-sim/src/belief_view.rs:70`. No `discrepancy_memory`, `blocker_memory`, `repair_memory`, or `learned_opportunity_memory` accessor exists on the trait today. Live AI readers still access these components directly through `World`/`RuntimeBeliefView` component getters, and T007 intentionally kept the repair/opportunity read seam local inside `worldwake-ai` rather than widening `GoalBeliefView`. This ticket still owns the additive read-only accessor family from spec D7; T007 only means those accessors are no longer a blocker for repair/opportunity semantics.
3. Shared abstraction boundary: the `GoalBeliefView` trait surface used by the AI crate, plus the additive discrepancy TTL contract on `CognitiveProfile`. The accessors remain read-only by contract; mutable access continues through direct component accessors.
8. No heuristic removed or bypassed. The new `discrepancy_ttl` function is additive alongside the unchanged `blocking_fact_ttl`. T004's emission migration will choose which function to call based on the `FailureClassification` the classifier returns.
13. Adjacent contradiction correction: T007 already added `repair_memory_ticks` / `learned_opportunity_memory_ticks`, so the drafted T003 literal fallout list is stale and incomplete. The still-live compile fallout is the 9 new discrepancy TTL fields plus the already-landed T007 fields that must remain present at every explicit `CognitiveProfile { ... }` literal. In addition to the originally drafted AI/core files, live explicit literals also exist in `crates/worldwake-cli/src/handlers/persistence.rs`, `crates/worldwake-cli/src/scenario/lints.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, and `crates/worldwake-ai/tests/conformance_execution_budget.rs`.

Correction from reassessment: the 9 new fields are not addable with `#[serde(default)]` alone because `CognitiveProfile` is constructed explicitly in Rust literals across core, sim, AI, CLI, and test code. Rust field-init syntax requires every full literal site to name the new fields or use `..CognitiveProfile::default()` spread. T003 therefore owns the explicit literal fallout needed to keep the workspace compiling after the new discrepancy TTL fields land.

## Architecture Check

1. Landing belief-view accessors and TTL infrastructure together keeps the additive surface small and self-contained. T004 can then consume both without introducing new trait methods or profile fields of its own. Splitting T003 further would create three one-file tickets (belief-view alone, CognitiveProfile alone, `discrepancy_ttl` alone); bundling them preserves reviewability because all three are small and interdependent.
2. No backwards-compatibility aliasing. New accessor methods on `GoalBeliefView` have `None` defaults, which is the standard lazy-extension pattern for this trait (see `agent_belief_store` at line 91 using the same pattern). `discrepancy_ttl` is a new function, not an alias. FND-28 compliant.

## Verification Layers

1. `CognitiveProfile` new-field defaults → focused unit test: `cognitive_profile_default_matches_split_defaults` extended with the 9 new fields at their spec defaults (30, 60, 2, 20, 120, 40, 200, 100, 4).
2. `CognitiveProfile` serde-default behavior → focused unit test: deserialize a minimal RON without the new fields and assert each surfaces at its default value. Mirrors the existing `cognitive_profile_deserialization_defaults_use_ff_heuristic` pattern.
3. `discrepancy_ttl` correctness → focused unit test: for each `Discrepancy` variant, `discrepancy_ttl(variant, &CognitiveProfile::default())` returns the documented default (spec D6 table).
4. `GoalBeliefView` default accessors return `None` on the trait default impl → focused unit test at `belief_view.rs`.
5. `PerAgentBeliefView` accessors return `Some(&DiscrepancyMemory)` / etc. when the component is registered on the actor and `None` for non-self entities → focused unit test at `per_agent_belief_view.rs`.
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
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — save/load roundtrip fixture literal)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — scenario lint fixture literal)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify — explicit cognitive profile fixture literals)

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
3. `cargo test -p worldwake-sim belief_view`
4. `cargo test -p worldwake-sim per_agent_belief_view`
5. Existing focused tests still pass: `cognitive_profile_default_matches_split_defaults`, `cognitive_profile_roundtrips_through_bincode`, `cognitive_profile_deserialization_defaults_use_ff_heuristic`, `cognitive_profile_deserialization_defaults_travel_candidate_cap_to_none`, `blocking_fact_ttl_uses_budget_classification`, `unknown_blocker_uses_dedicated_ttl`, `transient_blockers_unchanged_ttl`.
6. Existing full suite: `cargo test --workspace`.

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
4. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — add returns-Some-when-registered and non-self-returns-None tests for the new accessors.

### Commands

1. `cargo test -p worldwake-core cognitive_profile`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test -p worldwake-sim belief_view`
4. `cargo test -p worldwake-sim per_agent_belief_view`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completed the additive discrepancy/backoff slice for S109. `CognitiveProfile` now carries the 9 discrepancy TTL fields from spec D6 with serde defaults and default values, `failure_handling.rs` now exposes `discrepancy_ttl(&Discrepancy, &CognitiveProfile)`, and `GoalBeliefView` / `PerAgentBeliefView` now provide read-only accessors for `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, and `LearnedOpportunityMemory`.

Reassessment narrowed the live scope versus the original T003 draft. T007 had already landed `repair_memory_ticks` / `learned_opportunity_memory_ticks`, so this ticket only added the remaining discrepancy TTL fields and belief-view accessors. The real compile fallout was also narrower than the expanded reassessment inventory: explicit `CognitiveProfile` literals in core and AI helper/test code needed updates, while several drafted files and surveyed neighbors already used `..CognitiveProfile::default()` or serde omission paths and compiled unchanged.

## Deviations

1. T007 had already landed `repair_memory_ticks` / `learned_opportunity_memory_ticks`, so T003 no longer owned those fields and their associated constructor fallout.
2. The drafted `RuntimeBeliefView` implementation edit in `belief_view.rs` was not a real separate surface on the live branch; the correct implementation boundary was the existing `SocialBeliefView` forwarding path plus `PerAgentBeliefView`.
3. `discrepancy_ttl` is staged additive infrastructure for T004 and is not called yet on this branch. To keep CI-matching clippy honest without inventing premature production usage, the landed helper carries a narrow `#[allow(dead_code)]`.
4. No code changes were needed in several reassessed files that were initially listed or surveyed for fallout, including `crates/worldwake-cli/src/scenario/types.rs`, `crates/worldwake-cli/src/handlers/persistence.rs`, `crates/worldwake-cli/src/scenario/lints.rs`, and `crates/worldwake-ai/tests/conformance_execution_budget.rs`, because their live construction/deserialization paths already inherited defaults lawfully.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core cognitive_profile`
- Passed `cargo test -p worldwake-ai failure_handling`
- Passed `cargo test -p worldwake-sim belief_view`
- Passed `cargo test -p worldwake-sim per_agent_belief_view`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
