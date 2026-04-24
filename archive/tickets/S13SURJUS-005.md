# S13SURJUS-005: Land a truthful scenario punishment commit after accusation in `survival-justice`

**Status**: REJECTED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — ticket reassessment only
**Deps**: `archive/tickets/S13SURJUS-004.md`, `archive/tickets/S13SURJUS-002.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`archive/tickets/S13SURJUS-004.md` landed the narrow production fix that lets a co-located office holder emit punishment from an active local crime-register accusation without a redundant consult step. This ticket proposed finishing row 13 by extending the same authored theft/accusation run to a truthful punishment commit.

After reassessment against the live scenario and `docs/FOUNDATIONS.md`, that exact implementation claim is false. The current authored punishment-ending ticket collapses two different causal paths: the existing fine path implied by the stolen-apple accusation, and a not-yet-authored exile path that would require different institutional substrate. The live row-13 scenario does not yet support either punishment ending truthfully.

## Assumption Reassessment (2026-04-24)

1. `archive/tickets/S13SURJUS-002.md` already owns the landed accusation seam in `crates/worldwake-ai/tests/golden_survival_justice.rs`, and `archive/tickets/S13SURJUS-004.md` already owns the local-record punishment admission fix in `crates/worldwake-ai/src/candidate_generation.rs`. Those lower-layer dependencies are real and already landed.
2. The live shared boundary under audit is the scenario-authored fine punishment path, not generic "any punishment": the same authored apple-theft case in `scenarios/survival-justice.ron`, the `GoalKind::PunishAccused` candidate surface in `crates/worldwake-ai/src/candidate_generation.rs`, and the eventual verdict record in the crime register.
3. `docs/FOUNDATIONS.md` rules out forcing a punishment ending by workaround. Principle 4 requires punishment to remain tied to concrete transferred goods or other explicit institutional state, and Principles 1, 3, and 8 rule out synthetic golden-only holding patterns for the stolen stock.
4. Focused live repro on the current authored branch disproved the ticket's core claim. The scenario still reaches `steal -> investigate -> accuse`, but the thief's stolen apples are consumed before the accusation becomes punishable, so `GoalKind::PunishAccused` never becomes a truthful scenario continuation for the same case.
5. The live accusation case is specifically a fine-shaped case, not an exile-shaped case. The accusation recorded in the local crime register is about stolen apples from the merchant's staged stock. That makes `fine` the clean causal continuation of the current row-13 branch under `docs/FOUNDATIONS.md`.
6. A truthful exile continuation would require different authored substrate than the current row owns: governed-faction authority on the office plus authored faction membership for the accused. The current scenario does not provide that branch, and adding it would change the causal path rather than complete the existing one.
7. Mismatch + correction: this ticket should not continue as "land punishment commit" in the abstract. The truthful remaining work is narrower and different: reauthor the scenario's theft/economy/need seam so collectible stolen stock survives through the accusation window and enables the already-landed fine-admission path.
8. The surviving fine-path blocker now belongs to a new follow-up ticket, `archive/tickets/S13SURJUS-006.md`. This ticket remains as the rejection record for the disproved broader punishment-ending claim.

## Architecture Check

1. Rejecting the current ticket claim is cleaner than broadening row 13 into an arbitrary punishment branch. The authored accusation already centers on concrete stolen apples, so the robust next step is a fine-path follow-up that preserves those goods lawfully long enough to be collected.
2. This avoids introducing a second, larger institutional path just to make the golden pass. If exile is later desired, it should be authored as its own explicit causal branch with real faction/governance substrate instead of being smuggled into the current apple-theft case.
3. No backward-compatibility shims or test-only aliases are warranted.

## Verification Layers

1. `survival-justice` still proves the accusation seam for the authored theft case -> `crates/worldwake-ai/tests/golden_survival_justice.rs`
2. local accusation records can already emit punishment candidates when a lawful collectible fine exists -> focused `candidate_generation` coverage from `archive/tickets/S13SURJUS-004.md`
3. remaining punishment contradiction belongs to scenario-authored stock retention, not a missing local punishment admission rule -> combined live scenario reassessment against `scenarios/survival-justice.ron`, `golden_survival_justice.rs`, and `candidate_generation.rs`

## What to Change

### 1. Correct the ticket scope

Record that the proposed generic scenario punishment landing is not implementable on the live authored branch and that the truthful remaining path is a narrower fine-path scenario follow-up.

### 2. Hand off the remaining punishment work

Create a dedicated follow-up ticket for the surviving-stock scenario work required to make the current apple-theft accusation case reach lawful fine punishment.

## Files to Touch

- `archive/tickets/S13SURJUS-005.md` (modify)
- `archive/tickets/S13SURJUS-006.md` (new)
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- changing `crates/worldwake-ai/src/candidate_generation.rs`
- changing `crates/worldwake-systems/src/justice_actions.rs`
- adding an authored exile path for row 13
- forcing a punishment golden by golden-only helpers or scripted requests

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`

### Invariants

1. Row-13 punishment follow-through must stay tied to the same concrete apple-theft accusation case already proved by the scenario, not switch silently to a different institutional branch.
2. Any later row-13 punishment landing must respect `docs/FOUNDATIONS.md` by using a concrete causal path (`fine` with collectible stolen stock, or a separately authored exile path with explicit faction/governance substrate).

## Test Plan

### New/Modified Tests

1. None — this ticket is rejected after reassessment; verification relies on the existing accusation golden proving the remaining live baseline.

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`

## Outcome

- **Completion date**: 2026-04-24
- **What actually changed**: Reassessed the active ticket against the live scenario, the landed accusation/punishment-admission substrate, and `docs/FOUNDATIONS.md`. Corrected the ticket to reflect that the proposed generic punishment landing is false on the current branch and split the remaining work into a narrower fine-path follow-up.
- **Deviations from original plan**: No production or golden code changed under this ticket. The ticket is rejected as written because the current authored accusation case does not yet support a truthful punishment ending.
- **Verification results**: Revalidated the live accusation baseline with the existing ignored `golden_survival_justice` accusation test before finalizing the split.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
