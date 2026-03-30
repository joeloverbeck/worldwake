# E20COMBEH-006A: Golden test gap — actual travel interrupt from critical bladder escalation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E20COMBEH-006 (existing travel physiology golden tests)

## Problem

`golden_travel_interrupt` in `crates/worldwake-ai/tests/golden_travel_physiology.rs` was intended to prove that critical bladder escalation during travel causes the travel action to be interrupted and the agent to replan for `GoalKind::Relieve`. However, the test places the agent at EastFieldTrail (an outdoor place with Trail + Field tags), where `relieve_wilderness` is locally available. The agent relieves locally without ever starting travel, so no travel interrupt occurs.

The assertion `travel_was_interrupted || saw_relieve_action` passes via the `saw_relieve_action` branch. This proves "AI acts on Relieve when bladder is critical" but does NOT prove the spec-required invariant: "critical bladder during travel causes interrupt and replan."

This is a missing golden/E2E coverage gap for the travel-interrupt pathway. The existing test should be renamed to reflect what it actually proves, and a new test should exercise the actual travel-interrupt scenario.

## Assumption Reassessment (2026-03-30)

1. **`golden_travel_interrupt`**: Exists in `crates/worldwake-ai/tests/golden_travel_physiology.rs`. Agent starts at EastFieldTrail (outdoor: Trail + Field). `relieve_wilderness` is available locally via `Constraint::ActorAtPlaceWithAnyTag(OUTDOOR_RELIEF_TAGS)` in `crates/worldwake-systems/src/needs_actions.rs:104`. Confirmed: agent never travels in this test.
2. **Interrupt evaluation**: `evaluate_interrupt` in `crates/worldwake-ai/src/interrupts.rs` supports `InterruptForReplan { trigger: CriticalSurvival }` for `InterruptibleWithPenalty` actions when a critical-priority challenger exists. Travel uses `Interruptibility::InterruptibleWithPenalty` (confirmed `crates/worldwake-systems/src/travel_actions.rs:44`).
3. **Bladder critical threshold**: Default `DriveThresholds::default()` has bladder critical at `pm(930)` (confirmed `crates/worldwake-core/src/drives.rs:100`).
4. **Travel body cost**: Resolved from `MetabolismProfile` in `start_travel` handler. With `bladder_rate=15` and `travel_bladder_multiplier=800`, travel adds 12 permille/tick bladder (confirmed `crates/worldwake-systems/src/travel_actions.rs` unit test `start_travel_sets_body_cost_from_metabolism_profile`).
5. **Scenario isolation**: To force travel before relief, the agent must start at an indoor place (no outdoor tags) where no latrine is present, with a reason to travel somewhere distant. During travel, bladder escalation from body cost pushes past critical, triggering interrupt. The agent then replans for Relieve and detours to PublicLatrine or an outdoor place.
6. **Concrete arithmetic**: Agent starts at VillageSquare (indoor, no latrine, no wilderness relief). Hunger at pm(700) drives travel to OrchardFarm (7 ticks via SouthGate → EastFieldTrail → OrchardFarm). Bladder starts at pm(860). With basal 15 + travel 12 = 27/tick, bladder crosses critical (930) after ~3 ticks of travel: 860 + 27*3 = 941. This occurs mid-journey (at EastFieldTrail, an outdoor place where `relieve_wilderness` becomes available).
7. **Competing affordances**: Once the agent arrives at an outdoor place mid-route (EastFieldTrail has Trail + Field), `relieve_wilderness` becomes available. The interrupt system may trigger before arrival if the needs system runs before action completion. If the interrupt fires while in transit (effective_place is None), the agent must replan after arriving at the next place.

## Architecture Check

1. The actual travel-interrupt chain (travel body cost → critical threshold → interrupt → replan to Relieve) is the spec-required golden coverage from E20 Section T-TravelInterrupt. The existing test covers a weaker invariant (critical bladder → immediate local relief). Both are valid but the stronger one is missing.
2. No backward-compatibility shims. This adds a new test and renames the existing one.

## Verification Layers

1. Travel started and progressed for multiple ticks → action trace (`ActionTraceKind::Started` + tick progression for travel)
2. Bladder crossed critical threshold during travel → authoritative world state (`HomeostaticNeeds.bladder > 930` while travel active)
3. Travel interrupted → action trace (`ActionTraceKind::Aborted` for travel action)
4. Relieve goal appeared after interrupt → decision trace (`CandidateTrace.ranked` contains `GoalKind::Relieve` in a post-interrupt tick)
5. Agent performed relief action → action trace (`ActionTraceKind::Committed` for `toilet` or `relieve_wilderness`)

## What to Change

### 1. Rename existing `golden_travel_interrupt` to `golden_critical_bladder_local_relief`

The existing test proves that an agent at an outdoor place with critical bladder acts on Relieve via local wilderness relief. Rename to reflect the actual invariant.

### 2. New `golden_travel_interrupt_from_bladder_escalation`

**Setup**:
- Agent at VillageSquare (indoor: no wilderness relief, no latrine)
- Hunger at pm(700) to drive travel toward OrchardFarm (requires food setup via `place_workstation_with_source`)
- Bladder at pm(860) — close enough to critical (930) that 3 ticks of travel body cost push it over
- `bladder_rate=15`, `travel_bladder_multiplier=800` → 27 permille/tick during travel
- `bladder_weight=pm(900)` so Relieve outranks hunger when bladder is critical

**Assert**:
- Agent starts travel (action trace: travel Started)
- Travel progresses for at least 2 ticks (action trace: travel active across ticks)
- Travel is aborted (action trace: travel Aborted)
- Agent replans for Relieve (decision trace: Relieve in ranked candidates after abort)
- Agent performs relief (action trace: toilet or relieve_wilderness Committed)

**Scenario isolation**: The agent's only local relief option during travel is `relieve_wilderness` at EastFieldTrail (outdoor, reached after 5 ticks: VillageSquare → SouthGate (2) → EastFieldTrail (3)). PublicLatrine is closer from VillageSquare (2 ticks) but requires backtracking if interrupted mid-route. The planner should find whichever relief option is available at the agent's post-interrupt location.

## Files to Touch

- `crates/worldwake-ai/tests/golden_travel_physiology.rs` (modify: rename existing test + add new test)

## Out of Scope

- Changes to interrupt evaluation logic
- Changes to travel body cost resolution
- Relief fallback golden tests (E20COMBEH-007)
- Affordance traceability improvements (separate ticket)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_critical_bladder_local_relief` — renamed from `golden_travel_interrupt`, same assertions
2. `golden_travel_interrupt_from_bladder_escalation` — proves travel is actually interrupted by critical bladder escalation
3. Existing suite: `cargo test -p worldwake-ai --test golden_travel_physiology`
4. Full AI suite: `cargo test -p worldwake-ai`

### Invariants

1. Travel body cost escalation causes bladder to cross critical threshold during multi-hop travel
2. Critical bladder triggers interrupt of the `InterruptibleWithPenalty` travel action
3. Agent replans for Relieve after travel interrupt

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_travel_physiology.rs` — `golden_critical_bladder_local_relief` — renamed from `golden_travel_interrupt`; proves local relief at outdoor place
2. `crates/worldwake-ai/tests/golden_travel_physiology.rs` — `golden_travel_interrupt_from_bladder_escalation` — proves travel interrupted by bladder escalation mid-journey

### Commands

1. `cargo test -p worldwake-ai --test golden_travel_physiology`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-30
- **What changed**: Renamed `golden_travel_interrupt` to `golden_critical_bladder_local_relief` with updated scenario metadata. Added `golden_travel_interrupt_from_bladder_escalation` proving the spec-required travel-interrupt invariant: critical bladder escalation during active travel triggers `CriticalSurvival` interrupt, aborting the `InterruptibleWithPenalty` travel action, followed by replan to Relieve and relief action commit.
- **Deviations from original plan**: The ticket's original arithmetic (bladder=860, bladder_rate=15, travel_bladder_multiplier=800, ~3 ticks to critical) was incorrect. SouthGate has Road tag (outdoor), making `relieve_wilderness` available after the 2-tick first leg before critical was reached. Corrected to: bladder=799 (Medium), bladder_rate=70, travel_bladder_multiplier=900 → 133/tick during travel, crossing critical (932) after 1 tick mid-leg. Assertion strategy also changed: interrupt-based abort does not emit `ActionTraceKind::Aborted`, so verification uses decision trace (`InterruptForReplan { trigger: CriticalSurvival }`) instead.
- **Verification results**: `cargo test -p worldwake-ai --test golden_travel_physiology` (19 passed), `cargo test -p worldwake-ai` (36 passed), `cargo clippy --workspace` (clean).
