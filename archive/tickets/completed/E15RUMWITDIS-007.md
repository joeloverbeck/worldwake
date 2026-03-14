# E15RUMWITDIS-007: Tell Affordance Enumeration

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — Tell affordance generation plus belief/planning surface extensions in worldwake-sim and worldwake-ai
**Deps**: E15RUMWITDIS-005 (Tell ActionDef), E15RUMWITDIS-003 (TellProfile), E15RUMWITDIS-006 (Tell handler)

## Problem

The AI planner needs to know when a Tell action is available. The Tell affordance enumeration function must generate TellActionPayload candidates from the speaker's AgentBeliefStore — filtering by TellProfile.max_relay_chain_len and bounding by TellProfile.max_tell_candidates — for each co-located alive agent target. Without this, the AI cannot choose to tell anything.

## Assumption Reassessment (2026-03-14)

1. Affordance generation pattern: `get_affordances()` in `crates/worldwake-sim/src/affordance_query.rs` iterates action defs and calls each handler's `enumerate_affordances` callback. Confirmed.
2. The Tell action handler in `crates/worldwake-systems/src/tell_actions.rs` is already registered and implemented for start/commit validation, but it is **not** currently wired with `with_affordance_payloads(...)`. This ticket is still required.
3. `AgentBeliefStore.known_entities` in `crates/worldwake-core/src/belief.rs` is the authoritative per-agent subjective memory store, but `RuntimeBeliefView` does **not** currently expose a way to enumerate those beliefs or read `TellProfile` through the planner/runtime boundary.
4. `PerAgentBeliefView` internally has access to the actor's `AgentBeliefStore`, so the missing capability is a trait-surface gap, not missing state.
5. `TellProfile.max_relay_chain_len` and `TellProfile.max_tell_candidates` exist in core and are already enforced authoritatively for payload validation / commit-side relay checks, but cannot yet drive subjective affordance generation because the runtime belief surface lacks the needed reads.
6. Existing Tell tests cover handler registration, payload validation, source degradation, acceptance fidelity, and listener-memory behavior. There were initially no Tell affordance enumeration tests.
7. `PlanningState` in `crates/worldwake-ai/src/planning_state.rs` also implements `RuntimeBeliefView`, and planner affordance search runs against `PlanningSnapshot`/`PlanningState`, not only `PerAgentBeliefView`. The original ticket missed this.

## Architecture Check

1. The original ticket scope was too narrow. A `tell_actions.rs`-only change is not possible without violating the belief boundary or special-casing Tell inside the affordance engine.
2. The clean architecture is to extend `RuntimeBeliefView` with small, generic subjective-memory reads that let affordance generation inspect the actor's own known beliefs and `TellProfile` without reaching into authoritative `World`.
3. Do **not** downcast `&dyn RuntimeBeliefView` to `PerAgentBeliefView`, and do **not** read the actor's authoritative `AgentBeliefStore` directly from `World` during affordance generation. Both approaches would create brittle, non-extensible architecture and undercut the belief-only planning boundary.
4. The combinatorial explosion guard remains necessary and spec-aligned. `max_tell_candidates` should cap subject enumeration before listener expansion.
5. Determinism must be explicit: order by `observed_tick` descending and break ties by `EntityId` ascending. Do not rely on incidental sort stability.
6. `PlanningSnapshot` must preserve the actor's subjective Tell inputs. Extending only the live runtime view would make Tell visible at runtime but still invisible to planning search, which is worse than the original architecture because it creates a split-brain affordance surface.
7. No backwards-compatibility shims or Tell-specific aliases on the planner surface.

## What to Change

### 1. Implement Tell affordance enumeration

In `crates/worldwake-sim/src/belief_view.rs`, extend `RuntimeBeliefView` with the minimum generic reads Tell affordance generation needs:

```rust
fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)>;
fn tell_profile(&self, agent: EntityId) -> Option<TellProfile>;
```

Implementation requirements:

- `PerAgentBeliefView` returns the actor's own subjective beliefs and `TellProfile`.
- `PlanningSnapshot` / `PlanningState` must preserve and expose the actor's subjective belief list and Tell profile so Tell remains plan-visible after snapshotting.
- `StubBeliefView` test doubles in `worldwake-sim` must be updated to implement the expanded trait.
- Keep the methods generic to subjective memory / profile access. Do **not** add Tell-specific helper methods like `relayable_tell_subjects()`.

### 2. Implement Tell affordance payload enumeration

In `crates/worldwake-systems/src/tell_actions.rs`, implement the affordance payload callback:

1. Get actor's `AgentBeliefStore` from belief view.
2. Get actor's `TellProfile` from belief view.
3. Filter `known_entities` keys by chain_len eligibility:
   - For each belief, check if source chain_len ≤ TellProfile.max_relay_chain_len
   - DirectObservation counts as chain_len 0
   - Report { chain_len: n } counts as chain_len n
   - Rumor { chain_len: n } counts as chain_len n
   - Inference counts as chain_len 0 (produces Rumor { chain_len: 1 })
4. Sort eligible beliefs by `observed_tick` descending (most recent first).
5. Sort eligible beliefs by `observed_tick` descending, then `EntityId` ascending for deterministic tie-breaking.
6. Take top `max_tell_candidates` subjects.
6. For each co-located alive agent target (excluding self):
   - For each eligible subject:
     - Generate `Affordance` with `TellActionPayload { listener: target, subject_entity: subject }`.

### 3. Wire affordance enumeration into Tell handler

Ensure the Tell action handler definition includes the affordance enumeration function in its handler struct.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — expose subjective belief/profile reads to runtime affordance generation)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement the new trait methods)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — update `StubBeliefView` used by affordance tests)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — preserve actor belief memory and TellProfile in planning snapshots)
- `crates/worldwake-ai/src/planning_state.rs` (modify — expose snapshot-preserved belief memory / TellProfile through RuntimeBeliefView)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — add affordance enumeration)

## Out of Scope

- AI goal generation for Tell (deciding WHEN to tell — future work)
- Mismatch detection or discovery events
- belief_confidence() function
- Changes to affordance_query.rs framework
- Changes to other action affordance enumerations

## Acceptance Criteria

### Tests That Must Pass

1. Tell affordances generated for each co-located alive agent × eligible subject
2. Beliefs with chain_len > max_relay_chain_len are excluded from affordances
3. No more than max_tell_candidates subjects offered (most recent by observed_tick)
4. Speaker with max_relay_chain_len=1 does not generate affordances for Rumor beliefs
5. Speaker with no beliefs generates no Tell affordances
6. Dead targets excluded from affordances
7. Self excluded as target
8. Existing suite: `cargo test --workspace`
9. `cargo clippy --workspace`
10. `worldwake-sim` trait/object-safety and affordance-query tests still pass after the belief-view extension
11. Tell affordances remain visible through `PlanningState`, not just the live runtime belief view

### Invariants

1. Affordance count bounded by max_tell_candidates × co_located_agent_count
2. Deterministic ordering (BTreeMap + observed_tick sort ensures reproducibility)
3. No belief content leaks to non-listener agents through affordance enumeration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — new tests for Tell affordance enumeration: target filtering, relay-depth filtering, candidate limiting, deterministic ordering
2. `crates/worldwake-sim/src/affordance_query.rs` — update `StubBeliefView` for the expanded runtime trait surface
3. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add/adjust tests for the new subjective-memory/profile accessors
4. `crates/worldwake-ai/src/planning_state.rs` — add/adjust tests proving snapshot-backed planning preserves the actor's belief memory and TellProfile

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Added generic `RuntimeBeliefView` support for enumerating the actor's subjective known-entity beliefs and Tell profile, with safe default implementations for views that do not carry that data.
  - Implemented those reads in `PerAgentBeliefView`.
  - Preserved the actor's known-entity beliefs and Tell profile through `PlanningSnapshot` and `PlanningState`, so Tell affordances are visible to planner search instead of only to the live runtime view.
  - Wired Tell affordance payload enumeration into `tell_actions.rs`, including relay-depth filtering, `max_tell_candidates` limiting, and deterministic ordering by `observed_tick` descending then `EntityId` ascending.
  - Added focused tests for Tell affordance expansion/filtering plus snapshot/runtime accessor coverage.
- Deviations from original plan:
  - The original ticket was too narrow. The final implementation required `worldwake-ai` snapshot/state changes because planning affordances run against `PlanningState`, not only the live runtime belief view.
  - The final `RuntimeBeliefView` extension uses default method bodies to avoid forced churn across unrelated test doubles and adapters while still keeping the meaningful implementations explicit in the views that own subjective memory.
- Verification results:
  - `cargo test -p worldwake-sim` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
