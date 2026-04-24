# S13SURJUS-004: Land a punishment seam after accusation in `survival-justice`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — punishment candidate selection and/or scenario punishment setup
**Deps**: `archive/tickets/S13SURJUS-002.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`survival-justice` now lands a truthful authored accusation seam: the merchant witnesses the theft, investigates the missing case, commits `accuse`, and records the accusation in the crime register. The row still does not land punishment. After accusation, `GoalKind::PunishAccused` never becomes the retained branch, so no punishment action commits in the same survival run.

## Assumption Reassessment (2026-04-24)

1. `archive/tickets/S13SURJUS-002.md` now owns the landed accusation seam. This follow-up starts from the live baseline where `survival_justice_proves_accusation_substrate` passes.
2. The owned boundary here is the post-accusation punishment seam in row 13: the authored scenario state after accusation, the `GoalKind::PunishAccused` candidate-selection surface, and the scenario-backed golden proof that a punishment action commits.
3. Focused row-13 diagnostics on 2026-04-24 showed `accuse` committing at tick 14 with the accusation recorded in the crime register, but no punishment action ever followed.
4. Live punishment candidate selection in `crates/worldwake-ai/src/candidate_generation.rs` prefers `PunishmentKind::Fine` only when the accused is co-located and still has enough locally observed quantity of the stolen commodity. In the authored accusation run, the thief no longer retains apple quantity by the time punishment would need to emit.
5. The live fallback punishment path is `Exile`, but that branch requires faction-governed office eligibility and accused faction membership. The current scenario schema does not already expose a clean authored path for that branch in row 13.
6. Reassessment must determine whether truthful punishment ownership now belongs to authored commodity retention, a different lawful punishment path, or a bounded production change to punishment candidate selection. Do not assume `fine` is still the only honest punishment surface.
7. If the final landed punishment is not `fine`, update the row-13 golden and roadmap wording to `punishment` generically instead of preserving stale fine-specific prose.
8. Adjacent contradiction: row 13 search/report is still separately owned by `tickets/S13SURJUS-003.md` and must remain out of scope here.

## Architecture Check

1. This ticket should land one truthful punishment path after accusation rather than layering helper-only scaffolding on top of the accusation seam that already works.
2. No backwards-compatibility aliasing or golden-only punishment shortcuts are acceptable.

## Verification Layers

1. The post-accusation authored state exposes a lawful punishment candidate for the accused case -> decision trace / candidate-generation diagnostics
2. The retained punishment branch commits after the accusation case is recorded -> action trace in `golden_survival_justice.rs`
3. The crime register records the verdict or superseding punishment record for the same accusation -> authoritative `RecordData` state
4. The merchant still satisfies the authored survival envelope while punishment follows accusation -> `golden_survival_justice.rs`

## What to Change

### 1. Reassess the truthful punishment owner

Determine whether row 13 should land punishment by preserving enough authored commodity for a lawful fine, by authoring a different lawful punishment branch, or by making a bounded production change to punishment candidate selection.

### 2. Extend the row-13 golden only to the truthful punishment seam

Once the authored/runtime punishment path is real, extend `crates/worldwake-ai/tests/golden_survival_justice.rs` from accusation-only proof to the truthful punishment commit and verdict-state proof.

## Files to Touch

- `scenarios/survival-justice.ron` (modify only if authored punishment setup changes truthfully)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify only if live punishment admission still fails after truthful scenario setup)
- `crates/worldwake-systems/src/justice_actions.rs` (modify only if authoritative punishment start/commit rules are the actual blocker)
- `docs/scenario-roadmap.md` (modify only if row wording/status changes truthfully)

## Out of Scope

- Reopening the landed accusation seam from `archive/tickets/S13SURJUS-002.md`
- Search/report retained-seam work from row 13 (`tickets/S13SURJUS-003.md`)
- Golden-only helper seeding that bypasses authored scenario state

## Acceptance Criteria

### Tests That Must Pass

1. A row-13 golden in `crates/worldwake-ai/tests/golden_survival_justice.rs` that proves a punishment action commits after the authored accusation case
2. The same golden proves the crime register records the punishment/verdict for that case
3. Existing suite: `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

### Invariants

1. The punishment branch must follow the same authored theft/accusation case already proved in `archive/tickets/S13SURJUS-002.md`; it must not come from unrelated missing-item churn.
2. Row 13 remains `In Progress` until punishment and search/report are both proven at the scenario level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — extend the landed accusation proof to the truthful punishment seam once the runtime/setup supports it

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_justice <exact punishment test> -- --ignored --exact --test-threads=1`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
