# E17CRITHEJUS-011: Implement emit_justice_candidates()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate generation function in AI crate
**Deps**: E17CRITHEJUS-005 (needs planner support for Accuse/PunishAccused), E17CRITHEJUS-001 (needs JusticeDispositionProfile), E17CRITHEJUS-002 (needs SuspectedTheft in ViolationKind)

## Problem

Agents cannot form accusation or punishment goals. No candidate generation function exists for `GoalKind::Accuse` or `GoalKind::PunishAccused`. Without `emit_justice_candidates()`, agents with evidence of theft and institutional authority cannot pursue justice.

## Assumption Reassessment (2026-03-25)

1. `candidate_generation.rs` follows the established `emit_*` pattern. Justice candidates are a new family guarded by `JusticeDispositionProfile`.
2. `ViolationMemory` stores `SuspectedTheft` entries with `suspect: Option<EntityId>`. Only entries with `suspect: Some(entity)` produce accusation candidates (you can't accuse unknown suspects).
3. `AgentBeliefStore` can be queried for known `CrimeRegister` contents (from prior record consultation).
4. Institutional authority is checked via `can_exercise_control()` or office holder/controller queries on the belief view.
5. Punishment kind selection: prefer `Fine` when accused has commodities, otherwise `Exile`.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. A new `emit_justice_candidates()` function follows the exact `emit_*` pattern. Two sub-algorithms: one for accusation candidates (from ViolationMemory), one for punishment candidates (from known CrimeRegister entries). Both guarded by `JusticeDispositionProfile`.
2. No backwards-compatibility aliasing.

## Verification Layers

1. Accusation candidate generated when suspect known -> focused unit test
2. No accusation candidate when suspect unknown (`None`) -> focused unit test
3. No accusation candidate when accusation already filed -> focused unit test
4. Punishment candidate generated when agent has authority + unresolved accusation -> focused unit test
5. No punishment candidate without institutional authority -> focused unit test
6. Punishment kind selection: Fine when accused has commodity, Exile otherwise -> focused unit test

## What to Change

### 1. New `emit_justice_candidates()` in `candidate_generation.rs`

Guard: return early if agent has no `JusticeDispositionProfile` component.

**Accusation sub-algorithm**:
1. Scan agent's `ViolationMemory` for `ViolationKind::SuspectedTheft` entries with `suspect: Some(entity)`
2. For each: check that no existing accusation has been filed (query belief view for known CrimeRegister entries matching accused + violation)
3. Emit `GroundedGoal { kind: GoalKind::Accuse { accused: suspect, violation_id }, motive: accusation_motive_weight, priority_class: GoalPriorityClass::Low }` via `emit_candidate_with_trace()`

**Punishment sub-algorithm**:
1. Scan agent's known `CrimeRegister` entries (from `AgentBeliefStore` institutional record observations) for unresolved `Accusation` entries
2. For each: check if agent holds institutional authority (office holder/controller with jurisdiction)
3. Determine punishment: if accused believed to have commodities, `Fine`; otherwise `Exile`
4. Emit `GroundedGoal { kind: GoalKind::PunishAccused { accused, punishment }, motive: accusation_motive_weight, priority_class: GoalPriorityClass::Low }`

### 2. Wire into candidate generation dispatch

Add call to `emit_justice_candidates()` in the main dispatch function.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Theft candidate generation (E17CRITHEJUS-010)
- Accuse/Fine/Exile action handlers (E17CRITHEJUS-008/009)
- CrimeRegister entity setup (test infrastructure)
- Guard patrol crime response (E19)
- Appeal or contest logic (future spec)

## Acceptance Criteria

### Tests That Must Pass

1. Agent with `JusticeDispositionProfile` and `SuspectedTheft { suspect: Some(x) }` -> `Accuse { accused: x }` candidate emitted
2. Agent with `SuspectedTheft { suspect: None }` -> no accusation candidate
3. Agent without `JusticeDispositionProfile` -> no justice candidates at all
4. Accusation already filed for same accused + violation -> no duplicate candidate
5. Agent with authority + unresolved accusation in known CrimeRegister -> `PunishAccused` candidate emitted
6. Agent WITHOUT authority + unresolved accusation -> no punishment candidate
7. Punishment kind: `Fine` when accused has commodity, `Exile` when accused lacks commodity
8. Motive from `JusticeDispositionProfile.accusation_motive_weight`
9. All candidates at `GoalPriorityClass::Low`
10. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Only agents with `JusticeDispositionProfile` ever generate justice candidates
2. Accusation requires `suspect: Some(entity)` — can't accuse unknown suspect (P14)
3. Punishment requires institutional authority (P21)
4. Motive is profile-driven (P2)
5. No `HashMap`/`HashSet` in candidate scanning

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for `emit_justice_candidates()` covering accusation and punishment sub-algorithms

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
