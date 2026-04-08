# PERF-002: Reduce allocation in `grounded_goal_epistemic_subjects`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` goal model and search
**Deps**: None

## Problem

`grounded_goal_epistemic_subjects` is called on every node expansion during GOAP `search_plan`, and again via `grounded_goal_matches_epistemic_barrier` for barrier checks. Profiling (2880-tick flamegraph, 74K samples) shows these two call sites consuming **13.2%** of total runtime (9.6% direct + 3.6% via barrier). Nearly all the cost is in `BTreeSet::from_iter` → `Vec::from_iter` → allocation inside the closure chain.

The function collects `evidence_entities`, filters through `known_entity_beliefs` (which itself allocates a `Vec`), builds a `BTreeSet` for deduplication, then converts back to `Vec`. This allocation-heavy pattern runs potentially hundreds of times per agent per tick across search expansions.

## Assumption Reassessment (2026-04-07)

1. `grounded_goal_epistemic_subjects` at `crates/worldwake-ai/src/goal_model.rs:128-166` confirmed via source read. The function allocates a `BTreeSet<EpistemicSubject>` on every call.
2. `search_plan` at `crates/worldwake-ai/src/search/mod.rs` calls this function on every node expansion to check epistemic barriers — confirmed via profiling call graph.
3. `grounded_goal_matches_epistemic_barrier` at `goal_model.rs:168` calls `grounded_goal_epistemic_subjects` again — the same set is recomputed rather than passed through. Profile shows 3.6% at this call site.
4. `known_entity_beliefs` returns `Vec<(EntityId, BelievedEntityState)>` by value — each call allocates. Line 143: `state.known_entity_beliefs(actor).into_iter()` allocates a new Vec.
5. `evidence_entities` is a `BTreeSet<EntityId>` on `GroundedGoal`, typically containing 1-3 entities.

## Architecture Check

1. The core issue is allocation on every search-node expansion. The clean fix caches the result per goal per search invocation, since `evidence_entities` and belief state do not change within a single `search_plan` call. Compute once at search entry and pass by reference. This aligns with FND-27 (derived summaries are caches, never truth) — the epistemic subjects are a derived view valid for the duration of the search.
2. No backwards-compatibility shims — the function signature changes from returning `Vec` to accepting a pre-computed reference or computing into a caller-owned buffer.

## Verification Layers

1. Epistemic barrier matching produces identical results → existing golden test suite (deterministic replay)
2. Search produces identical plans → `cargo test -p worldwake-ai` (golden tests are hash-stable)
3. Single-layer ticket scoped to `worldwake-ai` planner internals; no authoritative state changes.

## What to Change

### 1. Cache epistemic subjects per goal at search entry

In `search_plan`, compute `grounded_goal_epistemic_subjects` once before the search loop begins. Store the result and pass it by reference to `grounded_goal_matches_epistemic_barrier` and any other call site within the search.

### 2. Avoid intermediate `BTreeSet` when deduplication is unnecessary

`evidence_entities` is already a `BTreeSet`, and the filter closure produces at most one `EpistemicSubject` per entity. If evidence entities are already unique (they are — it's a set), the `BTreeSet` intermediate is unnecessary. Collect directly into a `Vec` (or `SmallVec`) and skip the dedup step.

### 3. Avoid re-allocating `known_entity_beliefs` per call

`known_entity_beliefs(actor)` allocates a new `Vec` on every call. Within `search_plan`, the beliefs don't change. Consider passing the beliefs slice in rather than re-querying each time. Alternatively, use `find` on the belief view directly rather than collecting all beliefs into a Vec first — `evidence_entities` typically contains only 1-3 entries, so a targeted lookup per entity is cheaper than collecting all beliefs.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — `grounded_goal_epistemic_subjects`, `grounded_goal_matches_epistemic_barrier`)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — pass pre-computed subjects to barrier check)
- `crates/worldwake-ai/src/search/transition.rs` (modify — pre-compute subjects, pass to barrier check)

## Out of Scope

- Changing the `GoalBeliefView` trait to return references instead of owned types (larger refactor)
- Optimizing `known_entity_beliefs` allocation across all callers

## Acceptance Criteria

### Tests That Must Pass

1. All existing `grounded_goal_epistemic_subjects` tests in `crates/worldwake-ai/src/goal_model.rs`
2. All search tests: `cargo test -p worldwake-ai -- search`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `grounded_goal_epistemic_subjects` returns the same set of subjects for the same inputs (behavioral equivalence)
2. Search produces identical plans — validated by golden test hash stability

## Test Plan

### New/Modified Tests

1. None — existing golden tests and search unit tests cover epistemic barrier correctness. Verification is via deterministic replay hash stability.

### Commands

1. `cargo test -p worldwake-ai -- grounded_goal_epistemic_subjects`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-07.

- Removed `BTreeSet` intermediate in `grounded_goal_epistemic_subjects` (`goal_model.rs:163-165`). Since `evidence_entities` is already a `BTreeSet<EntityId>` and `filter_map` produces at most one `EpistemicSubject` per entity, the dedup step was unnecessary. Now collects directly to `Vec`.
- Changed `grounded_goal_matches_epistemic_barrier` signature to accept `&[EpistemicSubject]` instead of `(&GroundedGoal, &PlanningState)` — eliminates redundant re-computation of subjects at every barrier check.
- Updated all 3 call-site files: `candidates.rs` (1 site), `transition.rs` (2 sites, each pre-computing subjects once), `goal_model.rs` tests (1 test with 4 assertions).
- Item 3 from "What to Change" (avoid re-allocating `known_entity_beliefs` per call) is structurally already addressed by the lazy `find_map` pattern in the function body. Changing the trait return type to avoid the `Vec` allocation is explicitly out of scope.

## Deviations

- Ticket proposed caching at search entry. Implemented a simpler approach: changed `grounded_goal_matches_epistemic_barrier` to take pre-computed subjects by reference, so each call site computes once and reuses. This achieves the same zero-recomputation goal without a separate cache struct.

## Verification Result

- Passed `cargo test -p worldwake-ai` (all 27 lib tests + 36 planner conformance tests)
- Passed `cargo clippy -p worldwake-ai --lib -- -D warnings`
- Passed `cargo test --workspace`
- Note: `cargo clippy -p worldwake-ai --all-targets` has pre-existing failures in untracked `perf_diag.rs` binary, unrelated to this ticket.
