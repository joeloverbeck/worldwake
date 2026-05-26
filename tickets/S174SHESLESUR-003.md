# S174SHESLESUR-003: Belief-view rest-site accessors

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new methods on `FacilityBeliefView` trait with backing implementation on `RuntimeBeliefView` and forwarding through the blanket `GoalBeliefView` impl
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (consumes `RestCapacity` and `RestOccupancy` component types)

## Problem

The two-path Sleep schema (ticket 005) needs to read rest-site capacity and occupancy through belief-view accessors that enforce the FND-14A/14B source-class discipline. Without these accessors, the sleep candidate emitter would either (a) read authoritative `RestCapacity`/`RestOccupancy` directly (violating FND-14B for remote places), or (b) miss known rest-site candidates entirely. Per S174 D5, the accessors must return co-located reads via FND-14A, belief-backed reads for remote places, and `None` when no lawful source exists.

## Assumption Reassessment (2026-05-26)

1. Verified current code: `FacilityBeliefView` trait exists at `crates/worldwake-sim/src/belief_view.rs:1582-1608` with the precedent method `self_care_occupant(entity: EntityId) -> Option<EntityId>` at line 1593. `RuntimeBeliefView` aggregator trait lives at `belief_view.rs:1616-1631` and aggregates `FacilityBeliefView` (among other sub-traits) at line 1627. The blanket `impl<T> GoalBeliefView for T where T: ... + FacilityBeliefView + ...` lives at `belief_view.rs:1688-1704` and forwards `self_care_occupant` at line 1700.
2. Spec assumption verified against S174 D5's Belief-View Surface sub-section (added during the reassessment pass that produced this ticket wave). The three new methods are `rest_site_capacity(place_id) -> Option<NonZeroU32>`, `rest_site_occupant_count(place_id) -> Option<u32>`, and `is_co_located_with_rest_site(place_id) -> bool`. Source-class discipline: capacity is public topology (FND-14B "public structural substrate"), occupancy is FND-14A when co-located + belief-backed when remote.
3. Shared abstraction boundary under audit: the `FacilityBeliefView`/`RuntimeBeliefView`/`GoalBeliefView` trait pipeline. The new methods must be added consistently across all three layers (trait definition, runtime implementation, blanket forwarding) so the AI crate can call them through the existing `GoalBeliefView` interface.
4. Existing inline tests: `belief_view.rs` has unit tests for the `self_care_occupant` accessor pattern that establish the source-class verification idiom. New tests for the rest-site accessors should mirror that idiom — verify co-located reads succeed, remote-without-belief returns `None`, remote-with-belief reads from the belief carrier.
5. Mismatch + correction: the original spec draft D5 declared source-class semantics without enumerating the trait-surface deliverable. The reassessment pass added the Belief-View Surface sub-section enumerating the three integration points (trait, runtime impl, blanket forwarding) and this ticket implements them. No further mismatch.

## Architecture Check

1. Adding methods to `FacilityBeliefView` (rather than creating a new `RestSiteBeliefView` trait) preserves the existing aggregation pattern. A new trait would force every aggregator and consumer to import an additional symbol; method-level extension keeps the surface compact and matches the precedent set by `self_care_occupant`.
2. The source-class split (FND-14A for co-located, belief-backed for remote, public topology for capacity) is enforced inside the `RuntimeBeliefView` implementation, not at the call site. This means candidate generation (ticket 005) does not need to special-case source classes — the belief view is the authoritative arbiter of "what does this actor know?" This mirrors S173's `self_care_occupant` pattern (`S173SELCARINT-006` established the discipline).
3. `rest_site_capacity` returns `None` for places without a `RestCapacity` component, rather than returning a sentinel like `Some(0)`. `Some(_)` vs `None` is the discriminator for "this place is a known rest site" — matching the spec's contract that absence of `RestCapacity` means "not a known rest site; rough-sleep only."

## Verification Layers

1. `rest_site_capacity(co_located_place)` returns `Some(NonZeroU32::new(n))` when the place has `RestCapacity(n)` -> focused unit test in `belief_view.rs` tests
2. `rest_site_capacity(remote_place)` returns `Some` based on public-topology belief (capacity is scenario-authored, doesn't change at runtime) -> same test, remote branch
3. `rest_site_occupant_count(co_located_place)` reads authoritative `RestOccupancy.occupants.len()` via FND-14A -> focused unit test
4. `rest_site_occupant_count(remote_place_with_belief)` reads the belief-backed count -> focused unit test
5. `rest_site_occupant_count(remote_place_no_belief)` returns `None` -> focused unit test (proves no remote authoritative leak)
6. `is_co_located_with_rest_site(place)` returns `true` iff the actor is at `place` AND the place has `RestCapacity` -> focused unit test
7. Single-layer ticket boundary: belief-view trait extension is a pure AI-layer addition; no authoritative state mutation. Additional layer mapping is not applicable.

## What to Change

### 1. Add three new methods to `FacilityBeliefView` trait

In `crates/worldwake-sim/src/belief_view.rs:1582-1608` (the `FacilityBeliefView` trait definition), add alongside the existing `self_care_occupant`:

```rust
/// Returns the rest-site capacity of a Place if known.
///
/// Public topology (FND-14B "public structural substrate"): `RestCapacity` is
/// scenario-authored topology that does not change at runtime, so agents may
/// know remote capacity through ordinary topology beliefs. Returns `None` for
/// places without `RestCapacity` (not a known rest site — rough-sleep only).
fn rest_site_capacity(&self, place: EntityId) -> Option<NonZeroU32> {
    None
}

/// Returns the current occupant count of a Place's rest site if known.
///
/// FND-14A when co-located; belief-backed otherwise. Returns `None` if the
/// place has no `RestCapacity` (not a known rest site) OR if the actor has no
/// lawful belief about remote occupancy. Mirrors `self_care_occupant`'s
/// source-class discipline.
fn rest_site_occupant_count(&self, place: EntityId) -> Option<u32> {
    None
}

/// Returns `true` iff the actor is co-located with `place` AND `place` has
/// `RestCapacity` (i.e., is a known rest site the actor could occupy right now).
fn is_co_located_with_rest_site(&self, place: EntityId) -> bool {
    false
}
```

Default implementations return `None`/`false` so non-runtime impls (test fakes, mocks) do not need to implement the methods unless they specifically model rest-site state.

### 2. Implement the three methods on `RuntimeBeliefView`

In `belief_view.rs:1616-1631` (the `RuntimeBeliefView` blanket aggregator) — actually the implementations live where `RuntimeBeliefView`'s `FacilityBeliefView` impl lives. Locate the existing `RuntimeBeliefView::self_care_occupant` impl and add three sibling impls following the same pattern:

- `rest_site_capacity`: read `RestCapacity` directly from authoritative state (capacity is public topology). For co-located places, return `Some(capacity)`; for remote places, also return `Some(capacity)` because capacity is scenario-authored topology (FND-14B's public-substrate carve-out).
- `rest_site_occupant_count`: when the actor is co-located with `place`, read `RestOccupancy.occupants.len()` from authoritative state per FND-14A. When the actor is remote, read from belief-backed contention state (mirroring `self_care_occupant`'s `BelievedContentionState::grant_holder` pattern); return `None` if no belief carrier exists.
- `is_co_located_with_rest_site`: read the actor's `effective_place` and check whether it equals `place` AND whether `place` has a `RestCapacity` component (read authoritative since capacity is public topology).

Where the implementation requires reading the actor's identity (e.g., for the co-location check), use the same accessor pattern `RuntimeBeliefView` uses elsewhere (typically a `self.actor` or equivalent field).

### 3. Forward the three methods through the blanket `GoalBeliefView` impl

In `belief_view.rs:1688-1704` (the blanket impl), add three forwarding lines parallel to the existing `self_care_occupant` forward at line 1700:

```rust
fn rest_site_capacity(&self, place: EntityId) -> Option<NonZeroU32> {
    FacilityBeliefView::rest_site_capacity(self, place)
}
fn rest_site_occupant_count(&self, place: EntityId) -> Option<u32> {
    FacilityBeliefView::rest_site_occupant_count(self, place)
}
fn is_co_located_with_rest_site(&self, place: EntityId) -> bool {
    FacilityBeliefView::is_co_located_with_rest_site(self, place)
}
```

This requires `GoalBeliefView` to declare the three methods in its trait definition. Locate the `GoalBeliefView` trait declaration and add the method signatures with default implementations that forward to `FacilityBeliefView` (matching the `self_care_occupant` pattern).

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — extend `FacilityBeliefView` trait, add `RuntimeBeliefView` impl, extend `GoalBeliefView` trait + blanket forwarding)

## Out of Scope

- No `RestCapacity` or `RestOccupancy` component definitions (ticket 001 owns those)
- No consumer wiring in candidate generation (ticket 005 reads these accessors)
- No new test fakes or mocks beyond the focused unit tests in `belief_view.rs`
- No `BelievedContentionState::grant_holder`-equivalent extension for rest-site occupancy — the FND-14B remote-occupancy path reuses existing belief carriers; if scenario testing reveals a missing carrier, that becomes a follow-up ticket
- No social/relational rest-site facts (ownership, access rights) — those remain belief-gated per FND-14A's social-fact carve-out and are not exposed by these accessors

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test: `rest_site_capacity` returns `Some(NonZeroU32::new(2))` for a Place with `RestCapacity(NonZeroU32::new(2).unwrap())`
2. New focused unit test: `rest_site_capacity` returns `None` for a Place without `RestCapacity`
3. New focused unit test: `rest_site_occupant_count` returns `Some(1)` when the actor is co-located with a Place whose `RestOccupancy.occupants` contains exactly one agent
4. New focused unit test: `rest_site_occupant_count` returns `None` when the actor is remote and has no belief about the place's occupancy
5. New focused unit test: `is_co_located_with_rest_site` returns `true` iff the actor's `effective_place` equals the queried place AND the place has `RestCapacity`
6. New focused unit test: `is_co_located_with_rest_site` returns `false` when the actor is at the place but the place has no `RestCapacity`
7. Existing suite: `cargo test -p worldwake-sim belief_view` passes (no regression on existing accessors)

### Invariants

1. `rest_site_capacity` for a remote place returns `Some(capacity)` only because capacity is public topology — never a runtime-mutable value
2. `rest_site_occupant_count` for a remote place returns `Some(n)` only when a belief carrier supplies the count; never reads authoritative `RestOccupancy.occupants` for remote places (no FND-14B violation)
3. `is_co_located_with_rest_site` is a pure read; no side effects, no mutation

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` (extend existing `FacilityBeliefView` test module) — focused unit coverage for all 7 test cases above

### Commands

1. `cargo test -p worldwake-sim belief_view::tests::rest_site` (new tests)
2. `cargo test -p worldwake-sim belief_view` (full belief_view module regression)
3. `cargo test --workspace` (full suite — no consumer wires the new accessors yet, so regression risk is low)
4. `./scripts/verify.sh` (final pre-PR gate)
