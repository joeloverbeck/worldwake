# S149PARPLASEG-009: Golden coverage — typed terminals and resume/abandon

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: No (golden E2E tests only)
**Deps**: S149PARPLASEG-001, S149PARPLASEG-002, S149PARPLASEG-003, S149PARPLASEG-004, S149PARPLASEG-005, S149PARPLASEG-006, S149PARPLASEG-007, S149PARPLASEG-008

## Problem

D11 proves the typed-barrier + partial-plan-resumption layer end-to-end with golden scenarios: each barrier kind is raised, its resume condition is satisfied through lawful world change, and the suspended intention resumes to completion — plus the patience-exhausted abandon flow.

## Assumption Reassessment (2026-05-20)

1. After the S154 consolidation there is no standalone `golden_typed_plan_terminals.rs`; golden tests route through `crates/worldwake-ai/tests/golden_ai.rs` to `tests/scenarios/` (verify exact module layout with `cargo test -p worldwake-ai --test golden_ai -- --list` before authoring). The canonical golden inventory is `docs/generated/golden-e2e-inventory.md`; regenerate with `python3 scripts/golden_inventory.py --write --check-docs` after adding scenarios.
2. `SafetyBarrier` is out of scope (deferred with the variant per spec Non-Goals), so this ticket covers six scenarios + abandon, not seven: `InformationBarrier`, `CoordinationBarrier`, `ResourceBarrier`, `JurisdictionBarrier`, `SearchBudgetExhausted`, plus the patience-exhausted abandon flow.
3. Live `GoalKind`s under test: the primary goal of each scenario plus `AskWitness` for the information-barrier resume chain. Each scenario must route through the real affordance/operator surface for its barrier — confirm the live operator surface per scenario during authoring (no `ProduceCommodity`/`RestockCommodity` narrative assumptions unless the live planner uses them).
4. Coverage-gap classification: this is the missing golden/E2E layer; focused/unit coverage for each mechanism lands in its owning ticket (001–008). This ticket does not substitute for those unit tests.
5. Scenario isolation: each barrier scenario must remove unrelated lawful competing affordances so the intended barrier is the one raised. Document per scenario which competing branches were excluded and why (precision-rules §8).
6. Full action registries are required (the resume suffixes exercise real affordances: travel, trade, witness testimony, contention).

## Architecture Check

1. Golden E2E is the validation layer for the cross-system resume chain (barrier → suspend → world change → resume → completion), which no single unit test spans (FND-31). Per-mechanism unit coverage already lives in tickets 001–008; the goldens prove their composition.
2. No production code changes — this ticket is test-only, so it introduces no architectural surface and no backward-compat concern.

## Verification Layers

1. Barrier raised → decision/action trace per scenario shows the typed terminal.
2. Resume after lawful world change → action-trace ordering shows the suspended intention resuming from the prefix-tail and committing the suffix (use `(tick, sequence_in_tick)` ordering, not incidental tick numbers, per precision-rules §14).
3. Abandon flow → decision trace shows `PatienceExhausted` abandonment and the observer (ticket 008) surfaces it.
4. Golden stability → `terminal_kind_distribution` and scenario-diagnostics fixtures remain deterministic across seeds (regenerate inventory).

## What to Change

### 1. Six barrier scenarios

Author golden scenarios under `crates/worldwake-ai/tests/scenarios/` (per the post-S154 layout):
- `InformationBarrier`: agent lacks target location → barrier → companion `AskWitness` commits → primary resumes → completion.
- `CoordinationBarrier`: agent loses an oven reservation → barrier → `BlockingFact::ReservationConflict` recorded → grant re-available → resume.
- `ResourceBarrier`: market depleted → barrier → resupply observed → resume.
- `JurisdictionBarrier`: arrest attempted outside jurisdiction → barrier → travel to jurisdiction → resume.
- `SearchBudgetExhausted`: budget runs out → typed terminal → `search_exhaustion_backoff_ticks` TTL → resume.

### 2. Abandon flow

A scenario where a partial plan re-fails its tail until `resume_attempt_count` exceeds `patience_limit` → `PatienceExhausted` abandons; assert observer surfaces it.

### 3. Inventory regeneration

Run `python3 scripts/golden_inventory.py --write --check-docs` and commit the regenerated docs.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/` (new scenario module(s))
- `crates/worldwake-ai/tests/golden_ai.rs` (modify) — route new scenario module(s) if required
- `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/` (modify, regenerated)

## Out of Scope

- `SafetyBarrier` scenario (deferred with the variant).
- Any production-code change — failures here indicate a bug in tickets 001–008, fixed there, not by weakening the golden.

## Acceptance Criteria

### Tests That Must Pass

1. New: each of the five barrier scenarios raises its typed terminal and resumes to `GoalSatisfied` after the lawful world change.
2. New: the abandon scenario reaches `PatienceExhausted` and the intention/segment is cleared.
3. Existing suite: `cargo test -p worldwake-ai` and `python3 scripts/golden_inventory.py --check-docs` clean.

### Invariants

1. Goldens are deterministic (seeded; `BTreeMap` ordering) — same seed reproduces the same resume suffix (FND-9 / CLAUDE.md determinism).
2. No production semantics are altered to make a golden pass (precision-rules §7/§13).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/<barrier scenarios>.rs` — six E2E scenarios with scenario-isolation notes.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `scripts/verify.sh`
