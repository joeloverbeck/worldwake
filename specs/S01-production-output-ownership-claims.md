**Status**: PENDING

# Production Output Ownership Claims

## Summary
Define explicit ownership for goods produced by harvest and craft actions without collapsing them directly into actor inventory. Produced goods should continue to materialize as concrete world objects on the ground, but they must acquire an explicit owner according to facility/source policy so the simulation can distinguish ownership, possession, custody, and unlawful taking.

This spec closes a foundational gap left by the current Phase 2 implementation: harvest/craft outputs are materialized as unpossessed and unowned lots. That makes opportunistic taking possible, but it erases claim state exactly where later systems need it most.

The correct architectural direction is:
- keep progress barriers and ground materialization
- add explicit output ownership at production commit
- make lawful `pick_up` respect claim/control
- extend `can_exercise_control()` for institutional delegation (faction membership, office holding)
- reserve unauthorized transfer for explicit theft-style actions later

This spec establishes ownership semantics, not final merchant stock custody semantics. If the project later wants merchants, carriers, and institutions to deliver into explicit stock rooms or display stalls instead of treating destination-local carried stock as sufficient, that follow-on architecture is specified in [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md).

## Why This Exists
Current production commit behavior creates output lots and places them at the workstation location, but assigns no owner. This has three architectural costs:

1. It collapses the distinction between ownership and possession for newly produced goods.
2. It prevents later discovery/crime systems from knowing whether a ground lot was merely available or actually stolen.
3. It forces agents to reason about "food exists here" rather than "my produced goods are still here," which weakens expectation-based discovery.

Foundational alignment:
- FND-04: creation and transfer should remain explicit and traceable
- FND-15: surprise should come from violated expectation, which requires claim state
- FND-22: ownership, custody, access, and capability must remain distinct
- FND-23: offices/factions need ownable assets if institutions are to act through world state

## Phase
Phase 3: Information & Politics, Step 10 (parallel after E14)

## Crates
- `worldwake-core`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai`

## Dependencies
- E14 (perception/beliefs) — **implemented and archived**
- E15 (rumor/witness/discovery) — **implemented and archived**
- E16 (offices/succession/factions) — **implemented and archived**

`PerAgentBeliefView` exists. `OmniscientBeliefView` is deleted. The belief-based affordance filtering for ownership is achievable now. Faction membership queries (`factions_of()` in `social.rs:60`) and office holding queries (`offices_held_by()` in `social.rs:205`) are both available in the relation layer.

## Design Goals
1. Preserve concrete world materialization of outputs.
2. Preserve progress barriers after production.
3. Assign explicit ownership at the moment outputs are created.
4. Keep ownership policy contextual to the producer/source, not hardcoded into recipes.
5. Prevent lawful transport actions from bypassing theft semantics once goods are claimed.
6. Keep all interactions state-mediated and debuggable.
7. Avoid compatibility shims or silent fallback behavior.
8. Record ownership assignment in committed event deltas for traceability.
9. Enable institutional delegation so faction members and office holders can lawfully retrieve production output owned by their faction or office.

## Deliverables

### 1. `ProductionOutputOwnershipPolicy` Component
Attach explicit output-claim rules to producer entities.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
enum ProductionOutputOwner {
    Actor,
    ProducerOwner,
    Unowned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
struct ProductionOutputOwnershipPolicy {
    output_owner: ProductionOutputOwner,
}

impl Component for ProductionOutputOwnershipPolicy {}
```

Attach this to entities that can produce materialized outputs:
- harvest workstations / source-bearing facilities
- craft workstations
- place-based resource sources (e.g. public berry bushes on `Place` entities)

Rationale:
- ownership of output depends on who controls the producing place or source, not on the recipe itself
- the same bread recipe should produce actor-owned bread in a personal mill and guild-owned bread in a guild bakery

### 2. Explicit Producer Claim Resolution
At production commit, each created output lot must be assigned ownership by resolving the producer policy:

- `Actor`: owner is the acting agent
- `ProducerOwner`: owner is `owner_of(producer_entity)`
- `Unowned`: no owner is assigned

If `ProducerOwner` is selected and the producer has no owner, commit must fail authoritatively. Do not silently degrade to `Unowned`.

This is deliberate:
- ownershipless institutional production should be an explicit world setup choice
- missing claim configuration should be surfaced as illegal state, not hidden by fallback logic

### 3. `WorldTxn::create_item_lot_with_owner()` Helper
Atomic convenience method that prevents the "create lot, forget to set owner" failure mode:

```rust
pub fn create_item_lot_with_owner(
    &mut self,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    owner: Option<EntityId>,
) -> Result<EntityId, WorldError>
```

Creates item lot + sets ground location + optionally sets owner in a single transactional call. Used by both harvest and craft commit handlers. The `set_owner` call within the transaction produces the standard `RelationDelta::OwnerSet` in the committed event's delta list, ensuring ownership assignment is recorded in the append-only event log for traceability and debuggability (Principle 27).

File: `crates/worldwake-core/src/world_txn.rs`

### 4. Keep Output Custody Separate From Ownership
Outputs still materialize:
- as item lots
- on the ground at the producer's place
- unpossessed at commit time

This preserves:
- visible aftermath
- interruption windows
- local competition
- explicit follow-up actions (`pick_up`, `trade`, `steal`, etc.)

Ownership is added without auto-possession.

### 5. Extend `can_exercise_control()` for Institutional Delegation
The current `can_exercise_control()` (`ownership.rs:148-179`) only checks direct `actor == owner`. It does NOT traverse faction membership or office holding. This must be extended.

After the direct ownership check (line 164-168), add two new checks for unpossessed entities:

1. **Faction membership**: If the entity's owner is a Faction that the actor is a member of (`factions_of()` from `social.rs:60`), allow control.
2. **Office holding**: If the entity's owner is an Office that the actor holds (`offices_held_by()` from `social.rs:205`), allow control.

Both use existing relation tables — zero new data structures. The entity must still be unpossessed for this to apply (preserving the "possession overrides ownership" invariant).

File: `crates/worldwake-core/src/world/ownership.rs`

### 6. Lawful `pick_up` Must Respect Claim State
`pick_up` should become the lawful custody-taking action, not a universal "grab anything on the floor" action.

New lawful rule:
- actor may `pick_up` an unpossessed lot only if either:
  - the lot is unowned, or
  - `can_exercise_control(actor, lot)` succeeds

This prevents transport from bypassing ownership semantics.

Effects:
- actors may still pick up their own produced goods
- faction-owned goods may be picked up by whoever legitimately controls them (via Deliverable 5)
- office-owned goods may be picked up by the current office holder (via Deliverable 5)
- unowned goods remain freely claimable
- unauthorized taking must use explicit theft behavior later

Update both:
- authoritative validation in `action_validation.rs`
- belief-based validation in `affordance_query.rs`

### 7. Belief View Ownership Query
Add `believed_owner_of(entity) -> Option<EntityId>` to the `RuntimeBeliefView` trait so the AI planner can reason about ownership when filtering pickup affordances.

Files:
- `crates/worldwake-sim/src/belief_view.rs` (trait definition)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (implementation)

This enables the planner to distinguish "unowned ground lot I can freely pick up" from "faction-owned lot I can pick up as a member" from "someone else's lot I cannot lawfully take."

### 8. Ownership-Aware Affordance Filtering for `pick_up`
The `pick_up` action definition's preconditions should include an ownership check. Currently it only checks `TargetUnpossessed`. Add a constraint that filters out owned lots the actor cannot control, using the belief view's `believed_owner_of()` and `can_control()` methods.

File: `crates/worldwake-systems/src/transport_actions.rs`

### 9. Theft Becomes the Unauthorized Transfer Path
Once ownership-aware outputs exist, unauthorized acquisition of owned-but-unpossessed goods must not piggyback on lawful `pick_up`.

E17 theft should cover:
- taking owned, unpossessed goods without control
- transferring possession without transferring ownership
- hidden/public evidence generation

This spec does not implement theft itself, but it makes the ownership boundary precise enough for E17 to operate correctly.

### 10. Institutional Ownership Compatibility
This design must support:
- actor-owned production
- faction-owned workshops
- office-owned or treasury-owned stores if represented through existing ownership relations
- unowned natural sources

Examples:
- personal campfire craft: `Actor`
- guild bakery: `ProducerOwner` (faction owns the bakery)
- public berry bush: `Unowned`
- lord-owned orchard: `ProducerOwner` (office owns the orchard)

### 11. Migration Requirement
Do not add fallback defaults.

Instead:
- update seeded production fixtures, prototype facilities, and test harness setup to assign explicit `ProductionOutputOwnershipPolicy`
- migrate all existing production scenarios to declare whether outputs are actor-owned, producer-owned, or unowned

This is the correct no-backward-compatibility migration cost.

## Component Registration
Register in authoritative schema:
- `ProductionOutputOwnershipPolicy` on `EntityKind::Facility` **and** `EntityKind::Place`

This matches the exact registration scope of `ResourceSource` (`component_schema.rs:725`: `|kind| kind == EntityKind::Facility || kind == EntityKind::Place`). Place-based resource sources (e.g. a public berry bush attached to a Place) need the same ownership policy as facility-based workstations. Registering on only `Facility` would leave Place-based resource sources with no policy, creating a silent gap.

No duplicate owner cache is permitted on item lots. Ownership remains in the existing relation layer.

## SystemFn Integration

### `worldwake-core`
- add `ProductionOutputOwnershipPolicy` component and `ProductionOutputOwner` enum to `production.rs`
- register in component schema on `Facility` and `Place`
- add `ComponentValue::ProductionOutputOwnershipPolicy` variant and corresponding `ComponentDelta`
- add macro entry in `component_tables.rs`
- extend `can_exercise_control()` with faction membership and office holding checks (`ownership.rs`)
- add `WorldTxn::create_item_lot_with_owner()` — atomic lot creation + ground placement + optional ownership assignment (`world_txn.rs`)

### `worldwake-systems`
- update harvest commit (`production_actions.rs:511-564`) to resolve `ProductionOutputOwnershipPolicy` from workstation/source, call `create_item_lot_with_owner()` with resolved owner
- update craft commit (`production_actions.rs:566-610`) to follow the same pattern
- update `pick_up` validation (`transport_actions.rs:131-178`) to require lawful claim or unowned status
- leave `put_down` semantics unchanged: it clears possession without changing ownership

### `worldwake-sim`
- add `believed_owner_of()` to `RuntimeBeliefView` trait (`belief_view.rs`)
- implement `believed_owner_of()` in `PerAgentBeliefView` (`per_agent_belief_view.rs`)
- update authoritative pickup validation in `action_validation.rs`
- update belief-based affordance filtering in `affordance_query.rs` to exclude owned lots the actor cannot control

### `worldwake-ai`
- no special-case plan shims
- planning should naturally see:
  - lawful pickup of actor-owned or controlled outputs
  - no lawful pickup path for owned goods under another claim
- later E17 theft planning can add explicit unauthorized acquisition paths without rewriting production semantics

## Cross-System Interactions (Principle 12)
- production writes owned output lots into world state
- transport reads ownership/control state to decide lawful pickup
- AI reads lawful affordances only
- theft later reads the same ownership/control state to provide unlawful transfer behavior
- discovery later reads missing owned goods / custody mismatch to infer theft
- offices/factions act through ownership of producer entities and institutional delegation in `can_exercise_control()`

No direct system-to-system calls are introduced. All influence travels through:
- item lots
- ownership relations
- possession relations
- faction membership relations
- office holding relations
- facility ownership policy
- emitted events with `RelationDelta::OwnerSet` deltas

## Existing Functions to Reuse

| Function | Location | Purpose |
|----------|----------|---------|
| `can_exercise_control()` | `ownership.rs:148` | Base control check to extend |
| `factions_of()` | `social.rs:60` | Get actor's faction memberships |
| `offices_held_by()` | `social.rs:205` | Get actor's held offices |
| `owner_of()` | `ownership.rs:7` | Get entity owner |
| `set_owner()` | `ownership.rs:96` | Set ownership relation |
| `create_item_lot()` | `world_txn.rs:240` | Base lot creation to wrap |
| `set_ground_location()` | `world_txn.rs` | Place lot at location |

## FND-01 Section H

### Information-Path Analysis
- Ownership of produced output is not inferred from UI or planner state; it is written into authoritative world state at production commit via `set_owner()` inside `create_item_lot_with_owner()`.
- The ownership assignment is recorded as a `RelationDelta::OwnerSet` in the committed event's delta list, making it traceable in the append-only event log.
- Lawful pickup visibility is local and derived from current place plus current control relation.
- Later theft discovery can rely on explicit mismatch between expected owned goods and observed custody/location.
- Institutional ownership travels through existing faction membership (`factions_of()`) and office holding (`offices_held_by()`) relations, not through a manager singleton. This is a multi-hop path: production commit assigns ownership to faction/office entity -> agent queries faction membership or office holding -> `can_exercise_control()` traverses both to determine lawful access. No singleton or global registry is involved.
- Belief-based ownership queries (`believed_owner_of()`) follow the same information locality as other belief queries — agents see what their belief store contains, not raw world state.

### Positive-Feedback Analysis
- Owned production can reinforce wealth accumulation: more owned facilities -> more owned output -> more wealth -> ability to acquire more facilities.
- Institutional ownership amplifies this: factions with many members can operate many facilities simultaneously.

### Concrete Dampeners
- **Transport capacity**: goods still materialize on the ground and require follow-up custody actions; carry capacity limits immediate removal.
- **Theft risk**: unauthorized transfer requires explicit theft behavior later (E17), creating material consequence for accumulation.
- **Resource depletion**: finite resource sources with regeneration rates limit production throughput regardless of ownership.
- **Travel time**: agents must physically travel to production sites; distance dampens centralized accumulation.
- **Institutional bottleneck**: producer-owner policy can create bottlenecks where only authorized actors can remove stock, limiting throughput to available faction members/office holders at that location.
- **Factional conflict**: competing factions may contest facility ownership through political or combat systems, creating external pressure on accumulation.

### Stored vs Derived State
Stored authoritative state:
- `ProductionOutputOwnershipPolicy` (component on Facility/Place entities)
- output item lots (component)
- ownership relation (`owned_by` / `property_of`)
- possession relation (`possessed_by` / `possessions_of`)
- faction membership relation (existing)
- office holding relation (existing)
- production / transfer events with `RelationDelta::OwnerSet`

Derived transient state:
- whether a local pickup is lawful for a given actor (computed from ownership + control + faction/office membership)
- whether a disappearance counts as theft vs ordinary claim of unowned goods
- whether an institution currently has recoverable claim over produced stock
- `believed_owner_of()` result in `PerAgentBeliefView` (derived from belief store contents)
- "can I lawfully pick this up?" (derived from ownership relation + faction membership + office holding)

## Invariants
- every produced output lot has explicit ownership semantics at creation
- progress barriers remain intact after production
- ownership and possession are never conflated
- lawful pickup cannot transfer custody of claimed goods to an unauthorized actor
- unauthorized acquisition of claimed goods requires an explicit theft path
- no silent fallback from `ProducerOwner` to `Unowned`
- all producer iteration and claim resolution remain deterministic
- ownership assignment is recorded in committed event deltas
- `can_exercise_control()` respects faction membership and office holding for unpossessed entities

## Tests
- [ ] harvest with `Actor` policy creates actor-owned, unpossessed ground lot
- [ ] craft with `Actor` policy creates actor-owned, unpossessed ground lot
- [ ] producer-owned workstation creates producer-owner-owned output
- [ ] `ProducerOwner` policy on ownerless producer fails commit rather than degrading silently
- [ ] `create_item_lot_with_owner()` creates lot + sets ground location + sets owner atomically
- [ ] `create_item_lot_with_owner()` with `None` owner creates unowned lot
- [ ] lawful `pick_up` succeeds for actor-owned local output
- [ ] lawful `pick_up` succeeds for unowned output
- [ ] lawful `pick_up` rejects owned local output when actor lacks control
- [ ] `put_down` preserves ownership while clearing possession
- [ ] travel continues to move possessed lots without changing ownership
- [ ] `can_exercise_control()` succeeds for faction member on faction-owned unpossessed entity
- [ ] `can_exercise_control()` succeeds for office holder on office-owned unpossessed entity
- [ ] `can_exercise_control()` rejects non-member on faction-owned entity
- [ ] `can_exercise_control()` rejects non-holder on office-owned entity (vacant office means no one can pick up)
- [ ] ownership assignment produces `RelationDelta::OwnerSet` in committed event deltas
- [ ] `believed_owner_of()` returns correct owner from belief store
- [ ] affordance filtering excludes pickup of owned lots actor cannot control
- [ ] golden craft/barrier scenarios still work under explicit actor-owned output
- [ ] deterministic replay remains unchanged after policy migration

## Acceptance Criteria
- production output ownership is explicit, contextual, and deterministic
- produced goods remain concrete ground objects rather than teleporting into inventory
- lawful transport respects ownership/control
- institutional delegation allows faction members and office holders to retrieve faction/office-owned output
- theft-ready semantics exist without requiring hacks in production or planning
- institutional producers can own outputs through ordinary world relations
- no compatibility alias preserves ownershipless production as the hidden default
- ownership assignment is traceable in the event log

## Notes For Active Epics
- E15 inventory-audit discovery should use this model when deciding whether missing stock indicates theft or merely unowned-goods depletion.
- E16 faction/office assets should use `ProducerOwner` on faction- or office-owned facilities. The `can_exercise_control()` extension (Deliverable 5) enables faction members and office holders to lawfully retrieve production output.
- E17 theft must treat unauthorized taking of owned, unpossessed goods as theft rather than lawful pickup.

## References
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
- [E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md)
- [E15-rumor-witness-discovery.md](/home/joeloverbeck/projects/worldwake/specs/E15-rumor-witness-discovery.md)
- [E16-offices-succession-factions.md](/home/joeloverbeck/projects/worldwake/archive/specs/E16-offices-succession-factions.md)
- [E05-relations-ownership.md](/home/joeloverbeck/projects/worldwake/archive/specs/E05-relations-ownership.md)
- [E10-production-transport.md](/home/joeloverbeck/projects/worldwake/archive/specs/E10-production-transport.md)
- [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md)
