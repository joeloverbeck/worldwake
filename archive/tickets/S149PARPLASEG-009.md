# S149PARPLASEG-009: Golden coverage — typed terminals and resume/abandon

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: No (golden contract tests and generated docs only)
**Deps**: archive/tickets/S149PARPLASEG-001.md, archive/tickets/S149PARPLASEG-002.md, archive/tickets/S149PARPLASEG-003.md, archive/tickets/S149PARPLASEG-004.md, archive/tickets/S149PARPLASEG-005.md, archive/tickets/S149PARPLASEG-006.md, archive/tickets/S149PARPLASEG-007.md, archive/tickets/S149PARPLASEG-008.md, archive/tickets/S149PARPLASEG-010.md

## Problem

D11 adds generated golden coverage for the typed-barrier + partial-plan-resumption contract. Live reassessment showed the strongest honest new `golden_ai` seam is a focused contract module: each non-safety barrier terminal builds a persisted `PartialPlanSegment` with the correct resume/abandon and failure-attribution shape, suspended agenda entries preserve those segments, `try_resume_partial_plan` resumes/abandons through a real `PerAgentBeliefView`, and `CoordinationBarrier` routes through blocker memory rather than `Discrepancy`. The mechanism-owned producer suites from tickets 001-008 remain the proof that individual planner/search/agenda/observer mechanisms emit and consume those carriers.

## Assumption Reassessment (2026-05-20)

1. After the S154 consolidation there is no standalone `golden_typed_plan_terminals.rs`; golden tests route through `crates/worldwake-ai/tests/golden_ai.rs` to `tests/scenarios/`. Verified the live target with `cargo test -p worldwake-ai --test golden_ai -- --list`. The canonical golden inventory is `docs/generated/golden-e2e-inventory.md`; regenerate with `python3 scripts/golden_inventory.py --write --check-docs` after adding scenarios.
2. `SafetyBarrier` is out of scope (deferred with the variant per spec Non-Goals). The live terminal set covered here is `InformationBarrier`, `CoordinationBarrier`, `ResourceBarrier`, `JurisdictionBarrier`, and `SearchBudgetExhausted`, plus the patience-exhausted abandon flow. `CombatCommitment` remains a non-barrier terminal and `GoalSatisfied` is the success terminal.
3. Live `GoalKind`s under test are `AcquireCommodity` plus `AskWitness` metadata for the information-barrier topic. The new module does not stage autonomous trade, arrest, witness, or facility branches; it proves the shared S149 carrier/resume contract those branch-specific producer tests feed. Mechanism-specific producer and renderer coverage remains with archived tickets 001-008 and their focused tests.
4. Coverage-gap classification: this was missing generated golden coverage for the composed public S149 contract, not missing focused/unit coverage. Focused/unit coverage for individual mechanisms already exists in the owning tickets.
5. Scenario isolation: the new scenarios explicitly exclude autonomous rival affordance branches because the proof target is the carrier, persistence, resume/abandon, and blocker-memory boundary. This avoids inventing brittle authored worlds solely to force all barrier producers through one E2E file.
6. Full action registries are not required for the landed proof seam; `try_resume_partial_plan` is exercised with a concrete `PerAgentBeliefView`, while production affordance registries remain covered by the producer-owned tests.

## Architecture Check

1. Generated golden coverage is the validation layer for the public S149 carrier/resume contract: barrier terminal → `PartialPlanSegment` → suspended agenda storage/save payload → resume or abandon transition. Per-mechanism producer coverage already lives in tickets 001-008; the new goldens prove the shared composition those producers rely on.
2. No production code changes — this ticket is test-only, so it introduces no architectural surface and no backward-compat concern.

## Verified Layers

1. Barrier carrier shape -> `golden_s149_typed_terminal_segments_carry_resume_and_failure_shape` proves discriminant, `Discrepancy`/non-`Discrepancy` mapping, resume condition, and `PatienceExhausted` abandon condition for every non-safety barrier terminal.
2. Agenda persistence -> `golden_s149_suspended_agenda_entries_preserve_partial_plan_segments` proves suspended entries keep typed segments across the serialized agenda-state payload.
3. Resume/abandon lifecycle -> `golden_s149_partial_plan_resume_and_patience_abandon_lifecycle` proves `try_resume_partial_plan` resumes after `TickElapsed` through `PerAgentBeliefView` and abandons over-limit segments before replay.
4. Coordination attribution -> `golden_s149_coordination_barrier_records_blocker_memory_not_discrepancy` proves `CoordinationBarrier` records `BlockingFact::ReservationConflict` rather than `Discrepancy`.
5. Generated inventory -> `python3 scripts/golden_inventory.py --write --check-docs` proves scenario metadata and docs stay in sync.

## Implemented Changes

### 1. S149 partial-plan terminal contract scenarios

Added `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs` with generated scenario metadata for:
- all non-safety barrier terminal carriers and resume/failure-attribution shape
- suspended agenda-entry persistence of typed segments
- resume and patience-exhausted abandon lifecycle through `try_resume_partial_plan`
- coordination-barrier blocker-memory attribution

### 2. Inventory regeneration

Regenerated golden inventory docs with `python3 scripts/golden_inventory.py --write --check-docs`.

## Files Changed

- `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modified) — routed the new scenario module
- `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-scenario-details/` (modified, regenerated)
- `specs/S149-partial-plan-segments-and-typed-terminals.md` (modified) — truthed D11 to the landed contract seam

## Out of Scope

- `SafetyBarrier` scenario (deferred with the variant).
- Any production-code change — failures here indicate a bug in the S149 mechanism surfaces, fixed there, not by weakening the golden.

## Acceptance Criteria

### Tests Passed

1. Passed: the typed-terminal carrier scenario covers `InformationBarrier`, `CoordinationBarrier`, `ResourceBarrier`, `JurisdictionBarrier`, and `SearchBudgetExhausted`.
2. Passed: the resume/abandon lifecycle scenario resumes after elapsed backoff and clears the over-limit `PatienceExhausted` segment.
3. Existing suite: `cargo test -p worldwake-ai` and `python3 scripts/golden_inventory.py --check-docs` clean.

### Invariants

1. Goldens are deterministic (seeded harness where world state is needed; `BTreeMap`/`BTreeSet` ordering) — the same inputs reproduce the same segment and agenda-state payloads.
2. No production semantics are altered to make a golden pass (precision-rules §7/§13).

## Verification Result

### Test Surface

1. Passed: `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs` adds four focused golden contract scenarios with scenario-isolation notes.

### Proof Commands

1. Passed: `cargo test -p worldwake-ai --test golden_ai -- --list` confirmed the live post-S154 golden target layout.
2. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::partial_plan_terminals`.
3. Passed: `python3 scripts/golden_inventory.py --write --check-docs`.
4. Passed: `python3 scripts/golden_inventory.py --check-docs` through the `--write --check-docs` validation run.
5. Passed: `cargo test -p worldwake-ai`.
6. Waived for this ticket iteration: `scripts/verify.sh` remains the harness final pre-push gate after the S149 ticket family is archived.

## Outcome

Completed on 2026-05-20.

Added S149 generated golden coverage through `partial_plan_terminals.rs` and refreshed the generated golden inventory/docs. The landed proof seam is narrower and more truthful than the original autonomous-scenario draft: it proves the shared typed-terminal/partial-plan carrier, persistence, resume/abandon, and coordination-blocker contracts, while relying on the archived mechanism tickets for producer-specific planner/search/agenda/observer behavior. No production semantics changed.
