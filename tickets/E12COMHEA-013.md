# E12COMHEA-013: Heal action definition + handler

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (action def) + worldwake-systems (handler)
**Deps**: E12COMHEA-001 (Wound.bleed_rate_per_tick), E12COMHEA-002 (CombatProfile), E12COMHEA-004 (ActionPayload — may need HealActionPayload), E12COMHEA-006 (wound helpers)

## Problem

The Heal action allows an agent to treat another agent's wounds. It reduces `bleed_rate_per_tick` (stops bleeding) and reduces wound severity over the treatment duration. Medicine is consumed in the process. This accelerates the natural stabilization/recovery process.

## Assumption Reassessment (2026-03-11)

1. `CommodityKind::Medicine` exists — confirmed.
2. Medicine has a `consumable_profile` in `CommodityKindSpec` — confirmed.
3. Treatment consumes Medicine item lots — follows existing consumption pattern (E09).
4. Healer and target must be co-located — standard Place precondition.
5. Target must have wound(s) — needs a custom precondition or payload validation.

## Architecture Check

1. Heal uses existing consumable patterns: Medicine is consumed during treatment.
2. Duration derived from medicine profile and wound severity.
3. The handler iterates target's `WoundList`, reduces `bleed_rate_per_tick` first, then reduces severity.
4. A `HealActionPayload` may be needed to specify the target. Alternatively, reuse existing payload patterns. The simplest approach: add a `Heal(HealActionPayload)` variant to `ActionPayload` if needed, or use `Combat` payload if structure is the same.

## What to Change

### 1. Define HealActionPayload (if needed)

```rust
pub struct HealActionPayload {
    pub target: EntityId,
}
```

And add `Heal(HealActionPayload)` to `ActionPayload`. Or reuse an existing pattern.

### 2. Define Heal ActionDef

- Constraints: `ActorAlive`, `ActorNotDead`, `ActorNotIncapacitated`, `ActorNotInTransit`, `ActorHasControl`, `ActorHasCommodity { kind: Medicine, min_qty: 1 }`
- Targets: one target Agent at actor's place
- Preconditions: `TargetAtActorPlace(0)`, `TargetAlive(0)`, `TargetIsAgent(0)`
- Duration: derived from medicine profile and wound severity
- Interruptibility: `InterruptibleWithPenalty`
- Payload: `ActionPayload::Heal(HealActionPayload { target })`

### 3. Register Heal in ActionDefRegistry

### 4. Implement Heal handler

- Consume Medicine from healer's inventory
- Reduce `bleed_rate_per_tick` on target's wounds (immediate or over treatment ticks)
- Reduce wound `severity` over treatment duration
- Emit heal event (public at Place)

### 5. Register handler in ActionHandlerRegistry

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add Heal variant if needed)
- `crates/worldwake-sim/src/action_def_registry.rs` (modify — register Heal def)
- `crates/worldwake-systems/src/combat.rs` (modify — Heal handler)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — register handler)

## Out of Scope

- Medicine crafting recipes (E10 production)
- Medical skill / training profiles (future enhancement)
- Self-healing (may require design decision — can an agent heal themselves?)
- AI deciding when to heal (E13)
- Natural recovery without medicine (E12COMHEA-009)

## Acceptance Criteria

### Tests That Must Pass

1. Treatment reduces `bleed_rate_per_tick` on target's wounds
2. Treatment reduces wound `severity` over duration
3. Treatment consumes medicine (conservation: medicine item lot quantity decreases)
4. Cannot heal dead agents (precondition `TargetAlive`)
5. Cannot heal without medicine (constraint `ActorHasCommodity`)
6. Must be co-located (precondition `TargetAtActorPlace`)
7. Target must be an Agent with wounds (precondition `TargetIsAgent`)
8. Heal event emitted and visible at Place
9. Durations derive from medicine profile and wound severity, not hardcoded
10. Existing suite: `cargo test --workspace`

### Invariants

1. 9.5: Conservation — medicine consumed, no items created from nothing
2. No stored health component — wounds modified directly
3. Principle 11: treatment parameters derive from profiles, not magic numbers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — Heal handler tests
2. `crates/worldwake-sim/src/action_payload.rs` — Heal payload tests (if new variant added)

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`
