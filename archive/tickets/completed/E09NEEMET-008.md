# E09NEEMET-008: E09 integration tests and negative assertions

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected — tests and archival only unless a test exposes a real defect
**Deps**: E09NEEMET-001 through E09NEEMET-007

## Problem

E09 already has substantial unit coverage inside the production modules, but it still lacks a final scheduler-level verification pass for the physiology and care-action slice that actually exists today. This ticket should validate the real cross-module path rather than pretending the codebase is missing all E09 tests from scratch.

This ticket must not reintroduce speculative expectations about bed-quality sleep, facility-aware toilet/wash affordances, or forced-collapse behavior that the current implementation does not yet model.

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
8. `worldwake-systems` already has focused unit tests in:
   - `crates/worldwake-systems/src/needs.rs`
   - `crates/worldwake-systems/src/needs_actions.rs`
9. The missing value is therefore **integration coverage across `worldwake-sim` and `worldwake-systems`**, not a wholesale rewrite of existing physiology/action unit tests.
10. There is currently no `crates/worldwake-systems/tests/` integration suite for E09.

## Architecture Check

### What This Ticket Should Validate

1. End-to-end scheduler integration for the physiology loop that exists now.
2. End-to-end action execution for the care actions that exist now.
3. Negative assertions that prohibited stored state does not exist.
4. Deterministic behavior and conservation across the implemented care-action slice.
5. Clear separation between:
   - detailed unit behavior tests that already exist
   - cross-crate integration tests that this ticket should add

### What This Ticket Should Not Pretend To Validate

1. Bed or shelter sleep-quality bonuses.
2. Facility-aware toileting or washing.
3. Forced collapse / forced sleep.
4. AI planning behavior from E13.
5. Rewriting existing unit tests into integration tests just for test-style consistency.

If those behaviors are required later, they should be added in dedicated tickets after the underlying world-state carriers exist.

## Revised Scope

Add the missing integration-focused coverage for the currently implemented E09 slice:

1. metabolism progression through `step_tick` / scheduler integration
2. care-action execution through the real action framework plus the needs system
3. negative assertions for forbidden authoritative state at the authoritative schema level

Keep the existing inline unit tests unless a discovered defect or missing edge case makes strengthening them the cleanest option. Do not add or require speculative runtime behavior in this ticket.

## What To Change

### 1. Integration test: scheduler-driven metabolism cycle

Create an agent with `HomeostaticNeeds::new_sated()`, `MetabolismProfile`, `DriveThresholds`, and zeroed `DeprivationExposure`. Advance ticks through `worldwake_sim::step_tick` using `worldwake_systems::dispatch_table()`. Assert:

- needs increase according to physiology progression
- no deprivation wounds appear before tolerance windows are exceeded
- repeated identical setups produce identical outcomes
- progression happens without any special camera/visibility concept or human input

### 2. Integration test: eat / drink cycle through scheduler + action framework

Create a hungry/thirsty agent with controlled food and water. Queue `RequestAction` inputs and advance ticks through `step_tick`. Assert:

- the action only starts when the actor requests a currently valid affordance
- the target lot quantity drops by exactly 1 on commit
- hunger/thirst/bladder change according to `CommodityConsumableProfile`
- scheduler/system integration preserves conservation and deterministic completion

### 3. Integration test: sleep action through scheduler + action framework

Create a fatigued agent with no bed or special facility. Execute `Sleep` and assert:

- fatigue decreases by `rest_efficiency`
- no bed/facility requirement is present

### 4. Integration test: toilet and wash actions through scheduler + action framework

Assert:

- `Toilet` resets bladder and creates waste at the actor’s location
- `Wash` consumes controlled water and reduces dirtiness

### 5. Integration test: starvation / dehydration consequences through scheduler

Advance the actual tick loop long enough to cross tolerance windows. Assert:

- starvation adds deprivation wounds
- dehydration adds deprivation wounds

### 6. Integration test: divergent `MetabolismProfile` values

Create two otherwise identical agents with different metabolism rates or duration parameters. Advance the same number of ticks. Assert their physiology diverges accordingly.

### 7. Negative assertion: no stored fear component

Assert at the authoritative schema/component level that there is no stored `Fear` component and no `fear: Permille` field inside `HomeostaticNeeds` / `MetabolismProfile`.

### 8. Negative assertion: no stored AgentCondition / wellness component

Assert at the authoritative schema/component level that there is no stored `AgentCondition`, `wellness`, or equivalent aggregate body-state component.

## Files To Touch

- `crates/worldwake-systems/tests/e09_needs_integration.rs` (new)
- `crates/worldwake-systems/src/needs.rs` (only if a defect or missing edge case is best covered inline)
- `crates/worldwake-systems/src/needs_actions.rs` (only if a defect or missing edge case is best covered inline)

## Out Of Scope

- Bed-quality sleep tests
- Toilet-facility or wash-facility tests
- Forced-collapse / forced-sleep tests
- E13 AI decision tests
- E12 combat wound tests
- performance benchmarks

## Acceptance Criteria

### Tests That Must Pass

1. Need progression evolves by simulation tick through `step_tick`.
2. Off-input / off-observer scheduler progression holds for physiology progression.
3. Eating consumes one unit and applies commodity-defined effects.
4. Drinking consumes one unit and applies commodity-defined effects.
5. Sleep reduces fatigue without any bed requirement.
6. Toilet resets bladder and creates waste.
7. Wash reduces dirtiness and consumes controlled water.
8. Sustained critical hunger adds deprivation wound(s).
9. Sustained critical thirst adds deprivation wound(s).
10. Different `MetabolismProfile` values produce divergent outcomes.
11. There is no stored fear component and no stored AgentCondition / wellness component in authoritative schema.
12. Existing detailed unit tests continue to pass.
13. Existing suite: `cargo test --workspace`

### Invariants

1. Integration coverage matches implemented behavior, not speculative future behavior.
2. Conservation holds through eat/drink/wash flows.
3. No floating-point values are introduced in setup or assertions.
4. Deterministic seeds and identical inputs produce identical physiology outcomes.
5. Cross-crate integration tests complement existing unit tests instead of duplicating them wholesale.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/e09_needs_integration.rs`
   - scheduler/metabolism progression
   - scheduler-driven eat/drink/sleep/toilet/wash
   - starvation/dehydration integration
   - divergent metabolism-profile behavior
   - authoritative negative assertions for forbidden stored state
2. Existing inline unit suites in:
   - `crates/worldwake-systems/src/needs.rs`
   - `crates/worldwake-systems/src/needs_actions.rs`
   remain in place and must still pass

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - corrected the ticket assumptions and scope before implementation
  - added a new cross-crate integration suite at `crates/worldwake-systems/tests/e09_needs_integration.rs`
  - verified scheduler-level metabolism progression, scheduler-driven care actions, deprivation consequences, divergent metabolism profiles, and authoritative negative assertions
- Deviations from original plan:
  - kept the existing inline unit tests in `needs.rs` and `needs_actions.rs` instead of rewriting or duplicating them
  - consolidated the integration and negative assertions into one integration file rather than splitting them across two new files
  - no runtime code changes were needed because the corrected ticket matched the existing architecture
- Verification results:
  - `cargo test -p worldwake-systems` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
