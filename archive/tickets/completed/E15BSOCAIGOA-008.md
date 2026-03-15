# E15BSOCAIGOA-008: Golden social follow-up coverage (bystander locality, entity-missing discovery)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None expected — tests and report only unless coverage exposes a real bug
**Deps**: E15, E15BSOCAIGOA-001, E15BSOCAIGOA-007

## Problem

The original ticket assumed `golden_social.rs` did not yet exist, that T5–T7 were all still missing, and that this slice was only appending three tests to a freshly created social golden file. That is no longer true on the current branch.

`golden_social.rs` already exists and already covers autonomous Tell, rumor degradation, skeptical listener rejection, and the stale-belief travel/re-observe/replan path that this ticket originally called T6. The remaining real coverage gaps are narrower:

1. Golden end-to-end proof that a bystander witnesses the social act without receiving the transmitted belief content.
2. Golden end-to-end proof that `EntityMissing` discovery is emitted when an agent locally re-observes an expected place and the expected entity is absent.
3. Coverage/report synchronization, because `reports/golden-e2e-coverage-analysis.md` still describes the social suite as 4 tests and still lists the stale-belief scenario as backlog.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-ai/tests/golden_social.rs` already exists and already contains T1–T4 plus the stale-belief travel/re-observe/replan scenario this ticket originally described as T6.
2. Deliverable 1 architecture from `specs/E15b-social-ai-goals.md` is already partially landed on this branch: `GoalKind::ShareBelief`, `PlannerOpKind::Tell`, `social_weight`, social candidate generation, and ranking are present. This ticket is therefore a follow-up golden coverage slice, not a pre-Deliverable-1 placeholder.
3. Harness support assumed missing by the original ticket already exists: explicit belief seeding, tell/perception profile overrides, and belief accessors are present in `crates/worldwake-ai/tests/golden_harness/mod.rs`.
4. Bystander social observation exists and is already covered by focused integration tests in `crates/worldwake-systems/tests/e15_information_integration.rs`, but it is not yet proven through the full AI golden loop.
5. `EntityMissing` discovery exists and is emitted by passive observation, but the current architecture emits a discovery event without mutating the stale belief into a new authoritative location. The original ticket's expectation that the belief would be "updated (no longer at Village Square)" does not match current code.

## Architecture Check

1. No new production abstraction is justified here. The highest-value work is to strengthen proof of existing locality and discovery behavior without adding compatibility shims or duplicate harness layers.
2. The stale-belief travel/re-observe/replan scenario is already covered and should not be reimplemented under a second name.
3. The strongest architecture-facing correction is for entity-missing semantics: under the current belief model, "I expected them here and they were absent" is represented as a discovery event, not as omniscient relocation. A clean future architecture would likely add explicit negative/contradictory observation state rather than silently clearing or teleporting `last_known_place`.
4. Coverage-report synchronization is part of the real scope, because the current report is materially out of date relative to the code.

## What to Change

### 1. Add the missing golden coverage to `golden_social.rs`

**T5: `golden_bystander_sees_telling_but_gets_no_belief`**
- Setup: 3 agents (speaker, listener, bystander) co-located. The speaker has a fresh belief about a remote orchard resource and enough social motive to autonomously Tell. The bystander is co-located and can observe social events.
- Step simulation through the real AI loop.
- Assert: the bystander records `WitnessedTelling`, receives no belief about the remote subject, and does not leave the local place as if the remote belief had been transferred.
- Checks: determinism, information locality (Principle 7), bystander-belief isolation.

**T6 status on current branch**
- Already implemented as `golden_stale_belief_travel_reobserve_replan`.
- Do not duplicate or rename it in this ticket.

**T7: `golden_entity_missing_discovery_does_not_teleport_belief`**
- Setup: Agent A starts at the place where it believes Agent B should be. Agent B is actually elsewhere. A has a stale belief that B is at the current place.
- Step simulation through passive local observation.
- Assert: an `EntityMissing` discovery event fires for A's violated expectation. The test should prove local mismatch discovery without overclaiming a belief mutation that the current architecture does not implement.
- Checks: determinism, violated-expectation discovery, no telepathic location update.

### 2. Update the golden coverage report

Update `reports/golden-e2e-coverage-analysis.md` so it matches the current branch:

1. `golden_social.rs` currently has more than 4 tests once this ticket lands.
2. The stale-belief travel/re-observe/replan scenario is no longer backlog.
3. The social slice now proves bystander locality and entity-missing discovery in the golden loop.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify)
- `reports/golden-e2e-coverage-analysis.md` (modify)

## Out of Scope

- Reimplementing T1–T4 or the existing stale-belief scenario
- T8–T13 follow-on social coverage
- Inventing a backward-compatibility belief alias such as "entity missing means clear last_known_place"
- Broad planner or belief-model redesign
- Harness rewrites unless a real testability gap is discovered while adding the two missing golden proofs

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bystander_sees_telling_but_gets_no_belief` — bystander gets social observation but NOT the told belief content
2. Existing `golden_stale_belief_travel_reobserve_replan` remains green and continues to prove stale-belief travel plus correction
3. `golden_entity_missing_discovery_does_not_teleport_belief` — `EntityMissing` discovery fires from local violated expectation without claiming a new location for the missing entity
4. New scenarios verify determinism
5. `golden_social` passes end to end with the expanded coverage set
6. `reports/golden-e2e-coverage-analysis.md` no longer claims the social suite has only 4 tests and no longer lists the stale-belief scenario as backlog

### Invariants

1. Bystanders receive social observations but NOT belief content (Principle 7 — information locality)
2. Agents plan from beliefs, never from world state (even when beliefs are stale)
3. `EntityMissing` is caused by local violated expectation, not by omniscient absence detection
4. The current architecture does not silently infer a replacement location from absence alone
5. Conservation remains verified in the existing stale-belief resource scenario

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — add the missing bystander-locality and entity-missing golden scenarios; keep the existing stale-belief scenario as the authoritative T6 coverage
2. `reports/golden-e2e-coverage-analysis.md` — update counts, social coverage text, and backlog status

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed on 2026-03-15.
- Corrected the ticket first to match the real branch state:
  - `golden_social.rs` already existed.
  - T1–T4 and the stale-belief travel/re-observe/replan scenario were already implemented.
  - Social AI goal plumbing was already landed, so this was not a pre-Deliverable-1 placeholder anymore.
  - The original T7 expectation was wrong for the current architecture: `EntityMissing` emits violated-expectation evidence but does not invent a new location for the missing entity.
- What actually changed:
  - Added `golden_bystander_sees_telling_but_gets_no_belief` to prove bystander locality through the full golden AI loop.
  - Added `golden_entity_missing_discovery_does_not_teleport_belief` to prove local `EntityMissing` discovery without overclaiming belief mutation semantics the current model does not support.
  - Kept the existing stale-belief travel/re-observe/replan test as the authoritative T6 coverage rather than duplicating it.
  - Updated `reports/golden-e2e-coverage-analysis.md` to reflect the expanded social suite, add `ShareBelief`/`Social` domain coverage, and remove the stale golden backlog item.
- Architectural note:
  - The current `EntityMissing` behavior is coherent with the existing belief model, but it is not the ideal long-term representation. The cleaner future architecture is to add explicit negative or contradictory observation state rather than silently clearing `last_known_place` or teleporting belief to a guessed location.
- Verification:
  - `cargo test -p worldwake-ai --test golden_social`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
