# E05: Relations, Placement, Ownership & Reservations

## Epic Summary
Implement the authoritative relation layer for:
- physical placement
- containment
- possession
- ownership
- reservation
- office holding
- basic social / knowledge relations

This epic is where Phase 1 stops being a bag of components and becomes a legal world state.

## Phase
Phase 1: World Legality

## Crate
`worldwake-core`

## Dependencies
- E03 (typed world model)
- E04 (goods, unique items, and container rules)

## Why this revision exists
The original version made `LocatedIn` and `ContainedBy` mutually exclusive for items. That is too weak for this project.

An apple inside a crate inside the store still needs:
- an immediate holder (`ContainedBy`)
- an effective place (`LocatedIn`)
- possibly a possessor
- possibly an owner
- possibly a reservation

If those semantics are not separated cleanly now, T01, T04, T13, trade legality, theft legality, and later witness logic all get muddy.

## Deliverables

### TickRange
Define reservation windows as half-open intervals:
- `TickRange { start: Tick, end: Tick }`

Rules:
- `end > start`
- overlap means `[a.start, a.end)` intersects `[b.start, b.end)`
- adjacent windows where `a.end == b.start` do **not** conflict

### ReservationId and FactId
Introduce stable ids for relation-side records that need durable references.

- `ReservationId(u64)`
- `FactId(u64)`

Rules:
- ids are monotonic and serializable
- reservation ids survive save/load and replay
- `FactId` is only an opaque handle in Phase 1; belief propagation semantics land later

### Relation Ownership Model
All relation targets / sources should use `EntityId`, not hard-code ownership to agents only.

Reason:
- a person can own something
- a faction or office can own something
- a store institution may later own stock
- hostility and loyalty are not strictly person-to-person forever

### Physical Relations

#### 1. `LocatedIn(entity, place)`
Top-level effective place of a physical entity.

Rules:
- every physical entity has exactly one effective place **or** an explicit transit state
- if an entity is nested inside a container, `LocatedIn` still points to the top-level place
- this relation is what `entities_effectively_at(place)` queries

#### 2. `ContainedBy(entity, container)`
Immediate physical parent relation.

Rules:
- target must be an entity with a `Container` component
- containment is a direct relation only, not recursive
- contained entities inherit effective place from their container chain

#### 3. `PossessedBy(entity, holder)`
Immediate control / custody.

Rules:
- possession can differ from ownership
- possession is optional for ground items and unattended stock
- possession changes can occur without moving the item

#### 4. `OwnedBy(entity, owner)`
Legal or socially recognized owner.

Rules:
- owner is any `EntityId`
- ownership is optional for unclaimed world goods
- ownership changes require explicit legal transfer logic

#### 5. `ReservedBy(entity, reserver, TickRange)`
Temporary exclusive claim.

Rules:
- reservations use stable reservation ids internally
- no overlapping reservations for the same entity and time window
- reservation semantics apply to facility slots, carts, beds, unique items, and any single-use resource

### Social / Knowledge Relations
Provide deterministic storage and APIs now, even if their behavior is expanded later.

- `MemberOf(member, faction)`
- `LoyalTo(subject, target)`
- `HoldsOffice(holder, office)`
- `HostileTo(subject, target)`
- `KnowsFact(agent, fact_id)`
- `BelievesFact(agent, fact_id)`

Rules:
- `HoldsOffice` is unique per office
- social relations may be many-to-many except where documented otherwise
- knowledge / belief relations are stored here, but propagation logic lands later

### Relation Storage
Use explicit ordered tables plus reverse indices.

Examples:
- `located_in_by_entity: BTreeMap<EntityId, EntityId>`
- `contained_by_entity: BTreeMap<EntityId, EntityId>`
- `contents_by_container: BTreeMap<EntityId, BTreeSet<EntityId>>`
- `owned_by_entity: BTreeMap<EntityId, EntityId>`
- `possessed_by_entity: BTreeMap<EntityId, EntityId>`
- reservation table plus entity-to-reservations index
- office holder by office and office by holder indices

Do **not** use:
- `HashMap<RelationType, Vec<RelationInstance>>`
- untyped relation bags

### Placement / Movement APIs
Provide legal mutation helpers instead of raw row insertion:
- `set_ground_location(entity, place) -> Result<()>`
- `put_into_container(entity, container) -> Result<()>`
- `remove_from_container(entity) -> Result<()>`
- `move_container_subtree(container, new_place) -> Result<()>`
- `set_owner(entity, owner) -> Result<()>`
- `set_possessor(entity, holder) -> Result<()>`

Rules:
- moving a container updates `LocatedIn` for the whole descendant subtree
- moving an entity must preserve single effective placement
- movement helpers must reject containment cycles

### Inventory / Custody Helpers
Provide:
- `effective_place(entity) -> Option<EntityId>`
- `direct_container(entity) -> Option<EntityId>`
- `direct_contents_of(container) -> Vec<EntityId>`
- `recursive_contents_of(container) -> Vec<EntityId>`
- `entities_effectively_at(place) -> Vec<EntityId>`
- `ground_entities_at(place) -> Vec<EntityId>`
- `owner_of(entity) -> Option<EntityId>`
- `possessor_of(entity) -> Option<EntityId>`
- `can_exercise_control(actor, entity) -> Result<()>`

### Reservation API
Provide:
- `try_reserve(entity, reserver, range) -> Result<ReservationId>`
- `release_reservation(reservation_id) -> Result<()>`
- `reservations_for(entity) -> Vec<ReservationRecord>`

Requirements:
- conflict checks are deterministic
- failures return `ConflictingReservation`
- reservation tables remain valid after archival / load / replay

## Invariants Enforced
- Spec 9.4: every physical entity has one effective place at a time
- Spec 9.7: ownership and possession are distinct but consistent
- Spec 9.8: reservation exclusivity
- Spec 9.13: one office holder at a time
- Spec 9.18: containment graph is acyclic

## Tests
- [ ] T01: randomized moves never produce multiple effective locations
- [ ] T04: overlapping reservations for the same entity cannot both succeed
- [ ] T13: randomized container nesting never produces cycles
- [ ] Ownership and possession remain independently queryable
- [ ] Moving a container updates descendant `LocatedIn` relations
- [ ] `entities_effectively_at(place)` includes nested contents
- [ ] `ground_entities_at(place)` excludes nested contents
- [ ] `HoldsOffice` enforces at most one holder per office
- [ ] Social relations support intentional many-to-many cases
- [ ] Reservation windows use half-open interval semantics correctly

## Acceptance Criteria
- all five physical relation semantics are distinct and documented
- no raw relation insertion path bypasses legality helpers
- relation indices are deterministic and serializable
- containment cycles are rejected at insertion time
- the relation layer is strong enough to support event provenance in E06

## Spec References
- Section 5.4 (required relations)
- Section 5.5 (five distinct ownership / placement semantics)
- Section 9.4 (unique physical placement)
- Section 9.7 (ownership / possession consistency)
- Section 9.8 (reservation exclusivity)
- Section 9.13 (office uniqueness)
- Section 9.18 (no circular containment)
