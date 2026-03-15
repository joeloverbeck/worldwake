# E15BSOCAIGOA-004: Implement emit_social_candidates() for ShareBelief goals

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation in ai crate plus narrow belief-view surface update in sim crate
**Deps**: None beyond current `main` state

## Problem

The AI candidate generation has 4 emitter groups (needs, production, enterprise, combat) but none for social goals. `GoalKind::ShareBelief`, `PlannerOpKind::Tell`, and baseline ShareBelief ranking semantics already exist, but without `emit_social_candidates()` agents still never generate autonomous ShareBelief goals and Tell remains manual-only from the AI side.

## Assumption Reassessment (2026-03-15)

1. `generate_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` still calls only 4 emitter groups in sequence. Confirmed no social emitter.
2. `GoalKind::ShareBelief { listener, subject }` already exists in `crates/worldwake-core/src/goal.rs`.
3. `GoalKindTag::ShareBelief`, Tell planner semantics, and baseline ShareBelief ranking hooks already exist in `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/planner_ops.rs`, and `crates/worldwake-ai/src/ranking.rs`.
4. `TellProfile` exists in `crates/worldwake-core/src/belief.rs` with fields: `max_tell_candidates: u8`, `max_relay_chain_len: u8`, `acceptance_fidelity: Permille`.
5. `AgentBeliefStore` stores `known_entities: BTreeMap<EntityId, BelievedEntityState>`, and `BelievedEntityState` carries `source` plus `observed_tick`, which is enough to derive relay eligibility and deterministic recency ordering.
6. `emit_candidate()` already handles GoalKey deduplication and `BlockedIntentMemory` filtering. Reuse it.
7. `GoalBeliefView` currently does not expose `known_entity_beliefs()` or `tell_profile()`. Those subjective reads exist only on `RuntimeBeliefView`, but `candidate_generation.rs` is intentionally guarded to compile against `GoalBeliefView`, not `RuntimeBeliefView`.
8. Tell affordance payload generation in `crates/worldwake-systems/src/tell_actions.rs` already defines the canonical subject-selection behavior: filter by relay depth, sort by `observed_tick` descending then `subject` ascending, and truncate to `max_tell_candidates`. Social candidate generation should mirror that behavior rather than inventing a second policy.
9. `max_tell_candidates` currently caps candidate subjects per speaker-listener affordance, not the number of listeners. The original ticket assumption that it bounded listeners was incorrect.

## Architecture Check

1. Keep goal-reading modules on the narrow `GoalBeliefView` boundary. Broadening `candidate_generation.rs` to `RuntimeBeliefView` would violate an explicit architecture guard test and unnecessarily widen the AI read surface.
2. The clean fix is to extend `GoalBeliefView` with the two subjective reads autonomous social generation actually needs: `known_entity_beliefs()` and `tell_profile()`. That keeps social candidate generation belief-only and local without granting queue/reservation/runtime-only access.
3. Social candidate enumeration should mirror Tell affordance subject ordering and relay filtering so goal generation and action selection stay causally aligned.
4. Candidate generation remains pure derived computation (Principle 3): no new stored state, no caches promoted to truth.
5. Candidate count is locality-bounded by co-located live listeners times relayable subjects kept by `max_tell_candidates`; do not add a second listener cap unless a later spec introduces one explicitly.

## Note

This ticket is now the primary implementation ticket for the remaining autonomous-social-behavior gap in candidate generation. The ShareBelief goal/planner/ranking scaffolding already exists; what is missing is the autonomous candidate emission and the narrow belief-view support required to compute it cleanly.

## What to Change

### 1. Extend the narrow AI belief boundary

In `crates/worldwake-sim/src/belief_view.rs`:

```rust
pub trait GoalBeliefView {
    // existing methods...
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        let _ = agent;
        None
    }
}
```

Also forward these methods from `impl_goal_belief_view!` so `PerAgentBeliefView` and other runtime-backed views satisfy the narrow goal boundary without widening callers to `RuntimeBeliefView`.

### 2. Add emit_social_candidates() in candidate generation

In `crates/worldwake-ai/src/candidate_generation.rs`, add a social emitter that:

1. Returns early if the speaker lacks `TellProfile` or lacks an effective place.
2. Enumerates co-located live agent listeners from `entities_at(place)`, excluding self.
3. Enumerates relayable subjects from `known_entity_beliefs(agent)` using the same relay-depth filter and deterministic ordering Tell affordances already use.
4. Truncates subjects by `TellProfile.max_tell_candidates`.
5. Emits `GoalKind::ShareBelief { listener, subject }` for each listener-subject pair via `emit_candidate()`.

### 3. Wire into generate_candidates()

Call `emit_social_candidates(candidates, ctx)` as the 5th emitter group after combat candidates.

### 4. Relay depth semantics

Only emit ShareBelief for beliefs where the current chain length is `<= max_relay_chain_len`, matching Tell action validation and affordance generation. For `DirectObservation` and `Inference`, chain length is effectively `0`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- GoalKind::ShareBelief definition
- GoalKindTag::ShareBelief / planner Tell semantics
- social_weight in UtilityProfile
- Ranking/priority improvements for ShareBelief motive scoring (tracked separately in `E15BSOCAIGOA-005`)
- GoalKind::InvestigateMismatch (future spec)
- Tell action handler changes (none needed, E15 handler is complete)
- Tell affordance payload generation behavior beyond keeping candidate emission aligned with it
- Belief store capacity or retention changes
- Golden E2E social suites

## Acceptance Criteria

### Tests That Must Pass

1. Agent with `TellProfile` and relayable beliefs generates `ShareBelief` candidates for live co-located listeners.
2. Agent without `TellProfile` generates zero `ShareBelief` candidates.
3. Beliefs exceeding `max_relay_chain_len` are filtered out.
4. `DirectObservation` beliefs (`chain_len = 0`) pass the relay-depth filter.
5. `Report { chain_len: 2 }` passes when `max_relay_chain_len = 3` and fails when `max_relay_chain_len = 1`.
6. Subject selection is recency-ordered and truncated by `max_tell_candidates`, matching Tell affordances.
7. Blocked ShareBelief intents are excluded via the existing `emit_candidate()` mechanism.
8. Dead or non-agent co-located entities are not selected as listeners.
9. Existing suite: `cargo test -p worldwake-ai` — no regressions.

### Invariants

1. Candidate generation is a pure derived computation — no new state stored (Principle 3).
2. Agent queries only own beliefs and co-located locally visible agents (Principle 7).
3. Goal-reading AI modules continue to depend on `GoalBeliefView`, not `RuntimeBeliefView`.
4. No `HashMap` or `HashSet` used in candidate enumeration (determinism).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (module tests) — unit tests for social candidate emission and blocked-intent filtering.
2. `crates/worldwake-ai/src/agent_tick.rs` or existing architecture guard tests — keep the `GoalBeliefView` boundary assertion passing after the narrow trait extension.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Extended `GoalBeliefView` with `known_entity_beliefs()` and `tell_profile()` so social candidate generation could stay on the narrow AI belief boundary.
  - Added `emit_social_candidates()` to `crates/worldwake-ai/src/candidate_generation.rs` and wired it into `generate_candidates()`.
  - Implemented deterministic listener and subject selection using local co-location plus relay-depth and recency filtering aligned with Tell affordance behavior.
  - Added focused candidate-generation tests for ShareBelief emission, relay filtering, listener filtering, missing TellProfile, and blocked-intent suppression.
- Deviations from original plan:
  - Did not broaden candidate generation to `RuntimeBeliefView`; instead preserved the existing architecture guard and extended the narrow `GoalBeliefView` surface with only the missing subjective reads.
  - Corrected the original assumption that `max_tell_candidates` caps listeners. In current behavior it caps relayable subjects per speaker-listener affordance, and the implementation now follows that reality.
  - No golden social E2E tests were added here; this ticket stayed scoped to candidate generation and boundary support.
- Verification results:
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
