# E16DPOLPLAN-021: Political office facts remain local until belief update

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: `specs/E16d-political-planning-and-golden-coverage.md`, `docs/golden-e2e-testing.md`

## Problem

The office-planning stack already proves that agents can claim a remote office once they believe it is vacant, but it does not yet prove the negative half of the locality contract: a distant agent with no office belief must not generate political goals or start political travel until an explicit belief update occurs.

## Assumption Reassessment (2026-03-18)

1. The exact belief gate is `emit_political_candidates` in `crates/worldwake-ai/src/candidate_generation.rs`, which iterates `ctx.view.known_entity_beliefs(ctx.agent)` and only treats known office entities as political subjects. This confirms the architectural claim must be made at the AI candidate-generation layer, not at action validation.
2. Existing focused/unit coverage already exercises the positive and some negative political candidate paths:
   - `candidate_generation::tests::political_candidates_emit_claim_and_support_for_visible_vacant_office`
   - `candidate_generation::tests::political_candidates_require_visible_vacancy_and_skip_existing_declaration`
   - `candidate_generation::tests::political_candidates_skip_force_law_offices`
   - `candidate_generation::tests::political_candidates_record_ineligible_actor_and_support_target_omissions`
   None of these directly prove that an unknown remote office emits no political candidates until the office belief is acquired.
3. Existing runtime/golden coverage already proves adjacent parts of the architecture:
   - `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning` proves decision-trace assertions are already the intended runtime surface for political candidate absence.
   - `golden_travel_to_distant_jurisdiction_for_claim` in `crates/worldwake-ai/tests/golden_offices.rs` proves the positive half of the contract once the remote office belief already exists.
   - `golden_social` scenarios prove generic rumor/report/tell propagation, but there is no office-specific golden showing that political ambition stays inert before a remote office belief exists.
4. The original ticket had three mismatches that needed correction before implementation:
   - it cited nonexistent coverage docs paths in practice only incidentally correctly; the canonical docs are `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md`
   - it proposed event-log absence as the primary negative assertion, but `docs/golden-e2e-testing.md` requires decision traces for candidate-generation absence/suppression invariants
   - it treated the gap as golden-only; in reality the clean scope is one focused candidate-generation regression test plus one golden E2E scenario

## Architecture Check

1. The clean architecture is to strengthen coverage around the existing belief-gated design, not to add any new authority path, alias, or political special case. The current architecture is already correct: political ambition must emerge from believed office state, not global truth.
2. The robust test shape is two-layered:
   - focused candidate-generation coverage for the pure belief gate
   - golden coverage for the full runtime contract that absence persists until a belief update, after which ordinary planning/travel/claim behavior appears
3. The golden should use decision traces for the negative AI contract and authoritative/action-state checks for the positive execution contract. That is cleaner and less brittle than inferring absence from missing event-log records alone.

## What to Change

### 1. Add focused candidate-generation coverage

- Add a unit test in `crates/worldwake-ai/src/candidate_generation.rs` proving that a politically ambitious agent with no belief about a vacant office emits neither `ClaimOffice` nor `SupportCandidateForOffice`.
- Then seed the office belief and prove the same setup emits `ClaimOffice`.
- Keep this as a pure AI-layer test with no runtime scheduler dependency.

### 2. Add a golden office locality scenario

- Add a new scenario to `crates/worldwake-ai/tests/golden_offices.rs`.
- Setup:
  - vacant support-law office at `VillageSquare`
  - politically ambitious agent at `BanditCamp`
  - no initial belief about the office
- Phase 1:
  - run several ticks with decision tracing enabled
  - assert the agent never generates `ClaimOffice` or `SupportCandidateForOffice` for that office
  - assert the agent does not start political travel toward the office jurisdiction
- Phase 2:
  - inject an explicit office belief update using harness belief seeding
  - run additional ticks
  - assert decision traces now include `ClaimOffice`
  - assert the agent reaches the jurisdiction and becomes office holder after normal succession timing
- Prefer explicit belief seeding over introducing a new office-specific social propagation path in this ticket. Generic tell/rumor mechanics are already covered elsewhere; this ticket is about the political locality invariant, not social transport implementation.

### 3. Update golden coverage docs

- Update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` to record the new office-locality scenario and distinguish it from the already-covered “remote office belief already exists” travel scenario.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Changes to production political/planning code
- New office-specific tell, rumor, or notice mechanics
- Perception-system refactors
- Reworking existing social propagation architecture already covered in `golden_social.rs`

## Acceptance Criteria

### Tests That Must Pass

1. New focused candidate-generation regression proving unknown offices emit no political candidates until belief exists
2. `golden_information_locality_for_political_facts` proves no remote political goal generation before belief update and normal claim behavior after update
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Principle 7 / locality: political behavior cannot begin from remote office facts that have not reached the agent's belief state
2. `emit_political_candidates` remains belief-gated via `known_entity_beliefs`, not world-state-gated
3. Travel toward an office claim emerges only after the belief update exposes the office as a candidate subject

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — add a focused regression proving unknown vacant offices are invisible to political candidate generation until belief seeding occurs
2. `crates/worldwake-ai/tests/golden_offices.rs` — add a decision-trace-backed golden proving remote political inactivity before office knowledge and normal travel/claim behavior after the belief update
3. `docs/golden-e2e-coverage.md` — record the new office-locality scenario in the coverage matrix
4. `docs/golden-e2e-scenarios.md` — add the scenario narrative and distinguish it from the existing remote-office-travel scenario

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::political_candidates_require_known_office_belief_for_generation`
2. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`

## Outcome

- Completion date: 2026-03-18
- What actually changed:
  - added focused candidate-generation regression coverage proving unknown offices do not emit political candidates until the office belief exists
  - added a golden office scenario plus deterministic replay coverage proving a remote claimant stays politically inert before an explicit reported office belief update, then claims the office normally after the update
  - updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` to record the new scenario and renumber later office scenarios
- Deviations from original plan:
  - kept the work test-only, which is the cleaner architecture than introducing any office-specific propagation mechanism
  - strengthened the plan beyond the original ticket by adding focused/unit coverage and a deterministic replay companion
  - used decision-trace assertions for the negative AI contract instead of relying primarily on event-log absence
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation::tests::political_candidates_require_known_office_belief_for_generation` ✅
  - `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `scripts/verify.sh` ✅
