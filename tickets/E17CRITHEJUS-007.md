# E17CRITHEJUS-007: Extend investigate commit with SuspectedTheft detection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — targeted extension of existing investigate handler
**Deps**: E17CRITHEJUS-002 (needs `ViolationKind::SuspectedTheft`, `SocialObservationKind::SuspectedTheft`)

## Problem

When an owner investigates a missing entity (S27's `InvestigateViolation`), the handler currently records `WitnessedAbsence` unconditionally. It cannot distinguish "item depleted/moved" from "my owned item was stolen." E17 needs the investigate commit to detect the ownership mismatch and record `SuspectedTheft` when the investigator owned the missing entity.

## Assumption Reassessment (2026-03-25)

1. `investigate_actions.rs` in worldwake-systems contains the investigate action handler. The commit path records `SocialObservation(WitnessedAbsence)` and marks the violation as resolved in `ViolationMemory`.
2. `believed_owner_of()` is available on the belief view (added by S01). It returns `Option<EntityId>` for the believed owner of an entity.
3. `ViolationMemory` is a component on agents (added by S27). `record_violation()` adds entries with TTL from `ViolationDispositionProfile`.
4. `AgentBeliefStore` has `record_social_observation()` for adding social observations.
5. The extension is a conditional branch AFTER the existing `WitnessedAbsence` recording — no existing behavior changes.
6. N/A — not removing/weakening any heuristic.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. This is a targeted conditional extension of an existing handler, not a replacement. The existing `WitnessedAbsence` recording remains. The new `SuspectedTheft` recording is an ADDITIONAL branch that fires when `believed_owner_of(missing_entity) == Some(investigating_agent)`. This is the minimal change to bridge S27 violation detection to the E17 crime pipeline.
2. No backwards-compatibility aliasing. No old code removed.

## Verification Layers

1. Owner investigating their own missing item -> `SuspectedTheft` in `ViolationMemory` -> focused unit test
2. Owner investigating -> `SocialObservation(SuspectedTheft)` in `AgentBeliefStore` -> focused unit test
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

        // Record shareable SuspectedTheft observation
        belief_store.record_social_observation(SocialObservation {
            kind: SocialObservationKind::SuspectedTheft,
            subjects: (missing_entity, investigation_place),
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
- Updating `SuspectedTheft.suspect` from `None` to `Some(thief)` when evidence arrives (this happens through existing belief update mechanisms when Tell or perception delivers thief identity)
- Accuse/Fine/Exile actions (E17CRITHEJUS-008/009)
- AI candidate generation consuming `SuspectedTheft` (E17CRITHEJUS-010/011)
- Modifying the ViolationKind enum (done in E17CRITHEJUS-002)

## Acceptance Criteria

### Tests That Must Pass

1. Owner investigating their missing item: `ViolationMemory` contains `SuspectedTheft { missing_entity, expected_place, suspect: None }`
2. Owner investigating: `AgentBeliefStore` contains `SocialObservation(SuspectedTheft)`
3. Non-owner investigating same missing item: `ViolationMemory` does NOT contain `SuspectedTheft`
4. Non-owner investigating: still records `WitnessedAbsence` as before
5. Owner investigating: BOTH `WitnessedAbsence` and `SuspectedTheft` are recorded (SuspectedTheft is additive)
6. `SuspectedTheft.suspect` is `None` (thief identity not yet known at investigation time)
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Existing `WitnessedAbsence` behavior completely preserved (additive change only)
2. `SuspectedTheft` only recorded when `believed_owner_of(missing) == Some(investigator)` (P12 — uses belief, not world truth)
3. `suspect: None` at investigation time (P14 — ignorance is first-class)
4. `SocialObservation(SuspectedTheft)` is shareable via Tell (P13 — knowledge travels physically)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/investigate_actions.rs` — focused tests: owner vs non-owner investigation, SuspectedTheft recording, suspect=None assertion, WitnessedAbsence preservation

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
