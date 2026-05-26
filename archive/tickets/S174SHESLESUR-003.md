# S174SHESLESUR-003: Belief-view rest-site accessors

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new methods on `FacilityBeliefView` and `GoalBeliefView`, with backing implementation on `PerAgentBeliefView` and forwarding through the blanket `GoalBeliefView` impl
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (consumes `RestCapacity` and `RestOccupancy` component types)

## Problem

The two-path Sleep schema (ticket 005) needs to read rest-site capacity and occupancy through belief-view accessors that enforce the FND-14A/14B source-class discipline. Without these accessors, the sleep candidate emitter would either (a) read authoritative `RestCapacity`/`RestOccupancy` directly (violating FND-14B for remote places), or (b) miss known rest-site candidates entirely. Per S174 D5, the accessors must return co-located reads via FND-14A, belief-backed reads for remote places, and `None` when no lawful source exists.

## Assumption Reassessment (2026-05-26)

1. Verified current code: `FacilityBeliefView` trait exists at `crates/worldwake-sim/src/belief_view.rs:1582-1608` with the precedent method `self_care_occupant(entity: EntityId) -> Option<EntityId>` at line 1593. `RuntimeBeliefView` aggregator trait lives at `belief_view.rs:1616-1631` and aggregates `FacilityBeliefView` (among other sub-traits) at line 1627. The blanket `impl<T> GoalBeliefView for T where T: ... + FacilityBeliefView + ...` lives at `belief_view.rs:1688-1704` and forwards `self_care_occupant` at line 1700.
2. Spec assumption verified against S174 D5's Belief-View Surface sub-section (added during the reassessment pass that produced this ticket wave). The three new methods are `rest_site_capacity(place_id) -> Option<NonZeroU32>`, `rest_site_occupant_count(place_id) -> Option<u32>`, and `is_co_located_with_rest_site(place_id) -> bool`. Source-class discipline: capacity is public topology (FND-14B "public structural substrate"), occupancy is FND-14A when co-located + belief-backed when remote.
3. Shared abstraction boundary under audit: the `FacilityBeliefView`/`RuntimeBeliefView`/`GoalBeliefView` trait pipeline. The new methods must be added consistently across all three layers (trait definition, runtime implementation, blanket forwarding) so the AI crate can call them through the existing `GoalBeliefView` interface.
4. Existing inline tests: `per_agent_belief_view.rs` has unit tests for the `self_care_occupant` accessor pattern that establish the source-class verification idiom. The rest-site accessor tests mirror that idiom: co-located reads succeed, remote-without-belief returns `None`, and remote-with-belief reads from the belief carrier.
5. Mismatch + correction: the original spec draft D5 declared source-class semantics without enumerating the trait-surface deliverable. The reassessment pass added the Belief-View Surface sub-section enumerating the three integration points (trait, runtime impl, blanket forwarding) and this ticket implements them. No further mismatch.

## Architecture Check

1. Adding methods to `FacilityBeliefView` (rather than creating a new `RestSiteBeliefView` trait) preserves the existing aggregation pattern. A new trait would force every aggregator and consumer to import an additional symbol; method-level extension keeps the surface compact and matches the precedent set by `self_care_occupant`.
2. The source-class split (FND-14A for co-located, belief-backed for remote, public topology for capacity) is enforced inside the `PerAgentBeliefView` implementation, not at the call site. This means candidate generation (ticket 005) does not need to special-case source classes — the belief view is the authoritative arbiter of "what does this actor know?" This mirrors S173's `self_care_occupant` pattern (`S173SELCARINT-006` established the discipline).
3. `rest_site_capacity` returns `None` for places without a `RestCapacity` component, rather than returning a sentinel like `Some(0)`. `Some(_)` vs `None` is the discriminator for "this place is a known rest site" — matching the spec's contract that absence of `RestCapacity` means "not a known rest site; rough-sleep only."

## Verified Layers

1. `rest_site_capacity(co_located_place)` returns `Some(NonZeroU32::new(n))` when the place has `RestCapacity(n)` -> `per_agent_belief_view::tests::rest_site_capacity_reads_public_topology_for_local_and_remote_places`
2. `rest_site_capacity(remote_place)` returns `Some` based on public-topology state because capacity is scenario-authored and runtime-static -> `per_agent_belief_view::tests::rest_site_capacity_reads_public_topology_for_local_and_remote_places`
3. `rest_site_occupant_count(co_located_place)` reads authoritative `RestOccupancy.occupants.len()` via FND-14A -> `per_agent_belief_view::tests::rest_site_occupant_count_reads_colocated_world_state`
4. `rest_site_occupant_count(remote_place_with_belief)` reads the belief-backed contention carrier -> `per_agent_belief_view::tests::rest_site_occupant_count_uses_belief_not_remote_world_state`
5. `rest_site_occupant_count(remote_place_no_belief)` returns `None` despite authoritative remote occupancy existing -> `per_agent_belief_view::tests::rest_site_occupant_count_uses_belief_not_remote_world_state`
6. `is_co_located_with_rest_site(place)` returns `true` iff the actor is at `place` and the place has `RestCapacity` -> `per_agent_belief_view::tests::is_co_located_with_rest_site_requires_place_and_capacity`
7. Single-layer ticket boundary: belief-view trait extension is a pure read-surface addition; no authoritative state mutation landed.

## Landed Changes

### 1. Added three methods to the `FacilityBeliefView` and `GoalBeliefView` trait surfaces

In `crates/worldwake-sim/src/belief_view.rs`, the trait surfaces now include defaulted methods alongside `self_care_occupant`:

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

### 2. Implemented the three methods on `PerAgentBeliefView`

The concrete runtime implementation lives in `crates/worldwake-sim/src/per_agent_belief_view.rs`, where `PerAgentBeliefView` implements `FacilityBeliefView`. The landed behavior is:

- `rest_site_capacity`: read `RestCapacity` directly from authoritative state (capacity is public topology). For co-located places, return `Some(capacity)`; for remote places, also return `Some(capacity)` because capacity is scenario-authored topology (FND-14B's public-substrate carve-out).
- `rest_site_occupant_count`: when the actor is co-located with `place`, read `RestOccupancy.occupants.len()` from authoritative state per FND-14A. When the actor is remote, read from belief-backed contention state by treating `BelievedContentionState::grant_holder` as one occupied rest slot; return `None` if no belief carrier exists.
- `is_co_located_with_rest_site`: read the actor's `effective_place` and check whether it equals `place` AND whether `place` has a `RestCapacity` component (read authoritative since capacity is public topology).

### 3. Forwarded the three methods through the blanket `GoalBeliefView` impl

In `belief_view.rs`, the blanket `GoalBeliefView` impl forwards the new methods to `FacilityBeliefView` in parallel with `self_care_occupant`:

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

`GoalBeliefView` also declares default `None`/`false` methods so existing test fakes remain source-compatible unless they need to model rest-site state.

## Landed Files

- `crates/worldwake-sim/src/belief_view.rs` (modified — extended `FacilityBeliefView`, extended `GoalBeliefView`, added blanket forwarding)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modified — added concrete runtime implementation and focused unit tests)

## Out of Scope

- No `RestCapacity` or `RestOccupancy` component definitions (ticket 001 owns those)
- No consumer wiring in candidate generation (ticket 005 reads these accessors)
- No new test fakes or mocks beyond the focused unit tests in `belief_view.rs`
- No `BelievedContentionState::grant_holder`-equivalent extension for rest-site occupancy — the FND-14B remote-occupancy path reuses existing belief carriers; if scenario testing reveals a missing carrier, that becomes a follow-up ticket
- No social/relational rest-site facts (ownership, access rights) — those remain belief-gated per FND-14A's social-fact carve-out and are not exposed by these accessors

## Acceptance Result

### Tests Passed

1. `per_agent_belief_view::tests::rest_site_capacity_reads_public_topology_for_local_and_remote_places`
2. `per_agent_belief_view::tests::rest_site_occupant_count_reads_colocated_world_state`
3. `per_agent_belief_view::tests::rest_site_occupant_count_uses_belief_not_remote_world_state`
4. `per_agent_belief_view::tests::is_co_located_with_rest_site_requires_place_and_capacity`
5. Existing suite: `cargo test -p worldwake-sim belief_view`

### Invariants

1. `rest_site_capacity` for a remote place returns `Some(capacity)` only because capacity is public topology — never a runtime-mutable value
2. `rest_site_occupant_count` for a remote place returns `Some(n)` only when a belief carrier supplies the count; never reads authoritative `RestOccupancy.occupants` for remote places (no FND-14B violation)
3. `is_co_located_with_rest_site` is a pure read; no side effects, no mutation

## Test Plan Result

### Added Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused unit coverage for capacity, co-located occupancy, remote no-belief behavior, remote belief-backed behavior, and co-location gating

### Commands Run

1. `cargo test -p worldwake-sim rest_site -- --list`
2. `cargo test -p worldwake-sim per_agent_belief_view::tests::rest_site`
3. `cargo test -p worldwake-sim per_agent_belief_view::tests::is_co_located_with_rest_site_requires_place_and_capacity -- --exact`
4. `cargo test -p worldwake-sim belief_view`
5. `cargo test -p worldwake-sim`
6. `cargo test --workspace --quiet`

## Outcome

Completed on 2026-05-26.

- Added the S174 rest-site read surface to `FacilityBeliefView` and `GoalBeliefView`.
- Implemented the concrete source-class behavior on `PerAgentBeliefView`: public-topology capacity reads, FND-14A co-located occupancy reads, belief-backed remote occupancy reads, and rest-site co-location gating.
- Added focused unit tests proving the capacity, occupancy, remote no-leak, and co-location contracts.

## Deviations

- The concrete runtime implementation and focused tests landed in `crates/worldwake-sim/src/per_agent_belief_view.rs`, not directly inside `belief_view.rs`, because `PerAgentBeliefView` owns the actor-specific world/belief-store state required to enforce FND-14A/14B. `belief_view.rs` owns the trait declarations and blanket forwarding.
- Remote rest-site occupancy uses the existing `BelievedContentionState::grant_holder` carrier as a one-slot occupied/not-occupied belief. A richer remote count carrier remains out of scope until a later ticket proves that multi-occupant remote counts need stronger belief transport.
- `./scripts/verify.sh` is waived for this per-ticket closeout because the `implement-spec-tickets` harness owns the final pre-push verification gate for the full S174 branch.

## Verification Result

- Passed `cargo test -p worldwake-sim rest_site -- --list`
- Passed `cargo test -p worldwake-sim per_agent_belief_view::tests::rest_site`
- Passed `cargo test -p worldwake-sim per_agent_belief_view::tests::is_co_located_with_rest_site_requires_place_and_capacity -- --exact`
- Passed `cargo test -p worldwake-sim belief_view`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test --workspace --quiet`
- Waived `./scripts/verify.sh` because the full harness finalization phase owns the pre-push gate for the S174 branch.
