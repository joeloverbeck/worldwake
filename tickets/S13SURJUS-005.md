# S13SURJUS-005: Land a truthful scenario punishment commit after accusation in `survival-justice`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario punishment setup and/or bounded justice-path follow-through
**Deps**: `archive/tickets/S13SURJUS-004.md`, `archive/tickets/S13SURJUS-002.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`archive/tickets/S13SURJUS-004.md` landed the narrow production fix that lets a co-located office holder emit punishment from an active local crime-register accusation without a redundant consult step. Row 13 still does not land punishment at the scenario level. In the focused live repro, by the time `Merchant Sera` reaches `accuse`, `Thief Rana` has already consumed the stolen apples, so no lawful fine survives, and the scenario still has no authored exile path.

## Assumption Reassessment (2026-04-24)

1. `archive/tickets/S13SURJUS-002.md` already owns the landed accusation seam, and `archive/tickets/S13SURJUS-004.md` now owns the local-record punishment admission fix in `crates/worldwake-ai/src/candidate_generation.rs`.
2. The remaining owner is scenario-level: the authored row-13 theft/accusation run still fails to commit a punishment action even after local accusation records can feed punishment candidate generation.
3. The shared boundary under audit is now the post-accusation lawful punishment binding itself: authored retained theft stock in `scenarios/survival-justice.ron`, the surviving `GoalKind::PunishAccused` binding, and the action-trace / crime-register verdict proof in `golden_survival_justice.rs`.
4. Focused live diagnostics on 2026-04-24 showed the still-false premise concretely: in the original accusation repro, the thief's apple quantity was already `0` by the accusation tick, so no lawful fine remained collectible even after punishment admission was fixed locally.
5. The current scenario schema still does not expose a clean authored exile path for row 13: no governed-faction punishment route is authored for the office, and no truthful authored membership path has yet been identified for the thief.
6. Reassessment must determine whether row 13 can truthfully preserve collectible stolen commodity long enough for a fine, whether a different authored punishment path should exist, or whether the row needs another narrower split instead of forcing a synthetic passing punishment golden.
7. If the final landed punishment is not `fine`, update the row-13 golden and roadmap wording to `punishment` generically instead of preserving stale fine-specific prose.

## Architecture Check

1. The next pass should land one truthful scenario punishment branch, not paper over the remaining scenario contradiction with helper-only seeding or a scripted request.
2. No backwards-compatibility aliases or golden-only shortcuts are acceptable.

## Verification Layers

1. The authored post-accusation state exposes a lawful punishment candidate after `archive/tickets/S13SURJUS-004.md` -> decision trace / candidate-generation diagnostics
2. The retained punishment branch commits after the accusation case is recorded -> action trace in `golden_survival_justice.rs`
3. The crime register records the verdict or superseding punishment record for that same accusation -> authoritative `RecordData` state
4. The merchant still satisfies the authored survival envelope while punishment follows accusation -> `golden_survival_justice.rs`

## What to Change

### 1. Reassess the truthful remaining punishment owner

Determine whether the still-false row-13 punishment seam belongs to authored retained-stock setup for a lawful fine, a different authored punishment path, or another narrower split if the broader scenario punishment ending is still false.

### 2. Land the row-13 golden only at the truthful punishment seam

Once the authored/runtime punishment path is real, extend `crates/worldwake-ai/tests/golden_survival_justice.rs` from accusation-only proof to the truthful punishment commit and verdict-state proof.

## Files to Touch

- `scenarios/survival-justice.ron` (modify only if authored punishment setup changes truthfully)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify only if another narrow punishment admission contradiction remains after truthful scenario setup)
- `crates/worldwake-systems/src/justice_actions.rs` (modify only if authoritative punishment start/commit rules are the actual blocker)
- `docs/scenario-roadmap.md` (modify only if row wording/status changes truthfully)

## Out of Scope

- Reopening the landed accusation seam from `archive/tickets/S13SURJUS-002.md`
- Reopening the local-record punishment admission fix from `archive/tickets/S13SURJUS-004.md`
- Search/report retained-seam work from row 13 (`tickets/S13SURJUS-003.md`)
- Golden-only helper seeding that bypasses authored scenario state

## Acceptance Criteria

### Tests That Must Pass

1. A row-13 golden in `crates/worldwake-ai/tests/golden_survival_justice.rs` that proves a punishment action commits after the authored accusation case
2. The same golden proves the crime register records the punishment or verdict for that case
3. Existing suite: `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

### Invariants

1. The punishment branch must follow the same authored theft/accusation case already proved in `archive/tickets/S13SURJUS-002.md`; it must not come from unrelated missing-item churn.
2. Row 13 remains `In Progress` until punishment and search/report are both proven at the scenario level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — extend the landed accusation proof to the truthful punishment seam once the runtime/setup supports it
2. `<focused lower-layer test path decided by reassessment>` — only if another production contradiction remains after truthful scenario setup

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_justice <exact punishment test> -- --ignored --exact --test-threads=1`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
