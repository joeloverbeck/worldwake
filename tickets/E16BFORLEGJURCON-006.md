# E16BFORLEGJURCON-006: Add ForceControl institutional belief variant and perception wiring

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — institutional types (core), belief store (core), belief views (sim), perception (systems)
**Deps**: E16BFORLEGJURCON-004, E16BFORLEGJURCON-005, E16c (institutional belief pipeline exists)

## Problem

Force-control state changes must propagate through the E16c institutional belief pipeline: agents learn about coups through witnessed events and rumor chains, not through omniscient reads. This requires a new `InstitutionalClaim::ForceControl` variant, a new `InstitutionalBeliefKey::ForceControllerOf` variant, a belief query method, perception wiring, and Tell relayability.

## Assumption Reassessment (2026-03-22)

1. `InstitutionalClaim` in `institutional.rs` has `OfficeHolder`, `FactionMembership`, `SupportDeclaration` variants. `ForceControl` does not exist.
2. `InstitutionalBeliefKey` has `OfficeHolderOf`, `FactionMembersOf`, `SupportFor` variants. `ForceControllerOf` does not exist.
3. `AgentBeliefStore` in `belief.rs` has `believed_office_holder()` and similar query methods. `believed_force_controller()` does not exist.
4. `GoalBeliefView` and `RuntimeBeliefView` in `belief_view.rs` have institutional query methods. `believed_force_controller()` does not exist on either trait.
5. `institutional_claims_for_event()` in `perception.rs` extracts `InstitutionalClaim` from events. It already handles `OfficeHolder`, `FactionMembership`, `SupportDeclaration` — adding `ForceControl` follows the same pattern.
6. `relayable_institutional_beliefs_for_subject()` in `AgentBeliefStore` controls Tell propagation. It must include `ForceControllerOf` keys.
7. Mismatch corrected after `E16BFORLEGJURCON-005`: the force-control state machine now exists, but it does not yet emit canonical `InstitutionalClaim::ForceControl` metadata for controller-established / controller-maintained / contested / controller-lost transitions. `PressForceClaim` and `YieldForceClaim` actions alone are not sufficient to reconstruct current controller belief state. This ticket must therefore own the canonical event-metadata source for force-control belief projection rather than assuming it already exists.
8. The existing holder-installation path already emits `InstitutionalClaim::OfficeHolder` through the office register / holder relation surfaces. This ticket should add `ForceControl` alongside, not replace or duplicate holder-claim projection.
9. N/A — not an AI regression, ordering, or heuristic ticket.
10. N/A — not a political closure ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no golden scenario.
13. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the exact pattern established by E16c for `OfficeHolder` claims: enum variant → belief key → belief query → perception extraction → Tell relay. The only additional requirement is to ensure there is one canonical event source for force-controller state transitions before perception wiring is added.
2. No backward-compatibility shims.

## Verification Layers

1. Canonical force-control transition events carry `InstitutionalClaim::ForceControl` metadata -> committed event metadata check
2. Perception extracts `ForceControl` claim -> witness `AgentBeliefStore` check
3. `believed_force_controller()` returns correct `(controller, contested)` -> focused belief query test
4. `believed_force_controller()` returns `Unknown` when agent has no knowledge -> focused test
5. `ForceControllerOf` is relayable through Tell -> focused test on `relayable_institutional_beliefs_for_subject`
6. Belief view traits expose `believed_force_controller()` -> compilation + focused test
7. Force-control events project into witness beliefs with `InstitutionalKnowledgeSource::WitnessedEvent` -> focused test

## What to Change

### 1. Add `InstitutionalClaim::ForceControl` variant

```rust
InstitutionalClaim::ForceControl {
    office: EntityId,
    controller: Option<EntityId>,
    contested: bool,
    effective_tick: Tick,
}
```

### 2. Add `InstitutionalBeliefKey::ForceControllerOf` variant

```rust
InstitutionalBeliefKey::ForceControllerOf { office: EntityId }
```

### 3. Add `believed_force_controller()` to `AgentBeliefStore`

```rust
pub fn believed_force_controller(
    &self,
    office: EntityId,
) -> InstitutionalBeliefRead<(Option<EntityId>, bool)>
```

Returns `(controller, contested)` from institutional beliefs. Returns `Unknown` if no belief exists.

### 4. Add trait method to belief views

Add `believed_force_controller()` to `GoalBeliefView` and `RuntimeBeliefView` in `belief_view.rs`, with implementations in `PerAgentBeliefView` (or wherever the concrete impl lives).

### 5. Wire perception extraction

Add `ForceControl` handling to `institutional_claims_for_event()` in `perception.rs` so that witnesses of force-claim events receive `ForceControllerOf` institutional beliefs.

### 6. Wire Tell relay

Ensure `relayable_institutional_beliefs_for_subject()` includes `ForceControllerOf` keys so force-control beliefs propagate through the Tell/rumor system.

### 7. Add canonical `ForceControl` metadata emission at the source transitions

Ensure the authoritative sources that establish, lose, or contest controller state emit `InstitutionalClaim::ForceControl` metadata in a single canonical way. At minimum this must cover:
- `PressForceClaim` / `YieldForceClaim` action commits when they change the public contest state
- force-office controller transitions from the office control system (`controller established`, `controller lost`, `office contested`, and force installation cleanup)

Do not add a second parallel belief source that infers force controller state from unrelated holder events or direct omniscient reads.

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (modify — add `ForceControl` variant and `ForceControllerOf` key)
- `crates/worldwake-core/src/belief.rs` (modify — add `believed_force_controller()` query method)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` or equivalent (modify — implement trait method)
- `crates/worldwake-systems/src/perception.rs` (modify — add `ForceControl` extraction)
- `crates/worldwake-systems/src/office_actions.rs` and/or `crates/worldwake-systems/src/offices.rs` (modify — emit canonical `ForceControl` metadata at action/system transition sources)

## Out of Scope

- Force control system logic — that's E16BFORLEGJURCON-005
- AI affordance enumeration and planner ops — E16BFORLEGJURCON-007/008
- Golden E2E tests for belief propagation — E16BFORLEGJURCON-009
- Public order impact from contested offices — deferred to E19
- OfficeForceProfile grace-field semantics (`vacancy_claim_grace_ticks`, `challenger_presence_grace_ticks`) — no active owner in the current ticket set; requires a dedicated follow-up if these fields are to affect behavior
- Historical record integration for force-controller transitions beyond existing holder-register writes — not owned here

## Acceptance Criteria

### Tests That Must Pass

1. `InstitutionalClaim::ForceControl` can be constructed and pattern-matched
2. Force-control events project into witness `AgentBeliefStore` via perception pipeline
3. `believed_force_controller()` returns correct `(Some(controller), false)` for uncontested control
4. `believed_force_controller()` returns `(None, true)` for contested office
5. `believed_force_controller()` returns `Unknown` when agent has no force-control belief
6. `ForceControllerOf` beliefs are included in `relayable_institutional_beliefs_for_subject()` output
7. Remote agents do NOT learn contest outcomes without rumor/report propagation (no omniscient leakage)
8. Existing suite: `cargo test -p worldwake-core && cargo test -p worldwake-sim && cargo test -p worldwake-systems`

### Invariants

1. Force-control state propagates through institutional belief channels, not omniscient reads (Principle 12/13)
2. No remote agent learns a coup outcome without an actual carrier of information (Principle 7)
3. Belief sources are tracked via `InstitutionalKnowledgeSource` (witnessed vs reported)
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/institutional.rs` test module — variant construction tests
2. `crates/worldwake-core/src/belief.rs` test module — `believed_force_controller()` query tests
3. `crates/worldwake-systems/src/perception.rs` test module — perception extraction test for ForceControl claims
4. Focused integration test — full perception pipeline with force-claim event → witness belief update

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace`
5. `cargo test --workspace`
