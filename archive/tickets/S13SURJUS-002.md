# S13SURJUS-002: Isolate a retained accusation case in `survival-justice`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario authoring, accusation scenario proof, local office-holder belief fallback for justice candidate admission
**Deps**: `archive/tickets/S13SURJUS-001.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`survival-justice` still does not retain an accusation-ready theft branch even after the owner-transfer promotion fix from `S13SURJUS-001`. The merchant can now lawfully preserve theft suspicion through investigation, but the authored run still produces broad self-care and stock-movement `EntityMissing` churn, so the intended theft case never becomes the canonical `GoalKind::Accuse` / `GoalKind::PunishAccused` seam.

## Assumption Reassessment (2026-04-24)

1. `archive/tickets/S13SURJUS-001.md` landed the lower-layer transport fix in `investigate_actions`, so this follow-up should start from the live baseline where post-transfer subjective theft evidence can become `SuspectedTheft`.
2. The owned boundary here is the retained row-13 accusation/punishment scenario seam: authored `survival-justice` setup, the candidate/ranking surface that chooses which theft case matters, and the scenario-backed golden proof that `accuse` / punishment actually commit.
3. The first live contradiction is still scenario-level retention, not missing action definitions. `accuse`, `fine`, and `punishment` substrates already exist in code and lower-layer tests.
4. Focused golden diagnostics first showed broad local stock/water churn drowning out the theft case. After tightening the scenario, the authored theft branch now reaches direct theft evidence at tick 2, a retained `ViolationKind::SuspectedTheft` case at tick 13, and an `accuse` commit at tick 14.
5. The row-owned invariant is not "any justice action exists." It is that the authored theft branch becomes the retained accusation case in the same survival run, then reaches a truthful punishment commit.
6. Live reassessment disproved the original "scenario-only" premise. Once the theft case was isolated truthfully, `GoalKind::Accuse` still stayed absent because `PerAgentBeliefView::believed_office_holder()` exposed only remembered institutional beliefs, so a co-located authored office holder still looked `Unknown` to `emit_accusation_candidates()`.
7. The truthful current slice is therefore mixed: scenario isolation plus a narrow production fix at the local political-belief boundary. With that fix, `GoalKind::Accuse` becomes the retained row-13 accusation seam and records an accusation case in the crime register.
8. `GoalKind::PunishAccused` remains false after the accusation lands. In the live authored run, the thief has already consumed the stolen apple quantity by the time punishment selection would need a lawful fine target, and the scenario schema does not currently expose a clean exile fallback path. That belongs to a follow-up ticket.
9. Adjacent contradiction: row 13 search/report is still blocked by a separate stale `ask_about_person` seam and remains explicitly owned by `archive/tickets/S13SURJUS-003.md`, not this ticket.
10. Mismatch + correction: the original ticket overclaimed a full accusation-plus-punishment landing. The live complete slice for this pass is the retained accusation seam; punishment now has its own follow-up owner in `archive/tickets/S13SURJUS-004.md`.

## Architecture Check

1. Isolating the authored theft case in scenario data is cleaner than helper-only seeding, and fixing the office-holder gate in `PerAgentBeliefView` is cleaner than adding special-case accusation scaffolding in candidate generation. Together they make the local authored office-holder substrate visible through the normal runtime belief view.
2. No compatibility shim is acceptable here. The end state should be one truthful accusation-ready theft branch, not a special-case golden-only evidence lane.

## Verification Layers

1. The local authored office holder remains visible to justice candidate generation without a pre-seeded institutional belief -> focused `PerAgentBeliefView` unit coverage
2. The intended theft case survives investigation as the retained accusation candidate -> decision trace / candidate-generation diagnostics in `golden_survival_justice.rs`
3. The retained accusation branch commits `accuse` for the authored theft case -> action trace in `golden_survival_justice.rs`
4. The crime register records the accusation for that same theft case -> authoritative `RecordData` state in `golden_survival_justice.rs`
5. The merchant still satisfies the authored survival envelope while the accusation branch wins -> `golden_survival_justice.rs`

## What to Change

### 1. Reassess and tighten scenario isolation

Reduce or partition unrelated local churn in `scenarios/survival-justice.ron` so the intended theft case is the retained accusation owner rather than one of many equally lawful missing-item cases.

### 2. Prove the accusation seam truthfully

Expand `crates/worldwake-ai/tests/golden_survival_justice.rs` from the earlier investigation-only seam to a truthful accusation assertion only after the authored scenario and local office-holder belief surface actually support it.

## Files to Touch

- `scenarios/survival-justice.ron` (modify)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `docs/scenario-roadmap.md` (modify only if row wording/status changes truthfully)
- `archive/tickets/S13SURJUS-002.md` (modify)
- `archive/tickets/S13SURJUS-004.md` (new)

## Out of Scope

- Row-13 punishment follow-through after accusation (`archive/tickets/S13SURJUS-004.md`)
- Search/report retained-seam work from row 13 (`archive/tickets/S13SURJUS-003.md`)
- Golden-only helper seeding that bypasses authored scenario state

## Acceptance Criteria

### Tests That Must Pass

1. A row-13 golden in `crates/worldwake-ai/tests/golden_survival_justice.rs` that proves `accuse` commits for the intended theft case
2. The same golden proves the crime register records the accusation for that case
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
4. `cargo test -p worldwake-sim per_agent_belief_view::tests::believed_office_holder_falls_back_to_local_authoritative_office_relation -- --exact`

### Invariants

1. The retained accusation case must come from the authored theft branch, not unrelated generic missing-item churn.
2. Local co-located office-holder state must be visible to the accuser through the normal runtime belief view even without pre-seeded institutional memory.
3. Row 13 remains `In Progress` until punishment and search/report are proven at the scenario level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — add truthful accusation coverage once the scenario and local office-holder belief surface support it
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — prove local co-located office-holder fallback without widening remote-office knowledge

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
4. `cargo test -p worldwake-sim per_agent_belief_view::tests::believed_office_holder_falls_back_to_local_authoritative_office_relation -- --exact`
5. `cargo test -p worldwake-sim per_agent_belief_view::tests::believed_office_holder_keeps_remote_office_unknown_without_belief -- --exact`

## Outcome

Completed on 2026-04-24.

- Tightened `survival-justice` so the authored theft branch now reaches immediate theft evidence, a retained `SuspectedTheft` violation, and a truthful `accuse` commit under the same survival envelope.
- Fixed the remaining accusation gate by letting `PerAgentBeliefView` expose the holder of a co-located office when no institutional office-holder belief has been remembered yet.
- Expanded the row-13 golden from the earlier investigation-only seam to a truthful accusation seam backed by action trace, decision-trace timing, and crime-register state.

## Deviations

- Reassessment disproved the original punishment portion of this ticket after the accusation seam landed. The live authored run still does not emit a lawful punishment candidate once the accusation is recorded, so that broader seam moved to `archive/tickets/S13SURJUS-004.md`.
- Search/report remains separately owned by `archive/tickets/S13SURJUS-003.md`.

## Verification Result

- Passed `cargo test -p worldwake-sim per_agent_belief_view::tests::believed_office_holder_falls_back_to_local_authoritative_office_relation -- --exact`
- Passed `cargo test -p worldwake-sim per_agent_belief_view::tests::believed_office_holder_keeps_remote_office_unknown_without_belief -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
