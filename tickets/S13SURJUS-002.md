# S13SURJUS-002: Isolate a retained accusation case in `survival-justice`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario authoring, accusation/fine scenario proof, possible ranking/isolation fallout
**Deps**: `archive/tickets/S13SURJUS-001.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`survival-justice` still does not retain an accusation-ready theft branch even after the owner-transfer promotion fix from `S13SURJUS-001`. The merchant can now lawfully preserve theft suspicion through investigation, but the authored run still produces broad self-care and stock-movement `EntityMissing` churn, so the intended theft case never becomes the canonical `GoalKind::Accuse` / `GoalKind::PunishAccused` seam.

## Assumption Reassessment (2026-04-24)

1. `archive/tickets/S13SURJUS-001.md` landed the lower-layer transport fix in `investigate_actions`, so this follow-up should start from the live baseline where post-transfer subjective theft evidence can become `SuspectedTheft`.
2. The owned boundary here is the retained row-13 accusation/punishment scenario seam: authored `survival-justice` setup, the candidate/ranking surface that chooses which theft case matters, and the scenario-backed golden proof that `accuse` / punishment actually commit.
3. The first live contradiction is still scenario-level retention, not missing action definitions. `accuse`, `fine`, and `punishment` substrates already exist in code and lower-layer tests.
4. Focused golden diagnostics on 2026-04-24 showed `Merchant Sera` repeatedly committing `investigate` while final `ViolationMemory` filled with unrelated `EntityMissing` records from lawful local stock/water churn; no `accuse` commit appeared.
5. The row-owned invariant is not "any justice action exists." It is that the authored theft branch becomes the retained accusation case in the same survival run, then reaches a truthful punishment commit.
6. Reassessment must prove whether the remaining blocker is purely authored isolation (needs/perception/inventory setup) or whether live ranking/candidate selection still suppresses the intended theft case after the scenario is cleaned up.
7. `GoalKind::Accuse` and `GoalKind::PunishAccused` are the relevant live goal families. Reassessment should inspect the actual unresolved `ViolationMemory` contents, direct `SocialObservationDetail::SuspectedTheft` support, and final candidate/ranking output before changing code.
8. Scenario isolation is currently weak because the merchant continues to author many locally observed, owned, consumable item paths. Any isolation fix must stay lawful and scenario-authored; it must not use golden-only seeding or bypass the real evidence path.
9. Adjacent contradiction: row 13 search/report is blocked by a separate stale `ask_about_person` seam and is explicitly owned by `tickets/S13SURJUS-003.md`, not this ticket.

## Architecture Check

1. Isolating the authored theft case is cleaner than papering over the scenario with helper seeding because row 13 is a roadmap-owned golden and must prove the real runtime branch.
2. No compatibility shim is acceptable here. The end state should be one truthful accusation-ready theft branch, not a special-case golden-only evidence lane.

## Verification Layers

1. The intended theft case survives investigation as the retained accusation candidate -> decision trace / candidate-generation diagnostics
2. The retained accusation branch commits `accuse` and then a punishment action for the same theft case -> action trace
3. The crime register records the accusation/verdict for that same theft case -> authoritative `RecordData` state
4. The merchant still satisfies the authored survival envelope while the accusation branch wins -> `golden_survival_justice.rs`

## What to Change

### 1. Reassess and tighten scenario isolation

Reduce or partition unrelated local churn in `scenarios/survival-justice.ron` so the intended theft case is the retained accusation owner rather than one of many equally lawful missing-item cases.

### 2. Prove the accusation/punishment seam truthfully

Expand `crates/worldwake-ai/tests/golden_survival_justice.rs` from the current investigation-only seam to a truthful accusation/punishment assertion only after the authored scenario actually supports it.

## Files to Touch

- `scenarios/survival-justice.ron` (modify)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify only if live candidate retention still fails after truthful scenario isolation)
- `docs/scenario-roadmap.md` (modify only if row wording/status changes truthfully)

## Out of Scope

- Search/report retained-seam work from row 13
- Golden-only helper seeding that bypasses authored scenario state

## Acceptance Criteria

### Tests That Must Pass

1. A row-13 golden in `crates/worldwake-ai/tests/golden_survival_justice.rs` that proves `accuse` commits for the intended theft case
2. The same golden proves the follow-on punishment commit for that case
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

### Invariants

1. The retained accusation case must come from the authored theft branch, not unrelated generic missing-item churn.
2. Row 13 remains `In Progress` until accusation/punishment is proven at the scenario level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — add truthful accusation/punishment coverage once the scenario supports it

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_justice <exact accusation test> -- --ignored --exact --test-threads=1`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
