# S52EVIDAFT-002: Evidence emission from action handlers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modified action commit handlers plus a shared worldwake-systems evidence helper
**Deps**: S52EVIDAFT-001

## Problem

Evidence types exist (from 001) but no action ever creates them. This ticket modifies steal, attack, travel, and wilderness-relief commit handlers to emit `SceneEvidence` entries on the place where the action occurred, materializing aftermath as inspectable world state. For theft, the evidence slice is the container-backed/displayed theft case the spec defines; ground theft remains lawful `steal`, but does not invent fake container-tamper evidence.

## Assumption Reassessment (2026-04-05)

1. `commit_steal` at `crates/worldwake-systems/src/transport_actions.rs:589-608` — currently transfers possession with no evidence emission. Confirmed.
2. `commit_attack` at `crates/worldwake-systems/src/combat.rs:1739-1803` — currently applies wound with no evidence emission. Confirmed.
3. `commit_travel` is in `crates/worldwake-systems/src/travel_actions.rs:256-277` and already carries both departure tick and destination. `MovementTrace` should be emitted at the departure place after the move commits. Confirmed.
4. `commit_relieve_wilderness` is in `crates/worldwake-systems/src/needs_actions.rs:406-433` and already materializes waste at the relief location. `EventTag::WildernessRelief` at `crates/worldwake-core/src/event_tag.rs:30` should now also materialize as `DisturbanceKind::WildernessRelief`. Confirmed.
5. `WorldTxn` provides `set_component_*` for setting `SceneEvidence` on place entities. If a `SceneEvidence` component already exists on the place, new entries are appended to the existing `evidence` vector.
6. `SceneEvidence.next_entry_id` must be incremented when adding entries, ensuring unique `EvidenceEntryId` per place.
7. `validate_pick_up` in `crates/worldwake-systems/src/transport_actions.rs:300-377` explicitly rejects contained targets (`direct_container(target).is_some()`). The live “forced pickup from container” path does not exist through `pick_up`; forced-entry/container-tamper evidence belongs to `commit_steal`, not `commit_pick_up`.
8. `commit_attack` in `crates/worldwake-systems/src/combat.rs:1739-1797` applies a wound but does not set `DeadAt` directly. Fatality finalization is deferred to `combat_system`, so aftermath evidence here must key off the newly fatal wound state rather than a same-handler death transition.
9. **Authoritative-to-AI note**: These changes add post-commit evidence state but do NOT modify preconditions, affordance generation, or planning. Existing affordances, candidates, and plan shapes are unaffected.

## Architecture Check

1. Evidence emission is added to existing `on_commit` handlers — no new action handlers or action types. The change is purely additive (append SceneEvidence after the existing commit logic).
2. Each action handler emits evidence on the place entity, not on the actor or victim. This keeps evidence scene-level per the spec's design.
3. No backward-compatibility shims.

## Verification Layers

1. Contained/displayed theft commit creates `ContainerTampered` on place and entries accumulate without overwrite → authoritative world state
2. Attack commit creates `BloodTrail` on place → authoritative world state
3. Newly fatal attack commit creates `CombatAftermath` disturbance on place → authoritative world state
4. Travel commit creates `MovementTrace` at departure → authoritative world state
5. Evidence entries have unique IDs within place → focused unit test
6. Wilderness relief commit creates `WildernessRelief` disturbance on place → authoritative world state
7. Existing action behavior unchanged → existing action tests still pass

## What to Change

### 1. Modify `commit_steal`

In `crates/worldwake-systems/src/transport_actions.rs`, after existing steal logic:
- If the stolen lot came from a direct container/display surface, get the place where the theft occurred.
- Create or append to `SceneEvidence` on that place: `EvidenceKind::ContainerTampered { container: target_container, tampered_at: current_tick }`.
- Also materialize `EvidenceKind::DisturbanceMarker { place, kind: DisturbanceKind::ForcedEntry, created_at }` for the same unlawful container-tamper scene.
- Set default decay: 200 ticks for `ContainerTampered`, 50 ticks for `ForcedEntry`.

### 2. Modify `commit_attack`

In `crates/worldwake-systems/src/combat.rs`, after wound application:
- If wound was inflicted (non-zero severity): create `EvidenceKind::BloodTrail { from_place, severity: wound.severity, caused_by: Some(attacker) }`.
- If the newly updated wound load is fatal for the target's combat profile: additionally create `EvidenceKind::DisturbanceMarker { place, kind: DisturbanceKind::CombatAftermath, created_at }`.
- Default decay: BloodTrail 100 ticks, CombatAftermath 50 ticks.

### 3. Modify travel commit

In the travel action commit handler:
- At departure place, create `EvidenceKind::MovementTrace { entity: traveler, departed_from, direction: next_place, observed_at: current_tick }`.
- Default decay: 30 ticks.

### 4. Materialize WildernessRelief

In needs system wilderness relief handler:
- Create `EvidenceKind::DisturbanceMarker { place, kind: DisturbanceKind::WildernessRelief, created_at }`.
- Default decay: 50 ticks.

### 5. Helper function for evidence emission

Create a shared helper in `worldwake-systems` (for example `evidence_support.rs`) with `emit_evidence(txn, place, kind, decay_ticks)` that:
- Gets or creates `SceneEvidence` component on the place.
- Allocates next `EvidenceEntryId` from `next_entry_id`.
- Appends new `EvidenceEntry`.
- Sets the component back.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify — `commit_steal`)
- `crates/worldwake-systems/src/combat.rs` (modify — `commit_attack`)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — `commit_travel`)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — `commit_relieve_wilderness`)
- `crates/worldwake-systems/src/lib.rs` (modify — register shared helper module)
- `crates/worldwake-systems/src/evidence_support.rs` (new — shared `emit_evidence` helper)

## Out of Scope

- Evidence decay — ticket 003
- Evidence perception — ticket 004
- Golden tests — ticket 005
- Forensic analysis or evidence matching
- Evidence forging or planting

## Acceptance Criteria

### Tests That Must Pass

1. Contained/displayed theft commit creates `ContainerTampered` and `ForcedEntry` evidence on the place
2. Attack commit creating wound creates BloodTrail evidence on the place
3. Attack commit whose updated wound load is fatal creates CombatAftermath disturbance
4. Travel commit creates MovementTrace at departure place
5. Evidence entries have unique IDs within a place
6. Multiple evidence entries on same place accumulate (not overwrite)
7. Wilderness relief commit creates WildernessRelief disturbance on the place
8. Existing action behavior unchanged (all pre-existing action tests pass)
9. Existing suite: `cargo test --workspace`

### Invariants

1. Evidence emitted ONLY on commit — never during planning or precondition checks
2. Evidence placed on the Place entity, not on actor or victim
3. Each EvidenceEntry has a unique EvidenceEntryId within its SceneEvidence
4. Existing affordances, candidates, and planning unaffected (Authoritative-to-AI rule)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` — Unit test: contained/displayed steal produces ContainerTampered and ForcedEntry; repeated emissions accumulate
2. `crates/worldwake-systems/src/combat.rs` — Unit test: attack with wound produces BloodTrail; newly fatal wound produces CombatAftermath
3. `crates/worldwake-systems/src/travel_actions.rs` — Unit test: travel departure produces MovementTrace
4. `crates/worldwake-systems/src/needs_actions.rs` — Unit test: wilderness relief produces WildernessRelief disturbance

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- 2026-04-05
- Added shared evidence emission support in `crates/worldwake-systems/src/evidence_support.rs` and registered it from `crates/worldwake-systems/src/lib.rs`.
- Wired commit-time evidence emission into `commit_steal`, `commit_attack`, `commit_travel`, and `commit_relieve_wilderness` in `crates/worldwake-systems/src/transport_actions.rs`, `crates/worldwake-systems/src/combat.rs`, `crates/worldwake-systems/src/travel_actions.rs`, and `crates/worldwake-systems/src/needs_actions.rs`.
- Narrowed the theft proof surface during reassessment to the lawful contained/displayed theft subtype. Ground theft remains a live `steal` path but does not fabricate container-tamper evidence.
- Added focused tests for helper ID allocation and accumulation, contained theft evidence, blood-trail emission, fatal combat-aftermath emission, travel movement traces, and wilderness-relief evidence.
- Required verification also exposed a bounded unrelated clippy failure in `crates/worldwake-systems/src/artifact_actions.rs`; that one-line semicolon fix was absorbed so the ticket could complete the CI-matching lint gate honestly.
- Verification:
  - `cargo test -p worldwake-systems emit_evidence_allocates_unique_ids_and_accumulates_entries -- --nocapture`
  - `cargo test -p worldwake-systems contained_steal_emits_container_tamper_and_forced_entry_evidence_without_overwrite -- --nocapture`
  - `cargo test -p worldwake-systems attack_commit_emits_blood_trail_evidence -- --nocapture`
  - `cargo test -p worldwake-systems newly_fatal_attack_commit_emits_combat_aftermath_evidence -- --nocapture`
  - `cargo test -p worldwake-systems travel_commit_emits_movement_trace_at_departure_place -- --nocapture`
  - `cargo test -p worldwake-systems relieve_wilderness_commit_emits_scene_evidence -- --nocapture`
  - `cargo test -p worldwake-systems`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
