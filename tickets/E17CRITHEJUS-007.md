# E17CRITHEJUS-007: Extend investigate commit with SuspectedTheft detection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — targeted extension of existing investigate handler
**Deps**: E17CRITHEJUS-002 (needs `ViolationKind::SuspectedTheft`, `SocialObservationKind::SuspectedTheft`), E17CRITHEJUS-015 (needs typed social-evidence detail)

## Problem

When an owner investigates a missing entity (S27's `InvestigateViolation`), the handler currently records `WitnessedAbsence` unconditionally. It cannot distinguish "item depleted/moved" from "my owned item was stolen." E17 needs the investigate commit to detect the ownership mismatch and record `SuspectedTheft` when the investigator owned the missing entity.

## Assumption Reassessment (2026-03-25)

1. `investigate_actions.rs` in worldwake-systems contains the investigate action handler. The commit path records `SocialObservation(WitnessedAbsence)` and marks the violation as resolved in `ViolationMemory`.
2. `believed_owner_of()` is available on the belief view (added by S01). It returns `Option<EntityId>` for the believed owner of an entity.
3. `ViolationMemory` is a component on agents (added by S27). `record_violation()` adds entries with TTL from `ViolationDispositionProfile`.
4. `AgentBeliefStore` has `record_social_observation()` for adding social observations.
5. The extension cannot safely reuse the current `SocialObservation.subjects: (EntityId, EntityId)` tuple for `SuspectedTheft`. `E17CRITHEJUS-015` must land first so this ticket records explicit theft evidence fields instead of a positional tuple convention that later tickets cannot consume reliably.
6. N/A — not removing/weakening any heuristic.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch: the original ticket assumed `SocialObservation(SuspectedTheft)` could be added with `subjects: (missing_entity, investigation_place)`. That conflicts with later accusation tickets that need suspect-aware theft evidence. Correct scope is to emit `ViolationKind::SuspectedTheft` plus typed `SocialObservation` theft detail after `E17CRITHEJUS-015`, not a tuple-overloaded placeholder.
12. N/A.

## Architecture Check

1. This remains a targeted conditional extension of the existing handler, but the theft evidence must be recorded as explicit typed detail rather than overloading the shared `SocialObservation` tuple schema. That keeps the bridge from S27 to the crime pipeline concrete and extensible.
2. No backwards-compatibility aliasing. No old code removed.

## Verification Layers

1. Owner investigating their own missing item -> `SuspectedTheft` in `ViolationMemory` -> focused unit test
2. Owner investigating -> typed `SocialObservation` theft detail in `AgentBeliefStore` -> focused unit test
3. Non-owner investigating same missing item -> NO `SuspectedTheft` (only `WitnessedAbsence`) -> focused unit test
4. `SuspectedTheft.suspect` is `None` at investigation time (no thief known yet) -> focused unit test
5. Existing `WitnessedAbsence` recording still happens regardless -> focused unit test

## What to Change

### 1. Extend investigate commit handler in `investigate_actions.rs`

After the existing `WitnessedAbsence` observation recording:

```rust
// E17: If the investigating agent owned the missing entity, record SuspectedTheft
if let Some(owner) = belief_view.believed_owner_of(missing_entity) {
    if owner == investigating_agent {
        // Record SuspectedTheft in ViolationMemory
        violation_memory.record_violation(ViolationKind::SuspectedTheft {
            missing_entity,
            expected_place: investigation_place,
            suspect: None,
        }, current_tick, violation_retention_ticks);

        // Record typed theft evidence observation
        belief_store.record_social_observation(SocialObservation {
            // exact typed detail provided by E17CRITHEJUS-015
            tick: current_tick,
        });
    }
}
```

(Exact API signatures will need to match the actual codebase.)

## Files to Touch

- `crates/worldwake-systems/src/investigate_actions.rs` (modify)

## Out of Scope

- Steal action (E17CRITHEJUS-006)
- Suspect identification via witness Tell or possession observation (emerges from existing E15 + E14 systems)
- Updating `SuspectedTheft.suspect` from `None` to `Some(thief)` when evidence arrives (requires follow-up crime-evidence relay/consumption work; see `E17CRITHEJUS-016`)
- Accuse/Fine/Exile actions (E17CRITHEJUS-008/009)
- AI candidate generation consuming `SuspectedTheft` (E17CRITHEJUS-010/011)
- Modifying the ViolationKind enum (done in E17CRITHEJUS-002)

## Acceptance Criteria

### Tests That Must Pass

1. Owner investigating their missing item: `ViolationMemory` contains `SuspectedTheft { missing_entity, expected_place, suspect: None }`
2. Owner investigating: `AgentBeliefStore` contains typed theft evidence observation
3. Non-owner investigating same missing item: `ViolationMemory` does NOT contain `SuspectedTheft`
4. Non-owner investigating: still records `WitnessedAbsence` as before
5. Owner investigating: BOTH `WitnessedAbsence` and `SuspectedTheft` are recorded (SuspectedTheft is additive)
6. `SuspectedTheft.suspect` is `None` (thief identity not yet known at investigation time)
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Existing `WitnessedAbsence` behavior completely preserved (additive change only)
2. `SuspectedTheft` only recorded when `believed_owner_of(missing) == Some(investigator)` (P12 — uses belief, not world truth)
3. `suspect: None` at investigation time (P14 — ignorance is first-class)
4. This ticket records concrete typed theft evidence locally; relay of that evidence through Tell is handled by `E17CRITHEJUS-016`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/investigate_actions.rs` — focused tests: owner vs non-owner investigation, SuspectedTheft recording, typed theft observation emission, suspect=None assertion, WitnessedAbsence preservation

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
