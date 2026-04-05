# S50: Rights Lattice — Ownership, Access, and Jurisdiction

## Summary

Add typed rights queries alongside `can_exercise_control()` so the justice system, institutional actions, and force-control can reason about *why* an actor has control — not just *whether* they do. Migrate `OfficeData.jurisdiction` from single-place `EntityId` to multi-place `BTreeSet<EntityId>` so jurisdiction boundaries are explicit. The existing `can_exercise_control()` signature is preserved; a new `effective_rights()` function enumerates the specific rights an actor holds over an entity.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (rights types, world queries, `OfficeData.jurisdiction` migration)
- `worldwake-sim` (belief-facing rights queries on `GoalBeliefView`)
- `worldwake-systems` (jurisdiction check migration in `offices.rs`, `office_actions.rs`, perception of rights)
- `worldwake-ai` (justice candidate generation uses `believed_rights` to distinguish lawful vs unlawful)

## Dependencies

- E16b (force legitimacy) — completed
- E17 (crime/justice) — completed
- S44 (generalized contention) — completed

## Design Goals

- Add typed rights queries (`effective_rights`, `has_right`) that enumerate *which* rights an actor holds over an entity, without changing the existing `can_exercise_control()` return type
- Migrate `OfficeData.jurisdiction` to multi-place `BTreeSet<EntityId>` per P23/P24
- Enable justice-system affordances to distinguish lawful confiscation from theft
- Enable agents to reason about "I can access this but don't own it" vs "I own this but can't reach it"

## Non-Goals

- Debt, lien, or obligation tracking (deferred)
- Contract enforcement (deferred)
- Custody delegation system (deferred — no codebase substrate exists)
- Full property law simulation (this is a rights *query* layer, not a legal system)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | Rights are derived from concrete relations (possession, ownership, faction, office), not abstract scores |
| P4 (Persistent Identity) | Rights attach to specific entities with stable identity |
| P7 (Locality) | Rights queries use local knowledge — agent must know about the right to act on it |
| P23 (Roles/Offices Are World State) | Office jurisdiction becomes multi-place, matching "jurisdiction can stop at the town gate" |
| P24 (Ownership/Custody/Access/Jurisdiction Distinct) | Core motivation — typed rights enumerate which kind of control applies |
| P25 (Social Artifacts) | Jurisdiction boundaries are inspectable authoritative state |
| P26 (Systems Through State) | Rights queries read state; no cross-system calls |
| P28 (No Backward Compat) | `OfficeData.jurisdiction` migrated from `EntityId` to `BTreeSet<EntityId>` — no shim |

## Deliverables

### New Types

```rust
pub enum RightKind {
    PhysicalPossession,       // Actor directly possesses entity
    Ownership,                // Actor is legal owner (entity unpossessed)
    FactionAuthority,         // Actor's faction owns entity (entity unpossessed)
    OfficeAuthority,          // Actor's office owns entity (entity unpossessed)
    JurisdictionalAuthority,  // Actor's office has jurisdiction over entity's location
    ContainerAccess,          // Actor controls the container holding entity
}

pub struct EffectiveRight {
    pub kind: RightKind,
    pub via: Option<EntityId>,  // Intermediary (faction, office, or container entity)
}
```

`RightKind` and `EffectiveRight` must derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### OfficeData Migration

```rust
// Before (crates/worldwake-core/src/offices.rs)
pub struct OfficeData {
    pub title: String,
    pub jurisdiction: EntityId,            // Single place
    pub succession_law: SuccessionLaw,
    pub eligibility_rules: Vec<EligibilityRule>,
    pub succession_period_ticks: u64,
    pub vacancy_since: Option<Tick>,
}

// After
pub struct OfficeData {
    pub title: String,
    pub jurisdiction: BTreeSet<EntityId>,  // Multi-place jurisdiction
    pub succession_law: SuccessionLaw,
    pub eligibility_rules: Vec<EligibilityRule>,
    pub succession_period_ticks: u64,
    pub vacancy_since: Option<Tick>,
}
```

Callers migrated:
- `offices_with_jurisdiction(place, world)` in `crates/worldwake-systems/src/offices.rs`: change `office_data.jurisdiction == place` → `office_data.jurisdiction.contains(&place)`
- `office_actions.rs` (4 sites): change `office_data.jurisdiction != actor_place` → `!office_data.jurisdiction.contains(&actor_place)` and similar
- `world_txn.rs`: update office creation to accept `BTreeSet<EntityId>`
- Scenario/test office definitions: wrap single-place `entity(5)` in `BTreeSet::from([entity(5)])`
- `SAVE_FORMAT_VERSION` bump

### New Functions (on World)

```rust
/// Enumerate all rights an actor holds over an entity.
/// Returns empty vec if actor has no rights.
pub fn effective_rights(
    &self,
    actor: EntityId,
    entity: EntityId,
) -> Vec<EffectiveRight>

/// Check if actor holds a specific kind of right over entity.
pub fn has_right(
    &self,
    actor: EntityId,
    entity: EntityId,
    kind: RightKind,
) -> bool
```

`can_exercise_control()` is **unchanged** — keeps `Result<(), WorldError>` return type. Existing callers using `.is_ok()` / `.is_err()` continue to work. `effective_rights()` is the new detailed API.

### Belief-Facing Queries

New method on `GoalBeliefView`:
```rust
fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight>
```

Implementation follows the `believed_owner_of()` pattern: reads from authoritative state, filtered by agent's access/knowledge. Returns empty vec if agent doesn't know about the entity.

Used by justice candidate generation to distinguish lawful enforcement from unlawful force.

## Cross-System Interactions

- **Justice system** reads `JurisdictionalAuthority` right to determine if punishment is lawful at a location — `has_right(guard, accused, RightKind::JurisdictionalAuthority)` checks the guard's office jurisdiction includes the current place
- **Crime system** reads rights to distinguish theft (no right) from authorized access (any right kind)
- **Perception** exposes jurisdiction changes as institutional claims through existing perception propagation
- **AI candidate generation** uses `believed_rights` to generate lawful vs unlawful action variants — a guard with `JurisdictionalAuthority` can confiscate; without it, taking the item is theft
- **Office actions** use `jurisdiction.contains(&place)` for locality checks

## Profile-Driven Parameters

No new agent profiles. Jurisdiction is per-office (on `OfficeData`), not per-agent.

## Component Registration

No new components. `OfficeData.jurisdiction` field type changes from `EntityId` to `BTreeSet<EntityId>`.

## Authoritative-to-AI Impact Rule

This spec adds `effective_rights()` and `has_right()` alongside `can_exercise_control()`. The existing function is unchanged, so the impact is limited:

| Checklist Point | Status |
|----------------|--------|
| `get_affordances` still produces correct candidates | Pass — affordance code uses `can_exercise_control().is_ok()`, unchanged |
| `generate_candidates` emits right goal kinds | Pass — new `believed_rights()` adds information, doesn't remove candidates |
| `search_plan` finds valid plans | N/A — planner uses belief-facing queries, not authoritative rights |
| `BestEffort` action start handles gracefully | Pass — action validation uses `can_exercise_control()`, unchanged |
| `handle_plan_failure` replans correctly | N/A |
| Payload revalidation | N/A |
| ALL golden tests pass | Required — `OfficeData.jurisdiction` migration must preserve behavior |

## Testing

- All existing golden tests must pass after `OfficeData.jurisdiction` migration
- New golden test: jurisdiction-gated punishment — guard at jurisdiction place can punish; guard outside jurisdiction cannot
- Save/load round-trip verified after `SAVE_FORMAT_VERSION` bump

## Section H — Causal Hooks

1. **Information path**: Jurisdiction boundaries known through office records and institutional beliefs. Agents learn jurisdiction through `ConsultRecord` or `Tell` — they cannot query jurisdiction globally. Right enumeration (`believed_rights`) filtered by agent knowledge. (P7, P15)
2. **Positive feedback**: None identified. Rights are static relations until explicitly changed by world processes (transfer, theft, office succession). (P11)
3. **Dampeners**: N/A — no amplifying loops. (P11)
4. **Stored vs derived**: `OfficeData.jurisdiction` (BTreeSet) is stored authoritative state. `EffectiveRight` and `Vec<EffectiveRight>` are derived at query time from existing relations (possession, ownership, faction, office, container). (P3)
5. **Tie-breaking / arbitration**: N/A — rights queries are read-only; no contention. (P9)
6. **Agent-visible aftermath**: Rights changes are relational, not physical. No physical aftermath (no blood trail, no disturbance). Institutional changes (office succession, jurisdiction change) propagate through existing institutional belief system. (P10)
7. **Boundary processes**: N/A — rights are local to the simulated world. (P13)
