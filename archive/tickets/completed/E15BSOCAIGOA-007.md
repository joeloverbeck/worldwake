# E15BSOCAIGOA-007: Initial golden social coverage for autonomous Tell, rumor relay, and discovery correction

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Tests first; production changes only if a real end-to-end gap is exposed
**Deps**: E15, `specs/E15b-social-ai-goals.md`

## Problem

The golden E2E suite still has no social-domain file, even though the codebase already implements the E15b social AI path:

- `GoalKind::ShareBelief`
- `PlannerOpKind::Tell`
- `emit_social_candidates()`
- `UtilityProfile.social_weight`
- golden harness helpers for explicit belief seeding and profile overrides

That makes the old Tier 1 / Tier 2 split outdated. The remaining gap is not feature implementation in isolation; it is end-to-end proof that the current architecture actually produces stable social behavior under the real AI loop and that stale information gets corrected through local observation.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-ai/tests/golden_social.rs` does not exist yet. Confirmed.
2. The ticket can no longer assume "Tell mechanics only" or "no autonomous social AI". `ShareBelief`, `PlannerOpKind::Tell`, `emit_social_candidates()`, ranking, and `social_weight` already exist in `worldwake-ai` / `worldwake-core`.
3. The ticket can no longer assume harness work is pending. `crates/worldwake-ai/tests/golden_harness/mod.rs` already provides `seed_belief`, `agent_belief_about`, `agent_belief_count`, `set_agent_tell_profile`, and `set_agent_perception_profile`.
4. Manual `InputQueue` injection still exists and remains useful for tightly controlled provenance/relay assertions, but it should not be the default shape of golden coverage now that autonomous social planning exists.
5. Discovery events already exist, and systems-level E15 integration tests already cover low-level Tell propagation, witnessed telling, and replay of Tell-plus-discovery scenarios. During implementation, the actual missing discovery path turned out to be resource-source quantity contradiction coverage rather than a generic missing discovery event type.
6. `acceptance_fidelity: Permille(0)` still represents a valid hard rejection case and is appropriate for golden regression coverage.

## Architecture Reassessment

The original ticket assumed a lower-level, manually injected test tranche because the autonomous social layer did not yet exist. That is no longer the best test architecture.

The cleaner long-term shape is:

1. Golden tests should validate emergent behavior through the real AI loop when the architecture already supports it.
2. Manual queued Tell requests should be reserved for cases where the test is specifically about provenance and relay semantics, not because the AI layer is missing.
3. Social-domain golden coverage should complement, not duplicate, `crates/worldwake-systems/tests/e15_information_integration.rs`.

This ticket therefore becomes the first social golden slice for the current architecture, not a legacy Tier 1-only shim.

## Scope

Create `crates/worldwake-ai/tests/golden_social.rs` with 4 end-to-end tests:

### T1: `golden_agent_autonomously_tells_colocated_peer`

- Setup: two co-located agents with low needs and Tell profiles. Speaker has a fresh direct belief about a remote food source; listener does not.
- Step simulation under the normal AI loop.
- Assert: speaker generates and executes a Tell plan; listener receives a reported belief; listener subsequently replans toward the newly learned food source.
- Checks: determinism replay and conservation.

### T2: `golden_rumor_chain_degrades_through_three_agents`

- Setup: Alice, Bob, and Carol are co-located. Alice has a direct belief about a subject. Bob and Carol begin without that belief.
- Step simulation under the normal AI loop with Tell profiles that allow autonomous relay.
- Assert: Bob receives `Report { chain_len: 1 }`; Carol receives a degraded relayed belief with longer chain depth and lower confidence than Bob's.
- Checks: determinism.

### T3: `golden_stale_belief_travel_reobserve_replan`

- Setup: hungry agent starts away from Orchard Farm with a stale belief that the orchard still has apples, while the authoritative source is depleted.
- Step simulation under the real AI loop.
- Assert: the agent travels because of the stale belief, local observation emits a concrete resource-source mismatch discovery, the stale belief is corrected by direct observation, and the agent abandons the invalid harvest path instead of continuing to act on omniscient world state.
- Checks: determinism and conservation.

### T4: `golden_skeptical_listener_rejects_told_belief`

- Setup: two co-located agents, low needs, speaker has a fresh belief, listener has `acceptance_fidelity: Permille(0)`.
- Step simulation with the real AI loop for speaker behavior.
- Assert: Tell may execute, but listener belief state does not change and no follow-up travel toward the rumored target occurs.
- Checks: determinism.

## Files To Touch

- `crates/worldwake-ai/tests/golden_social.rs` (new)
- `reports/golden-e2e-coverage-analysis.md` (update after implementation so the report matches reality)

## Out Of Scope

- Re-implementing social AI goals that already exist
- Reworking the golden harness unless a real missing helper is exposed
- New backward-compatibility wrappers or alias paths
- Full social-suite completion through T13
- `GoalKind::InvestigateMismatch` or a separate investigation architecture

## Acceptance Criteria

### Tests That Must Pass

1. `golden_agent_autonomously_tells_colocated_peer`
2. `golden_rumor_chain_degrades_through_three_agents`
3. `golden_stale_belief_travel_reobserve_replan`
4. `golden_skeptical_listener_rejects_told_belief`
5. `cargo test -p worldwake-ai --test golden_social`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Golden social coverage reflects the architecture that actually exists now: autonomous social planning where available, manual queueing only where it provides sharper provenance assertions.
2. Agents still plan from beliefs, not authoritative world state.
3. Discovery remains local and evidence-driven.
4. No backward-compatibility shims are introduced.

## Implementation Notes

1. Reuse the existing golden harness helpers over creating a parallel social-specific harness.
2. If a test exposes architectural waste in current social planning, document it in the ticket outcome and only change production code if the failing test demonstrates a real correctness gap.
3. Keep the file focused on social-domain golden scenarios rather than duplicating lower-level systems integration coverage.
4. The social loop should not generate low-value same-place chatter about subjects the actor already believes are colocated; if a regression appears there, fix the planner/candidate architecture rather than encoding it as a tolerated behavior.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — 4 new golden E2E tests

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added `crates/worldwake-ai/tests/golden_social.rs` with four social golden scenarios covering autonomous Tell, rumor relay degradation, stale-belief correction after travel and re-observation, and listener rejection via `acceptance_fidelity: Permille(0)`.
  - Completed the missing planner wiring for social goals by teaching `GoalKind::ShareBelief` to build `TellActionPayload` overrides and to treat Tell as the terminal progress barrier for the share-belief plan.
  - Extended discovery mismatch handling to compare believed versus observed `ResourceSource` quantities, introducing `MismatchKind::ResourceSourceDiscrepancy` so stale harvest beliefs are contradicted by local evidence.
  - Tightened social candidate generation to skip relaying subjects already believed to be colocated with the speaker, which removed low-value chatter and restored unrelated care golden behavior.
  - Updated `reports/golden-e2e-coverage-analysis.md` so the report reflects the new social-domain golden coverage.
- Deviations from original plan:
  - The original ticket expected mostly test-only work plus optional manual queued Tell coverage. Actual implementation showed the architecture was only partially wired: autonomous Tell ranking/candidate generation existed, but planner payload construction and terminal Tell handling were incomplete, so production fixes were required.
  - The rumor-chain scenario is covered through the real autonomous loop rather than strict queued Tell injection because the autonomous path now exists and is the better architectural contract to lock down.
  - The stale-belief scenario exposed a concrete `ResourceSource` contradiction gap rather than an inventory-lot discrepancy path.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_social`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
