# S34GENEPIACT-013: Retarget deliberate epistemic goldens to the fact-sensitive barrier contract

**Status**: SUPERSEDED BY S34GENEPIACT-010
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None expected beyond S34GENEPIACT-012 and ticket/spec correction work
**Deps**: [tickets/S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md), [tickets/S34GENEPIACT-012.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-012.md), [archive/tickets/completed/S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-011.md)

## Problem

The pending deliberate-epistemic golden coverage plan still assumes that stale entity-location and stale supply-source scenarios should always prove a committed `verify_belief` action. After reassessment against the foundations and live runtime behavior, that is no longer the right contract. Goldens should prove the canonical fact-sensitive barrier path, not force explicit epistemic actions where lawful arrival perception is already the right causal mechanism.

This ticket is now superseded because that retargeting work has been folded directly into [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md). Keep `S34GENEPIACT-012` as the architecture prerequisite and implement `S34GENEPIACT-010` against the corrected post-012 contract.

## Assumption Reassessment (2026-03-28)

1. The current pending coverage ticket [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md) is partially stale: it still frames stale entity-location and stale supply verification as generic `Travel -> VerifyBelief` golden obligations.
2. The live adjacent coverage already proves important neighboring behaviors:
   - `golden_stale_prerequisite_belief_discovery_replan` in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) proves stale-branch recovery for `GoalKind::RestockCommodity`
   - `goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload` proves the focused `AskWitness` barrier path
   - `goal_model::tests::search_restock_goal_returns_travel_then_verify_belief_barrier_for_remote_stale_source` currently proves the older duplicate-path assumption and should be revisited by S34GENEPIACT-012
3. The exact mixed-layer boundary under audit here is:
   - decision trace / focused planner proof for which epistemic barrier path was selected
   - action trace proof only for cases that still require explicit `AskWitness` or `VerifyBelief`
   - authoritative belief/violation aftermath
   - later decision-trace proof of continuation or replan
4. The same fact should not be given two equal proof contracts in golden tests. If arrival perception is canonical for a fact class after S34GENEPIACT-012, goldens must prove that path directly rather than continuing to demand action-trace proof of a `VerifyBelief` commit.
5. Scenario isolation must explicitly exclude lawful competing branches only when those branches are not part of the contract under test. Goldens should not suppress lawful arrival refresh just to force a deliberate action.
6. Adjacent contradiction classification:
   - required consequence of this ticket: retarget or replace the stale parts of S34GENEPIACT-010
   - future cleanup if needed: add new deliberate `VerifyBelief` goldens only after the world exposes a fact class that truly requires targeted inspection rather than arrival perception
7. Mismatch + correction: the clean post-012 golden surface should likely split into:
   - arrival-observable stale fact recovery goldens
   - `AskWitness` deliberate epistemic goldens
   - `VerifyBelief` goldens only for any surviving inspection-only fact class, if such a class exists after implementation

## Architecture Check

1. Retargeting the goldens is cleaner than preserving a now-wrong “every stale prerequisite must commit `verify_belief`” narrative. Goldens should validate the canonical architecture, not pin the engine to an obsolete one.
2. This keeps action-trace assertions for truly explicit actions and avoids using them as a universal proof surface for facts that should be learned through ordinary local observation.
3. No backwards-compatibility aliasing is acceptable in the test contract either. The golden inventory should converge on one canonical epistemic path per fact class.
4. Because `S34GENEPIACT-010` now carries this corrected contract directly, keeping this ticket active would duplicate scope and weaken implementation ordering clarity.

## Verification Layers

1. barrier path selection for a stale prerequisite -> decision trace or focused planner/runtime coverage
2. lawful arrival refresh for arrival-observable facts -> authoritative belief-state checks and scenario-level continuation/replan assertions
3. explicit `AskWitness` commit identity -> action trace
4. explicit `VerifyBelief` commit identity -> action trace only if a surviving inspection-only fact class exists after S34GENEPIACT-012
5. deterministic replay -> replay companion tests

## What to Change

### 1. Correct or replace the pending golden-coverage plan

Either update [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md) in place or supersede it during implementation so the golden plan matches the fact-sensitive barrier contract.

### 2. Keep arrival-observable stale-fact goldens explicit about their real contract

For stale entity-location and stale supply-source scenarios, goldens should prove:

- originating goal selection
- barrier path to lawful arrival
- arrival refresh through local perception
- continuation or replan after refreshed belief / contradiction

without requiring a committed `verify_belief` unless that fact class still survives as inspection-only after S34GENEPIACT-012.

### 3. Keep deliberate epistemic goldens where the action remains truly first-class

The strongest likely survivor is `AskWitness`, because conversation has its own duration, occupancy, source attribution, and downstream consequence independent of passive arrival perception. If `VerifyBelief` retains any fact class after S34GENEPIACT-012, add or keep goldens for that narrower class only.

## Files to Touch

- `tickets/S34GENEPIACT-010.md` (modify or supersede — correct stale golden assumptions)
- `crates/worldwake-ai/tests/golden_social.rs` and/or `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — align goldens with the fact-sensitive epistemic contract)
- `docs/generated/golden-scenario-map.md` or `docs/generated/golden-e2e-inventory.md` (regenerate if new goldens are added or renamed)

## Out of Scope

- changing the core planner/search architecture itself
- forcing explicit `VerifyBelief` commits by weakening lawful perception
- adding speculative `VerifyBelief` golden scenarios before the surviving fact class is known

## Acceptance Criteria

### Tests That Must Pass

1. Goldens prove arrival-observable stale-fact recovery without overclaiming `VerifyBelief` commits
2. Goldens prove explicit `AskWitness` prerequisite chains through action trace and aftermath
3. Any surviving `VerifyBelief` golden proves a truly inspection-only fact class rather than a duplicate arrival-perception case
4. Replay companions pass for each new or updated golden
5. `cargo test -p worldwake-ai`

### Invariants

1. Golden assertions match the canonical information path for the fact class under test.
2. Action traces are used only where an explicit action commit is itself part of the contract.
3. Scenario isolation does not suppress lawful competing branches merely to force a preferred outcome.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` — restate stale-source recovery around the canonical arrival-refresh contract.
2. `crates/worldwake-ai/tests/golden_social.rs` or another fitting `golden_*.rs` suite — keep or add a deliberate `AskWitness` chain with action-trace proof.
3. `None — if S34GENEPIACT-012 removes all surviving inspection-only `VerifyBelief` cases, this ticket should correct coverage and scenario docs rather than forcing a new `VerifyBelief` golden.`

### Commands

1. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact`
2. `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact`
3. `cargo test -p worldwake-ai`
