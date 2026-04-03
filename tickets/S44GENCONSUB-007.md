# S44GENCONSUB-007: Attach contention to Phase 1 entities

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — entity setup in worldwake-systems and worldwake-cli
**Deps**: S44GENCONSUB-004

## Problem

The contention substrate now exists at the type and maintenance-system layers, but no Phase 1 entities actually have `ContentionQueue` + `ContentionPolicy` attached yet. Phase 1 targets — corpses (loot/bury), unique items on ground (pickup), and wounded agents (heal) — must be equipped with contention components and appropriate policies before the later validation/perception tickets can exercise them.

## Assumption Reassessment (2026-04-03)

1. Dead agents are marked with `DeadAt` component (`crates/worldwake-core/src/combat.rs:57-61`). They remain `EntityKind::Agent`. Confirmed.
2. Loot and bury actions target dead agents (ActionDomain::Corpse). Confirmed.
3. Heal action targets wounded agents (ActionDomain::Care). Confirmed.
4. Unique items use `EntityKind::UniqueItem`. Component registration constraint for `ContentionQueue` is `Agent || Facility` — must be extended to include `UniqueItem` for item pickup contention. This is a discrepancy from spec; the spec says "Extend to additional entity kinds as future phases require" but Phase 1 already includes unique items.
5. Corpses get their ContentionQueue when they die (combat system creates DeadAt). The contention components should be attached at the same point.
6. Wounded agents get their ContentionQueue when wounds are inflicted (or when heal affordance targets them). Policy: one healer at a time.
7. Unique items on ground get ContentionQueue with race-mode policy (`max_waiters: Some(0)`) when they exist unowned at a place.

## Architecture Check

1. Attaching contention components at entity lifecycle points (death, wound, item placement) is clean — the components are added when the exclusive affordance becomes relevant, not pre-attached to all entities.
2. No backward-compatibility shims.

## Verification Layers

1. Corpse entity after death has ContentionQueue + ContentionPolicy → authoritative world state check
2. Unique item on ground has ContentionQueue with race-mode policy → authoritative world state check
3. Wounded agent has ContentionQueue with queue policy → authoritative world state check
4. Single-layer ticket — attachment logic is straightforward state mutation.

## What to Change

### 1. Extend component registration constraint

In `component_schema.rs`: update `ContentionQueue` and `ContentionPolicy` entity kind constraint to include `EntityKind::UniqueItem` (in addition to Agent and Facility).

### 2. Attach contention to corpses on death

In the combat system (or wherever `DeadAt` is set): when an agent dies, attach `ContentionQueue::default()` and `ContentionPolicy { grant_hold_ticks: NonZeroU32::new(5).unwrap(), auto_promote: true, max_waiters: None }` to the corpse entity.

### 3. Attach contention to heal targets

When a heal action targets a wounded agent: if the target doesn't already have a `ContentionQueue`, attach one with queue policy (`auto_promote: true, max_waiters: None`).

### 4. Attach contention to unique items on ground

When a unique item is placed on the ground (unowned, at a place): attach `ContentionQueue::default()` and `ContentionPolicy { grant_hold_ticks: NonZeroU32::new(3).unwrap(), auto_promote: false, max_waiters: Some(0) }` (race mode).

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — extend entity kind constraint)
- `crates/worldwake-systems/src/combat.rs` (modify — attach contention on death)
- `crates/worldwake-systems/src/combat.rs` (modify — attach contention on heal target if needed)

## Out of Scope

- Phase 2 contention domains (bounty claims, storage, witness time)
- Perception of contention state (S44GENCONSUB-008)
- Golden tests proving contention behavior (S44GENCONSUB-009)

## Acceptance Criteria

### Tests That Must Pass

1. Agent that dies has ContentionQueue and ContentionPolicy components
2. Corpse ContentionPolicy has `auto_promote: true, max_waiters: None`
3. Unique item on ground has ContentionPolicy with `max_waiters: Some(0)` (race mode)
4. Existing suite: `cargo test --workspace`

### Invariants

1. ContentionQueue is only attached when exclusive affordance becomes relevant (on death, on wound, on item placement)
2. Policy values match Phase 1 table from spec

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` (tests) — contention attachment on death and heal targeting

### Commands

1. `cargo test -p worldwake-systems combat`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
