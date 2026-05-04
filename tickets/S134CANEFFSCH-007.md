# S134CANEFFSCH-007: Travel, patrol, and bandit camp schemas

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in travel, patrol, and establish_camp actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating movement and positioning actions — `travel` (in `travel_actions.rs`), `patrol` (in `patrol_actions.rs`), and `establish_camp` (in `bandit_camp_actions.rs`) — to declarative `EffectSchema` evaluation. Travel exercises the place-graph traversal substrate (edge-time consumption, arrival event emission). Patrol exercises route-following with periodic perception emission. Establish_camp exercises facility-creation semantics. The planner continues to use the old `apply_hypothetical_transition` path (with `Travel`, `Patrol`, `EstablishCamp` all routing to `GoalModelFallback` per `planner_ops.rs:188–199`); goldens for these actions must produce bitwise-identical event logs.

## Assumption Reassessment (2026-05-04)

1. Movement registrations live at `crates/worldwake-systems/src/travel_actions.rs` via `register_travel_actions`, `crates/worldwake-systems/src/patrol_actions.rs` via `register_patrol_action`, and `crates/worldwake-systems/src/bandit_camp_actions.rs` via `register_establish_camp_action`.
2. After ticket 001, each `ActionDef` literal has `effect_schema: EffectSchema::empty()`. This ticket populates real schemas.
3. Travel is duration-bearing: the existing handler updates `ActionInstance.local_state: Option<ActionState>` with the `Travel { edge_id, origin, destination, departure_tick, arrival_tick }` variant during the action's lifetime, with the commit happening on arrival. The schema must encode arrival-time effect (place mutation) and event emission. The duration field already lives on `ActionDef.duration: DurationExpr` — preserved.
4. Patrol is route-following with periodic perception. The schema's step list likely includes route-position update and perception-tick scheduling; the current handler may use special tick/abort logic that the schema must encode declaratively.
5. Establish_camp creates a new facility entity at the actor's current place. Schema: precondition on place-suitability, step creating the facility entity (likely a new `EffectStep` variant — `CreateEntity { kind, place }` or similar — confirm during reassessment).
6. Existing focused/unit coverage:
   - `travel_actions.rs`, `patrol_actions.rs`, `bandit_camp_actions.rs` `#[cfg(test)]` blocks
   - Goldens — `golden_travel_*.rs`, `golden_patrol_*.rs`, `golden_bandit_camp_*.rs`, `golden_movement_*.rs`. Enumerate during reassessment.
   - Conformance test `conformance_travel` at `planner_conformance.rs:932`.
7. Shared abstraction boundary under audit: place-graph traversal and entity-creation semantics. Travel and patrol must produce identical `Place`-component mutations (effective_place updates) pre- and post-ticket; establish_camp must produce identical entity-creation event sequences.

## Architecture Check

1. Place-graph traversal as a declarative `EffectStep::Move { entity, destination, edge }` (or analog) makes the substrate explicit rather than handler-internal — improves introspection and aligns the authoritative path with what the planner has been computing all along through `GoalModelFallback`.
2. Entity-creation through `EffectStep::CreateEntity { kind, place }` (if added) aligns establish_camp's authoritative effect with the planner's hypothetical projection — currently the planner has no way to model camp creation hypothetically because `establish_camp` falls under `GoalModelFallback`'s per-`GoalKind` `apply_planner_step` (which ticket 010 deletes). Adding `CreateEntity` to the schema language gives the planner a declarative way to project camp creation post-ticket-010.
3. Patrol's periodic perception is part of its tick handler, not its commit handler — confirm during reassessment whether the schema covers tick-time behavior or only commit-time. If the schema only covers commit, patrol's tick behavior remains imperative; document this scope cleanly.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on travel-touching, patrol-touching, and bandit-camp-touching goldens.
2. Place-mutation invariant → action trace: travel commit produces identical `Place` component delta and arrival event ordering pre- and post-ticket.
3. Entity-creation invariant → event-log delta: `establish_camp` produces identical facility-creation event sequence (same `EventTag` ordering, same component-initialization events).
4. Conformance-tests parity invariant → `conformance_travel` continues to pass, comparing imperative authoritative path (now schema-driven) against `apply_hypothetical_transition` (unchanged) — both must match byte-for-byte.
5. Canonical state hash invariant → soak: identical hashes on the three soak scenarios.

## What to Change

### 1. Construct `EffectSchema` literal for travel

Sketch:

```rust
EffectSchema {
    preconditions: vec![
        // edge-existence precondition
        // route-knowledge precondition (if encoded today)
    ],
    steps: vec![
        EffectStep::Move { entity: actor, destination, edge: edge_id },
        EffectStep::EmitEvent { tag: EventTag::TravelArrive },
    ],
}
```

The duration is on `ActionDef.duration` — not in the schema. The tick-time edge-traversal state is in `ActionInstance.local_state` — not in the schema.

### 2. Construct `EffectSchema` literal for patrol

Schema covers commit-time behavior (if any) and the perception-on-tick concern stays in the imperative tick handler unless the schema language extends to tick-time. Most likely, patrol's tick handler remains imperative for periodic perception; the commit-time schema captures patrol-completion event emission.

### 3. Construct `EffectSchema` literal for establish_camp

Sketch:

```rust
EffectSchema {
    preconditions: vec![
        EffectPrecondition::CoLocated { actor, target: place },
        // place-suitability precondition (no existing camp, suitable PlaceTag, etc.)
    ],
    steps: vec![
        EffectStep::CreateEntity { kind: EntityKind::Facility, place, components: /* … */ },
        EffectStep::EmitEvent { tag: EventTag::CampEstablished },
    ],
}
```

`CreateEntity` is likely a new variant — confirm against ticket 001's enum during reassessment and add if needed.

### 4. Replace commit handler bodies with `apply_effects` delegation

Each `commit_*` handler shrinks to the standard delegation. Tick handlers (which carry duration-bearing state for travel and patrol) remain imperative for now — they're not in scope for the schema language unless ticket 010 surfaces a need.

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify)
- `crates/worldwake-systems/src/bandit_camp_actions.rs` (modify)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs `Move` or `CreateEntity` variants)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-movement actions (tickets 003, 004, 005, 006, 008, 009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Migrating tick-time handler logic for travel/patrol — only commit-time effects are in scope; tick-time edge-traversal state remains in `ActionInstance.local_state`.
- Changing place-graph traversal semantics or `DurationExpr` evaluation.

## Acceptance Criteria

### Tests That Must Pass

1. All travel-touching, patrol-touching, and bandit-camp-touching goldens produce bitwise-identical event logs.
2. Conformance test `conformance_travel` continues to pass.
3. `cargo test -p worldwake-systems travel patrol bandit_camp` — existing inline tests pass.
4. `cargo test -p worldwake-ai golden_survival` — soak goldens produce identical canonical state hashes.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Travel arrival produces the same `Place`-component mutation timing as today — duration semantics on `ActionDef.duration` are unchanged.
2. Patrol's periodic perception is preserved — tick-time behavior is unchanged; only commit-time effect goes through the schema.
3. `establish_camp` produces the same facility-creation event sequence (same component-initialization order).
4. Bitwise-identical canonical state hash on the three soak scenarios.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` `#[cfg(test)]` block — modify existing tests to exercise schema-driven commit path; verify duration and tick-time behavior unchanged.
2. `crates/worldwake-systems/src/patrol_actions.rs` and `bandit_camp_actions.rs` — analogous modifications.
3. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems travel patrol bandit_camp`
2. `cargo test -p worldwake-ai conformance_travel`
3. `cargo test -p worldwake-ai golden_travel golden_patrol`
4. `cargo test -p worldwake-ai golden_survival`
5. `./scripts/verify.sh`
