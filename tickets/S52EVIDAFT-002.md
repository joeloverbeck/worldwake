# S52EVIDAFT-002: Evidence emission from action handlers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modified action commit handlers to emit SceneEvidence
**Deps**: S52EVIDAFT-001

## Problem

Evidence types exist (from 001) but no action ever creates them. This ticket modifies steal, attack, travel, and forced-pickup commit handlers to emit `SceneEvidence` entries on the place where the action occurred, materializing aftermath as inspectable world state.

## Assumption Reassessment (2026-04-05)

1. `commit_steal` at `crates/worldwake-systems/src/transport_actions.rs:589-608` — currently transfers possession with no evidence emission. Confirmed.
2. `commit_attack` at `crates/worldwake-systems/src/combat.rs:1739-1803` — currently applies wound with no evidence emission. Confirmed.
3. Travel action commit exists in worldwake-systems — creates movement from place A to B. `MovementTrace` should be emitted at departure place.
4. `EventTag::WildernessRelief` at `crates/worldwake-core/src/event_tag.rs:30` — needs materialization as `DisturbanceKind::WildernessRelief`.
5. `WorldTxn` provides `set_component_*` for setting `SceneEvidence` on place entities. If a `SceneEvidence` component already exists on the place, new entries are appended to the existing `evidence` vector.
6. `SceneEvidence.next_entry_id` must be incremented when adding entries, ensuring unique `EvidenceEntryId` per place.
7. **Authoritative-to-AI note**: These changes add post-commit evidence state but do NOT modify preconditions, affordance generation, or planning. Existing affordances, candidates, and plan shapes are unaffected.

## Architecture Check

1. Evidence emission is added to existing `on_commit` handlers — no new action handlers or action types. The change is purely additive (append SceneEvidence after the existing commit logic).
2. Each action handler emits evidence on the place entity, not on the actor or victim. This keeps evidence scene-level per the spec's design.
3. No backward-compatibility shims.

## Verification Layers

1. Theft commit creates ContainerTampered on place → authoritative world state (SceneEvidence component check)
2. Attack commit creates BloodTrail on place → authoritative world state
3. Travel commit creates MovementTrace at departure → authoritative world state
4. Death creates CombatAftermath disturbance → authoritative world state
5. Evidence entries have unique IDs within place → focused unit test
6. Existing action behavior unchanged → existing action tests still pass

## What to Change

### 1. Modify `commit_steal`

In `crates/worldwake-systems/src/transport_actions.rs`, after existing steal logic:
- Get the place where the theft occurred.
- Create or append to `SceneEvidence` on that place: `EvidenceKind::ContainerTampered { container: target_container, tampered_at: current_tick }`.
- Set default decay: 200 ticks.

### 2. Modify `commit_attack`

In `crates/worldwake-systems/src/combat.rs`, after wound application:
- If wound was inflicted (non-zero severity): create `EvidenceKind::BloodTrail { from_place, severity: wound.severity, caused_by: Some(attacker) }`.
- If target dies: additionally create `EvidenceKind::DisturbanceMarker { place, kind: DisturbanceKind::CombatAftermath, created_at }`.
- Default decay: BloodTrail 100 ticks, CombatAftermath 50 ticks.

### 3. Modify travel commit

In the travel action commit handler:
- At departure place, create `EvidenceKind::MovementTrace { entity: traveler, departed_from, direction: next_place, observed_at: current_tick }`.
- Default decay: 30 ticks.

### 4. Add forced-pickup evidence

In pickup/ground-item handlers:
- If item taken from a container not owned by the actor: `EvidenceKind::DisturbanceMarker { place, kind: DisturbanceKind::ForcedEntry, created_at }`.
- Default decay: 50 ticks.

### 5. Materialize WildernessRelief

In needs system wilderness relief handler:
- Create `EvidenceKind::DisturbanceMarker { place, kind: DisturbanceKind::WildernessRelief, created_at }`.
- Default decay: 50 ticks.

### 6. Helper function for evidence emission

Create a shared helper `emit_evidence(txn, place, kind, decay_ticks)` that:
- Gets or creates `SceneEvidence` component on the place.
- Allocates next `EvidenceEntryId` from `next_entry_id`.
- Appends new `EvidenceEntry`.
- Sets the component back.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify — commit_steal, pickup)
- `crates/worldwake-systems/src/combat.rs` (modify — commit_attack)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — travel commit, if this is the file)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — wilderness relief, if applicable)
- `crates/worldwake-systems/src/lib.rs` (modify — if helper module needed)

## Out of Scope

- Evidence decay — ticket 003
- Evidence perception — ticket 004
- Golden tests — ticket 005
- Forensic analysis or evidence matching
- Evidence forging or planting

## Acceptance Criteria

### Tests That Must Pass

1. Steal commit creates ContainerTampered evidence on the place
2. Attack commit creating wound creates BloodTrail evidence on the place
3. Attack commit causing death creates CombatAftermath disturbance
4. Travel commit creates MovementTrace at departure place
5. Evidence entries have unique IDs within a place
6. Multiple evidence entries on same place accumulate (not overwrite)
7. Existing action behavior unchanged (all pre-existing action tests pass)
8. Existing suite: `cargo test --workspace`

### Invariants

1. Evidence emitted ONLY on commit — never during planning or precondition checks
2. Evidence placed on the Place entity, not on actor or victim
3. Each EvidenceEntry has a unique EvidenceEntryId within its SceneEvidence
4. Existing affordances, candidates, and planning unaffected (Authoritative-to-AI rule)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` — Unit test: steal produces ContainerTampered
2. `crates/worldwake-systems/src/combat.rs` — Unit test: attack with wound produces BloodTrail; death produces CombatAftermath
3. `crates/worldwake-systems/src/travel_actions.rs` — Unit test: travel departure produces MovementTrace

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
