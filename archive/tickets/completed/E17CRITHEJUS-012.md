# E17CRITHEJUS-012: Golden test — theft creates EntityMissing violation for owner

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E17CRITHEJUS-006 (steal action), E17CRITHEJUS-007 (investigate SuspectedTheft), E17CRITHEJUS-010 (theft candidates), E17CRITHEJUS-015 (typed social-evidence detail)

## Problem

No golden test currently proves the full theft-to-owner-discovery chain under the live architecture: `steal` commits, the owner later arrives lawfully, local belief-vs-observation mismatch records `EntityMissing`, and investigate commits the owner-specific `SuspectedTheft` aftermath. Focused runtime coverage already proves the lower-layer investigate behavior; the missing proof is the cross-system chain that starts with theft and ends with owner-local discovery.

## Assumption Reassessment (2026-03-26)

1. Golden tests live in `crates/worldwake-ai/tests/golden_*.rs`, and `cargo test -p worldwake-ai -- --list` confirms `golden_emergent` is the existing cross-system binary for violation/discovery scenarios. The live structural precedent is Scenario 36 in `crates/worldwake-ai/tests/golden_emergent.rs`: `golden_entity_missing_triggers_investigation`.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` provides the required harness surface: `GoldenHarness::step_once()`, `enable_action_tracing()`, driver tracing, belief seeding helpers, and access to the scheduler input queue for explicit human travel requests. The ticket does not need a new harness abstraction.
3. Lower-layer coverage already exists for the owner/non-owner investigation split. `crates/worldwake-systems/src/investigate_actions.rs` already proves that owner investigation records both `SocialObservationDetail::SuspectedTheft` and a new `ViolationKind::SuspectedTheft`, while non-owner investigation does not. The missing coverage is specifically the theft-to-discovery golden, not the owner-specific investigate commit logic itself.
4. The exact shared abstraction boundary under audit is: `transport_actions::commit_steal` moves possession without ownership transfer; `candidate_generation` records `ViolationKind::EntityMissing` from local mismatch; `GoalKind::InvestigateViolation` binds the unresolved violation id; `investigate_actions::commit_investigate` upgrades owner investigation into `SocialObservationDetail::SuspectedTheft` plus `ViolationKind::SuspectedTheft`.
5. The intended invariant is not "owners autonomously decide to revisit a stash." The invariant is: once the owner physically returns to the expected place through a lawful movement path, discovery happens through local violated expectation, not omniscience. The golden should therefore isolate the discovery branch from unrelated travel-goal motivation.
6. Live goal/action surface under test: thief side uses `GoalKind::StealItem { target_item }` and the `steal` transport action; owner side uses `GoalKind::InvestigateViolation { violation_id, place }` and the `investigate` generic action. The scenario depends on the owner having a prior belief that the owned lot was at the theft place, because `candidate_generation` only emits `EntityMissing` when `last_known_place == current_place`.
7. This is a mixed-layer golden, not a candidate-generation-only ticket. Full action registries are required because the scenario crosses transport, perception, AI planning, and investigate execution.
8. Ordering contract: the required ordering is mixed-layer and explicit. The proof boundary is: `steal` must commit before the owner's arrival-triggered mismatch; the owner's `investigate` commit must occur after the `EntityMissing` record exists. We should prove action lifecycle ordering with action traces, not by overfitting to incidental tick counts.
9. Information-path audit: the same high-level fact ("my owned item is gone") can lawfully reach the owner through multiple paths in the live architecture, including witness Tell and direct possession observation. This ticket intentionally proves the canonical owner-local path from `EntityMissing` to `SuspectedTheft` and excludes witness/accusation paths; those remain separate E17 tickets.
10. Isolation choice: use one thief AI with `TheftDispositionProfile`, one owner with `ViolationDispositionProfile`, and one owned lot. The scenario must also move the thief and stolen lot away from the stash before the owner returns. Otherwise the lot remains locally observable at the stash via the thief's possession, and the intended `EntityMissing` / `suspect: None` branch is no longer the live architecture. The owner's return travel should be issued as an explicit human request so the scenario proves physical locality without depending on unrelated planner motivations for why the owner revisits the stash.
11. Mismatch from the original ticket: the live `ViolationKind::SuspectedTheft` payload is not `{ missing_entity, expected_place, suspect }`. It is `SuspectedTheft { theft: TheftFacts, suspect }`, and `SocialObservationDetail::SuspectedTheft` uses the same typed `TheftFacts` payload. The ticket scope and assertions must use the live typed contract.
12. Timing math: `steal` duration comes from `TheftDispositionProfile.steal_duration_ticks`; investigate duration comes from `ViolationDispositionProfile.investigation_duration_ticks`; prototype-world travel between `VillageSquare` and both `GeneralStore` and `CommonHouse` is 1 tick per `docs/golden-e2e-testing.md`. The scenario should use only adjacent places so theft, thief departure, and owner return all remain explicit but the budget stays narrow.
13. Adjacent contradictions found during reassessment: none that must be pulled into this ticket. Witness-based suspect identification, accusation, and punishment remain separate follow-on work and should stay out of scope here.
14. Scope correction: keep `Engine Changes: None`. The right fix is a new golden scenario plus replay companion in `golden_emergent.rs`, not a production-code change, because the production boundary already exists and has focused coverage.

## Architecture Check

1. A golden at the existing `golden_emergent.rs` boundary is cleaner than adding more focused runtime assertions because the missing risk is integration drift across transport, perception, candidate generation, and investigate commit. Lower layers already prove the owner-specific commit semantics.
2. The clean isolation choice is explicit human-requested travel for the thief's departure and the owner's return. That preserves Principle 7 locality, avoids unlawful teleport/setup mutation, and prevents the scenario from drifting into the different branch where the owner simply sees the stolen lot still present in the thief's hands.
3. No backwards-compatibility aliasing or duplicate evidence paths are introduced. The test should assert the live `TheftFacts`-based contract directly.

## Verification Layers

1. `steal` commits and transfers possession without changing ownership -> action trace + authoritative world state
2. Owner only discovers loss after lawful return to the expected place and after the thief has already departed with the lot -> authoritative world state + decision trace/candidate record at the arrival tick
3. `EntityMissing` is recorded before investigation resolves -> authoritative `ViolationMemory`
4. Owner investigate commit resolves the original violation id -> action trace + authoritative `ViolationMemory`
5. Investigate aftermath records typed `SocialObservationDetail::SuspectedTheft { theft: TheftFacts, suspect: None }` -> authoritative `AgentBeliefStore`
6. Owner-specific crime upgrade persists as `ViolationKind::SuspectedTheft { theft: TheftFacts, suspect: None }` -> authoritative `ViolationMemory`
7. Conservation of the stolen lot remains intact across theft and discovery -> authoritative conservation helper
8. Deterministic replay of the whole chain -> replay companion test

## What to Change

### 1. New golden test scenario

Add a new scenario to `crates/worldwake-ai/tests/golden_emergent.rs` adjacent to the existing violation scenarios.

**Setup**:
- Theft place: `VillageSquare`
- Owner start: adjacent place `GeneralStore`
- Thief retreat destination after theft: adjacent place `CommonHouse`
- Agent A (thief AI) at `VillageSquare` with `TheftDispositionProfile`
- Agent B (owner) at `GeneralStore` with `ViolationDispositionProfile`
- One item lot at `VillageSquare`, owned by B, initially unpossessed
- Owner belief store seeded with the lot's last known place at `VillageSquare`
- Both agents have `PerceptionProfile`

**Execution**:
- Step until A commits `steal`
- After theft commits, issue a human `travel` request moving A from `VillageSquare` to `CommonHouse` so the stolen lot is no longer locally observable at the stash
- Then issue a human `travel` request moving B from `GeneralStore` to `VillageSquare`
- After arrival, return B to AI control and step until `EntityMissing` is recorded and `investigate` commits

**Assertions**:
- After A's steal commits: `possessor_of(item) == A`, `owner_of(item) == B`
- After B arrives at `VillageSquare` and A is already away at `CommonHouse`: B's `ViolationMemory` contains `EntityMissing { entity: item, expected_place: VillageSquare }`
- After B investigates: B's `ViolationMemory` contains `SuspectedTheft { theft: TheftFacts { missing_entity: item, expected_place: VillageSquare, commodity, quantity }, suspect: None }`
- B's `AgentBeliefStore` contains `SocialObservationDetail::SuspectedTheft` with the same `TheftFacts`
- Conservation: `verify_live_lot_conservation()` passes throughout

### 2. Deterministic replay companion

Add the standard replay companion for the scenario in the same binary.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)

## Out of Scope

- Witness-to-accusation chain (E17CRITHEJUS-013)
- Fine/Exile outcomes (E17CRITHEJUS-013)
- Guard response (E19)
- Multiple concurrent thefts
- Theft at occupied locations (witness perception)
- Owner seeing the thief still carrying the stolen lot at the stash
- Why the owner chose to revisit the stash in the first place
- AI planner search debugging beyond the proof surfaces named above

## Acceptance Criteria

### Tests That Must Pass

1. `golden_theft_leads_owner_to_local_suspected_theft_discovery` (or final exact name): full pipeline from `steal` -> owner return -> `EntityMissing` -> `investigate` -> `SuspectedTheft`
2. `<same name>_replays_deterministically`: deterministic replay companion
3. Conservation invariant holds throughout the scenario
4. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. P15: Owner discovers theft through violated expectation (EntityMissing), not through omniscience
2. P7: Discovery requires owner to physically visit the theft location
3. P3: Evidence is concrete world state (`TheftFacts` in `SocialObservationDetail::SuspectedTheft` and `ViolationKind::SuspectedTheft`), not an abstract score
4. P12: Owner's belief store records the suspicion; world truth and subjective discovery remain separate
5. P14: `suspect` remains `None` without witness testimony or possession observation
6. Conservation maintained across all ticks

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the theft-to-owner-discovery scenario and replay companion because that exact cross-system proof is missing today

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent -- --exact golden_theft_leads_owner_to_local_suspected_theft_discovery`
2. `cargo test -p worldwake-ai --test golden_emergent -- --exact golden_theft_leads_owner_to_local_suspected_theft_discovery_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-26
- What changed:
  - Added `golden_theft_leads_owner_to_local_suspected_theft_discovery` and its deterministic replay companion to `crates/worldwake-ai/tests/golden_emergent.rs`.
  - Corrected the ticket scope to match the live architecture: the missing proof was the cross-system golden, not the owner-specific investigate commit logic, which already had focused runtime coverage.
  - Refreshed golden inventory/docs metadata via `python3 scripts/golden_inventory.py --write --check-docs`.
- Deviations from original plan:
  - The implemented scenario explicitly moves the thief and stolen lot away from the stash before the owner returns. This was required by the live architecture; otherwise the owner would still be able to observe the lot locally at the stash and the intended `EntityMissing` / `suspect: None` branch would not be the contract under test.
  - The scenario stayed in `golden_emergent.rs`; no new `golden_crime.rs` binary was added.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent -- --exact golden_theft_leads_owner_to_local_suspected_theft_discovery` ✅
  - `cargo test -p worldwake-ai --test golden_emergent -- --exact golden_theft_leads_owner_to_local_suspected_theft_discovery_replays_deterministically` ✅
  - `cargo test -p worldwake-ai --test golden_emergent` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `python3 scripts/golden_inventory.py --write --check-docs` ✅
  - `cargo clippy --workspace` ✅
