# S09INDACTREEVA-005: Reassess finite defend golden coverage and archive delivered scope

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The original S09 bug was real: a wounded fighter could enter `ReduceDanger -> defend` and remain there forever when defend was indefinite. This ticket was written to add the missing golden proof that the deadlock was gone.

On reassessment, the ticket's original scope no longer matches the repository. The finite-defend architecture is already implemented, and the golden suite already contains coverage for both halves of the behavior. The remaining work is to correct the ticket so it reflects the current code and verification surfaces, then archive it instead of planning duplicate implementation.

## Assumption Reassessment (2026-03-20)

1. The foundational S09 engine work is already present, so `S09INDACTREEVA-004` is not an active dependency for this ticket anymore.
   - `CombatProfile.defend_stance_ticks` already exists in [crates/worldwake-core/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/combat.rs).
   - `DurationExpr::ActorDefendStance` already exists in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs).
   - `defend` already uses `DurationExpr::ActorDefendStance` in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs).
   - Planner duration estimation already consumes finite defend ticks in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs).
2. The ticket's claimed test gap is partially stale.
   - Golden combat coverage already includes `golden_reduce_danger_defensive_mitigation` and `golden_defend_replans_after_finite_stance_expires` in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs).
   - Focused lower-layer coverage already exists for finite defend duration and commit behavior in `worldwake-systems` and `worldwake-sim`, including the defend affordance/commit tests in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs).
3. The original ticket overstated the need for a brand-new dedicated golden test.
   - `golden_reduce_danger_defensive_mitigation` already proves the AI/belief layer selects `ReduceDanger` and enters a concrete mitigation path under attack.
   - `golden_defend_replans_after_finite_stance_expires` already proves the action-lifecycle boundary that mattered for the deadlock: finite defend commits, the defender re-enters planning, and later starts or commits another action.
4. The current architecture does not justify forcing one more brittle end-to-end test that requires defend to beat every other lawful mitigation branch in a fully emergent scenario.
   - In the current planner, `ReduceDanger` may lawfully resolve through defend or relocation depending on state.
   - Requiring a single golden scenario to prove both initial mitigation choice and post-defend replanning would overconstrain lawful behavior and make the suite less robust.
5. The original report reference is stale.
   - `reports/golden-e2e-testing.md` does not exist.
   - No `reports/golden-e2e*` files exist under [reports](/home/joeloverbeck/projects/worldwake/reports), so there is no report file to update for this ticket.
6. Real test names were verified from the current binary layout with `cargo test -p worldwake-ai -- --list`.
   - `golden_reduce_danger_defensive_mitigation`
   - `golden_defend_replans_after_finite_stance_expires`

## Architecture Check

1. The current split verification is cleaner than the original proposed rewrite.
   - AI/goal-selection proof belongs in the real golden mitigation scenario.
   - Finite action lifecycle and replanning proof belongs at the defend lifecycle boundary, where the existing golden test seeds the active defend state explicitly and avoids unrelated branch noise.
2. Replacing that split with a single stricter golden test would make the suite more fragile without improving the production architecture.
3. No backward-compatibility shims or alias paths are involved. The architecture is already on the clean path: finite profile-driven defend duration, normal commit, then normal replanning.

## Verification Layers

1. `ReduceDanger` candidate selection under live attack -> decision trace assertions in `golden_reduce_danger_defensive_mitigation`
2. Concrete mitigation path becomes active -> golden combat world/action observation in `golden_reduce_danger_defensive_mitigation`
3. Finite defend commits after a real finite duration -> action trace assertions in `golden_defend_replans_after_finite_stance_expires`
4. Defender re-enters the decision pipeline after defend resolves -> decision trace assertions in `golden_defend_replans_after_finite_stance_expires`
5. Finite defend duration is profile-driven in authoritative action semantics -> focused tests in `worldwake-sim` and `worldwake-systems`

## What To Change

1. Do not add new production code.
2. Do not add a duplicate golden test unless a real uncovered invariant is found.
3. Re-verify the existing coverage with targeted and broader suites.
4. Archive this ticket as already satisfied by the current architecture and test surfaces.

## Files To Touch

- `tickets/S09INDACTREEVA-005.md` (reassess and finalize before archival)

## Out Of Scope

- Any engine, planner, or action-definition changes
- Rewriting the combat golden suite to force defend over other lawful mitigation branches
- Creating a new report file solely to satisfy the stale `reports/golden-e2e-testing.md` reference

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_combat golden_reduce_danger_defensive_mitigation`
2. `cargo test -p worldwake-ai --test golden_combat golden_defend_replans_after_finite_stance_expires`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

### Invariants

1. Defend is finite and profile-driven; no indefinite action path remains.
2. After finite defend resolves, the agent can re-enter planning and continue with another lawful action.
3. Golden combat coverage remains split across the correct layers rather than collapsed into one brittle scenario.

## Test Plan

### New/Modified Tests

1. None. Reassessment concluded the current test architecture already covers the delivered invariant without needing duplicate golden coverage.

### Rationale

1. `golden_reduce_danger_defensive_mitigation` already covers the emergent AI selection side of the bug.
2. `golden_defend_replans_after_finite_stance_expires` already covers the finite defend lifecycle and replanning boundary directly.
3. Lower-layer finite-duration tests already cover the authoritative semantics beneath both golden scenarios.

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_reduce_danger_defensive_mitigation`
2. `cargo test -p worldwake-ai --test golden_combat golden_defend_replans_after_finite_stance_expires`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-20
- **What actually changed**:
  - Reassessed the ticket against the current S09 implementation and test suite.
  - Corrected the ticket's stale assumptions about missing finite-defend engine work and missing golden coverage.
  - Verified that the current architecture already proves the invariant through a split set of golden and focused tests.
- **Deviations from original plan**:
  - No new golden test was added.
  - No production code changed.
  - The stale `reports/golden-e2e-testing.md` reference was not acted on because no such report file exists in the repository.
- **Verification results**:
  - Targeted golden combat tests passed before archival reassessment.
  - Broader suite and lint verification completed at finalization.
