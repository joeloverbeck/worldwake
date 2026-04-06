# S56PEREXP-004: Integrate perception modulation into perception system

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — perception system internals, function signatures, perception trace metadata
**Deps**: S56PEREXP-002, S56PEREXP-003

## Problem

The perception system currently uses a flat `observation_fidelity` roll for all observations. After 001-003 provide the types and data, this ticket wires the modulation into the actual observation checks so fatigue, action attention cost, and place concealment affect observation probability.

## Assumption Reassessment (2026-04-06)

1. `perception_system` at `crates/worldwake-systems/src/perception.rs:35` destructures `active_actions` and `action_defs` from `SystemExecutionContext` (lines 40-41) but does NOT pass them to `observe_passive_local_entities` (line 51) or `process_witness_event` (line 69).
2. `observe_passive_local_entities` (line 205) passes `profile.observation_fidelity.value()` to `collect_direct_local_observation_batch` (line 239).
3. `process_witness_event` (line 109) calls `passes_observation_check(profile.observation_fidelity.value(), rng)` at line 125.
4. `passes_observation_check` (line 778) takes `fidelity: u16` and uses `rng.next_range(0, 1000) < u32::from(value)`.
5. `world.get_component_homeostatic_needs(agent)` is the pattern for reading fatigue — already used elsewhere in the codebase.
6. `world.get_component_place_visibility_profile(place)` will be available after S56PEREXP-003.
7. The `ActionInstance` struct has an `actor` field (EntityId) and `def_id` field (ActionDefId) — confirmed from `observe_active_actions` usage at perception.rs:337-346.
8. Cross-system ticket: perception reads from needs system (fatigue), action framework (attention_cost), and topology (place concealment). All reads are state-mediated (P26).
9. `SystemExecutionContext.action_defs` is `&ActionDefRegistry`, not a raw `BTreeMap<ActionDefId, ActionDef>`. Helper signature examples should use the live registry type.
10. `PerceptionTraceEvent` lives in `crates/worldwake-sim/src/perception_trace.rs` and currently records witnessed-event observation outcomes only. Extending that trace with `effective_fidelity` is in scope; adding passive-local observation tracing would be a broader trace-surface change and is out of scope for this ticket.

## Architecture Check

1. Modulation is purely multiplicative on existing `passes_observation_check` — the check function itself is unchanged. Only the fidelity value passed to it changes.
2. No backwards-compatibility shims — the flat fidelity path is replaced, not wrapped.

## Verification Layers

1. Fatigue penalty reduces effective fidelity -> focused unit test on `fatigue_observation_penalty`
2. Attention cost from active action reduces effective fidelity -> focused unit test on `active_attention_cost`
3. Place concealment reduces effective fidelity -> integration test with PlaceVisibilityProfile set
4. Combined modulation produces correct effective value -> unit test via `ObservationContext::effective_fidelity` (covered by S56PEREXP-003 tests)
5. Existing perception behavior unchanged when all modifiers are zero -> regression via existing golden tests

## What to Change

### 1. Add `fatigue_observation_penalty` function

In `crates/worldwake-systems/src/perception.rs`:

```rust
fn fatigue_observation_penalty(fatigue: Permille) -> Permille {
    if fatigue.value() <= 500 {
        Permille::ZERO
    } else {
        Permille::new_unchecked((fatigue.value() - 500) * 300 / 500)
    }
}
```

### 2. Add `active_attention_cost` function

```rust
fn active_attention_cost(
    agent: EntityId,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
) -> Permille {
    for instance in active_actions.values() {
        if instance.actor == agent {
            if let Some(def) = action_defs.get(&instance.def_id) {
                return def.attention_cost;
            }
        }
    }
    Permille::ZERO
}
```

### 3. Thread `active_actions` and `action_defs` to observation functions

Update `observe_passive_local_entities` signature to accept `active_actions` and `action_defs`. Update the call site in `perception_system` (line 51).

Update `process_witness_event` signature similarly. Update the call site in the event loop (line 69).

### 4. Replace flat fidelity with `ObservationContext` in `observe_passive_local_entities`

Where `profile.observation_fidelity.value()` is passed to `collect_direct_local_observation_batch` (line 239), instead compute:

```rust
let fatigue_penalty = fatigue_observation_penalty(
    world.get_component_homeostatic_needs(agent)
        .map_or(Permille::ZERO, |n| n.fatigue)
);
let occupancy_penalty = active_attention_cost(agent, active_actions, action_defs);
let place_concealment = world.get_component_place_visibility_profile(place)
    .map_or(Permille::ZERO, |p| p.base_concealment);

let context = ObservationContext {
    base_fidelity: profile.observation_fidelity,
    fatigue_penalty,
    occupancy_penalty,
    place_concealment,
    entity_concealment: Permille::ZERO,
};
```

Pass `context.effective_fidelity().value()` instead of `profile.observation_fidelity.value()`.

### 5. Replace flat fidelity in `process_witness_event`

Same pattern as above at line 125.

### 6. Update `PerceptionTraceEvent` for debugging

Include the effective fidelity value in witnessed-event trace output so debugging can distinguish "missed because low base fidelity" from "missed because fatigued in a concealed location" for the existing `PerceptionTraceEvent` surface.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-sim/src/perception_trace.rs` (modify)

## Out of Scope

- Scenario integration for `PlaceVisibilityProfile` (S56PEREXP-005) — this ticket assumes the component exists but places may not have it set yet (graceful `map_or` default)
- Golden E2E tests (S56PEREXP-006)
- Active concealment actions (hiding, disguise) — `entity_concealment` stays `Permille::ZERO`

## Acceptance Criteria

### Tests That Must Pass

1. `fatigue_observation_penalty(Permille::new_unchecked(0))` == `Permille::ZERO`
2. `fatigue_observation_penalty(Permille::new_unchecked(500))` == `Permille::ZERO`
3. `fatigue_observation_penalty(Permille::new_unchecked(1000))` == `Permille::new_unchecked(300)`
4. `active_attention_cost` returns `Permille::ZERO` when no active action
5. `active_attention_cost` returns the action's `attention_cost` when agent has active action
6. Existing golden tests pass with no regressions (all agents start with zero fatigue and no place concealment in existing scenarios)
7. Existing suite: `cargo test -p worldwake-systems` and `cargo test -p worldwake-ai`

### Invariants

1. When all modifiers are zero, behavior is identical to pre-S56 (flat fidelity roll)
2. Perception system still reads only local agent state — no global queries (P7)
3. Cross-system interaction is state-mediated only (P26)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` (inline tests) — unit tests for `fatigue_observation_penalty` and `active_attention_cost`, plus focused perception integration coverage for concealment / witnessed-event modulation
2. `crates/worldwake-sim/src/perception_trace.rs` — update trace tests for the new `effective_fidelity` field

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-06.

- Added perception-context modulation in `crates/worldwake-systems/src/perception.rs` for both passive local observation and witnessed-event processing, using `ObservationContext` plus new `fatigue_observation_penalty`, `active_attention_cost`, and shared effective-fidelity computation.
- Applied place concealment, fatigue, and active-action attention cost as multiplicative reductions on the existing observation gate without changing the underlying random check contract.
- Extended `crates/worldwake-sim/src/perception_trace.rs` with an `effective_fidelity` field so witnessed-event trace entries now report the actual modulated fidelity used for that observation check.
- Added focused helper tests, passive concealment integration coverage, and a witnessed-event trace assertion proving modulated fidelity reaches the trace/debug surface.

## Verification Result

- Passed `cargo test -p worldwake-sim -- perception_trace`
- Passed `cargo test -p worldwake-systems -- perception`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
