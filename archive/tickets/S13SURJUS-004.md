# S13SURJUS-004: Admit punishment from a local accusation record in `survival-justice`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` punishment candidate generation
**Deps**: `archive/tickets/S13SURJUS-002.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

After `survival-justice` records the accusation case, punishment candidate generation still required the same case to come back through `InstitutionalKnowledgeSource::RecordConsultation` before `GoalKind::PunishAccused` could exist. That left a narrow production contradiction inside the punishment seam: a co-located office holder who had just written the accusation into the local crime register could not immediately consider punishment from that active local record.

## Assumption Reassessment (2026-04-24)

1. `archive/tickets/S13SURJUS-002.md` already landed the row-13 accusation seam. The live baseline still proves `steal -> investigate -> accuse`, with the accusation recorded in the crime register.
2. The first live punishment contradiction was lower-layer, not scenario-only: `crates/worldwake-ai/src/candidate_generation.rs` emitted punishment only from `BelievedInstitutionalClaim` values whose source was `InstitutionalKnowledgeSource::RecordConsultation`.
3. The shared boundary under audit is the punishment candidate admission surface in `emit_punishment_candidates()`: `GoalKind::PunishAccused`, local crime-register state (`RecordData`), and the office-holder/jurisdiction checks already enforced there.
4. Focused live diagnostics on 2026-04-24 showed a mixed outcome. The broader row-13 golden remained false, but the narrower production bug was real: after the accusation commit, the scenario still had no punishment candidate source from the freshly written local crime register unless a separate consult step had already populated institutional belief memory.
5. The truthful narrow fix is local only. Remote punishment knowledge must remain belief-backed; only co-located active accusations in a known local authority crime register should bypass the redundant consult requirement.
6. The broader row-13 punishment golden is still false after this narrow fix. In the focused scenario repro, the thief had already consumed the stolen apples by the accusation tick, so no lawful fine survived; the scenario also still lacks an authored exile path. That remaining scenario-level owner moved first to `archive/tickets/S13SURJUS-005.md`, which then rejected the generic punishment claim and split the surviving fine-path blocker to `archive/tickets/S13SURJUS-006.md`.
7. Mismatch + correction: the original ticket overclaimed a full scenario punishment landing. The live complete slice for this pass is the local-record punishment admission fix plus focused proof at candidate generation.

## Architecture Check

1. Reading a co-located active accusation from the local crime register is cleaner than requiring a redundant consult step after the same actor just recorded the accusation. It keeps the punishment admission path aligned with the physical local record rather than inventing a helper-only scenario bypass.
2. The fallback stays narrow: remote punishment knowledge still requires remembered or consulted institutional belief, so this does not reopen omniscient punishment planning.

## Verification Layers

1. A co-located office holder can emit `GoalKind::PunishAccused` from an active local crime-register accusation without a prior consult belief -> focused `candidate_generation` unit coverage
2. Existing consulted-record punishment admission still compiles against the same shared helper path -> same focused `candidate_generation` unit surface
3. Row-13 accusation proof remains unchanged after the narrow fix -> `golden_survival_justice.rs`
4. Broader scenario punishment remains unproven and stays owned by a follow-up ticket -> focused golden diagnostic repro from the same session, not part of the landed proof surface

## What to Change

### 1. Land the local-register punishment admission fix

Extend `emit_punishment_candidates()` so a co-located office holder may emit punishment from an active accusation entry in a known local crime register, while preserving the existing office-holder, jurisdiction, and fine/exile legality checks.

### 2. Add focused proof for the new local-record path

Add `candidate_generation` coverage proving that a local active accusation record can emit a fine punishment goal without first seeding `InstitutionalKnowledgeSource::RecordConsultation`.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `tickets/S13SURJUS-004.md` (modify)
- `archive/tickets/S13SURJUS-005.md` (new, later superseded by `archive/tickets/S13SURJUS-006.md` for the fine-path remainder)
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- Landing the full row-13 scenario punishment seam
- Search/report work from `archive/tickets/S13SURJUS-003.md`
- Scenario tuning that tries to preserve punishable theft stock without first fixing the local-record admission contradiction

## Acceptance Criteria

### Tests That Must Pass

1. A focused `candidate_generation` test proves a local active accusation record can emit `GoalKind::PunishAccused` without a prior consult belief
2. Existing consulted-record punishment coverage still passes
3. Existing suite: `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

### Invariants

1. Remote punishment knowledge remains belief-backed; only the co-located local crime-register path bypasses redundant consultation.
2. Row 13 remains `In Progress` until a scenario-backed punishment action commit and search/report are both proven.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — prove punishment admission from a local active accusation record without a consult belief
2. `crates/worldwake-ai/tests/golden_survival_justice.rs` — no new assertions; existing accusation/determinism coverage remains the truthful scenario-level regression surface for this narrow fix

### Commands

1. `cargo test -p worldwake-ai local_active_accusation_record`
2. `cargo test -p worldwake-ai justice_candidates_emit_fine_punishment_from_consulted_accusation`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
4. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

## Outcome

Completed on 2026-04-24.

- `emit_punishment_candidates()` now admits punishment from an active accusation entry in a co-located local crime register without requiring a redundant consult step first.
- Added focused unit coverage for the new local-record path.
- Reassessed the broader row-13 punishment golden honestly and moved the still-false scenario seam first to `archive/tickets/S13SURJUS-005.md`, which later rejected the generic punishment claim and split the surviving fine-path remainder to `archive/tickets/S13SURJUS-006.md`.

## Deviations

- The original ticket aimed to land the full scenario punishment seam. Focused live proof disproved that broader ending after the narrow production fix: in the scenario repro, the thief had already consumed the stolen apples by the accusation tick, so no lawful fine survived, and there is still no authored exile path.

## Verification Result

- Passed `cargo test -p worldwake-ai local_active_accusation_record`
- Passed `cargo test -p worldwake-ai justice_candidates_emit_fine_punishment_from_consulted_accusation`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
