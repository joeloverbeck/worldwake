# E15RUMWITDIS-001: Extract Shared Belief Snapshot Projection Builder

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — shared belief snapshot projection in `worldwake-core`, integration updates in `worldwake-systems`
**Deps**: `archive/tickets/completed/E14PERBEL-005.md`, `archive/tickets/completed/E14PERBEL-004.md`, `specs/E14-perception-beliefs.md`, `specs/E15-rumor-witness-discovery.md`

## Problem

`E14PERBEL-005` implemented direct-perception belief updates by projecting authoritative post-event `World` state into `BelievedEntityState` snapshots inside `crates/worldwake-systems/src/perception.rs`. That was the correct implementation choice for the ticket, but it leaves the projection logic owned by the systems crate instead of by a shared belief-model layer.

That ownership is not ideal long-term. E15 already plans additional belief-write paths:

- report / tell updates
- rumor propagation
- record consultation refresh
- later discovery refreshes

Those paths should not each re-encode how authoritative state becomes a `BelievedEntityState`. Without a shared projection builder, the codebase will drift into multiple slightly different snapshot constructors, which is exactly the kind of architectural duplication we want to eliminate early.

## Assumption Reassessment (2026-03-14)

1. No active ticket in `tickets/` currently owns extraction of the `World -> BelievedEntityState` projection logic. `E14PERBEL-006` owns AI migration, `E14PERBEL-007` owns integration tests, and `E14PERBEL-009` owns the planner/executor trait boundary, not snapshot projection ownership.
2. `specs/E15-rumor-witness-discovery.md` already requires non-perception belief writes (`Report`, `Rumor`, record consultation refresh), which will need to produce or update `AgentBeliefStore` entries using the same state-snapshot model as E14.
3. `specs/E14-perception-beliefs.md` defines `BelievedEntityState` as the shared state-snapshot belief model and explicitly says later paths should add or refresh those entries. The spec does not currently assign ownership of the projection builder itself.
4. The current projection logic in `crates/worldwake-systems/src/perception.rs` is correct in behavior but located in the wrong long-term ownership layer.
5. The authoritative data needed for the projection already lives in `worldwake-core`: `World`, `BelievedEntityState`, `CommodityKind`, wound state, placement, and death state. That makes `worldwake-core` the natural ownership point.
6. This is not the same issue as the mixed `BeliefView` boundary tracked by `E14PERBEL-009`. The trait-boundary cleanup and the shared snapshot builder are adjacent, but independent.

## Architecture Check

1. The clean architecture is one canonical authoritative projection path from `World` into `BelievedEntityState`, owned near the authoritative model, then reused by perception, reports, rumors, and consultation refreshes.
2. This is cleaner than leaving projection logic in `worldwake-systems` because the belief snapshot model is not system-specific. It is a cross-epic data contract.
3. This is also cleaner than duplicating builder helpers separately for perception and E15 report/consultation logic. One projection function prevents silent schema drift when `BelievedEntityState` grows.
4. No backwards-compatibility aliasing should be introduced. The existing systems-local helper should be removed after the shared builder exists; callers should migrate directly.

## What to Change

### 1. Move canonical snapshot projection into `worldwake-core`

Introduce a shared authoritative helper owned by `worldwake-core` for constructing `BelievedEntityState` from current world state.

Acceptable shapes include:

```rust
pub fn build_believed_entity_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) -> Option<BelievedEntityState>
```

or an equivalent `World` method / dedicated module API.

Requirements:

- authoritative place comes from `World::effective_place`
- authoritative alive/dead state comes from current world state
- wounds come from the authoritative wound component
- inventory snapshot logic is canonical and deterministic
- the helper returns `None` for entities that do not currently exist / are not projectable

### 2. Remove systems-local projection ownership

Update `crates/worldwake-systems/src/perception.rs` to call the shared builder rather than owning its own projection logic.

Any systems-local helper that duplicates the projection contract should be deleted once callers are migrated.

### 3. Define extension rules for E15 sources

The shared builder should support all current E14/E15 source kinds without cloning projection code:

- `DirectObservation`
- `Report { from, chain_len }`
- `Rumor { chain_len }`
- future record-consultation refreshes using the same snapshot model

That does not mean E15 must be implemented in this ticket. It means the projection API must be shaped so E15 can reuse it directly.

### 4. Correct active planning docs if needed

If implementation reveals that an active spec or ticket still implies system-local projection ownership or ad hoc belief-entry construction, update that doc so future work points at the shared builder rather than duplicating logic.

The likely review points are:

- `specs/E14-perception-beliefs.md`
- `specs/E15-rumor-witness-discovery.md`

Only make the minimal doc changes needed to keep ownership clear.

## Files to Touch

- `crates/worldwake-core/src/` (new or modify — shared belief snapshot projection module/API)
- `crates/worldwake-core/src/lib.rs` (modify — export the shared builder if needed)
- `crates/worldwake-systems/src/perception.rs` (modify — consume shared builder, delete local duplication)
- `specs/E14-perception-beliefs.md` (modify if ownership wording needs correction)
- `specs/E15-rumor-witness-discovery.md` (modify if ownership wording needs correction)

## Out of Scope

- Implementing E15 rumor/report/tell mechanics themselves
- Redesigning `BeliefView` or planner/executor boundaries (`E14PERBEL-009`)
- Expanding `BelievedEntityState` schema beyond what current specs already require
- Adding compatibility wrappers that keep both systems-local and shared projection paths alive
- Refactoring unrelated AI planning code

## Acceptance Criteria

### Tests That Must Pass

1. A shared `worldwake-core` projection helper constructs the same `BelievedEntityState` shape currently expected by perception behavior.
2. `crates/worldwake-systems/src/perception.rs` uses the shared builder instead of a systems-local duplicate.
3. Projection remains deterministic for the same world state, tick, and source.
4. Inventory / place / alive / wounds projection behavior stays unchanged for the existing perception tests.
5. Existing suite: `cargo test -p worldwake-systems`
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo clippy --workspace`
8. Existing suite: `cargo test --workspace`

### Invariants

1. There is exactly one canonical authoritative `World -> BelievedEntityState` projection path in production code.
2. Future E15 belief-write paths can reuse that path rather than re-encoding snapshot semantics.
3. The systems crate does not become the long-term owner of belief-model projection semantics.
4. No backwards-compatibility alias preserves duplicated projection helpers after migration.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/<belief projection module>.rs` — unit tests for authoritative projection of place, inventory, life/death, wounds, and deterministic source/tick stamping.
   Rationale: the snapshot builder becomes a shared contract and needs direct coverage at its ownership point.
2. `crates/worldwake-systems/src/perception.rs` — update existing perception tests to rely on the shared builder through behavior, not on a systems-local constructor.
   Rationale: ensures the perception path stays correct after ownership moves.
3. If docs are updated: no doc-specific tests, but cross-reference review must confirm E14/E15 now point future work at the shared builder.
   Rationale: prevents E15 from reintroducing duplicate projection logic by specification drift.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
