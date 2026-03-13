**Status**: DRAFT

# Production Output Ownership Claims

## Summary
Define explicit ownership for goods produced by harvest and craft actions without collapsing them directly into actor inventory. Produced goods should continue to materialize as concrete world objects on the ground, but they must acquire an explicit owner according to facility/source policy so the simulation can distinguish ownership, possession, custody, and unlawful taking.

This spec closes a foundational gap left by the current Phase 2 implementation: harvest/craft outputs are materialized as unpossessed and unowned lots. That makes opportunistic taking possible, but it erases claim state exactly where later systems need it most.

The correct architectural direction is:
- keep progress barriers and ground materialization
- add explicit output ownership at production commit
- make lawful `pick_up` respect claim/control
- reserve unauthorized transfer for explicit theft-style actions later

## Why This Exists
Current production commit behavior creates output lots and places them at the workstation location, but assigns no owner. This has three architectural costs:

1. It collapses the distinction between ownership and possession for newly produced goods.
2. It prevents later discovery/crime systems from knowing whether a ground lot was merely available or actually stolen.
3. It forces agents to reason about “food exists here” rather than “my produced goods are still here,” which weakens expectation-based discovery.

Foundational alignment:
- FND-04: creation and transfer should remain explicit and traceable
- FND-15: surprise should come from violated expectation, which requires claim state
- FND-22: ownership, custody, access, and capability must remain distinct
- FND-23: offices/factions need ownable assets if institutions are to act through world state

## Phase
Foundational follow-on to Phase 2 systems. Do not schedule ahead of current phase gates without explicit reprioritization, but this should be implemented before E17 theft/justice is considered complete.

## Crates
- `worldwake-core`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai`

## Dependencies
- Archived E05 ownership/relations semantics
- Archived E10 production/transport
- Archived E13 decision architecture
- Active E16 offices/factions for institutional ownership use-cases
- Active E17 theft/justice for unlawful transfer once ownership-aware outputs exist

## Design Goals
1. Preserve concrete world materialization of outputs.
2. Preserve progress barriers after production.
3. Assign explicit ownership at the moment outputs are created.
4. Keep ownership policy contextual to the producer/source, not hardcoded into recipes.
5. Prevent lawful transport actions from bypassing theft semantics once goods are claimed.
6. Keep all interactions state-mediated and debuggable.
7. Avoid compatibility shims or silent fallback behavior.

## Deliverables

### 1. `ProductionOutputOwnershipPolicy` Component
Attach explicit output-claim rules to producer entities.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum ProductionOutputOwner {
    Actor,
    ProducerOwner,
    Unowned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ProductionOutputOwnershipPolicy {
    output_owner: ProductionOutputOwner,
}

impl Component for ProductionOutputOwnershipPolicy {}
```

Attach this to entities that can produce materialized outputs:
- harvest workstations / source-bearing facilities
- craft workstations

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

### 3. Keep Output Custody Separate From Ownership
Outputs still materialize:
- as item lots
- on the ground at the producer’s place
- unpossessed at commit time

This preserves:
- visible aftermath
- interruption windows
- local competition
- explicit follow-up actions (`pick_up`, `trade`, `steal`, etc.)

Ownership is added without auto-possession.

### 4. Lawful `pick_up` Must Respect Claim State
`pick_up` should become the lawful custody-taking action, not a universal “grab anything on the floor” action.

New lawful rule:
- actor may `pick_up` an unpossessed lot only if either:
  - the lot is unowned, or
  - `can_exercise_control(actor, lot)` succeeds

This prevents transport from bypassing ownership semantics.

Effects:
- actors may still pick up their own produced goods
- faction-owned goods may be picked up by whoever legitimately controls them
- unowned goods remain freely claimable
- unauthorized taking must use explicit theft behavior later

### 5. Theft Becomes the Unauthorized Transfer Path
Once ownership-aware outputs exist, unauthorized acquisition of owned-but-unpossessed goods must not piggyback on lawful `pick_up`.

E17 theft should cover:
- taking owned, unpossessed goods without control
- transferring possession without transferring ownership
- hidden/public evidence generation

This spec does not implement theft itself, but it makes the ownership boundary precise enough for E17 to operate correctly.

### 6. Institutional Ownership Compatibility
This design must support:
- actor-owned production
- faction-owned workshops
- office-owned or treasury-owned stores if represented through existing ownership relations
- unowned natural sources

Examples:
- personal campfire craft: `Actor`
- guild bakery: `ProducerOwner`
- public berry bush: `Unowned`
- lord-owned orchard: `ProducerOwner`

### 7. Migration Requirement
Do not add fallback defaults.

Instead:
- update seeded production fixtures, prototype facilities, and test harness setup to assign explicit `ProductionOutputOwnershipPolicy`
- migrate all existing production scenarios to declare whether outputs are actor-owned, producer-owned, or unowned

This is the correct no-backward-compatibility migration cost.

## Component Registration
Register in authoritative schema:
- `ProductionOutputOwnershipPolicy` on `EntityKind::Facility`

No duplicate owner cache is permitted on item lots. Ownership remains in the existing relation layer.

## SystemFn Integration

### `worldwake-core`
- add `ProductionOutputOwnershipPolicy`
- preserve existing `owner_of`, `possessor_of`, and `can_exercise_control`
- add any needed world-txn helpers for “create lot + assign owner + ground it”

### `worldwake-systems`
- update harvest commit to assign owner based on policy
- update craft commit to assign owner based on policy
- update `pick_up` validation to require lawful claim or unowned status
- leave `put_down` semantics unchanged: it clears possession without changing ownership

### `worldwake-sim`
- add any precondition/support needed for lawful pickup affordance filtering
- ensure affordance enumeration does not expose lawful `pick_up` for owned lots the actor cannot control

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
- offices/factions later act through ownership of producer entities

No direct system-to-system calls are introduced. All influence travels through:
- item lots
- ownership relations
- possession relations
- facility ownership policy
- emitted events

## FND-01 Section H

### Information-Path Analysis
- ownership of produced output is not inferred from UI or planner state; it is written into authoritative world state at production commit
- lawful pickup visibility is local and derived from current place plus current control relation
- later theft discovery can rely on explicit mismatch between expected owned goods and observed custody/location
- institutional ownership travels through existing relations, not through a manager singleton

### Positive-Feedback Analysis
- owned production can reinforce wealth accumulation, inventory stability, and institutional power
- explicit ownership also enables theft, confiscation, and enforcement responses that counterbalance accumulation

### Concrete Dampeners
- goods still materialize on the ground and require follow-up custody actions
- transport capacity still limits immediate removal
- unauthorized transfer requires explicit theft behavior later, not free pickup
- producer-owner policy can expose institutional bottlenecks where only authorized actors can remove stock
- finite resource sources and travel time continue to limit production loops

### Stored vs Derived State
Stored authoritative state:
- `ProductionOutputOwnershipPolicy`
- output item lots
- ownership relation
- possession relation
- producer ownership relation
- production / transfer events

Derived transient state:
- whether a local pickup is lawful for a given actor
- whether a disappearance counts as theft vs ordinary claim of unowned goods
- whether an institution currently has recoverable claim over produced stock

## Invariants
- every produced output lot has explicit ownership semantics at creation
- progress barriers remain intact after production
- ownership and possession are never conflated
- lawful pickup cannot transfer custody of claimed goods to an unauthorized actor
- unauthorized acquisition of claimed goods requires an explicit theft path
- no silent fallback from `ProducerOwner` to `Unowned`
- all producer iteration and claim resolution remain deterministic

## Tests
- [ ] harvest with `Actor` policy creates actor-owned, unpossessed ground lot
- [ ] craft with `Actor` policy creates actor-owned, unpossessed ground lot
- [ ] producer-owned workstation creates producer-owner-owned output
- [ ] `ProducerOwner` policy on ownerless producer fails commit rather than degrading silently
- [ ] lawful `pick_up` succeeds for actor-owned local output
- [ ] lawful `pick_up` succeeds for unowned output
- [ ] lawful `pick_up` rejects owned local output when actor lacks control
- [ ] `put_down` preserves ownership while clearing possession
- [ ] travel continues to move possessed lots without changing ownership
- [ ] golden craft/barrier scenarios still work under explicit actor-owned output
- [ ] deterministic replay remains unchanged after policy migration

## Acceptance Criteria
- production output ownership is explicit, contextual, and deterministic
- produced goods remain concrete ground objects rather than teleporting into inventory
- lawful transport respects ownership/control
- theft-ready semantics exist without requiring hacks in production or planning
- institutional producers can own outputs through ordinary world relations
- no compatibility alias preserves ownershipless production as the hidden default

## Notes For Active Epics
- E15 inventory-audit discovery should use this model when deciding whether missing stock indicates theft or merely unowned-goods depletion.
- E16 faction/office assets should use `ProducerOwner` on faction- or office-owned facilities.
- E17 theft must treat unauthorized taking of owned, unpossessed goods as theft rather than lawful pickup.

## References
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
- [E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md)
- [E15-rumor-witness-discovery.md](/home/joeloverbeck/projects/worldwake/specs/E15-rumor-witness-discovery.md)
- [E16-offices-succession-factions.md](/home/joeloverbeck/projects/worldwake/specs/E16-offices-succession-factions.md)
- [E05-relations-ownership.md](/home/joeloverbeck/projects/worldwake/archive/specs/E05-relations-ownership.md)
- [E10-production-transport.md](/home/joeloverbeck/projects/worldwake/archive/specs/E10-production-transport.md)
