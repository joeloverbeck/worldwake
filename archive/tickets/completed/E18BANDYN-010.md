# E18BANDYN-010: Split faction camp policy from place-backed bandit camp state

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-core (bandit camp data contract), worldwake-sim (duration/query consumers), worldwake-systems (EstablishCamp consumer), active E18 spec/ticket references
**Deps**: archive/tickets/completed/E18BANDYN-004.md, specs/E18-bandit-dynamics.md

## Problem

The live E18 contract still stores faction-wide regroup policy on place entities via `BanditCampProfile`, even though that policy is not place-scoped. `min_regroup_count`, `establishment_duration_ticks`, `flee_wound_threshold`, and `rally_place` describe faction doctrine for camp re-formation, not mutable state of whichever place currently hosts the camp.

That mismatch already leaked into the implementation delivered by E18BANDYN-004:

- `EstablishCamp` scans place profiles by faction to find a canonical policy,
- `EstablishCamp` commit rehomes that profile from one place to another,
- downstream tickets have to reason about a movable place component as if it were faction law.

This is the wrong long-term architecture. It couples faction doctrine to current camp location, creates a movable alias path for one fact, and makes abandonment/regroup tickets harder to keep lawful and debuggable.

## Assumption Reassessment (2026-03-29)

1. The live authoritative split after E18BANDYN-004 is:
   - [`BanditCamp`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) on `Place`, storing `faction` and `supplies`
   - [`BanditCampProfile`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) on `Place`, storing `faction`, `min_regroup_count`, `establishment_duration_ticks`, `flee_wound_threshold`, and `rally_place`
2. The exact shared abstraction boundary under audit is: active camp state vs faction-scoped regroup policy across [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs), [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), and [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs).
3. The current canonical duration path for EstablishCamp is wrong-shaped. [`resolve_bandit_camp_establishment_duration`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) scans every place profile for the faction and errors on multiplicity. That is a movable alias search, not a clean source of truth.
4. The current commit path for EstablishCamp is also wrong-shaped. [`rehome_camp_profile`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs) physically removes the faction policy from one place and writes it onto another. Camp relocation currently mutates the storage location of faction doctrine.
5. The same fact currently travels through two conceptual paths:
   - active camp presence: `BanditCamp` on the current place
   - faction regroup policy: `BanditCampProfile` on whichever place currently happens to host the active camp
   Canonical end state after this ticket:
   - active camp presence remains `BanditCamp` on `Place`
   - faction regroup policy becomes a new component on the `Faction` entity
   - the old place-level `BanditCampProfile` path is removed in-scope, not left as a functional alias
6. `E18BANDYN-005` already needs `abandonment_grace_ticks`, but no such field exists in live code. If we are correcting the policy boundary anyway, that missing grace-period input belongs on the faction-scoped policy component rather than on a place profile.
7. `E18BANDYN-006` and `E18BANDYN-009` depend on rally-point and regroup policy facts that must survive camp destruction and relocation. Those facts are better modeled as faction policy than as a component that must be reattached to a place.
8. This cleanup is a required architectural consequence of E18, not cosmetic follow-up. Leaving the movable place-backed policy in place would keep violating `docs/FOUNDATIONS.md` Principle 3 (concrete state over abstract aliases) and Principle 7 (information paths should not depend on hidden global inference).
9. The active epic spec [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md) is stale relative to the intended correction. It still defines `BanditCampProfile` as a place component and still describes rally-point observation through that place-backed profile. This ticket must update the live spec alongside code so the canonical contract does not keep pointing future tickets back to the superseded alias path.

## Architecture Check

1. The cleaner model is a hard split:
   - `BanditCamp` remains the authoritative statement that a specific place currently hosts a bandit camp
   - a new `BanditFactionPolicy` component on the faction entity becomes the only authoritative source for regroup/establishment/abandonment policy
2. This is better than the current architecture because one fact gets one lawful storage path. Camp relocation stops rewriting doctrine, EstablishCamp stops scanning arbitrary places for policy, and downstream tickets can query faction policy directly without inferring from camp placement.
3. No backwards-compatibility aliasing. `BanditCampProfile` should be removed from live code once `BanditFactionPolicy` exists. Do not keep both components wired up “for transition.”

## Verification Layers

1. Faction regroup policy has a single canonical storage path -> authoritative component registration / focused core unit coverage on the new faction-scoped component, plus absence of live `BanditCampProfile` registration
2. EstablishCamp duration resolves from faction policy rather than place scans -> focused sim duration semantics/unit coverage
3. EstablishCamp commit no longer rehomes policy between places -> authoritative world-state assertions in focused systems coverage
4. Downstream tickets no longer normalize place-backed faction policy -> ticket/document reassessment in affected active tickets
5. Active E18 spec no longer documents the removed alias path -> spec diff in `specs/E18-bandit-dynamics.md`
6. This ticket is mixed-layer but not planner- or golden-driven; the contract under audit is the authoritative data path, not decision-trace behavior

## What to Change

### 1. Replace `BanditCampProfile` with faction-scoped policy

In [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs):

- remove `BanditCampProfile`
- add a new faction-only component, for example:

```rust
pub struct BanditFactionPolicy {
    pub min_regroup_count: u8,
    pub establishment_duration_ticks: NonZeroU32,
    pub abandonment_grace_ticks: NonZeroU32,
    pub flee_wound_threshold: Permille,
    pub rally_place: Option<EntityId>,
}
```

This component must be legal on `EntityKind::Faction`, not on `Place`.

### 2. Update core registration and world plumbing

Update every core registration surface so the new component is first-class and the old one is removed:

- component tables
- component schema
- world/world-txn setters, queries, and tests
- deltas and roundtrip coverage

No live `BanditCampProfile` references should remain in the authoritative component schema after this ticket.

### 3. Retarget EstablishCamp to the faction policy source

Update [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) and [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs) so they:

- resolve establishment duration from the actor payload faction's `BanditFactionPolicy`
- validate regroup count against that policy
- stop scanning place components for canonical policy
- remove profile rehoming from `EstablishCamp` commit entirely

### 4. Update downstream ticket contract references

Update the active E18 tickets that currently rely on place-backed policy:

- [`tickets/E18BANDYN-005.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-005.md)
- [`tickets/E18BANDYN-006.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-006.md)
- [`tickets/E18BANDYN-009.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-009.md)

They should depend on this ticket and describe faction policy as coming from the faction entity, not from `BanditCampProfile` on places.

### 5. Update the active E18 spec contract

Update [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md) so the live epic contract matches the new architecture:

- replace the place-backed `BanditCampProfile` section with a faction-scoped policy component
- update EstablishCamp, abandonment, and rally-point information-path text to reference faction policy plus agent belief acquisition at an active camp
- remove language that implies faction doctrine migrates with the currently occupied camp place

## Files to Touch

- [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) (modify)
- [`crates/worldwake-core/src/component_tables.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs) (modify)
- [`crates/worldwake-core/src/component_schema.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs) (modify)
- [`crates/worldwake-core/src/delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs) (modify)
- [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) (modify)
- [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) (modify)
- [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) (modify)
- [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs) (modify)
- [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md) (modify)
- [`tickets/E18BANDYN-005.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-005.md) (modify)
- [`tickets/E18BANDYN-006.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-006.md) (modify)
- [`tickets/E18BANDYN-009.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-009.md) (modify)

## Out of Scope

- Implementing `bandit_camp_system()` itself
- AI candidate generation for raid/regroup goals
- Planner search/ranking behavior for regroup goals
- Golden T22 implementation
- Changing the lawful belief-side regroup contract beyond updating consumers to the new authoritative policy source

## Acceptance Criteria

### Tests That Must Pass

1. No live authoritative component registration remains for `BanditCampProfile`
2. `BanditFactionPolicy` roundtrips and is legal on `Faction` entities
3. EstablishCamp duration resolves from `BanditFactionPolicy`, not place scans
4. EstablishCamp no longer rehomes faction policy during commit
5. Existing EstablishCamp focused coverage still passes after the contract split
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo test -p worldwake-sim`
8. Existing suite: `cargo test -p worldwake-systems`
9. Existing suite: `cargo clippy --workspace`

### Invariants

1. Active camp state is place-scoped; regroup/abandonment policy is faction-scoped
2. One authoritative fact has one lawful storage path; no duplicate place-backed alias remains
3. No magic numbers are introduced while adding `abandonment_grace_ticks`

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) — prove the new faction-scoped policy contract and removal of the old place-scoped profile
2. [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) — prove EstablishCamp duration resolves from faction policy
3. [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs) — prove EstablishCamp still validates and commits without policy rehoming
4. [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md) plus the affected E18 tickets — document the canonical faction-scoped policy path so follow-on work no longer targets the removed place-backed alias

### Commands

1. `cargo test -p worldwake-core bandit_camp`
2. `cargo test -p worldwake-sim action_semantics`
3. `cargo test -p worldwake-systems bandit_camp_actions`
4. `cargo test -p worldwake-systems action_registry::tests::build_full_action_registries_returns_complete_action_catalog`
5. `cargo clippy --workspace`
6. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Replaced place-backed `BanditCampProfile` with faction-scoped `BanditFactionPolicy`
  - Removed the old authoritative component/schema/query/setter path entirely rather than keeping a transition alias
  - Retargeted EstablishCamp duration/validation to the faction entity policy and removed policy rehoming from commit
  - Updated the active E18 spec plus dependent E18 tickets to describe the new canonical contract
- Deviations from original plan:
  - No production-code behavior beyond the data-path split was broadened; the work stayed focused on the authoritative contract and EstablishCamp consumers
  - Focused tests were strengthened inside existing core/sim/systems files instead of creating new standalone test files
- Verification results:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace`
  - `cargo build --workspace`
