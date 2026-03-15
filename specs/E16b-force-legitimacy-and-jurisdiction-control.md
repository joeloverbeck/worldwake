# E16b: Force Legitimacy & Jurisdiction Control

## Epic Summary

Replace the thin E16 force-succession shortcut with explicit jurisdiction-control state: public claims, contested control, uncontested hold periods, and eventual installation into office. The goal is to make “rule by force” arise from concrete local state rather than a hidden timing heuristic.

## Phase

Phase 3: Information & Politics

## Crate

`worldwake-core` (new components, relations, enums)
`worldwake-systems` (office control system, force-claim actions)
`worldwake-ai` (later follow-up only; this spec does not require AI delivery in its first pass)

## Dependencies

- E16 (offices, succession laws, factions, support declarations, public order baseline)
- E14 (belief boundary for local awareness of contests and installations)
- E15 (rumor/witness propagation for coups, contests, and recognition)
- E12 (combat remains the mechanism for violence between claimants)

## Problem

Current force succession is only a conservative placeholder: if one eligible agent is present at the jurisdiction after the vacancy period, they are installed. That is acceptable as an implementation stopgap, but it is not strong long-term architecture.

It is missing four things that `docs/FOUNDATIONS.md` requires:

1. Concrete state distinguishing physical control from recognized office holding.
2. Explicit continuity state for “held uncontested for long enough.”
3. Public, local information paths for how a coup becomes known and recognized.
4. Physical dampeners preventing force succession from degenerating into an invisible instant-flip rule.

This spec fills that gap.

## Existing Infrastructure (Leveraged, Not Reimplemented)

| Infrastructure | Location | Usage in E16b |
|----------------|----------|---------------|
| `EntityKind::Office` | `entity.rs` | Office identity remains authoritative |
| `OfficeData` + `SuccessionLaw::Force` | `offices.rs` | Determines which offices use this path |
| `office_holder / offices_held` | `world/social.rs` | Recognized institutional holder; remains canonical office installation relation |
| `member_of / members_of` | `world/social.rs` | Claim eligibility and faction-backed coups |
| `hostile_to / hostility_from` | `world/social.rs` | Active political conflict and force opposition |
| `DeadAt` | `combat.rs` | Claimants and incumbents can die during contest |
| `EventTag::Political` | `event_tag.rs` | Contest, claim, control, and installation event classification |
| `VisibilitySpec::SamePlace` | `visibility.rs` | Local perception of coups and control changes |
| `WorldTxn` | `world_txn.rs` | Atomic transition boundary for contest/control/installation changes |
| `perception_system` | `worldwake-systems` | Local observation of political events |
| `Tell` and rumor propagation | E15/E15b | Remote awareness of coups and contested rule |

## Deliverables

### 1. New Office Force-State Components

#### `OfficeForceProfile`

Attached to `EntityKind::Office` for offices with `succession_law == Force`.

```rust
pub struct OfficeForceProfile {
    pub uncontested_hold_ticks: NonZeroU32,
    pub vacancy_claim_grace_ticks: NonZeroU32,
    pub challenger_presence_grace_ticks: NonZeroU32,
    pub local_recognition_threshold: Permille,
}
```

- `uncontested_hold_ticks`: how long an office must remain under one uncontested controller before installation.
- `vacancy_claim_grace_ticks`: how long a vacant force office may remain leaderless before the first uncontested controller counts as established control.
- `challenger_presence_grace_ticks`: how long a newly arrived challenger may remain before the system upgrades control to a true contested state.
- `local_recognition_threshold`: minimum support/acceptance threshold for uncontested force control to convert into recognized installation when the office design requires local acquiescence.

This keeps timing and threshold values in explicit office-local policy, not hardcoded system constants.

#### `OfficeForceState`

Attached to `EntityKind::Office`.

```rust
pub struct OfficeForceState {
    pub current_controller: Option<EntityId>,
    pub control_since: Option<Tick>,
    pub contested_since: Option<Tick>,
    pub last_uncontested_tick: Option<Tick>,
}
```

- `current_controller`: who physically controls the office seat/jurisdiction right now.
- `control_since`: when that controller first became controller.
- `contested_since`: when an active challenge first made the office contested.
- `last_uncontested_tick`: latest tick at which the office was known to be under a single uncontested controller.

Recognized office holding remains on `office_holder`. This component is about physical control, not legal office title.

### 2. New Relations

#### `contests_office / contested_by`

Stored in `RelationTables`:

```rust
pub contests_office: BTreeMap<EntityId, BTreeSet<EntityId>>, // claimant -> offices
pub contested_by: BTreeMap<EntityId, BTreeSet<EntityId>>,    // office -> claimants
```

This relation represents explicit, public participation in a force contest. A claimant is not inferred merely from being nearby; they become a claimant by declaring or sustaining a challenge.

#### `office_controller / offices_controlled`

Stored as a 1:1 relation:

```rust
pub office_controller: BTreeMap<EntityId, EntityId>,
pub offices_controlled: BTreeMap<EntityId, BTreeSet<EntityId>>,
```

This relation mirrors `OfficeForceState.current_controller` for graph-style querying and verification. It is physical control only, not recognized office title.

### 3. New Actions

#### `PressForceClaim`

- **Domain**: `ActionDomain::Social`
- **Preconditions**:
  - actor is alive
  - actor is at the office jurisdiction
  - office uses `SuccessionLaw::Force`
  - actor is eligible under office rules
- **Duration**: 1 tick
- **Effect on commit**:
  - add `contests_office(actor, office)`
  - emit visible `Political` event at jurisdiction
- **Meaning**: public declaration that the actor is actively contesting the office

#### `YieldForceClaim`

- **Domain**: `ActionDomain::Social`
- **Preconditions**:
  - actor currently contests the office
  - actor is at the office jurisdiction or holds a current belief allowing explicit withdrawal from afar via messenger/record in a later follow-up
- **Duration**: 1 tick
- **Effect on commit**:
  - remove `contests_office(actor, office)`
  - emit visible `Political` event at jurisdiction

This gives contests explicit start and stop transitions instead of inferring every state from nearby presence.

### 4. Office Control System

Add `office_force_control_system()` in `worldwake-systems`.

This may remain the `Politics` slot implementation directly, or it may be folded into the existing succession system once the old force branch is removed. No compatibility alias should remain.

Per tick, for each force office:

1. Gather all active claimants:
   - live claimants in `contested_by(office)`
   - optionally include the current recognized holder if present and physically defending the seat
2. Filter to claimants physically present at the office jurisdiction.
3. Derive one of four concrete situations:
   - **No claimants present**
   - **Exactly one claimant present**
   - **Multiple claimants present**
   - **Current controller present but challenged**
4. Update control state:
   - no claimants present:
     - clear `office_controller`
     - clear `current_controller`
     - preserve recognized `office_holder` unless the office is already vacant
   - one claimant present and office uncontrolled:
     - set that claimant as controller
     - set `control_since = tick`
     - clear `contested_since`
   - same sole controller remains:
     - keep control continuity
     - set `last_uncontested_tick = tick`
   - multiple claimants present:
     - clear `office_controller`
     - set `contested_since` if absent
     - do not install anyone
5. Installation rule:
   - if one controller has remained uncontested for `uncontested_hold_ticks`
   - and local recognition conditions are satisfied
   - then atomically:
     - set `office_holder = controller`
     - clear vacancy
     - clear other active force claims for that office
     - keep controller aligned with holder
     - emit visible installation event

### 5. Local Recognition Rule

Force control is not automatically legitimacy.

Installation requires a local recognition check derived from concrete state:

```rust
pub fn office_force_recognition(place: EntityId, office: EntityId, world: &World) -> Permille
```

Inputs may include:

- present faction supporters of the controller
- present faction supporters of competing claimants
- declared support at the office
- hostile claimants still physically present
- active guard presence once E19 exists

This function is derived only; it is never stored. It exists to answer “is the controller merely occupying the seat, or have they actually consolidated enough local compliance to be installed?”

### 6. Belief and Information Flow

The system must emit ordinary political events, not hidden system magic:

- claim pressed
- claim yielded
- control established
- control lost
- office contested
- office installed by force

All are `SamePlace` visible at the jurisdiction. Remote agents learn through E15 channels only.

### 7. Explicit Aftermath States

Force contests must leave persistent aftermath:

- hostile relations strengthened or created
- stale claims remaining until explicitly yielded or invalidated by death
- public order degradation while office is contested
- guards later inheriting a real contested-office state instead of reacting to a boolean vacancy

## Component Registration

Add to `with_component_schema_entries!`:

| Component | Kind Predicate | Storage Field |
|-----------|---------------|---------------|
| `OfficeForceProfile` | `kind == EntityKind::Office` | `office_force_profile` |
| `OfficeForceState` | `kind == EntityKind::Office` | `office_force_state` |

## SystemFn Integration

Add the force-control logic to the `Politics` system flow:

- Reads:
  - `OfficeData`
  - `OfficeForceProfile`
  - `OfficeForceState`
  - `contested_by`
  - `office_controller`
  - support declarations
  - placement and alive status
- Writes:
  - `OfficeForceState`
  - `office_controller`
  - `contests_office`
  - `office_holder`
  - visible political event records

If this spec is implemented, the provisional E16 force branch should be removed rather than kept alongside it.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path to Agent |
|-------------|--------|---------------|
| Claim announcement | `PressForceClaim` event | same-place observation -> belief update -> Tell/rumor |
| Office contested | political event emitted by control system | same-place observation -> rumor/report |
| Controller identity | control-established event + observed occupancy | same-place observation or witnessed combat aftermath |
| Installation by force | installation event at jurisdiction | same-place observation -> institutional relay |
| Failed coup / withdrawal | yield/loss-of-control event | same-place observation -> rumor/report |

No remote agent learns a coup outcome without an actual carrier of information.

### H.2 Positive-Feedback Analysis

**Loop 1: Control -> fear/compliance -> easier continued control**
- Amplifier: once one claimant controls the seat, weaker locals may stop openly contesting.

**Loop 2: Contest -> disorder -> weaker institutional response -> easier contest spread**
- Amplifier: a contested office can lower public order and make additional challenges easier.

### H.3 Concrete Dampeners

**Loop 1 dampeners**
- challengers must physically travel to the jurisdiction
- controller must remain present and uncontested for a concrete duration
- support declarations and local recognition can still block installation
- controller death, retreat, or displacement immediately breaks continuity

**Loop 2 dampeners**
- guards and office loyalists can physically intervene later through E19
- factions incur actual casualties and travel costs when contesting
- public contests create visible political evidence, enabling retaliation and coalition response
- force claims are explicit and therefore socially costly; they create persistent hostility and can reduce later compliance

### H.4 Stored State vs Derived

**Stored**
- `OfficeForceProfile`
- `OfficeForceState`
- `contests_office / contested_by`
- `office_controller / offices_controlled`
- `office_holder / offices_held` (existing)

**Derived**
- local recognition level for force installation
- whether a controller has remained uncontested long enough
- public-order impact from contested offices
- AI interpretation of who currently appears likely to win

## Invariants Enforced

- no office has more than one recognized holder
- no office has more than one current controller
- controller and recognized holder are distinct concepts
- installation by force requires explicit uncontested control continuity
- no hidden “time at place” heuristic may substitute for stored control state
- all values remain deterministic and integer/newtype-based (`Permille`, `Tick`, `NonZeroU32`)

## Acceptance Criteria

- force offices use explicit control and contest state rather than presence-only installation
- claims are explicit world actions, not inferred from arbitrary proximity alone
- a contested office remains contested until challengers leave, yield, die, or lose physically
- installation by force requires a stored uncontested hold period
- political events for claim/control/install are locally visible and belief-propagated through existing information systems
- E19 can later react to contested offices and controllers as concrete state
- the provisional E16 force shortcut is removed rather than preserved as a legacy path

## Tests

- [ ] a claimant can press a force claim only at the office jurisdiction
- [ ] force claim adds `contests_office` and emits a visible political event
- [ ] yielding a claim removes `contests_office` and emits a visible political event
- [ ] one uncontested claimant becomes controller but not immediately recognized holder
- [ ] controller continuity breaks when another claimant arrives and contests the office
- [ ] controller continuity breaks when the controller dies or leaves
- [ ] after `uncontested_hold_ticks`, a sole controller can be installed atomically as `office_holder`
- [ ] multiple simultaneous claimants keep the office contested and block installation
- [ ] `office_controller` and `office_holder` never diverge into invalid multiplicity
- [ ] remote agents do not learn contest outcomes without rumor/report propagation
- [ ] contested offices reduce derived public order once E16/E19 integration is wired

## Cross-Spec Notes

- This spec supersedes the naive “exactly one eligible agent present after the vacancy period” force branch from E16.
- E19 should depend on this spec for contested-office awareness, guard responses to coups, and patrol escalation around disputed seats.
- E17 may later use contested-office state as a justice/jurisdiction modifier when no uncontested officeholder exists.

## Critical Files To Modify

| File | Change |
|------|--------|
| `crates/worldwake-core/src/offices.rs` | add force-control state/profile types or split into dedicated office-force module |
| `crates/worldwake-core/src/component_tables.rs` | add force-state/profile storage |
| `crates/worldwake-core/src/component_schema.rs` | register new components |
| `crates/worldwake-core/src/relations.rs` | add `contests_office` and `office_controller` relations |
| `crates/worldwake-core/src/world/social.rs` | add authoritative getters/setters for claim/controller relations |
| `crates/worldwake-core/src/world_txn.rs` | add transactional mutation helpers |
| `crates/worldwake-systems/src/offices.rs` | replace provisional force branch with explicit control-state logic |
| `crates/worldwake-systems/src/office_actions.rs` | add `PressForceClaim` / `YieldForceClaim` handlers |

## Spec References

- `docs/FOUNDATIONS.md` Principles 1, 3, 7, 8, 10, 12, 16, 21, 24, 26, 28
- Section 4.5 (offices, factions, institutional propagation)
- Section 7.4 (vacancy, legitimacy, loyalty, enforcement)
- Section 9.13 (office uniqueness)
