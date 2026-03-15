# E15BSOCAIGOA-004: Implement emit_social_candidates() for ShareBelief goals

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation in ai crate
**Deps**: E15BSOCAIGOA-001, E15BSOCAIGOA-003

## Problem

The AI candidate generation has 4 emitter groups (needs, production, enterprise, combat) but none for social goals. Without `emit_social_candidates()`, agents will never generate `GoalKind::ShareBelief` candidates, and Tell actions will remain manual-only.

## Assumption Reassessment (2026-03-15)

1. `generate_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` calls 4 emitter groups in sequence. Confirmed no social emitter.
2. `TellProfile` exists in `crates/worldwake-core/src/belief.rs` with fields: `max_tell_candidates: u8`, `max_relay_chain_len: u8`, `acceptance_fidelity: Permille`.
3. `AgentBeliefStore` has `known_entities: BTreeMap<EntityId, BelievedEntityState>` with `source` field on each entry containing `PerceptionSource` (DirectObservation, Report, Rumor, Inference).
4. `emit_candidate()` helper exists for deduplication via GoalKey — reuse it.
5. `BlockedIntentMemory` filtering is already applied in `emit_candidate()`.
6. The belief view provides co-located agent queries — need to verify exact API.

## Architecture Check

1. Follows existing emitter pattern: function takes `(candidates, ctx)` and pushes to the candidates vec.
2. Bounded by `max_tell_candidates * memory_capacity` (defaults: 3 * 12 = 36 max candidates) — no explosion risk.
3. Pure derived computation (Principle 3) — no state stored, all computed from beliefs + profiles.

## Note

This ticket is now the primary implementation ticket for the remaining autonomous-social-behavior gap. `ShareBelief` goal/planner plumbing already exists, but without a social candidate emitter agents still cannot decide to Tell on their own. Completing this ticket is what changes Tell from manual-only behavior into AI-generated behavior.

## What to Change

### 1. Add emit_social_candidates() function

In `crates/worldwake-ai/src/candidate_generation.rs`, add:

```rust
fn emit_social_candidates(candidates: &mut Vec<GroundedGoal>, ctx: &CandidateContext) {
    // 1. Check if agent has TellProfile component. If not, return early.
    // 2. Get agent's AgentBeliefStore beliefs.
    // 3. Get co-located alive agents from belief view.
    // 4. For each co-located agent (up to TellProfile.max_tell_candidates):
    //    - For each belief passing max_relay_chain_len filter:
    //      - Emit GoalKind::ShareBelief { listener, subject }
    // 5. BlockedIntentMemory filtering handled by emit_candidate() helper.
}
```

### 2. Wire into generate_candidates()

Call `emit_social_candidates(candidates, ctx)` as 5th emitter group after combat candidates.

### 3. Chain length filtering

Only emit ShareBelief for beliefs where the current chain_len < max_relay_chain_len. For DirectObservation, chain_len is effectively 0.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- GoalKind::ShareBelief definition (E15BSOCAIGOA-001 — prerequisite)
- social_weight in UtilityProfile (E15BSOCAIGOA-003 — prerequisite for ranking, not generation)
- Ranking/priority for ShareBelief (E15BSOCAIGOA-005)
- PlannerOpKind::Tell (E15BSOCAIGOA-002)
- GoalKind::InvestigateMismatch (future spec)
- Tell action handler changes (none needed, E15 handler is complete)
- Belief store capacity or retention changes

## Acceptance Criteria

### Tests That Must Pass

1. Agent with TellProfile and fresh DirectObservation beliefs generates ShareBelief candidates for co-located agents
2. Agent without TellProfile generates zero ShareBelief candidates
3. Beliefs exceeding max_relay_chain_len are filtered out (no candidate emitted)
4. DirectObservation beliefs (chain_len=0) always pass the chain length filter
5. Report{chain_len: 2} belief with max_relay_chain_len=3 passes filter; with max_relay_chain_len=1 does not
6. Candidate count bounded by max_tell_candidates (no more listeners than limit)
7. Blocked intents are excluded via BlockedIntentMemory (existing emit_candidate mechanism)
8. Dead agents (not alive in belief view) are not selected as listeners
9. Existing suite: `cargo test -p worldwake-ai` — no regressions

### Invariants

1. Candidate generation is a pure derived computation — no state stored (Principle 3)
2. Agent queries only own beliefs + co-located agents visible locally (Principle 7)
3. No `HashMap` or `HashSet` used in candidate enumeration (determinism)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (inline or module tests) — unit tests for emit_social_candidates covering all 8 acceptance criteria above

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
