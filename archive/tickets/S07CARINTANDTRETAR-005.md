# S07CARINTANDTRETAR-005: Belief-driven care candidate generation with direct-observation gate

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation in worldwake-ai
**Deps**: S07CARINTANDTRETAR-001 (TreatWounds variant), S07CARINTANDTRETAR-004 (goal model ops)

## Problem

Care intent is currently split across `emit_heal_goals()` (which excludes self and gates on medicine) and `emit_treatment_candidates()` (which emits `AcquireCommodity { purpose: Treatment }`). These must be replaced with a single `emit_care_goals()` that:
- Emits `TreatWounds { patient: agent }` for self-care when agent believes self wounded (no medicine gate)
- Emits `TreatWounds { patient: other }` for third-party care only when `source == DirectObservation`
- Does NOT emit for `Report`/`Rumor`/`Inference` sources
- Never emits `AcquireCommodity { purpose: Treatment }`

## Assumption Reassessment (2026-03-17)

1. `emit_heal_goals()` exists at `candidate_generation.rs:536-558` — confirmed via grep
2. `emit_treatment_candidates()` exists — confirmed via grep
3. `local_heal_targets()` excludes self (`filter(|target| *target != agent)`) — confirmed per spec
4. `emit_heal_goals()` early-returns when agent has zero medicine — confirmed per spec (this gate is wrong and removed)
5. `emit_combat_candidates()` calls both `emit_treatment_candidates()` and `emit_heal_goals()` — confirmed per spec line 138-148
6. E14's `AgentBeliefStore` provides `known_entity_beliefs()` with `PerceptionSource` — confirmed (E14 completed)

## Architecture Check

1. Single `emit_care_goals()` replaces three functions (`emit_heal_goals`, `emit_treatment_candidates`, `local_heal_targets`). Cleaner, no split.
2. No medicine gate — care intent forms regardless. The planner handles supply via Trade/Craft/Harvest ops in `TREAT_WOUNDS_OPS`. This is the correct architecture per spec key decision C.
3. Direct-observation gate respects Principle 7 (locality) — no "psychic healing" from stale rumors.

## What to Change

### 1. Replace `emit_heal_goals()` + `emit_treatment_candidates()` + `local_heal_targets()` with `emit_care_goals()`

New function `emit_care_goals()`:

**Self-care**: If agent believes self wounded (`view.has_wounds(agent)` or equivalent), emit `TreatWounds { patient: agent }`. No medicine check.

**Third-party care**: Iterate `view.known_entity_beliefs(agent)`. For each entity with non-empty wounds:
- If `source == PerceptionSource::DirectObservation`: emit `TreatWounds { patient }`
- If `source` is `Report`/`Rumor`/`Inference`: skip

### 2. Update `emit_combat_candidates()` call site

Replace calls to `emit_treatment_candidates()` + `emit_heal_goals()` with single call to `emit_care_goals()`.

### 3. Remove obsolete functions

Delete `emit_heal_goals()`, `emit_treatment_candidates()`, `local_heal_targets()`.

### 4. Remove any `AcquireCommodity { purpose: Treatment }` emission

Ensure no code path emits `AcquireCommodity` with `CommodityPurpose::Treatment` (which no longer exists after ticket 001).

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Changing how other candidate types are generated (combat, social, enterprise, etc.)
- Adding an `InvestigateReport` goal kind (future extension)
- Changing the belief system or perception sources (E14 is complete)
- Ranking changes (ticket 006)
- Goal model/planner changes (ticket 004)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `emit_care_goals` emits `TreatWounds { patient: self }` when agent believes self wounded — even without medicine
2. `emit_care_goals` emits `TreatWounds { patient: other }` when other is wounded via `DirectObservation`
3. `emit_care_goals` does NOT emit `TreatWounds` for `Report`/`Rumor`/`Inference` sources
4. No `AcquireCommodity { purpose: Treatment }` goals emitted anywhere
5. `emit_heal_goals`, `emit_treatment_candidates`, `local_heal_targets` no longer exist (compile check)
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Care intent is patient-anchored: every emitted care goal carries the specific patient entity
2. Only `DirectObservation` triggers third-party care
3. Self-care has no medicine gate — planner handles supply
4. No split between treatment procurement and treatment application goals

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — new: self-wounded agent emits `TreatWounds { patient: self }` without medicine
2. `crates/worldwake-ai/src/candidate_generation.rs` — new: self-wounded agent emits `TreatWounds { patient: self }` with medicine
3. `crates/worldwake-ai/src/candidate_generation.rs` — new: directly-observed wounded other emits `TreatWounds { patient: other }`
4. `crates/worldwake-ai/src/candidate_generation.rs` — new: `Report`-source wounded other does NOT emit care goal
5. `crates/worldwake-ai/src/candidate_generation.rs` — new: `Rumor`-source wounded other does NOT emit care goal
6. `crates/worldwake-ai/src/candidate_generation.rs` — remove/update old tests for `emit_heal_goals` and `emit_treatment_candidates`

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**:
  - Replaced `emit_heal_goals()` + `emit_treatment_candidates()` + `local_heal_targets()` with single `emit_care_goals()` in `candidate_generation.rs`
  - Self-care emits `TreatWounds { patient: agent }` when agent has wounds (no medicine gate)
  - Third-party care iterates `known_entity_beliefs`, emits `TreatWounds` only for `DirectObservation` source
  - Removed `serves_treatment` from recipe relevance check (medicine production subordinate to `TreatWounds` plan)
  - Added `PerceptionSource` to non-test imports
  - Replaced 5 old tests with 5 new belief-driven tests
- **Deviations**: Removed `serves_treatment` from recipe candidate filter (not in original ticket scope but `local_heal_targets` deletion required it; consistent with spec intent that medicine production is subordinate to `TreatWounds`)
- **Verification**: `cargo test --workspace` (1877 passed, 0 failed), `cargo clippy --workspace` clean
