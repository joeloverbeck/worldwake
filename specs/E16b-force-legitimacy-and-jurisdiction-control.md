# E16b: Force Legitimacy & Jurisdiction Control

## Epic Summary

Replace the thin E16 force-succession shortcut with explicit jurisdiction-control state: public claims, contested control, uncontested hold periods, and eventual installation into office. The goal is to make "rule by force" arise from concrete local state rather than a hidden timing heuristic.

## Phase

Phase 3: Information & Politics

## Crate

`worldwake-core` (new components, relations, enums, institutional claim variant)
`worldwake-sim` (action payloads)
`worldwake-systems` (office control system, force-claim actions)
`worldwake-ai` (affordance enumeration, planner op mapping for force claims)

## Dependencies

- E16 (offices, succession laws, factions, support declarations, public order baseline)
- E16c (institutional beliefs, record consultation, belief projection pipeline)
- E14 (belief boundary for local awareness of contests and installations)
- E15 (rumor/witness propagation for coups, contests, and recognition)
- E12 (combat remains the mechanism for violence between claimants)

## Problem

Current force succession is only a conservative placeholder: if one eligible agent is present at the jurisdiction after the vacancy period, they are installed. That is acceptable as an implementation stopgap, but it is not strong long-term architecture.

It is missing four things that `docs/FOUNDATIONS.md` requires:

1. Concrete state distinguishing physical control from recognized office holding.
2. Explicit continuity state for "held uncontested for long enough."
3. Public, local information paths for how a coup becomes known and recognized.
4. Physical dampeners preventing force succession from degenerating into an invisible instant-flip rule.

This spec fills that gap.

## Existing Infrastructure (Leveraged, Not Reimplemented)

| Infrastructure | Location | Usage in E16b |
|----------------|----------|---------------|
| `EntityKind::Office` | `entity.rs` | Office identity remains authoritative |
| `OfficeData` + `SuccessionLaw::Force` | `offices.rs` | Determines which offices use this path |
| `office_holder / offices_held` | `relations.rs`, `world/social.rs` | Recognized institutional holder; remains canonical office installation relation |
| `member_of / members_of` | `world/social.rs` | Claim eligibility and faction-backed coups |
| `hostile_to / hostility_from` | `world/social.rs` | Active political conflict and force opposition |
| `DeadAt` | `combat.rs` | Claimants and incumbents can die during contest |
| `EventTag::Political` | `event_tag.rs` | Contest, claim, control, and installation event classification |
| `VisibilitySpec::SamePlace` | `visibility.rs` | Local perception of coups and control changes |
| `WorldTxn` | `world_txn.rs` | Atomic transition boundary for contest/control/installation changes |
| `perception_system` | `worldwake-systems` | Local observation of political events; projects `InstitutionalClaim` into witness beliefs |
| `Tell` and rumor propagation | E15/E15b | Remote awareness of coups and contested rule |
| `InstitutionalClaim` | `institutional.rs` | E16c institutional belief claim types; E16b adds `ForceControl` variant |
| `InstitutionalBeliefKey` | `institutional.rs` | E16c belief key types; E16b adds `ForceControllerOf` variant |
| `InstitutionalKnowledgeSource` | `institutional.rs` | Source tracking for how agents learned institutional facts |
| `RecordData` + `RecordKind::OfficeRegister` | `institutional.rs` | Political records; force-control events append entries to existing office registers |
| `GoalKind::ClaimOffice` | `goal.rs` | Existing goal kind for office claiming; E16b wires force-claim actions into this goal |
| `GoalKindTag::ClaimOffice` | `goal_model.rs` | AI planner tag; E16b adds `PressForceClaim` planner op serving this tag |

## Deliverables

### 1. New Office Force-State Components

#### `OfficeForceProfile`

Attached to `EntityKind::Office` for offices with `succession_law == Force`.

```rust
pub struct OfficeForceProfile {
    pub uncontested_hold_ticks: NonZeroU32,
    pub vacancy_claim_grace_ticks: NonZeroU32,
    pub challenger_presence_grace_ticks: NonZeroU32,
}
```

- `uncontested_hold_ticks`: how long an office must remain under one uncontested controller before installation.
- `vacancy_claim_grace_ticks`: how long a vacant force office may remain leaderless before the first uncontested controller counts as established control.
- `challenger_presence_grace_ticks`: how long a newly arrived challenger may remain before the system upgrades control to a true contested state.

This keeps timing values in explicit office-local policy, not hardcoded system constants.

#### `OfficeForceState`

Attached to `EntityKind::Office`.

```rust
pub struct OfficeForceState {
    pub control_since: Option<Tick>,
    pub contested_since: Option<Tick>,
    pub last_uncontested_tick: Option<Tick>,
}
```

- `control_since`: when the current controller (identified via `office_controller` relation) first became controller.
- `contested_since`: when an active challenge first made the office contested.
- `last_uncontested_tick`: latest tick at which the office was known to be under a single uncontested controller.

Controller identity lives in the `office_controller` relation (Deliverable 2), not in this component. This component tracks only temporal continuity. This avoids dual authoritative sources for the same fact (Principle 26).

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

Stored as a 1:1 relation (one controller per office, an agent may control multiple offices):

```rust
pub office_controller: BTreeMap<EntityId, EntityId>,            // office -> controller
pub offices_controlled: BTreeMap<EntityId, BTreeSet<EntityId>>, // controller -> offices
```

This is the single authoritative source for who physically controls an office. It is physical control only, not recognized office title.

#### WorldTxn Helpers

Add transactional mutation helpers following the existing pattern (`declare_support`, `add_hostility`):

- `txn.add_force_claim(actor, office)` — adds `contests_office(actor, office)`, records `RelationDelta`
- `txn.remove_force_claim(actor, office)` — removes `contests_office(actor, office)`, records `RelationDelta`
- `txn.set_office_controller(office, controller)` — sets `office_controller(office, controller)`, records `RelationDelta`
- `txn.clear_office_controller(office)` — clears `office_controller(office)`, records `RelationDelta`

#### World (social.rs) Helpers

Add authoritative getters/setters on `World`:

- `force_claimants_for_office(office) -> Vec<EntityId>` — read `contested_by(office)`
- `offices_contested_by(agent) -> Vec<EntityId>` — read `contests_office(agent)`
- `office_controller(office) -> Option<EntityId>` — read `office_controller`
- `offices_controlled_by(agent) -> Vec<EntityId>` — read `offices_controlled`

### 3. New Actions

#### `PressForceClaim`

- **Domain**: `ActionDomain::Social`
- **Preconditions**:
  - actor is alive
  - actor is at the office jurisdiction
  - office uses `SuccessionLaw::Force`
  - actor is eligible under office rules
  - actor does not already contest this office
- **Duration**: 1 tick (NonInterruptible)
- **Payload**:
  ```rust
  pub struct PressForceClaimActionPayload {
      pub office: EntityId,
  }
  ```
- **Effect on commit**:
  - `txn.add_force_claim(actor, office)`
  - if office has a recognized holder who is not the claimant: `txn.add_hostility(actor, holder)` — pressing a force claim against an incumbent is an act of political aggression
  - emit `InstitutionalClaim::ForceControl` as event metadata for perception projection
  - emit visible `Political` event at jurisdiction (`VisibilitySpec::SamePlace`)
- **Meaning**: public declaration that the actor is actively contesting the office

#### `YieldForceClaim`

- **Domain**: `ActionDomain::Social`
- **Preconditions**:
  - actor currently contests the office (`contests_office(actor, office)` exists)
  - actor is at the office jurisdiction
- **Duration**: 1 tick (NonInterruptible)
- **Payload**:
  ```rust
  pub struct YieldForceClaimActionPayload {
      pub office: EntityId,
  }
  ```
- **Effect on commit**:
  - `txn.remove_force_claim(actor, office)`
  - emit `InstitutionalClaim::ForceControl` as event metadata for perception projection
  - emit visible `Political` event at jurisdiction (`VisibilitySpec::SamePlace`)

This gives contests explicit start and stop transitions instead of inferring every state from nearby presence.

#### ActionPayload Integration

Add `ActionPayload::PressForceClaim(PressForceClaimActionPayload)` and `ActionPayload::YieldForceClaim(YieldForceClaimActionPayload)` variants to the `ActionPayload` enum in `action_payload.rs`, with corresponding `as_press_force_claim()` and `as_yield_force_claim()` accessor methods. Follow the existing pattern from `DeclareSupportActionPayload`.

### 4. Office Control System

Replace the provisional `resolve_force_succession()` in `worldwake-systems/src/offices.rs` with explicit force-control-state logic. The old function is removed, not kept alongside the new one (Principle 26).

Per tick, for each force office with `OfficeForceProfile`:

1. Gather all active claimants:
   - live claimants in `contested_by(office)`
   - optionally include the current recognized holder if present and physically defending the seat
2. Filter to claimants physically present at the office jurisdiction.
3. **Departure rule**: if the current `office_controller` is no longer physically present at the jurisdiction, immediately clear control. Control requires physical presence (Principle 8). `control_since` resets if/when the agent returns and regains control.
4. Derive one of four concrete situations:
   - **No claimants present**: clear `office_controller`, preserve recognized `office_holder` unless already vacant
   - **Exactly one claimant present and office uncontrolled**: set that claimant as controller via `set_office_controller`, set `control_since = tick`, clear `contested_since`
   - **Same sole controller remains**: keep control continuity, set `last_uncontested_tick = tick`
   - **Multiple claimants present**: clear `office_controller`, set `contested_since` if absent, do not install anyone
5. **Installation rule**:
   - if one controller has remained uncontested for `uncontested_hold_ticks` (derived from `control_since` and `last_uncontested_tick`)
   - and no other live claimants exist in `contested_by(office)`
   - then atomically:
     - set `office_holder = controller` via `install_office_holder()`
     - clear `vacancy_since` on `OfficeData`
     - clear all active force claims for that office (`contested_by`)
     - emit visible installation event with `InstitutionalClaim::OfficeHolder` metadata
     - append entry to jurisdiction's office register record (if one exists)

### 5. Installation Gate

Installation requires that the controller has remained the sole present claimant for `uncontested_hold_ticks` consecutive ticks with no other live claimant in `contested_by(office)`. The uncontested hold period IS the recognition mechanism.

Future specs (E19) may add additional installation gates through state-mediated checks (e.g., guard acquiescence, local faction support thresholds). These are deferred because their input data does not yet exist.

### 6. Institutional Belief Integration

Force-control state changes must propagate through the E16c institutional belief pipeline, not through hidden omniscient reads.

#### New `InstitutionalClaim` Variant

```rust
InstitutionalClaim::ForceControl {
    office: EntityId,
    controller: Option<EntityId>,
    contested: bool,
    effective_tick: Tick,
}
```

This claim encodes who physically controls a force-succession office and whether the office is actively contested. `controller: None` means no one currently controls it.

#### New `InstitutionalBeliefKey` Variant

```rust
InstitutionalBeliefKey::ForceControllerOf { office: EntityId }
```

Used to key force-control beliefs in `AgentBeliefStore.institutional_beliefs`.

#### Belief Query

Add to `AgentBeliefStore`:

```rust
pub fn believed_force_controller(
    &self,
    office: EntityId,
) -> InstitutionalBeliefRead<(Option<EntityId>, bool)>
```

Returns `(controller, contested)` from the agent's institutional beliefs. Returns `Unknown` if the agent has no force-control belief for this office. Returns `Conflicted` if multiple contradictory claims exist (e.g., from different rumor sources).

Add corresponding trait method to `GoalBeliefView` and `RuntimeBeliefView` in `belief_view.rs`.

#### Perception Wiring

Force-claim events (`PressForceClaim` commit, `YieldForceClaim` commit, control-established, control-lost, installation-by-force) emit `InstitutionalClaim::ForceControl` as event metadata. The existing perception pipeline (`institutional_claims_for_event`) extracts these claims and projects them into witness institutional beliefs with `source: InstitutionalKnowledgeSource::WitnessedEvent`.

#### Tell Propagation

Force-control claims are relayable through the existing Tell/rumor system. The `relayable_institutional_beliefs_for_subject()` method in `AgentBeliefStore` must include `ForceControllerOf` keys. Remote agents learn about coups through information carriers, not omniscience.

#### Record Integration

Force-control state changes (control established, control lost, installation) append entries to the jurisdiction's office register record (if one exists) using `RecordKind::OfficeRegister`. This allows agents to consult records to learn about force-control transitions they did not witness.

### 7. Belief and Information Flow

The system must emit ordinary political events, not hidden system magic:

- claim pressed
- claim yielded
- control established
- control lost
- office contested
- office installed by force

All are `SamePlace` visible at the jurisdiction. Remote agents learn through E15 channels only. Each event carries `InstitutionalClaim::ForceControl` metadata for belief projection.

No remote agent learns a coup outcome without an actual carrier of information.

### 8. Explicit Aftermath States

#### In-scope for E16b

- **Hostility on claim**: pressing a force claim against an incumbent creates a `hostile_to(claimant, holder)` relation. Force-claiming is an act of political aggression with persistent social consequence.
- **Stale claims**: claims in `contests_office` persist until explicitly yielded via `YieldForceClaim` or invalidated by the claimant's death. A claimant who leaves the jurisdiction retains their claim but loses physical control.
- **Contested state**: tracked in `OfficeForceState.contested_since`, queryable by any system.

#### Deferred to E19

- Public order degradation while office is contested
- Guard responses to coups and contested-office awareness
- Patrol escalation around disputed seats

### 9. AI Integration

#### Affordance Enumeration

`enumerate_press_force_claim_payloads`: returns `ActionPayload::PressForceClaim` for each force-succession office the actor is eligible for at their current location and does not already contest. Uses `RuntimeBeliefView` to check believed succession law and eligibility.

`enumerate_yield_force_claim_payloads`: returns `ActionPayload::YieldForceClaim` for each office the actor currently contests (authoritative check via `contests_office`).

#### PlannerOp Mapping

Add `PlannerOpKind::PressForceClaim` with semantics:
- **Goal relevance**: `GoalKindTag::ClaimOffice`
- **Terminal condition**: actor has pressed claim and is the sole controller for the target office (belief-level check)
- **Barriers**: not at jurisdiction (triggers travel prerequisite)
- **Mid-plan viability**: check that agent still believes the office uses force succession

Add `PlannerOpKind::YieldForceClaim` with semantics:
- **Goal relevance**: none directly (used as part of retreat/replan sequences)
- **Terminal condition**: agent no longer contests the office

#### Candidate Generation

`ClaimOffice { office }` candidates are already generated by the existing political candidate generation. E16b ensures that when an agent believes a force-succession office is vacant or held by an enemy, and the agent is eligible, the candidate generation emits `ClaimOffice` with the force-succession office. The planner then discovers `PressForceClaim` as a valid action through affordance binding.

No new `GoalKind` variant is needed. `ClaimOffice` already covers both support-law and force-law office claiming; the planner discovers the correct action type through affordances.

## Force Office Lifecycle State Machine

```
Vacant ──[PressForceClaim by A]──> A Controls (sole claimant)
   ^                                    |
   |                          [PressForceClaim by B]
   |                                    |
   |                                    v
   |                              Contested (A+B claimants, no controller)
   |                                    |
   |                          [B yields/dies/leaves; A present]
   |                                    |
   |                                    v
   |                              A Controls (uncontested)
   |                                    |
   |                          [uncontested_hold_ticks elapsed,
   |                           no live claimants in contested_by]
   |                                    |
   |                                    v
   └───────────────────────────> A Installed (office_holder = A)

At any point:
- Controller leaves jurisdiction → control clears immediately (control_since resets)
- Controller dies → claim removed from contests_office, control clears
- All claimants leave/die/yield → Vacant
- Installed holder is distinct from controller; installation clears vacancy and claims
```

## Component Registration

Add to `with_component_schema_entries!`:

| Component | Kind Predicate | Storage Field |
|-----------|---------------|---------------|
| `OfficeForceProfile` | `kind == EntityKind::Office` | `office_force_profile` |
| `OfficeForceState` | `kind == EntityKind::Office` | `office_force_state` |

## SystemFn Integration

Replace the force branch in the `Politics` system flow:

- Reads:
  - `OfficeData`
  - `OfficeForceProfile`
  - `OfficeForceState`
  - `contested_by`
  - `office_controller`
  - placement and alive status
- Writes:
  - `OfficeForceState`
  - `office_controller` / `offices_controlled`
  - `contests_office` / `contested_by`
  - `office_holder` / `offices_held`
  - `InstitutionalClaim::ForceControl` event metadata
  - visible political event records

If this spec is implemented, the provisional E16 force branch (`resolve_force_succession`) is removed rather than kept alongside it (Principle 26).

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path to Agent |
|-------------|--------|---------------|
| Claim announcement | `PressForceClaim` event | same-place observation → institutional belief projection → Tell/rumor |
| Office contested | political event emitted by control system | same-place observation → belief update → rumor/report |
| Controller identity | control-established event + `InstitutionalClaim::ForceControl` | same-place observation → institutional belief → Tell relay |
| Installation by force | installation event at jurisdiction | same-place observation → `InstitutionalClaim::OfficeHolder` projection → institutional relay |
| Failed coup / withdrawal | yield/loss-of-control event | same-place observation → belief update → rumor/report |
| Remote force-control state | Tell/rumor chain carrying `ForceControl` claim | Tell → `InstitutionalKnowledgeSource::Report { from, chain_len }` |
| Historical force transitions | Office register record consultation | `ConsultRecord` action → `InstitutionalKnowledgeSource::RecordConsultation` |

No remote agent learns a coup outcome without an actual carrier of information.

### H.2 Positive-Feedback Analysis

**Loop 1: Control -> fear/compliance -> easier continued control**
- Amplifier: once one claimant controls the seat, weaker locals may stop openly contesting.

**Loop 2: Contest -> disorder -> weaker institutional response -> easier contest spread**
- Amplifier: a contested office can lower public order and make additional challenges easier.

### H.3 Concrete Dampeners

**Loop 1 dampeners**
- challengers must physically travel to the jurisdiction
- controller must remain physically present and uncontested for a concrete duration (`uncontested_hold_ticks`)
- controller departure immediately breaks control continuity (Principle 8)
- controller death, retreat, or displacement immediately breaks continuity
- pressing a claim creates persistent hostility, making future cooperation harder

**Loop 2 dampeners**
- guards and office loyalists can physically intervene later through E19
- factions incur actual casualties and travel costs when contesting
- public contests create visible political evidence (institutional beliefs), enabling retaliation and coalition response
- force claims are explicit and therefore socially costly; they create persistent hostility and can reduce later compliance

### H.4 Stored State vs Derived

**Stored**
- `OfficeForceProfile` — per-office force timing parameters
- `OfficeForceState` — temporal continuity tracking (control_since, contested_since, last_uncontested_tick)
- `contests_office / contested_by` — explicit force-claim relations
- `office_controller / offices_controlled` — physical control relation (single authoritative source)
- `office_holder / offices_held` (existing) — recognized institutional holder
- `InstitutionalClaim::ForceControl` entries in office register records — historical force transitions
- `hostile_to` entries created by force claims — persistent political aftermath

**Derived**
- whether a controller has remained uncontested long enough (computed from `control_since` + `last_uncontested_tick` vs `uncontested_hold_ticks`)
- public-order impact from contested offices (deferred to E19)
- AI interpretation of who currently appears likely to win
- `InstitutionalBeliefRead<(Option<EntityId>, bool)>` for force controller beliefs (per-agent derived from their institutional beliefs)

## Invariants Enforced

- no office has more than one recognized holder (`office_holder` is 1:1)
- no office has more than one current controller (`office_controller` is 1:1)
- controller and recognized holder are distinct concepts stored in separate relations
- controller identity is stored exclusively in the `office_controller` relation, never duplicated in a component field (Principle 26)
- installation by force requires explicit uncontested control continuity
- physical presence at jurisdiction is required to hold control; departure clears control immediately (Principle 8)
- no hidden "time at place" heuristic may substitute for stored control state
- all values remain deterministic and integer/newtype-based (`Permille`, `Tick`, `NonZeroU32`)
- force-control state propagates through institutional belief channels, not omniscient reads (Principle 12/13)

## Acceptance Criteria

- force offices use explicit control and contest state rather than presence-only installation
- claims are explicit world actions, not inferred from arbitrary proximity alone
- a contested office remains contested until challengers leave, yield, die, or lose physically
- installation by force requires a stored uncontested hold period
- political events for claim/control/install are locally visible and belief-propagated through the E16c institutional belief pipeline
- force-control beliefs propagate through Tell/rumor channels to remote agents
- pressing a force claim against an incumbent creates hostility as persistent aftermath
- AI agents can discover and plan force-claim actions through affordance enumeration and `ClaimOffice` goal
- E19 can later react to contested offices and controllers as concrete state
- the provisional E16 force shortcut (`resolve_force_succession`) is removed rather than preserved as a legacy path

## Tests

- [ ] a claimant can press a force claim only at the office jurisdiction
- [ ] pressing a force claim when not eligible fails precondition
- [ ] force claim adds `contests_office` and emits a visible political event
- [ ] yielding a claim removes `contests_office` and emits a visible political event
- [ ] one uncontested claimant becomes controller but not immediately recognized holder
- [ ] controller continuity breaks when another claimant arrives and contests the office
- [ ] controller continuity breaks when the controller dies
- [ ] controller loses control immediately upon leaving the jurisdiction
- [ ] returning to the jurisdiction after departure restarts the control clock (control_since resets)
- [ ] after `uncontested_hold_ticks`, a sole controller with no other live claimants is installed atomically as `office_holder`
- [ ] multiple simultaneous claimants keep the office contested and block installation
- [ ] `office_controller` and `office_holder` never diverge into invalid multiplicity
- [ ] pressing a claim against an incumbent creates `hostile_to` relation
- [ ] force-control events project `InstitutionalClaim::ForceControl` into witness institutional beliefs
- [ ] remote agents do not learn contest outcomes without rumor/report propagation
- [ ] force-control beliefs are relayable through Tell
- [ ] office register records contain force-control transition entries
- [ ] affordances enumerate `PressForceClaim` at correct locations for eligible agents
- [ ] affordances enumerate `YieldForceClaim` for agents with active claims
- [ ] AI agent generates `ClaimOffice` candidate for eligible vacant force office

## Cross-Spec Notes

- This spec supersedes the naive "exactly one eligible agent present after the vacancy period" force branch from E16.
- E19 should depend on this spec for contested-office awareness, guard responses to coups, and patrol escalation around disputed seats. E19 may add additional installation gates (e.g., guard acquiescence) through state-mediated checks.
- E17 may later use contested-office state as a justice/jurisdiction modifier when no uncontested officeholder exists.

## Critical Files To Modify

| File | Change |
|------|--------|
| `crates/worldwake-core/src/offices.rs` | add `OfficeForceProfile`, `OfficeForceState` types |
| `crates/worldwake-core/src/component_tables.rs` | add force-state/profile storage |
| `crates/worldwake-core/src/component_schema.rs` | register new components |
| `crates/worldwake-core/src/relations.rs` | add `contests_office`, `contested_by`, `office_controller`, `offices_controlled` |
| `crates/worldwake-core/src/world/social.rs` | add authoritative getters/setters for claim/controller relations |
| `crates/worldwake-core/src/world_txn.rs` | add `add_force_claim`, `remove_force_claim`, `set_office_controller`, `clear_office_controller` |
| `crates/worldwake-core/src/institutional.rs` | add `InstitutionalClaim::ForceControl`, `InstitutionalBeliefKey::ForceControllerOf` |
| `crates/worldwake-core/src/belief.rs` | add `believed_force_controller()` query |
| `crates/worldwake-sim/src/belief_view.rs` | add `believed_force_controller()` trait method to `GoalBeliefView` and `RuntimeBeliefView` |
| `crates/worldwake-sim/src/action_payload.rs` | add `PressForceClaim` and `YieldForceClaim` payload variants |
| `crates/worldwake-systems/src/offices.rs` | replace provisional `resolve_force_succession` with explicit control-state logic |
| `crates/worldwake-systems/src/office_actions.rs` | add `PressForceClaim` / `YieldForceClaim` handlers and registration |
| `crates/worldwake-systems/src/perception.rs` | wire `ForceControl` claims into `institutional_claims_for_event` |
| `crates/worldwake-ai/src/planner_ops.rs` | add `PressForceClaim` and `YieldForceClaim` planner op semantics |
| `crates/worldwake-ai/src/candidate_generation.rs` | ensure `ClaimOffice` candidates emit for force-succession offices |

## Spec References

- `docs/FOUNDATIONS.md` Principles 1, 3, 7, 8, 10, 12, 13, 16, 21, 24, 26, 28
- Section 4.5 (offices, factions, institutional propagation)
- Section 7.4 (vacancy, legitimacy, loyalty, enforcement)
- Section 9.13 (office uniqueness)
