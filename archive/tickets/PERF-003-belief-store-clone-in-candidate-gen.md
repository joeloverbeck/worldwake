# PERF-003: Eliminate `AgentBeliefStore` clone in candidate generation and persist paths

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief view traits, `worldwake-ai` candidate generation + planning state
**Deps**: None

## Problem

`AgentBeliefStore` is cloned repeatedly per agent per tick:

1. **In `generate_candidates_with_travel_horizon`** (5.5%): `agent_belief_store()` on `PerAgentBeliefView` clones the entire store (all BTreeMaps and Vecs) at `crates/worldwake-sim/src/per_agent_belief_view.rs:435`.
2. **In `persist_active_goal`** (2.8%): `WorldTxn::new` clones the store as part of snapshot diffing.
3. **In `tick_action`** (3.3%): `WorldTxn::new` clones again.

Combined, these clones account for **~10%** of total runtime (profiled at 2880 ticks, 74K samples). The `AgentBeliefStore` contains `BTreeMap<EntityId, BelievedEntityState>` (~31 entries), `Vec<SocialObservation>` (~430 entries at steady state), `BTreeMap<TellMemoryKey, ToldBeliefMemory>`, and several other maps. Cloning all of this per agent per tick is the largest single allocation cost.

## Assumption Reassessment (2026-04-07)

1. `PerAgentBeliefView::agent_belief_store` at `per_agent_belief_view.rs:434-436` does `self.belief_store.clone()` — confirmed. The `GoalBeliefView` trait defines `fn agent_belief_store(&self, agent: EntityId) -> Option<AgentBeliefStore>` returning owned.
2. The clone in candidate generation is triggered by `emit_social_candidates` at `candidate_generation.rs:1201` which calls `ctx.view.agent_belief_store(ctx.agent)`. This is the only call site in candidate generation.
3. `WorldTxn::new` at `crates/worldwake-core/src/world_txn.rs` snapshots component state for diff computation on commit — the belief store clone there is part of the transactional model.
4. The `GoalBeliefView` trait is implemented by `PerAgentBeliefView`, `PlanningState`, and test mocks. Changing the return type from owned to borrowed affects all implementors.
5. `social_observations` Vec is the single largest member by allocation cost (430 entries × ~48 bytes each at steady state).

## Architecture Check

1. For the candidate-generation path: the caller at `emit_social_candidates` only needs to call `classify_communication` which reads the store immutably. A borrowed reference suffices. The clean fix is to change the `GoalBeliefView::agent_belief_store` signature to return `Option<&AgentBeliefStore>` where the belief view holds a reference. This is feasible for `PerAgentBeliefView` which already borrows the world. Test mocks can own their data and return references.
2. For the `WorldTxn` path: this is a deeper architectural concern. `WorldTxn::new` snapshots to compute diffs on commit. Eliminating this clone requires a structural change to the transaction model (e.g., COW or explicit field-level diffs). This is out of scope for this ticket.
3. No backwards-compatibility shims — the trait signature changes, all implementors update.

## Verification Layers

1. Candidate generation produces identical candidates → golden test hash stability
2. Social candidate emission unchanged → decision trace equivalence in golden tests
3. Cross-crate change (`worldwake-sim` trait, `worldwake-ai` callers) — verification via full `cargo test --workspace`.

## What to Change

### 1. Change `GoalBeliefView::agent_belief_store` to return `Option<&AgentBeliefStore>`

Update the trait method signature. `PerAgentBeliefView` already borrows `&AgentBeliefStore` internally — it can return a reference directly.

### 2. Update all implementors

- `PerAgentBeliefView` — return `Some(self.belief_store)` (reference, no clone)
- `PlanningState` — return reference from its internal store
- Test mocks — own data, return reference

### 3. Update callers in candidate generation

`emit_social_candidates` at `candidate_generation.rs:1201` currently does `let Some(speaker_beliefs) = ctx.view.agent_belief_store(ctx.agent)`. Update to work with a reference.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait signature)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — impl)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — caller)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — PlanningState impl if it implements the trait)
- Test mocks in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs`

## Out of Scope

- `WorldTxn::new` belief store clone (requires transactional model changes)
- Changing other `GoalBeliefView` methods that return owned types (`known_entity_beliefs`, `known_social_observations`)
- The `tick_action` WorldTxn clone

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai`
2. Existing suite: `cargo test -p worldwake-sim`
3. Existing suite: `cargo test --workspace`

### Invariants

1. Candidate generation produces identical results (deterministic replay unchanged)
2. No new `unsafe` code

## Test Plan

### New/Modified Tests

1. None — verification via existing golden tests and unit tests. Behavioral equivalence guaranteed by deterministic replay.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-04-07.

- Changed `GoalBeliefView::agent_belief_store` and `RuntimeBeliefView::agent_belief_store` trait signatures from `Option<AgentBeliefStore>` (owned) to `Option<&AgentBeliefStore>` (borrowed) in `belief_view.rs`.
- Updated the forwarding macro `impl_goal_belief_view_for_runtime!` to match the new return type.
- `PerAgentBeliefView` now returns `Some(self.belief_store)` (zero-cost reference) instead of `self.belief_store.clone()`.
- `PlanningState` now returns `Some(&self.snapshot.actor_belief_store)` instead of cloning.
- `PlanningSnapshot::new` uses `.cloned().unwrap_or_default()` since it needs an owned copy for snapshot construction.
- `emit_social_candidates` in `candidate_generation.rs` hoisted the `agent_belief_store` call above the `for topic` loop, eliminating per-topic repeated lookup.
- Test mock `TestBeliefView::agent_belief_store` simplified to `self.belief_stores.get(&agent)`. Added `sync_belief_store` helper to synthesize stores from scattered test fields. Updated 8 social-candidate tests to call `sync_belief_store` before `generate_candidates`.

## Deviations

- Ticket proposed only changing the trait signature. Implementation also hoisted the call above the loop in `emit_social_candidates`, which is complementary — the trait change eliminates the clone, and the hoist eliminates the repeated HashMap lookup.
- `Files to Touch` listed `planning_snapshot.rs` as optional. It was required because `PlanningSnapshot::new` uses `.unwrap_or_default()` which doesn't work on `&AgentBeliefStore`. Added `.cloned()` before `.unwrap_or_default()` — this is the one remaining clone, needed because `PlanningSnapshot` must own its data.

## Verification Result

- Passed `cargo test -p worldwake-ai` (1069 tests including 152 candidate_generation tests + 36 planner conformance)
- Passed `cargo clippy -p worldwake-ai --lib -- -D warnings`
- Passed `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
- Passed `cargo test --workspace`
- Note: `cargo clippy -p worldwake-ai --all-targets` has pre-existing failures in untracked `perf_diag.rs` binary, unrelated to this ticket.
