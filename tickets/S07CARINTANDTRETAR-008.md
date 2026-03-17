# S07CARINTANDTRETAR-008: Golden tests and full workspace validation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: S07CARINTANDTRETAR-001 through 007 (all prior tickets must be complete)

## Problem

After all type, validation, and logic changes from tickets 001-007, the full workspace must compile and all tests must pass. This ticket adds golden integration tests proving the unified care model works end-to-end, and validates deterministic replay.

## Assumption Reassessment (2026-03-17)

1. Golden tests for healing exist — `archive/tickets/completed/GOLDENE2E-004-healing-wounded-agent.md` indicates prior golden test work was done
2. The AI harness provides `enable_tracing()` and `enable_action_tracing()` for diagnosis
3. `PerceptionProfile` must be present on agents that need to observe post-production/combat output — per CLAUDE.md golden test notes
4. Decision traces show candidates, ranking, plan search, and selection — available via `dump_agent()`
5. Action traces show Started/Committed/Aborted lifecycle — available via `events_for_at()`

## Architecture Check

1. Golden tests prove the full causal loop: wound → belief → care goal → plan → action → treatment
2. Self-care and third-party care are tested under the same goal family
3. Direct-observation gate is tested by contrasting direct observation with indirect report
4. Deterministic replay is verified by `ReplayState` after care scenarios

## What to Change

### 1. Add golden test: Wounded agent self-treats

Setup: Agent with wounds, Medicine in inventory, alone at a place.
Expected: Agent emits `TreatWounds { patient: self }`, plans Heal action, executes, wounds heal.
Variant: Agent without Medicine — agent should still emit care goal, plan should include acquisition steps (Trade/Craft) or fail gracefully.

### 2. Add golden test: Healer treats directly-observed wounded patient

Setup: Two agents at same place. Agent B wounded. Agent A has Medicine and `PerceptionProfile`.
Expected: Agent A directly observes B's wounds → emits `TreatWounds { patient: B }` → plans Travel (if needed) + Heal → executes treatment.

### 3. Add golden test: Indirect wound report does NOT trigger care goal

Setup: Agent A at Place 1. Agent B wounded at Place 2. Agent C tells Agent A about B's wounds (Report source).
Expected: Agent A does NOT emit `TreatWounds { patient: B }`. Agent A may travel independently; upon arrival and direct observation, care goal forms.

### 4. Add golden test: Care goal invalidates when patient heals

Setup: Agent A plans to treat Agent B. Before A reaches B, B self-treats and heals.
Expected: A's `TreatWounds` goal is satisfied (patient pain == 0), plan drops cleanly.

### 5. Verify deterministic replay for all care scenarios

Each golden test must verify `replay_and_verify()` succeeds.

### 6. Update existing golden tests

Any existing golden tests that reference `GoalKind::Heal` or `AcquireCommodity { purpose: Treatment }` must be updated.

## Files to Touch

- `crates/worldwake-ai/tests/` or existing golden test file(s) (modify/new)
- Any existing golden test files referencing `Heal` or `Treatment` (modify)

## Out of Scope

- Changing any production code (all changes are in tickets 001-007)
- Adding non-medicine treatment methods
- Adding `InvestigateReport` goal kind
- Performance optimization
- Soak testing (that's E22)

## Acceptance Criteria

### Tests That Must Pass

1. Golden: Wounded agent self-treats (with initial medicine)
2. Golden: Wounded agent self-treats (without initial medicine — care goal forms, plan may fail or acquire)
3. Golden: Healer treats directly-observed wounded patient
4. Golden: Indirect wound report does NOT trigger care goal — agent must travel and observe
5. Golden: Care goal invalidates when patient heals before treatment arrives
6. Deterministic replay holds for all care scenarios
7. Full workspace: `cargo test --workspace`
8. Full lint: `cargo clippy --workspace`

### Invariants

1. No `GoalKind::Heal` references exist anywhere in the workspace
2. No `CommodityPurpose::Treatment` references exist anywhere in the workspace
3. No `SelfTargetActionKind::Heal` references exist anywhere in the workspace
4. No `emit_heal_goals`, `emit_treatment_candidates`, `local_heal_targets` functions exist
5. No `treatment_pain`, `treatment_score` functions exist
6. Self-treatment is lawful
7. Only `DirectObservation` triggers third-party care
8. Conservation invariants hold across all care scenarios

## Test Plan

### New/Modified Tests

1. Golden test: self-care with medicine
2. Golden test: self-care without medicine (intent forms, supply path)
3. Golden test: third-party care via direct observation
4. Golden test: indirect report does NOT trigger care
5. Golden test: care goal invalidation on patient heal
6. Update any existing golden tests referencing old Heal/Treatment variants

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
