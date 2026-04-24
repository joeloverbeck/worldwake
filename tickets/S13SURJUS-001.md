# S13SURJUS-001: Isolate accusation-ready justice evidence in `survival-justice`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario authoring, AI candidate/trace verification, justice/investigation seam
**Deps**: `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`survival-justice` can now start from a lawful office-holder substrate, stage a real theft branch, and keep investigation alive under the survival envelope, but the row still cannot land truthfully. In the live 1440-tick scenario, the merchant accumulates broad `EntityMissing` churn from lawful local stock movement and never reaches a stable accusation-ready theft case, and the authored search/report branch is not yet selected as a retained seam either.

## Assumption Reassessment (2026-04-24)

1. Live roadmap row 13 in [docs/scenario-roadmap.md](/home/joeloverbeck/projects/worldwake/docs/scenario-roadmap.md:166) still owns `Justice / accusation + violation investigation + report / witness + search`, while row 12 explicitly left justice/search/report unlanded in the row 12 writeup.
2. The new `survival-justice` scenario and golden were reassessed against live code rather than old roadmap prose. The retained passing seam is `lawful office-holder substrate + staged theft branch + investigation survives under the same survival envelope`, proven by [scenarios/survival-justice.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-justice.ron:1) and [golden_survival_justice.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_justice.rs:1).
3. The shared abstraction boundary under audit is the theft-case transport path from perception/investigation into accusation planning: `worldwake-systems` theft/investigation/perception surfaces produce evidence, `worldwake-ai` candidate generation must synthesize `GoalKind::Accuse`, and the scenario must author the needed lawful substrate without test-only seeding.
4. The intended invariant is not merely "justice actions exist"; it is that the survival scenario reaches a truthful accusation-ready case for the theft branch rather than being satisfied by unrelated missing-entity investigation churn.
5. The live golden depends on `GoalKind::InvestigateViolation`, `GoalKind::Accuse`, `GoalKind::PunishAccused`, `GoalKind::SearchForMissing`, and `GoalKind::ReportFound`. Reassessment showed only `InvestigateViolation` is currently proven in the retained scenario seam. `Accuse`, `PunishAccused`, `SearchForMissing`, and `ReportFound` remain unproven in the roadmap-owned scenario.
6. The current failure is candidate-generation / retained-seam level, not a missing action definition. `investigate` commits in the scenario, but the merchant's live `ViolationMemory` fills with unrelated `EntityMissing` records instead of one theft-specific accusation-ready case.
7. Ordering claims are mixed-layer here. The missing proof is not action lifecycle ordering but causal evidence selection: which violation record and which witness evidence the AI carries into accusation planning.
8. No heuristic is being removed today. The ticket needs missing substrate and/or tighter authored isolation so the runtime can lawfully surface the intended theft case without reopening unrelated churn.
9. The first live failure boundary is before accusation start: the scenario never reaches an `accuse` commit because the evidence-selection/candidate-generation seam does not produce a retained accusation path for the intended theft case.
10. Scenario isolation is currently insufficient. The authored merchant still produces lawful stock/water/item motion that creates many unrelated local `EntityMissing` records, so the row-owned theft case is not isolated enough to become the canonical accusation seam.
11. Reassessment exposed two adjacent contradictions: `search/report_found` also failed to become a retained seam in the same scenario, and the scenario schema previously lacked an initial-office-holder surface plus a colocated `CrimeRegister`. The initial-holder / crime-register gap was fixed in the current pass; accusation/search remain for this follow-up.
12. Concrete evidence:
`cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_local_crime_response -- --ignored --test-threads=1` failed because no `accuse` commit appeared, while the debug output showed many unrelated `EntityMissing` records in merchant `ViolationMemory`.
`cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_search_and_investigation_substrate -- --ignored --test-threads=1` failed because `search_place` never became the retained branch even after raising `care_weight`.
13. Mismatch + correction: the truthful retained seam for this session is narrower than the row's full claim. The roadmap/golden should keep row 13 `In Progress`, not `Landed`, until accusation/punishment and search/report are each proven at the intended causal boundary.

## Architecture Check

1. Fixing the evidence-isolation / authored-substrate gap is cleaner than weakening the roadmap row or hard-coding test-only state in the golden. The scenario should author lawful inputs, and the AI/runtime should surface the theft case through the real pipeline.
2. No backwards-compatibility shims are needed. The end state should expose one truthful scenario-authoring path and one truthful accusation-ready information path rather than parallel helper-only seams.

## Verification Layers

1. Theft-case evidence becomes accusation-ready for the intended branch -> decision trace and candidate-generation proof for `GoalKind::Accuse`
2. Investigation still commits on the intended missing-lot case rather than unrelated churn -> action trace plus focused lower-layer investigation/runtime coverage
3. Crime register mutation for accusation/verdict -> authoritative `RecordData` state and/or event-log delta
4. Search/report branch becomes retained and writes the intended missing-person status -> action trace for `search_place` / `report_found` plus authoritative `OfficeRegister` state
5. Survival envelope remains intact while the new justice/search branch wins -> scenario-backed golden survival contract from authored `survival_health_contract`
6. If scenario traces still prove outcomes without enough provenance to explain the chosen violation/evidence path, open a traceability follow-up instead of broadening weaker downstream assertions

## What to Change

### 1. Isolate the accusation-ready theft case

Reduce or partition unrelated local `EntityMissing` churn in the scenario and/or runtime path so one theft case becomes the canonical accusation candidate. If new authored surfaces are required, they must be lawful scenario schema, not golden-only seeding.

### 2. Land the search/report seam truthfully

Reassess why `SearchForMissing` / `ReportFound` do not become the retained branch in `survival-justice`, then either fix the authored ranking/isolation math or split that seam into a narrower truthful owner if the row needs multiple follow-ups.

### 3. Restore truthful roadmap-owned proof

Expand [golden_survival_justice.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_justice.rs:1) from the current investigation-only partial seam to the full row-owned accusation and search/report proof only after the live branch actually supports it.

## Files to Touch

- `scenarios/survival-justice.ron` (modify)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, if the live issue is candidate synthesis rather than authored setup)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify, if the theft-case promotion boundary is wrong)
- `crates/worldwake-cli/src/scenario/types.rs` (modify, only if lawful scenario authoring still lacks the needed evidence substrate)
- `docs/scenario-roadmap.md` (modify when the row status moves again)

## Out of Scope

- Re-landing row 11 office claiming inside row 13
- Weakening the roadmap row to call row 13 landed without accusation/search proof
- Golden-only helper seeding that bypasses the scenario authoring contract

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_theft_investigation_substrate -- --ignored --test-threads=1`
2. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --test-threads=1`
3. A future expanded `golden_survival_justice` test that proves the full accusation/punishment and search/report seams

### Invariants

1. Row 13 is not marked `Landed` until the retained scenario proves accusation/punishment and search/report at the intended causal surface.
2. Any new scenario-authoring surface for justice evidence must be lawful repo-owned authored state, not a test-only bypass.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — retain the truthful partial seam now, then expand to accusation/search once the live contradiction is fixed
2. `crates/worldwake-cli/src/scenario/mod.rs` — keep unit coverage for authored office substrate such as initial holder and colocated crime register

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_theft_investigation_substrate -- --ignored --test-threads=1`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --test-threads=1`
4. `cargo test -p worldwake-cli test_spawn_office_creates_local_crime_register_for_office_issuer -- --exact`
