# E15CCONMEMANDRECKNO-002: Runtime And Planning Conversation Memory Views

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief/runtime traits and AI planning snapshot/state
**Deps**: `E15CCONMEMANDRECKNO-001`, `specs/E15c-conversation-memory-and-recipient-knowledge.md`

## Problem

E15c requires actor-local told-memory reads on both live belief views and planning/runtime search surfaces. Today `GoalBeliefView`, `RuntimeBeliefView`, `PerAgentBeliefView`, `PlanningSnapshot`, and `PlanningState` expose entity beliefs and TellProfile, but not conversation memory. That would force AI and affordance code either to cheat with listener truth or to diverge between runtime and planning.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-sim/src/belief_view.rs` currently exposes `known_entity_beliefs()` and `tell_profile()`, but no actor-local told/heard memory lookup.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` already gates subjective reads to the acting agent and returns empty data for other agents. That matches E15c’s locality requirement and should be extended rather than bypassed.
3. `crates/worldwake-ai/src/planning_snapshot.rs` currently persists `actor_known_entity_beliefs` and `actor_tell_profile`; `crates/worldwake-ai/src/planning_state.rs` preserves those same actor-local surfaces in search.
4. Existing focused coverage proves that belief and TellProfile data already cross this boundary: `planning_snapshot::tests::build_snapshot_preserves_present_actor_tell_profile` and `planning_state::tests::planning_state_preserves_actor_belief_memory_and_tell_profile_from_snapshot`.
5. The spec makes planning-snapshot persistence optional only if later planning does not consume conversation memory. In this codebase, Tell affordance expansion during search uses `RuntimeBeliefView`, so actor-local told-memory must be reachable from planning/runtime code.
6. Mismatch and correction: this is not just a sim-trait ticket. The AI planning snapshot/state must carry the same actor-local resend inputs or the planner and live runtime will split.

## Architecture Check

1. Extending the existing belief/runtime trait boundary is cleaner than introducing a second social-memory adapter because E14 already established that AI reads must flow through these traits.
2. Planning snapshot/state should continue to preserve only actor-local social memory needed for lawful planning, not whole-world or counterparty belief stores.

## Verification Layers

1. Live actor-local told-memory lookup is retention-aware and inaccessible for non-actors -> focused unit tests in `worldwake-sim`
2. Planning snapshot preserves actor-local conversation memory and recipient-knowledge status inputs -> focused unit tests in `worldwake-ai`
3. Planning state exposes the same actor-local conversation memory as the snapshot -> focused unit tests in `worldwake-ai`
4. Later action/decision traces are out of scope for this plumbing-only ticket.

## What to Change

### 1. Extend belief/runtime traits

Add actor-local told-memory and recipient-knowledge query surfaces to `GoalBeliefView` and `RuntimeBeliefView`.

### 2. Implement retention-aware live reads

Update `PerAgentBeliefView` to expose only the acting agent’s conversation memory, using the retention-aware helpers from ticket 001 rather than raw map access.

### 3. Preserve actor-local conversation memory in planning

Extend `PlanningSnapshot` and `PlanningState` so Tell affordance expansion and planning-time reasoning can read the same actor-local told memory that live candidate generation reads.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)

## Out of Scope

- Changing Tell commit semantics
- Changing social candidate generation behavior
- Adding new decision-trace omission kinds
- Golden E2E social scenarios

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_exposes_retention_aware_told_belief_memory`
2. `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_hides_other_agents_conversation_memory`
3. `cargo test -p worldwake-ai planning_snapshot::tests::build_snapshot_preserves_actor_conversation_memory`
4. `cargo test -p worldwake-ai planning_state::tests::planning_state_preserves_actor_conversation_memory_from_snapshot`
5. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`

### Invariants

1. Planning/runtime code may read only the actor’s local conversation memory, never the listener’s live belief store.
2. Retention-aware reads must behave identically in live runtime and planning snapshot/state surfaces.
3. Snapshot ordering and storage remain deterministic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add actor-only conversation-memory visibility tests.
2. `crates/worldwake-ai/src/planning_snapshot.rs` — add snapshot preservation tests for actor-local conversation memory.
3. `crates/worldwake-ai/src/planning_state.rs` — add planning-state preservation tests for actor-local conversation memory.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_exposes_retention_aware_told_belief_memory`
2. `cargo test -p worldwake-ai planning_state::tests::planning_state_preserves_actor_conversation_memory_from_snapshot`
3. `cargo test -p worldwake-ai`
