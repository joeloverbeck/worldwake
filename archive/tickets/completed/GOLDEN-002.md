# GOLDEN-002: Belief Isolation Backlog Reassessment (Scenario 10)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected; ticket scope is documentation/report cleanup unless the reassessment finds a real test gap
**Deps**: None

## Problem

`reports/golden-e2e-coverage-analysis.md` still lists Scenario 10 as a missing golden test. That backlog item assumes the codebase lacks end-to-end proof for belief isolation and therefore needs a new `golden_perception.rs` scenario about "unseen theft."

Those assumptions need to be rechecked against the current tree before adding another test layer. If the invariant is already covered elsewhere in a cleaner way, the ticket should remove the stale backlog item instead of duplicating coverage.

## Assumption Reassessment (2026-03-14)

1. The current tree already contains runtime-level belief-isolation tests in `crates/worldwake-ai/src/agent_tick.rs`:
   - `same_place_perception_seeds_seller_belief_for_runtime_candidates`
   - `unseen_seller_relocation_preserves_stale_acquisition_belief`
   - `unseen_death_does_not_create_corpse_reaction_without_reobservation`
2. `archive/tickets/completed/E14PERBEL-007.md` already documented that the old "unseen theft" framing was not an honest E14 target and that generic unseen state-change isolation was the correct architectural proof.
3. `crates/worldwake-ai/tests/golden_harness/mod.rs` already has belief-seeding helpers (`seed_actor_beliefs`, `seed_actor_local_beliefs`, `seed_actor_world_beliefs`). The original ticket assumption that a new harness helper might be needed is stale.
4. `crates/worldwake-ai/tests/golden_perception.rs` still does not exist, but that absence is no longer sufficient reason to add it for Scenario 10.
5. The golden report's scope is specifically `crates/worldwake-ai/tests/golden_*.rs`, so the report can still be stale even when the repository already has stronger non-golden coverage for the underlying invariant.
6. `tickets/GOLDEN-003.md` currently assumes GOLDEN-002 will create `golden_perception.rs`; that cross-ticket dependency becomes incorrect if Scenario 10 is removed from the golden backlog rather than implemented as a new golden file.

## Architecture Check

1. Adding a new golden scenario for Scenario 10 is not cleaner than the current architecture. The repo already has targeted runtime-integration coverage at the actual planner boundary (`PerAgentBeliefView` -> runtime candidate generation), which is a better fit for this invariant than a second end-to-end harness layer.
2. Repeating the same contract in `golden_perception.rs` would increase overlap between test layers without adding a stronger architectural guarantee.
3. The right fix is to align backlog/reporting with current reality:
   - keep the existing runtime tests as the proof for generic belief isolation
   - remove Scenario 10 from the golden backlog
   - keep future perception-golden work focused on scenarios that still lack coverage, such as Scenario 11 if it remains valuable after its own reassessment
4. No backward-compatibility shims, aliases, or duplicate test abstractions should be introduced just to preserve the old ticket plan.

## What to Change

### 1. Correct the ticket scope before any implementation work

Rewrite this ticket around backlog cleanup, not new golden test creation.

### 2. Update `reports/golden-e2e-coverage-analysis.md`

- Remove Scenario 10 from the "Missing Scenarios" backlog.
- Add a short removal note explaining that the invariant is already proven by focused runtime tests in `crates/worldwake-ai/src/agent_tick.rs`.
- Update the pending backlog summary and recommended order accordingly.

### 3. Keep code changes out unless the reassessment finds a real uncovered edge case

If the existing runtime tests already pass and still prove the contract, do not add `golden_perception.rs` or modify the harness.

### 4. Maintain ticket cross-reference consistency

Adjust any directly affected ticket assumptions that still claim GOLDEN-002 will create `golden_perception.rs`.

## Files to Touch

- `tickets/GOLDEN-002.md` (this reassessment)
- `reports/golden-e2e-coverage-analysis.md` (remove stale backlog item, update summary)
- `tickets/GOLDEN-003.md` (only if needed to keep direct cross-ticket references honest)

## Out of Scope

- New production-code changes in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai/src/`
- New `golden_perception.rs` coverage for Scenario 10 unless the current runtime tests prove insufficient
- Reframing Scenario 10 back into theft/crime-specific behavior
- Expanding E14 architecture beyond the current `PerAgentBeliefView` boundary

## Acceptance Criteria

### Tests That Must Pass

1. Existing runtime belief-isolation tests in `crates/worldwake-ai/src/agent_tick.rs` pass.
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

### Invariants

1. The ticket and golden report no longer claim Scenario 10 is missing if the current repository already proves the underlying invariant elsewhere.
2. Belief isolation remains validated through the existing runtime tests:
   - unknown entities stay hidden until observed
   - stale non-self beliefs do not silently refresh from authoritative state
   - unseen death does not create corpse reactions without re-observation
3. No duplicate golden layer is added just to preserve a stale plan.

## Test Plan

### New/Modified Tests

1. None expected. Existing runtime tests are the subject of this reassessment.

### Commands

1. `cargo test -p worldwake-ai same_place_perception_seeds_seller_belief_for_runtime_candidates`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - corrected the ticket scope from "add a new golden perception scenario" to "remove a stale golden backlog item after reassessing current coverage"
  - updated `reports/golden-e2e-coverage-analysis.md` to remove Scenario 10 from the golden backlog and document why it no longer belongs there
  - updated `tickets/GOLDEN-003.md` so it no longer depends on GOLDEN-002 creating `golden_perception.rs`
- Deviations from original plan:
  - did not add `crates/worldwake-ai/tests/golden_perception.rs`
  - did not modify the golden harness
  - did not add new code tests because the current runtime tests in `crates/worldwake-ai/src/agent_tick.rs` already prove the relevant belief-isolation invariant more directly than a new golden file would
- Verification results:
  - `cargo test -p worldwake-ai same_place_perception_seeds_seller_belief_for_runtime_candidates`
  - `cargo test -p worldwake-ai unseen_seller_relocation_preserves_stale_acquisition_belief`
  - `cargo test -p worldwake-ai unseen_death_does_not_create_corpse_reaction_without_reobservation`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
