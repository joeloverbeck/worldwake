# S13SURJUS-006: Let row-13 accusation mature before in-place stolen stock is fully consumed in `survival-justice`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — local theft-evidence violation detection, row-13 fine golden proof, and a small truthful scenario stock adjustment
**Deps**: `archive/tickets/S13SURJUS-004.md`, `archive/tickets/S13SURJUS-002.md`, `archive/tickets/S13SURJUS-005.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

Row 13 already proves the authored theft / investigate / accuse seam for the apple-theft case in `survival-justice`, and the lower-layer punishment admission fix from `archive/tickets/S13SURJUS-004.md` is already landed. Reassessment on the live branch disproved this ticket's original scenario-only premise: the remaining blocker is not just authored stock retention.

In the current row-13 branch, the merchant's accusation-ready theft case matures only after the stolen display lot becomes an `EntityMissing` contradiction at `Market Square`. While the thief stays co-located and consumes the apples in place, that contradiction does not mature until the lot's quantity reaches zero. The same case therefore reaches `accuse` only after the stolen apples are already gone, so the landed fine-admission path never sees a lawful collectible quantity.

The next truthful step is to fix that local theft-investigation / accusation boundary so the same witnessed local theft case can mature into accusation while a collectible portion of the stolen stock still exists, then extend the row-13 golden to the resulting fine seam.

## Assumption Reassessment (2026-04-24)

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` already proves the current row-13 baseline: the authored scenario reaches `steal -> investigate -> accuse`, and the accusation is recorded in the local crime register.
2. `archive/tickets/S13SURJUS-004.md` already landed the relevant production fix in `crates/worldwake-ai/src/candidate_generation.rs`: once a co-located active accusation record exists and a lawful punishment binding is available, the office holder can emit `GoalKind::PunishAccused` without a redundant consult step.
3. Focused live repro on 2026-04-24 confirmed the exact contradiction in `crates/worldwake-ai/tests/golden_survival_justice.rs`: the thief steals at tick 2, commits `eat` at ticks 4, 6, and 8, and row 13 still does not reach `accuse` until tick 14, when the thief's apple quantity is already `Quantity(0)`.
4. The shared boundary under audit is therefore broader and more truthful than the original draft: local theft evidence and violation maturation in `crates/worldwake-systems/src/investigate_actions.rs`, the resulting accusation / punishment candidate path in `crates/worldwake-ai/src/candidate_generation.rs`, and the row-13 fine / verdict seam in `crates/worldwake-ai/tests/golden_survival_justice.rs`.
5. The intended invariant is specific: the same authored apple-theft case should mature into accusation and then lawful fine punishment while a collectible portion of the same stolen commodity still exists. The row must not switch to a different commodity-loss case or a different punishment mechanism.
6. `docs/FOUNDATIONS.md` rules out solving this by contriving the scenario around the current blind spot. The clean causal fix is not "make the thief walk away first" or another authored workaround; it is to let the local justice path react to the concrete witnessed theft case before full in-place consumption erases the collectible stock.
7. Auto-correction: the ticket said the remaining owner was scenario-only stock retention in `scenarios/survival-justice.ron`; live code has the accusation seam gated by an `EntityMissing`-driven investigation path in `crates/worldwake-systems/src/investigate_actions.rs`; correction applied: this ticket now owns the local theft-investigation / accusation production fix plus the row-13 golden extension. Safe because the live contradiction and the `FOUNDATIONS`-aligned direction are both unambiguous.
8. Mismatch + correction: the original broader punishment ticket (`S13SURJUS-005`) was correctly rejected for collapsing fine and exile into one claim. The truthful remainder is still fine-shaped, but it is no longer honest to describe it as scenario-only stock retention.
9. Focused implementation narrowed the real edit surface again: `crates/worldwake-systems/src/investigate_actions.rs` is the live owner of the investigation action, but the truthful fix landed earlier in `crates/worldwake-ai/src/candidate_generation.rs` by reusing the existing `EntityMissing -> investigate` lane for same-place stolen displayed stock that has matching local theft evidence. No `investigate_actions.rs` edit was required.
10. After that production fix, the row-13 golden reached `investigate` at tick 7 and `accuse` at tick 8 on the live branch. A small authored adjustment from 3 staged apples to 5 then preserved enough collectible stock for the same case to commit `fine` and record a fine verdict.

## Architecture Check

1. Fixing the local theft-investigation / accusation boundary is cleaner than reauthoring the scenario around a disappearance artifact. It keeps row 13 tied to explicit theft evidence, explicit accusation state, and the already-landed local crime-register punishment admission path.
2. This approach respects Principles 1, 3, 4, 8, and 10 from `docs/FOUNDATIONS.md`: the punishment outcome still emerges from concrete goods, local timing, and explicit action consequences rather than from a workaround or a new abstract branch.
3. No backward-compatibility shims or golden-only setup paths should be introduced.

## Verification Layers

1. the same authored theft case now matures into accusation before in-place consumption fully erases the collectible stock -> focused investigation / accusation coverage plus `crates/worldwake-ai/tests/golden_survival_justice.rs`
2. the office holder now emits and commits a `fine` punishment for that same accusation case -> action trace in `crates/worldwake-ai/tests/golden_survival_justice.rs`
3. the crime register records the verdict superseding that accusation -> authoritative `RecordData` state in `crates/worldwake-ai/tests/golden_survival_justice.rs`
4. the merchant still satisfies the authored survival envelope while the punishment branch lands -> `crates/worldwake-ai/tests/golden_survival_justice.rs`

## What to Change

### 1. Fix the local theft-investigation / accusation blocker

Adjust the live local justice path so the same witnessed apple-theft case can mature into accusation before the stolen display lot has been fully consumed in place. The fix should stay grounded in the same concrete theft case and should not require a contrived scenario workaround such as forcing the thief to leave the place first.

### 2. Extend the row-13 golden to the truthful fine seam

Once the production path supports it, extend `crates/worldwake-ai/tests/golden_survival_justice.rs` from accusation-only proof to a fine commit plus verdict-state assertion for the same accusation case.

### 3. Update roadmap wording if needed

If the final landed seam remains specifically `fine`, keep the row wording tied to that truthful fine-path continuation rather than generic punishment prose.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `scenarios/survival-justice.ron` (modify)
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- adding a new exile/faction-governance substrate for row 13
- reopening the landed accusation seam from `archive/tickets/S13SURJUS-002.md`
- reopening the local-record punishment admission fix from `archive/tickets/S13SURJUS-004.md`
- search/report work from `tickets/S13SURJUS-003.md`

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof shows the same local theft case can mature into accusation before the stolen stock is fully consumed in place.
2. A row-13 golden in `crates/worldwake-ai/tests/golden_survival_justice.rs` proves a `fine` commit after that same accusation case.
3. The same golden proves the crime register records the fine verdict for that case.
4. Existing suite: `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

### Invariants

1. The punishment branch must continue the same authored apple-theft accusation case already proved by row 13; it must not switch to a different commodity-loss case or a different punishment mechanism.
2. The row-13 punishment landing must remain grounded in the same concrete local theft case rather than scripted or helper-only setup.

## Test Plan

### New/Modified Tests

1. Focused investigate / accusation coverage at the local theft-evidence seam
2. `crates/worldwake-ai/tests/golden_survival_justice.rs` — extend the landed accusation proof to the truthful fine-punishment seam once the production fix supports it

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai candidate_generation::tests::same_place_stolen_display_stock_emits_investigate_candidate -- --exact`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
4. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_fine_punishment_for_same_theft_case -- --ignored --exact --test-threads=1`
5. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

## Outcome

Completed on 2026-04-24.

- Reused the existing generic investigate lane for same-place stolen displayed stock when the owner still locally observes the lot, no longer controls it, and has matching local `SuspectedTheft` evidence for that exact lot. This lets the same theft case mature into accusation before full in-place consumption erases the collectible stock.
- Extended `crates/worldwake-ai/tests/golden_survival_justice.rs` with a row-13 fine assertion proving the same accusation case now commits `fine` and records a fine verdict.
- Increased the staged apple stock in `scenarios/survival-justice.ron` from 3 to 5 so the now-earlier accusation window still leaves a collectible quantity for the same theft case.
- Updated `docs/scenario-roadmap.md` so row 13 truthfully records punishment as landed while search/report remains the blocking remainder.

## Deviations

- Reassessment first pointed at `crates/worldwake-systems/src/investigate_actions.rs` as the likely owner because the old branch matured through `EntityMissing -> investigate`. Focused implementation proved the narrower honest fix was earlier in `crates/worldwake-ai/src/candidate_generation.rs`; `investigate_actions.rs` was a no-change cited file.
- The final landed seam is not production-only: after the earlier accusation timing fix, a small authored stock increase remained necessary to preserve collectible apples for the same case's `fine` path.

## Verification Result

- Passed `cargo test -p worldwake-ai candidate_generation::tests::same_place_stolen_display_stock_emits_investigate_candidate -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_fine_punishment_for_same_theft_case -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
