# E09NEEMET-008: E09 integration tests and negative assertions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E09NEEMET-001 through E09NEEMET-007

## Problem

E09 needs a final integration-focused verification pass, but the prior ticket text assumed capabilities that do not exist in the current codebase. After reassessing `E09NEEMET-007`, this ticket should validate the physiology and care-action slice that is actually implemented today, plus the negative architectural assertions that E09 requires.

This ticket should not reintroduce speculative expectations about bed-quality sleep, facility-aware toilet/wash affordances, or forced-collapse behavior that the current implementation does not yet model.

## Assumption Reassessment (2026-03-10)

1. `E09NEEMET-007` landed:
   - control-aware item targeting
   - consumable-effect filtering
   - profile-driven action durations
   - explicit `Eat`, `Drink`, `Sleep`, `Toilet`, and `Wash` action registration
2. `Sleep` currently exists as a repeatable one-tick rest action that reduces fatigue by `MetabolismProfile.rest_efficiency`. It is **not** a bed-aware or shelter-aware system.
3. Voluntary `Toilet` currently resets bladder and creates waste. It does **not** use latrine facilities or toilet-stall reservations.
4. `Wash` currently consumes actor-controlled water lots. It does **not** use water-source or wash-facility affordances.
5. The needs system already covers:
   - basal progression
   - body-cost application from active actions
   - starvation/dehydration wounds
   - involuntary bladder relief
6. The needs system still does **not** implement forced collapse / forced sleep for critical fatigue, even though the E09 spec expects that future behavior.
7. There is still no stored `fear` component and no stored `AgentCondition` / wellness component in authoritative state.

## Architecture Check

### What This Ticket Should Validate

1. End-to-end scheduler integration for the physiology loop that exists now.
2. End-to-end action execution for the care actions that exist now.
3. Negative assertions that prohibited stored state does not exist.
4. Deterministic behavior and conservation across the implemented care-action slice.

### What This Ticket Should Not Pretend To Validate

1. Bed or shelter sleep-quality bonuses.
2. Facility-aware toileting or washing.
3. Forced collapse / forced sleep.
4. AI planning behavior from E13.

If those behaviors are required later, they should be added in dedicated tickets after the underlying world-state carriers exist.

## Revised Scope

Add integration tests for the currently implemented E09 slice:

1. metabolism progression through scheduler/tick integration
2. care-action execution through the actual action framework
3. negative assertions for forbidden authoritative state

Do not add or require new runtime behavior in this ticket.

## What To Change

### 1. Integration test: full metabolism cycle

Create an agent with `HomeostaticNeeds::new_sated()`, default `MetabolismProfile`, `DriveThresholds`, empty `WoundList`, and zeroed `DeprivationExposure`. Advance ticks through the actual system path. Assert:

- needs increase according to physiology progression
- no deprivation wounds appear before tolerance windows are exceeded
- results are deterministic

### 2. Integration test: eat / drink cycle through action framework

Create a hungry/thirsty agent with controlled food and water. Use the action framework to start and tick `Eat` and `Drink` to completion. Assert:

- the action is discoverable as an affordance only when the actor controls the target
- the target lot quantity drops by exactly 1
- hunger/thirst/bladder change according to `CommodityConsumableProfile`

### 3. Integration test: sleep action through action framework

Create a fatigued agent with no bed or special facility. Execute `Sleep` and assert:

- fatigue decreases by `rest_efficiency`
- no bed/facility requirement is present

### 4. Integration test: toilet and wash actions through action framework

Assert:

- `Toilet` resets bladder and creates waste at the actor’s location
- `Wash` consumes controlled water and reduces dirtiness

### 5. Integration test: starvation / dehydration consequences

Advance the needs system long enough to cross tolerance windows. Assert:

- starvation adds deprivation wounds
- dehydration adds deprivation wounds

### 6. Integration test: divergent `MetabolismProfile` values

Create two otherwise identical agents with different metabolism rates or duration parameters. Advance the same number of ticks. Assert their physiology diverges accordingly.

### 7. Negative assertion: no stored fear component

Assert there is no authoritative stored `Fear` component or `fear: Permille` physiology field.

### 8. Negative assertion: no stored AgentCondition / wellness component

Assert there is no authoritative `AgentCondition`, `wellness`, or equivalent aggregate score component.

## Files To Touch

- `crates/worldwake-systems/tests/needs_integration.rs` (new)
- `crates/worldwake-systems/tests/needs_negative_assertions.rs` (new)

## Out Of Scope

- Bed-quality sleep tests
- Toilet-facility or wash-facility tests
- Forced-collapse / forced-sleep tests
- E13 AI decision tests
- E12 combat wound tests
- performance benchmarks

## Acceptance Criteria

### Tests That Must Pass

1. Need progression evolves by simulation tick.
2. Camera independence / visibility independence holds for physiology progression.
3. Eating consumes one unit and applies commodity-defined effects.
4. Drinking consumes one unit and applies commodity-defined effects.
5. Sleep reduces fatigue without any bed requirement.
6. Toilet resets bladder and creates waste.
7. Wash reduces dirtiness and consumes controlled water.
8. Active action body costs increase physiology pressure deterministically.
9. Sustained critical hunger adds deprivation wound(s).
10. Sustained critical thirst adds deprivation wound(s).
11. Different `MetabolismProfile` values produce divergent outcomes.
12. There is no stored fear component and no stored AgentCondition / wellness component.
13. Existing suite: `cargo test --workspace`

### Invariants

1. Integration coverage matches implemented behavior, not speculative future behavior.
2. Conservation holds through eat/drink/wash flows.
3. No floating-point values are introduced in setup or assertions.
4. Deterministic seeds and identical inputs produce identical physiology outcomes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/needs_integration.rs`
   - scheduler/metabolism progression
   - action-driven eat/drink/sleep/toilet/wash
   - starvation/dehydration integration
   - divergent metabolism-profile behavior
2. `crates/worldwake-systems/tests/needs_negative_assertions.rs`
   - no stored fear
   - no stored AgentCondition / wellness

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
