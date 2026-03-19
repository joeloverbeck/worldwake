# E15CCONMEMANDRECKNO-002: Runtime And Planning Conversation Memory Views

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief/runtime traits and AI planning snapshot/state
**Deps**: `E15CCONMEMANDRECKNO-001`, `archive/specs/E15c-conversation-memory-and-recipient-knowledge.md`

## Problem

E15c requires actor-local told-memory reads on both live belief views and planning/runtime search surfaces. Today `GoalBeliefView`, `RuntimeBeliefView`, `PerAgentBeliefView`, `PlanningSnapshot`, and `PlanningState` expose entity beliefs and TellProfile, but not conversation memory. That would force AI and affordance code either to cheat with listener truth or to diverge between runtime and planning.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-core/src/belief.rs` already contains the E15c core data model: `AgentBeliefStore.told_beliefs`, `AgentBeliefStore.heard_beliefs`, `TellMemoryKey`, `ToldBeliefMemory`, `HeardBeliefMemory`, `recipient_knowledge_status()`, and retention-aware read helpers. Existing focused coverage already proves this layer: `conversation_memory_read_helpers_ignore_expired_entries_before_cleanup`, `enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently`, and `recipient_knowledge_status_distinguishes_current_and_stale_tells`.
2. `crates/worldwake-sim/src/belief_view.rs` still exposes `known_entity_beliefs()` and `tell_profile()`, but no actor-local told-memory or recipient-knowledge surface on either `GoalBeliefView` or `RuntimeBeliefView`.
3. `crates/worldwake-sim/src/per_agent_belief_view.rs` already enforces actor-local subjective reads and returns empty data for other agents, as proven by `per_agent_belief_view::tests::known_entity_beliefs_expose_only_actor_subjective_memory`. That is the correct place to extend locality-preserving conversation-memory reads.
4. `crates/worldwake-ai/src/planning_snapshot.rs` currently persists `actor_known_entity_beliefs` and `actor_tell_profile`, and `crates/worldwake-ai/src/planning_state.rs` preserves those exact actor-local surfaces. Existing coverage proves the current boundary only for belief memory and tell profile: `planning_snapshot::tests::build_snapshot_preserves_present_actor_tell_profile` and `planning_state::tests::planning_state_preserves_actor_belief_memory_and_tell_profile_from_snapshot`.
5. The current E15c consumers are not switched yet: `crates/worldwake-ai/src/candidate_generation.rs::emit_social_candidates` still suppresses by same-place subject belief, proven by `candidate_generation::tests::social_candidates_skip_subjects_already_known_to_be_colocated`, and `crates/worldwake-systems/src/tell_actions.rs::enumerate_tell_payloads` still expands raw `relayable_social_subjects` without conversation memory. `crates/worldwake-systems/src/tell_actions.rs::commit_tell` also does not yet record told/heard memory.
6. Mismatch and correction: the original ticket understated what already exists in `worldwake-core` and slightly overstated what is currently required by live callers. This ticket should not claim to introduce conversation memory itself or to fix resend behavior. Its real scope is to expose already-existing actor-local told-memory and recipient-knowledge reads through live/runtime and planning surfaces so later tickets can switch AI generation and tell affordance expansion without introducing a runtime/planning split.

## Architecture Check

1. Extending the existing belief/runtime trait boundary is cleaner than introducing a second social-memory adapter because E14 already established that AI reads must flow through `GoalBeliefView` and `RuntimeBeliefView`, not through direct `World` or ad hoc social caches.
2. Planning snapshot/state should preserve only actor-local speaker memory required for lawful planning and diagnostics. Persisting the whole actor-local told-memory lane is acceptable; persisting counterparty belief stores or whole-world social state is not.
3. This ticket remains beneficial even before the consumer switch because it lets later E15c tickets remove the same-place heuristic and update tell affordance expansion without adding a new compatibility layer or duplicating lookup logic in planning.

## Verification Layers

1. Live actor-local told-memory lookup and derived recipient-knowledge status are retention-aware and inaccessible for non-actors -> focused unit tests in `worldwake-sim`
2. Planning snapshot preserves actor-local told-memory inputs needed for later social planning -> focused unit tests in `worldwake-ai`
3. Planning state exposes the same actor-local told-memory and recipient-knowledge answers as the snapshot -> focused unit tests in `worldwake-ai`
4. Candidate suppression, tell affordance filtering, and tell commit mutation semantics are intentionally not proven here because those authoritative/AI behavior changes belong to later E15c tickets, not this plumbing ticket.

## What to Change

### 1. Extend belief/runtime traits

Add actor-local told-memory and recipient-knowledge query surfaces to `GoalBeliefView` and `RuntimeBeliefView`.

### 2. Implement retention-aware live reads

Update `PerAgentBeliefView` to expose only the acting agent’s conversation memory, using the existing retention-aware helpers in `worldwake-core` rather than raw map access.

### 3. Preserve actor-local conversation memory in planning

Extend `PlanningSnapshot` and `PlanningState` so future Tell affordance expansion and planning-time reasoning can read the same actor-local told memory and recipient-knowledge status that live candidate generation will use once the consumer switch lands.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)

## Out of Scope

- Changing `tell` commit semantics to record told/heard memory
- Replacing the same-place social suppression heuristic in candidate generation
- Updating tell affordance enumeration to filter by conversation memory
- Adding new decision-trace omission kinds
- Golden E2E social scenarios

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_exposes_retention_aware_told_belief_memory_and_recipient_status`
2. `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_hides_other_agents_conversation_memory`
3. `cargo test -p worldwake-ai planning_snapshot::tests::build_snapshot_preserves_actor_told_belief_memory`
4. `cargo test -p worldwake-ai planning_state::tests::planning_state_preserves_actor_conversation_memory_from_snapshot`
5. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`

### Invariants

1. Planning/runtime code may read only the actor’s local conversation memory, never the listener’s live belief store.
2. Retention-aware reads must behave identically in live runtime and planning snapshot/state surfaces.
3. Snapshot ordering and storage remain deterministic.
4. This ticket does not change live social suppression, tell payload enumeration, or tell commit outcomes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add actor-only conversation-memory visibility and derived recipient-status tests.
2. `crates/worldwake-ai/src/planning_snapshot.rs` — add snapshot preservation tests for actor-local told-memory.
3. `crates/worldwake-ai/src/planning_state.rs` — add planning-state preservation tests for actor-local told-memory and derived recipient status.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_exposes_retention_aware_told_belief_memory_and_recipient_status`
2. `cargo test -p worldwake-ai planning_state::tests::planning_state_preserves_actor_conversation_memory_from_snapshot`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - extended `GoalBeliefView` and `RuntimeBeliefView` with actor-local told-memory and recipient-knowledge query surfaces plus a current-tick surface for retention-aware reads
  - updated `PerAgentBeliefView` to expose retention-aware told-memory and derived recipient knowledge only for the acting agent
  - extended `PlanningSnapshot` and `PlanningState` to preserve actor-local told-memory and reproduce the same actor-local recipient-knowledge answers during planning
  - added focused tests in `worldwake-sim` and `worldwake-ai` for live-view visibility, snapshot persistence, and planning-state preservation
- Deviations from original plan:
  - the reassessment found that `worldwake-core` conversation-memory infrastructure was already implemented, so this ticket did not introduce core storage or retention helpers
  - the ticket remained plumbing-only; it did not change `emit_social_candidates`, tell affordance filtering, or `commit_tell` mutation semantics, which still belong to later E15c tickets
- Verification results:
  - `cargo test -p worldwake-sim per_agent_belief_view::tests::runtime_view_exposes_retention_aware_told_belief_memory_and_recipient_status` passed
  - `cargo test -p worldwake-ai planning_snapshot::tests::build_snapshot_preserves_actor_told_belief_memory` passed
  - `cargo test -p worldwake-ai planning_state::tests::planning_state_preserves_actor_conversation_memory_from_snapshot` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
