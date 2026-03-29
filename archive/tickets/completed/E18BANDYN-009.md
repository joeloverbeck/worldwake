# E18BANDYN-009: Golden test T22 — bandit camp destruction chain

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` golden coverage and any minimal production fixes required if reassessment proves a real T22 architecture gap
**Deps**: E18BANDYN-003, archive/tickets/completed/E18BANDYN-004.md, archive/tickets/completed/E18BANDYN-005.md, archive/tickets/completed/E18BANDYN-006.md, archive/tickets/completed/E18BANDYN-007.md, archive/tickets/completed/E18BANDYN-008.md, archive/tickets/completed/E18BANDYN-010.md, archive/tickets/completed/E18BANDYN-011.md

## Problem

The brainstorming/spec intent for T22 is a golden end-to-end proof that the shipped E18 architecture composes lawfully across systems: active bandit camp under attack -> surviving members disperse -> rally-point belief drives regroup travel -> `EstablishCamp` can recreate faction camp presence elsewhere -> stale danger beliefs decay -> at least one downstream travel/trade decision changes because the belief substrate changed. This ticket is the capstone verification for the delivered E18 architecture and the Phase 4 acceptance gate for the bandit dynamics epic.

## Assumption Reassessment (2026-03-30)

Shared abstraction boundary under audit: authoritative bandit-camp lifecycle plus belief-side rally doctrine plus planner-local route-threat reasoning, as exercised through the golden harness in `crates/worldwake-ai/tests/golden_*.rs`.

1. The live authoritative camp contract is not the one described in older E18 text. [`BanditCamp`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) now stores `faction`, `supplies`, and `empty_since_tick`; [`BanditFactionPolicy`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) lives on the faction entity and stores `min_regroup_count`, `establishment_duration_ticks`, `abandonment_grace_ticks`, `flee_wound_threshold`, and `rally_place`.
2. The live regroup information path is already implemented and must be treated as canonical: active camp + colocated faction member -> passive perception projects [`InstitutionalClaim::FactionRallyPoint`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) -> `GoalBeliefView` exposes the belief -> `generate_candidates()` emits [`GoalKind::RegroupWithFaction`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) -> planner resolves travel from belief. The golden must prove this path, not bypass it with direct policy reads.
3. The live predation/combat contract intentionally does not have a separate authoritative `raid` action. [`archive/tickets/E18BANDYN-003.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/E18BANDYN-003.md) already corrected that architecture: bandit semantics live at `GoalKind::RaidTarget`, while the authoritative action remains the shared `attack` path. Any ticket text still requiring a dedicated `Raid` action is stale and must be removed.
4. The live route-danger contract is a planner-local derived heuristic over belief memory and social conflict observations, not a stored patrol-route system. [`route_threat_estimate()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/route_threat.rs) derives threat from believed hostile activity / wounds and `WitnessedConflict` social observations with normal confidence decay. The golden should prove the downstream route/trade consequence through planner choice or authoritative travel outcome, not by asserting on a nonexistent stored "former patrol route" subsystem.
5. The golden harness already provides the required verification surfaces: authoritative world state, decision traces, action traces, request-resolution traces, and the prototype world topology in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs). The ticket's old invented place graph should be corrected to the live prototype topology unless a custom world becomes strictly necessary.
6. `cargo test -p worldwake-ai -- --list` confirms there is currently no `golden_t22_*` or other bandit-camp-destruction golden in the live inventory. This is a real golden coverage gap, not a duplicate of existing E18 focused tests.
7. The current codebase already has focused coverage for abandonment (`crates/worldwake-systems/src/bandit_camp.rs`), camp establishment (`crates/worldwake-systems/src/bandit_camp_actions.rs`), rally-point belief transport (`crates/worldwake-systems/src/perception.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/goal_model.rs`), and route threat decay (`crates/worldwake-ai/src/route_threat.rs`). The remaining missing proof surface is the cross-system chain.
8. The old ticket overstated certainty that this was "test only." Reassessment shows the desired T22 scenario may still expose a real production gap in the downstream merchant-response leg or in the exact camp-destruction-to-regroup chain. If that happens, the right response is a minimal architectural fix proven by the golden, not a brittle test workaround.
9. The live acceptance target should be phrased in terms of lawful architecture, not a hardcoded narrative. The golden must prove:
   - old camp presence is removed after lawful abandonment,
   - survivors who hold the rally belief can regroup via travel and re-establish camp,
   - stale danger evidence decays under the normal belief-confidence model,
   - at least one downstream travel or trade decision changes because the agent's belief-backed route assessment changed.
10. The strongest downstream economic proof surface may be a merchant restock/travel decision rather than a bespoke "route reopened" scalar. The ticket should not force a weaker or more fictional assertion when the live planner already exposes stronger decision-trace or authoritative-travel evidence.

## Architecture Check

1. A golden E2E test remains the correct primary proof surface because T22 is explicitly about emergent composition across subsystems, and the missing coverage is no longer at any single focused layer.
2. The clean architecture is to make the golden consume only the live canonical paths:
   - authoritative camp state via `BanditCamp`
   - regroup doctrine via institutional belief transport
   - travel safety via belief-derived route threat
   - downstream merchant response via ordinary planner/travel/trade behavior
3. This is more beneficial than the old ticket narrative because it proves the architecture that actually shipped instead of forcing obsolete abstractions such as a dedicated `Raid` action, place-backed policy, or stored route-danger state.
4. If the scenario reveals a real missing architectural link, fix that link directly and delete the gap. Do not add a special-case golden helper, temporary alias path, or bandit-only bypass just to make T22 pass.
5. No backwards-compatibility shims. Prefer one new golden file plus minimal touched focused tests only where the bug/edge case needs stronger local proof.

## Verification Layers

1. Camp abandonment after attack -> authoritative world state (`BanditCamp` removed from the original place after grace-period expiry)
2. Flee / regroup travel lifecycle -> action trace (travel starts/commits for surviving bandits)
3. Rally doctrine consumption -> decision trace (`RegroupWithFaction` candidate/selection present only for survivors with the institutional rally belief)
4. New camp establishment -> authoritative world state plus action trace (`establish_camp` starts/commits and a new `BanditCamp` appears at the regroup place)
5. Danger memory decay -> focused value query or decision trace using the live belief-backed route-threat surface, not a stored route scalar
6. Downstream merchant response -> decision trace and/or authoritative travel outcome proving changed route/restock behavior under the decayed belief state
7. Durable aftermath -> authoritative world state (dead bandits, abandoned supplies, conserved lots)
8. Causal depth across systems -> concise event-log / trace chain only as supporting proof after the stronger per-layer assertions above

## What to Change

### 1. Create the T22 golden file

Add a new golden file:

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`

The scenario should use the live prototype world unless reassessment during implementation proves a custom world is strictly required. Prefer the existing route chain around `VillageSquare`, `NorthCrossroads`, `ForestPath`, and `BanditCamp`, plus one lawful regroup destination such as `OrchardFarm` or another reachable prototype place.

### 2. Build the scenario around the canonical E18 contracts

The scenario setup should include:

- one active bandit faction with `BanditFactionPolicy` and an initial `BanditCamp`
- at least three bandit members so regroup / re-establishment is meaningful
- supplies in or on behalf of the camp so aftermath and camp re-establishment remain concrete
- at least one merchant whose normal work requires travel or restock decisions affected by route-danger beliefs
- an external attack trigger that is lawful in the current architecture

The attack trigger does not need a special bandit-only combat path. It may be:

- AI hostility that lawfully produces combat in-place, or
- human-driven `RequestAction` inputs if that is the narrowest clean way to seed the exogenous attack

### 3. Prove the scenario in assertable phases

The final golden should prove, in order:

- initial camp + rally doctrine + merchant danger-sensitive setup
- external attack causes bandit deaths/wounds and at least one surviving member leaves the original camp
- only survivors with the lawful rally belief select regroup travel
- the original camp becomes abandoned after the grace period while concrete aftermath remains
- enough surviving members regroup to commit `establish_camp` at the new place
- stale danger evidence decays without new conflict reports
- a merchant later plans or travels differently because the old danger evidence is no longer strong enough to dominate route choice

### 4. Add any minimal focused tests required by the discovered gap

If the T22 work exposes a real invariant or edge case that is not well proved locally, add the narrowest focused tests needed in the owning file rather than bloating the golden. Examples:

- a focused route-threat / travel-choice regression if the merchant-response leg exposes a planner bug
- a focused camp/regroup regression if abandonment or establishment fails for a lawful survivor case
- a focused belief/provenance regression if the rally-doctrine path is missing evidence needed for debugging

## Files to Touch

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (new)
- any minimal focused test files required by a real gap exposed during implementation

## Out of Scope

- speculative architecture changes unrelated to the lawful T22 chain
- Guard response to bandit reports (E19)
- introducing a dedicated authoritative `Raid` action or any other duplicate combat path
- adding a stored route-danger / patrol-route state model
- Performance optimization of the golden test
- CLI integration for bandit camp display

## Acceptance Criteria

### Tests That Must Pass

1. A new T22 golden passes end-to-end under a deterministic seed.
2. The old camp is removed only through the lawful abandonment path; aftermath remains concrete (dead bodies and/or supplies persist lawfully).
3. Surviving bandits retain injuries, inventory, and faction membership; no reset/respawn path is introduced.
4. Regrouping requires ordinary travel and uses the rally-point institutional belief path rather than direct faction-policy reads.
5. Survivors lacking the rally belief do not select `RegroupWithFaction`.
6. `establish_camp` requires the live faction-policy threshold and commits through the ordinary action lifecycle.
7. Stale danger evidence decays under the live belief-confidence model when no new conflict refreshes it.
8. At least one merchant later plans or travels differently because the danger belief substrate changed.
9. If the scenario exposes a real production gap, the implementation fixes that gap cleanly and the new focused regression proves it.
10. `cargo test -p worldwake-ai` passes.
11. `cargo clippy --workspace` passes.
12. `cargo build --workspace` passes.

### Invariants

1. FND-1: the scenario may use exogenous setup/input, but the tested consequences must emerge from normal systems rather than scripted test-side mutations after kickoff.
2. FND-4: dead bandits stay dead, supplies are conserved, and aftermath remains concrete.
3. FND-7 / FND-12: regroup knowledge reaches agents through beliefs, not direct reads of authoritative faction policy.
4. FND-17: bandits, attackers, and merchants use the same action/request/planning framework.
5. FND-25: danger is belief-derived, not stored as authoritative route state.
6. Determinism: the golden replays identically under the same seed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`
Rationale: proves the full cross-system T22 chain on the live E18 architecture rather than on outdated abstractions.
2. Any focused regression tests added during implementation
Rationale: prove the narrow invariant exposed by the golden at the strongest lower layer, so the golden stays an integration proof instead of absorbing all debugging detail.

### Commands

1. `cargo test -p worldwake-ai golden_t22_bandit_camp_destruction`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
5. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - Added [`golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs) to prove the live E18 chain end to end: camp attack aftermath, rally-belief regrouping, lawful camp re-establishment, stale threat decay, and downstream merchant-route change.
  - Added explicit AI support for rally-driven camp re-establishment through `GoalKind::EstablishBanditCamp { faction }`, declaration/dispatch wiring, ranking, feasibility, and root-goal synthesis into `establish_camp`.
  - Corrected `establish_camp` to use concrete locally controlled edible supplies at the place rather than requiring one survivor to personally carry all food.
  - Hardened same-place same-faction camp reuse by replacing an undersized stash container instead of rejecting lawful re-establishment.
  - Strengthened focused regressions for candidate generation, goal synthesis, ranking, and `establish_camp` validation/commit behavior.
- Deviations from original plan:
  - Reassessment showed the issue was not purely a golden-test gap. The regroup-to-re-establishment chain exposed a real production architecture hole, so the completion included minimal production fixes instead of a test-only workaround.
  - The original draft assumed older E18 architecture such as a dedicated raid action and place-backed policy. The shipped proof follows the live architecture instead: shared `attack`, faction-scoped `BanditFactionPolicy`, institutional rally beliefs, and belief-derived route threat.
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation::tests::establish_bandit_camp_ -- --nocapture`
  - `cargo test -p worldwake-ai goal_model::tests::grounded_goal_synthesizes_establish_camp_root_targets_from_goal_place -- --nocapture`
  - `cargo test -p worldwake-ai golden_t22_bandit_camp_destruction -- --nocapture`
  - `cargo test -p worldwake-systems bandit_camp_actions::tests::establish_camp_ -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo build --workspace`
  - `python3 scripts/golden_inventory.py --write --check-docs`
