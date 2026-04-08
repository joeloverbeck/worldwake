# PERCAPILEAK-001: Seal remaining AgentBeliefStore direct field access in perception.rs

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — `AgentBeliefStore` mutation API and perception departure projection call site
**Deps**: None (S70 — Belief Store Query Encapsulation — completed the bulk migration; this ticket addresses the one remaining access)

## Problem

S70 migrated ~24 direct field accesses in `perception.rs` to use `AgentBeliefStore` API methods. One direct field access remains: `store.known_entities.get_mut(subject)` at `perception.rs:402`, which mutates `last_known_place`, `observed_tick`, and `source` fields on a `BelievedEntityState` during departure-direction projection. This bypasses the encapsulated API and couples perception.rs to the internal field layout of `BelievedEntityState`.

## Assumption Reassessment (2026-04-08)

1. **Direct field access confirmed**: `crates/worldwake-systems/src/perception.rs:402` — `store.known_entities.get_mut(subject)` followed by direct writes to `belief.last_known_place`, `belief.observed_tick`, `belief.source` at lines 409-411. This is the only remaining `known_entities.get_mut` call in perception.rs (S70 addressed all others).
2. **S70 spec reference**: `archive/specs/S70-belief-store-query-encapsulation.md` — completed spec that added 7 accessor/mutation methods. The departure-projection case was not covered because it requires a partial update (3 fields) rather than full entity replacement via `update_entity()`.
3. **Shared boundary**: `AgentBeliefStore` in `worldwake-core/src/belief.rs` is the encapsulation boundary. The `known_entities` field is `pub` (public), which is the root cause — but making it private is a broader change. This ticket adds a targeted API method.
4. Not applicable — no failing golden scenario motivates this ticket.
5. Not applicable — not a planner/golden-driven ticket.
6. Not applicable — not an AI regression.
7. Not applicable — no ordering dependency.
8. Not applicable — no heuristic removal.
9. Not applicable — not a stale-request/contested-affordance ticket.
10. Not applicable — not a political office-claim ticket.
11. Not applicable — no ControlSource manipulation.
12. Not applicable — no golden scenario isolation.
13. No adjacent contradictions discovered.
14. No mismatch — the direct access at line 402 is confirmed present.
15. Not applicable — no authoritative arithmetic.
16. **Auto-correction — verification boundary**: ticket says departure-projection proof should rely on broad `worldwake-ai` golden suites and `cargo test --workspace`; live code already contains a focused perception unit test at `crates/worldwake-systems/src/perception.rs` named `departed_subject_with_active_travel_projects_destination_as_believed_place`, so the honest owned proof surface is `worldwake-systems` plus a new `worldwake-core` unit test for the extracted API. Correction applied: narrowed `Verification Layers`, `Acceptance Criteria`, and `Test Plan` to focused crate-level commands. Why safe: this ticket is a local encapsulation change with an existing direct behavior proof in the owning systems crate.
17. **Auto-correction — test classification**: ticket says "documentation-only ticket"; live change adds a new production API method in `crates/worldwake-core/src/belief.rs`, so a focused core unit test is the correct coverage boundary. Correction applied: `Engine Changes`, `Tests That Must Pass`, and `New/Modified Tests` now reflect code + test changes. Why safe: the change is mechanical and directly owned by this ticket.

## Architecture Check

1. Adding `update_departure_projection()` to `AgentBeliefStore` follows the same pattern S70 established for `update_believed_activity()` and `clear_believed_activity()` — targeted mutation methods that encapsulate field-level writes. This is cleaner than the alternative of making `known_entities` private (which would require auditing all callers across all crates) or leaving the field access as-is (which leaves a hole in the S70 encapsulation).
2. No backward-compatibility shims introduced. The direct field access is replaced, not wrapped.

## Verification Layers

1. Departure-projection correctness -> focused `worldwake-systems` unit test `perception::tests::departed_subject_with_active_travel_projects_destination_as_believed_place`
2. `AgentBeliefStore` API mutation correctness -> focused `worldwake-core` unit tests for `update_departure_projection`
3. API encapsulation completeness -> grep for `known_entities.get_mut` in `crates/worldwake-systems/src/perception.rs` returns zero results after change
4. Single-layer ticket — no cross-system invariant mapping needed. The change is a method extraction within the core/systems boundary.

## What to Change

### 1. Add `update_departure_projection()` to AgentBeliefStore

In `crates/worldwake-core/src/belief.rs`, add:

```rust
/// Update a known entity's believed place when an observer witnesses
/// a departure with a visible travel destination.  Returns true if
/// the entity was known and the update was applied.
pub fn update_departure_projection(
    &mut self,
    id: &EntityId,
    destination: EntityId,
    observed_tick: Tick,
) -> bool {
    if let Some(belief) = self.known_entities.get_mut(id) {
        belief.last_known_place = Some(destination);
        belief.observed_tick = observed_tick;
        belief.source = PerceptionSource::DirectObservation;
        true
    } else {
        false
    }
}
```

### 2. Replace direct field access in perception.rs

In `crates/worldwake-systems/src/perception.rs:402-414`, replace:

```rust
if let Some(belief) = store.known_entities.get_mut(subject)
    && let Some(instance) = active_by_actor.get(subject)
{
    let is_travel = action_defs
        .get(instance.def_id)
        .is_some_and(|def| def.domain == worldwake_core::ActionDomain::Travel);
    if is_travel && let Some(destination) = instance.targets.first().copied() {
        belief.last_known_place = Some(destination);
        belief.observed_tick = tick;
        belief.source = PerceptionSource::DirectObservation;
        changed = true;
    }
}
```

With:

```rust
if let Some(instance) = active_by_actor.get(subject) {
    let is_travel = action_defs
        .get(instance.def_id)
        .is_some_and(|def| def.domain == worldwake_core::ActionDomain::Travel);
    if is_travel && let Some(destination) = instance.targets.first().copied() {
        if store.update_departure_projection(subject, destination, tick) {
            changed = true;
        }
    }
}
```

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add method)
- `crates/worldwake-systems/src/perception.rs` (modify — replace direct access)

## Out of Scope

- Making `known_entities` field private (broader change affecting multiple crates)
- Other direct field accesses on `AgentBeliefStore` outside perception.rs
- Auditing other `BelievedEntityState` field accesses

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core update_departure_projection`
2. `cargo test -p worldwake-systems departed_subject_with_active_travel_projects_destination_as_believed_place`
3. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No `known_entities.get_mut` calls remain in `perception.rs` after this change.
2. Departure-direction projection behavior is identical — same fields updated, same conditions checked.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add focused unit tests for `AgentBeliefStore::update_departure_projection`
2. `crates/worldwake-systems/src/perception.rs` — existing departure-projection unit test remains the behavior proof; no scenario change needed

### Commands

1. `cargo test -p worldwake-core update_departure_projection -- --nocapture`
2. `cargo test -p worldwake-systems departed_subject_with_active_travel_projects_destination_as_believed_place -- --nocapture`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Added `AgentBeliefStore::update_departure_projection()` in `crates/worldwake-core/src/belief.rs` to encapsulate the departure-direction mutation of `last_known_place`, `observed_tick`, and `source`.
- Replaced the remaining direct `known_entities.get_mut` access in `crates/worldwake-systems/src/perception.rs` with the new belief-store API.
- Added focused core unit tests for the new API and kept the existing perception departure-projection test as the behavior proof.

## Verification Result

- Passed `cargo test -p worldwake-core update_departure_projection -- --nocapture`
- Passed `cargo test -p worldwake-systems departed_subject_with_active_travel_projects_destination_as_believed_place -- --nocapture`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
