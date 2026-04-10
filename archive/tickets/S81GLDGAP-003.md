# S81GLDGAP-003: Need-based mortality and death event tagging

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- needs system mortality check, combat death event tagging
**Deps**: S81GLDGAP-001

## Problem

Agents cannot die from unmet needs. The needs system creates deprivation wounds but never checks whether wound load has become fatal. Additionally, death events (both combat and future need-based) are not tagged with `EventTag::Death`, making them unqueryable from the event log. This blocks GT-2 (death traceability test) and limits debuggability (P29).

## Assumption Reassessment (2026-04-09)

1. `apply_deprivation_consequences` at `crates/worldwake-systems/src/needs.rs:213` creates/worsens deprivation wounds but returns `(Option<WoundList>, Option<EntityId>)` -- no mortality. Confirmed via read.
2. `is_wound_load_fatal(wounds: &WoundList, profile: &CombatProfile) -> bool` at `crates/worldwake-core/src/wounds.rs:128` checks `wounds.wound_load() >= profile.wound_capacity`. Used by combat system's `collect_fatalities`. Confirmed via grep.
3. The needs system fn iterates agents at `crates/worldwake-systems/src/needs.rs:78-90`, skipping dead agents (`get_component_dead_at(entity).is_some()`). After deprivation consequences, it sets wound list and handles waste. The mortality check inserts between wound-list update and waste handling.
4. Combat fatality at `crates/worldwake-systems/src/combat.rs:176-189` shows the death transaction pattern: create WorldTxn, add tags, extend evidence, clear contention state, set DeadAt, set contention queue/policy, commit. This pattern must be replicated for need-based death.
5. `EventTag::Death` will exist after S81GLDGAP-001. Combat fatality currently tags with `System + WorldMutation + Combat` (combat.rs:176-178). This ticket adds `Death` tag there and uses `Death + System + WorldMutation` for need-based death.
6. The needs system fn has access to `world` (for component queries), `tick`, and `event_log`. It needs to open a `WorldTxn` for the death transaction. The existing function signature at the call site must be checked for `&mut` access patterns.
7. Live boundary correction: `needs_system` currently batches all metabolism/exposure/wound/waste mutations into one hidden `WorldTxn` tagged `System + WorldMutation` in `crates/worldwake-systems/src/needs.rs:22-58`. A death-tagged aftermath event therefore cannot be added honestly by only inserting logic "between wound-list update and waste handling" inside the current commit loop; the death-causing agent needs a dedicated tagged transaction, while non-death updates can remain batched.
8. Cause-selection correction: only starvation and dehydration currently create deprivation wounds in `apply_deprivation_consequences` (`crates/worldwake-systems/src/needs.rs:225-244`). Fatigue and bladder do not participate in lethal wound-load accumulation, so the lawful `DeathCause::NeedDeprivation { need }` choice must be based on hunger vs thirst pressure, not the highest value across all `HomeostaticNeeds`.
9. Combat cleanup helper `clear_entity_contention_state` already exists at `crates/worldwake-systems/src/combat.rs:843`, but is private to that module. Reusing the same teardown semantics from needs-based death likely requires widening that helper to crate visibility rather than duplicating the contention-clear logic.

## Architecture Check

1. Reusing `is_wound_load_fatal` for needs-based death ensures consistent death semantics across combat and needs systems. Agents with higher `CombatProfile.wound_capacity` survive both combat and deprivation longer -- emergent diversity (P22) without a separate mortality parameter.
2. No backward-compatibility shims. The needs system gains new behavior (mortality) that did not exist before.
3. Systems interact through state (P26): the needs system reads `WoundList` and `CombatProfile` components, writes `DeadAt`. No direct calls to the combat system.

## Verification Layers

1. Need-based mortality triggers when wound load exceeds capacity -> focused unit test in `crates/worldwake-systems/src/needs.rs` tests
2. `DeadAt.cause` is `NeedDeprivation` with correct need -> focused unit test assertion
3. Death event tagged `EventTag::Death` -> event-log delta assertion in unit test
4. Combat death event also tagged `EventTag::Death` -> modify existing combat test or add focused test
5. Contention state cleared on need-based death -> authoritative world state assertion
6. Non-death agents still use the existing batched `System + WorldMutation` needs update path -> focused unit test / existing needs tests remain green
7. Post-death planning halt -> verified in S81GLDGAP-005 (golden E2E)

## What to Change

### 1. Add mortality check to needs system fn

In `crates/worldwake-systems/src/needs.rs`, extend the pending-update path so death-causing agents are emitted through a dedicated tagged transaction rather than folded only into the existing hidden batch event:

1. Query `CombatProfile` for the agent.
2. If present and `is_wound_load_fatal(&updated_wounds, &combat_profile)`:
   - Determine the lethal deprivation cause from the higher of hunger vs thirst pressure in the post-update `HomeostaticNeeds` snapshot (ties resolved deterministically).
   - Open a dedicated `WorldTxn` with `CauseRef::SystemTick(tick)`, the dying agent as actor/target, and the agent's effective place.
   - Add tags: `EventTag::Death`, `EventTag::System`, `EventTag::WorldMutation`.
   - Apply the same tick's needs/exposure/wound/waste mutations for that agent inside this death transaction.
   - Set `DeadAt { tick, cause: DeathCause::NeedDeprivation { need } }`.
   - Reuse the combat contention teardown + corpse-policy pattern for the dying agent.
   - Commit the transaction.
   - Exclude that agent from the general hidden batch txn for non-death updates.

Import `DeathCause`, `DeadAt`, `EventTag`, `HomeostaticNeedId`, `is_wound_load_fatal`, and the reused contention helper/types as needed.

### 2. Add EventTag::Death to combat fatality path

In `crates/worldwake-systems/src/combat.rs:176-178`, add `.add_tag(EventTag::Death)` to the existing tag chain:

```rust
txn.add_tag(EventTag::System)
    .add_tag(EventTag::WorldMutation)
    .add_tag(EventTag::Combat)
    .add_tag(EventTag::Death)
    .add_target(fatality.entity);
```

### 3. Add focused unit tests for need-based mortality

In `crates/worldwake-systems/src/needs.rs` test module, add:

- Test that an agent with deprivation wounds exceeding wound_capacity gets `DeadAt` set with `DeathCause::NeedDeprivation`.
- Test that the event log contains an event tagged `EventTag::Death`.
- Test that an agent without a `CombatProfile` does not die from deprivation wounds.
- Test that the selected cause is the higher of hunger vs thirst pressure and remains deterministic on ties.

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify -- mortality check + tests)
- `crates/worldwake-systems/src/combat.rs` (modify -- add EventTag::Death to fatality path)

## Out of Scope

- Fixing the root cause of missing affordances (S79)
- Exploration mechanics (S80)
- Plan search budget tuning
- Observer tooling improvements
- Golden tests (S81GLDGAP-004 through S81GLDGAP-006)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: agent dies from deprivation wound load with correct `DeathCause::NeedDeprivation`
2. New focused test: death event tagged `EventTag::Death` in event log
3. New focused test: agent without `CombatProfile` survives deprivation wounds
4. Existing combat tests still pass (EventTag::Death addition is additive)
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `is_wound_load_fatal` is the single lethality predicate for both combat and needs (no duplicate threshold logic)
2. Every `DeadAt` set in the needs system is accompanied by a `EventTag::Death`-tagged event (no silent deaths)
3. Agents without `CombatProfile` cannot die from wound load (consistent with combat system)
4. Dead agents are skipped in subsequent needs system ticks (existing `is_some()` guard at needs.rs:83)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` -- new focused tests for mortality check (3-4 tests)
2. `crates/worldwake-systems/src/combat.rs` -- verify existing combat death test includes `EventTag::Death` tag

### Commands

1. `cargo test -p worldwake-systems -- needs`
2. `cargo test -p worldwake-systems -- combat`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-04-09.

- Added a neutral internal contention helper so combat, escort handoff, and need-based death reuse the same care/corpse queue teardown and setup semantics without making `needs.rs` depend directly on `combat.rs`.
- Extended `needs_system` so deprivation-fatal agents leave the hidden batch path, emit a dedicated visible `EventTag::Death` transaction, persist same-tick needs/exposure/wound/waste mutations in that death transaction, and set `DeadAt.cause` to `DeathCause::NeedDeprivation { need }`.
- Added `EventTag::Death` to the combat fatality path and covered both needs and combat death tagging with focused tests.

## Deviations

- Reassessment showed the combat contention cleanup could not be lawfully reused by calling into `combat.rs` from `needs.rs`. The shared logic was extracted into `crates/worldwake-systems/src/contention_support.rs` instead of widening a cross-system module dependency.
- The need-cause selection was constrained to hunger vs thirst because those are the only deprivation channels that currently create lethal wound load in the live needs system.

## Verification Result

- Passed focused tests:
  - `cargo test -p worldwake-systems needs_system_kills_agent_from_deprivation_and_emits_death_event`
  - `cargo test -p worldwake-systems needs_system_does_not_kill_agents_without_combat_profile`
  - `cargo test -p worldwake-systems determine_need_death_cause_prefers_higher_pressure_and_breaks_ties_toward_hunger`
  - `cargo test -p worldwake-systems combat_system_attaches_dead_at_and_emits_combat_event_for_fatal_wounds`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
