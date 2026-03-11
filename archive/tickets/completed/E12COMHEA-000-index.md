# E12COMHEA — Combat, Wounds & Healing: Ticket Index

**Status**: COMPLETED

## Spec
`specs/E12-combat-health.md`

## Dependency Graph

```text
E12COMHEA-001 (Wound bleed_rate + CombatWeaponRef)  ──┐
                                                       │
E12COMHEA-002 (CombatProfile + DeadAt components)  ───┤
                                                       │
E12COMHEA-003 (Sword/Bow + CombatWeaponProfile)  ─────┤
                                                       │
E12COMHEA-004 (ActionPayload Combat+Loot)  ────────────┤
                                                       │
E12COMHEA-005 (Constraint/Precondition/Duration ext)  ─┤
                                                       │
001 + 002 + 003 ── E12COMHEA-006 (Wound helpers)      │
                                                       │
005 ────────────── E12COMHEA-007 (Constraint/Precon    │
                   validation in start_gate)           │
                                                       │
002 + 006 ──────── E12COMHEA-008 (Death detection +    │
                   scheduler exclusion)                │
                                                       │
006 ────────────── E12COMHEA-009 (Wound progression:   │
                   bleeding + clotting + recovery)     │
                                                       │
001+002+003+004+005+006 ── E12COMHEA-010 (Attack       │
                           action def + handler)       │
                                                       │
005 ────────────── E12COMHEA-011 (Defend action def    │
                   + handler)                          │
                                                       │
004+006 ────────── E12COMHEA-012 (Loot action def      │
                   + handler)                          │
                                                       │
004+006 ────────── E12COMHEA-013 (Heal action def      │
                   + handler)                          │
                                                       │
009+010+011 ────── E12COMHEA-014 (Combat system tick   │
                   + dispatch wiring)                  │
                                                       │
ALL ────────────── E12COMHEA-015 (Integration tests)  ─┘
```

## Recommended Execution Order

### Wave 1 (parallel, no cross-dependencies)
- **E12COMHEA-001**: Extend Wound with bleed_rate_per_tick, add CombatWeaponRef + WoundCause::Combat
- **E12COMHEA-002**: CombatProfile + DeadAt components with registration
- **E12COMHEA-003**: Sword/Bow CommodityKind variants + CombatWeaponProfile
- **E12COMHEA-004**: ActionPayload Combat + Loot variants
- **E12COMHEA-005**: Constraint/Precondition/DurationExpr extensions

### Wave 2 (depends on Wave 1 types)
- **E12COMHEA-006**: Wound helper functions (wound_load, is_incapacitated, is_dead)
- **E12COMHEA-007**: Constraint/Precondition validation for new variants in start_gate

### Wave 3 (depends on helpers)
- **E12COMHEA-008**: Death detection logic + scheduler DeadAt exclusion
- **E12COMHEA-009**: Wound progression system (bleeding, clotting, recovery)

### Wave 4 (depends on Wave 1-3, action logic)
- **E12COMHEA-010**: Attack action definition + handler
- **E12COMHEA-011**: Defend action definition + handler
- **E12COMHEA-012**: Loot action definition + handler
- **E12COMHEA-013**: Heal action definition + handler

### Wave 5 (system integration)
- **E12COMHEA-014**: Combat system tick function + SystemDispatch wiring

### Wave 6 (integration)
- **E12COMHEA-015**: Integration tests (cross-system, multi-tick scenarios)

## Verification Gate
After all tickets complete, the following must hold:
- `cargo test --workspace` -- all tests pass
- `cargo clippy --workspace` -- no warnings
- No stored `Health` component exists
- No `f32`/`f64` in combat logic
- No `HashMap`/`HashSet` in authoritative state
- Death is final: dead agents cannot plan, act, trade, vote, or consume
- Wounds are the unified bodily-harm carrier for combat and deprivation
- Conservation holds after any combat/loot sequence
- All combat outcomes deterministic given RNG state

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Tickets `E12COMHEA-001` through `E12COMHEA-014` were already implemented and archived.
  - `E12COMHEA-015` finished the remaining scheduler/replay/conservation integration coverage in `crates/worldwake-systems/tests/e12_combat_integration.rs`.
  - The E12 spec was corrected before archival to remove the contradictory dead-agent `BodyCostPerTick` assumption.
- Deviations from original plan:
  - Final verification relied on the existing broad combat test surface plus targeted scheduler-level additions, rather than duplicating all E12 assertions in a new monolithic suite.
- Verification results:
  - `cargo test --workspace`
  - `cargo clippy --workspace`
