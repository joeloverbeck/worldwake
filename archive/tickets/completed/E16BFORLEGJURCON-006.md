# E16BFORLEGJURCON-006: Complete force-control institutional belief projection

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — institutional types (core), belief store (core), belief views/traces (sim), perception + political event metadata (systems)
**Deps**: E16BFORLEGJURCON-004, E16BFORLEGJURCON-005, E16c (institutional belief pipeline exists)

## Problem

Force-control state changes already exist authoritatively, but they still do not project through the E16c institutional belief pipeline. Agents can reason about office holders and support declarations through witnessed events, reports, and records; force control is the remaining gap. Without a canonical `InstitutionalClaim::ForceControl` path, coups and contested control remain invisible to the existing institutional belief / Tell / record-consultation surfaces.

## Assumption Reassessment (2026-03-22)

1. `OfficeForceProfile`, `OfficeForceState`, `contests_office`, and `office_controller` already exist in `worldwake-core`, and `resolve_force_succession()` in `crates/worldwake-systems/src/offices.rs` already drives authoritative force-control transitions. The ticket must not restate those structures as unimplemented scope.
2. `PressForceClaim` / `YieldForceClaim` payloads, handlers, validation, and tests already exist in `crates/worldwake-sim/src/action_payload.rs` and `crates/worldwake-systems/src/office_actions.rs`.
3. `InstitutionalClaim` in `institutional.rs` still only has `OfficeHolder`, `FactionMembership`, and `SupportDeclaration`. `ForceControl` does not yet exist.
4. `InstitutionalBeliefKey` still only has `OfficeHolderOf`, `FactionMembersOf`, and `SupportFor`. `ForceControllerOf` does not yet exist.
5. `AgentBeliefStore` in `belief.rs` has institutional query helpers such as `believed_office_holder()` and relay filtering via `relayable_institutional_beliefs_for_subject()`, but no `believed_force_controller()` query.
6. `GoalBeliefView` / `RuntimeBeliefView` and `PerAgentBeliefView` expose current institutional reads for office-holder / support beliefs, but no force-controller read.
7. `institutional_claims_for_event()` in `perception.rs`, Tell relay keying in `tell_actions.rs`, record consultation keying in `consult_record_actions.rs`, and institutional knowledge trace summarization in `institutional_knowledge_trace.rs` all normalize only existing institutional claim variants. `ForceControl` must be threaded through all of those surfaces together.
8. The office-holder installation path already emits canonical `InstitutionalClaim::OfficeHolder` record updates via `WorldTxn::assign_office()`. This ticket must add a parallel `ForceControl` projection path for physical controller state without duplicating or replacing holder claims.
9. The current office control system mutates `office_controller`, `contests_office`, and `OfficeForceState`, but it does not yet emit canonical `InstitutionalClaim::ForceControl` metadata or office-register updates for controller-established / controller-lost / contested transitions. `PressForceClaim` and `YieldForceClaim` deltas alone are not enough to reconstruct current controller belief state.
9. N/A — not an AI regression, ordering, or heuristic ticket.
10. N/A — not a political closure ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no golden scenario.
13. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the exact pattern established by E16c for `OfficeHolder` claims: enum variant → belief key → belief query → perception extraction → Tell relay → record consultation / trace summarization. Force control should join that single pipeline instead of growing a parallel bespoke read path.
2. No backward-compatibility shims.
3. The current architecture with authoritative `office_controller` / `OfficeForceState` is better than the older placeholder design and matches the E16b spec: controller identity and contest continuity are concrete state, not hidden heuristics. The missing belief projection is beneficial and should be added. Introducing an alternate derived or omniscient force-control read path would be worse architecture and is out of bounds.

## Verification Layers

1. Canonical force-control transition events carry `InstitutionalClaim::ForceControl` metadata -> committed event metadata check
2. Perception extracts `ForceControl` claim from canonical event metadata and/or canonical relation deltas -> witness `AgentBeliefStore` check
3. `believed_force_controller()` returns correct `(controller, contested)` and supports contradiction handling via `InstitutionalBeliefRead` -> focused belief query test
4. `believed_force_controller()` returns `Unknown` when agent has no knowledge -> focused test
5. `ForceControllerOf` is relayable through Tell -> focused test on `relayable_institutional_beliefs_for_subject`
6. Belief view traits and institutional knowledge trace summarization expose force-controller reads -> compilation + focused tests
7. Record consultation can ingest `InstitutionalClaim::ForceControl` office-register entries -> focused consultation / trace test

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

Add `ForceControl` handling to `institutional_claims_for_event()` in `perception.rs` so that witnesses of canonical force-control events receive `ForceControllerOf` institutional beliefs.

### 6. Wire Tell relay and record/trace normalization

Ensure every institutional-claim normalization site includes `ForceControllerOf`:
- `relayable_institutional_beliefs_for_subject()` sharing / Tell propagation
- `tell_actions.rs` institutional key derivation
- `consult_record_actions.rs` institutional key derivation
- `institutional_knowledge_trace.rs` read summarization / key derivation

### 7. Add canonical `ForceControl` metadata emission at the source transitions

Ensure the authoritative sources that establish, lose, or contest controller state emit `InstitutionalClaim::ForceControl` metadata in a single canonical way. At minimum this must cover:
- `PressForceClaim` / `YieldForceClaim` action commits when they change the public contest state
- force-office controller transitions from the office control system (`controller established`, `controller lost`, `office contested`, and installation cleanup)
- office-register updates for those force-control transitions when an `OfficeRegister` exists at the jurisdiction

Do not add a second parallel belief source that infers force controller state from unrelated holder events or direct omniscient reads.

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (modify — add `ForceControl` variant and `ForceControllerOf` key)
- `crates/worldwake-core/src/belief.rs` (modify — add `believed_force_controller()` query method and relay coverage tests)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement trait method)
- `crates/worldwake-sim/src/institutional_knowledge_trace.rs` (modify — summarize `ForceControllerOf` reads)
- `crates/worldwake-systems/src/perception.rs` (modify — add `ForceControl` extraction)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — map `ForceControl` to `ForceControllerOf`)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify — map `ForceControl` to `ForceControllerOf`)
- `crates/worldwake-systems/src/office_actions.rs` and/or `crates/worldwake-systems/src/offices.rs` (modify — emit canonical `ForceControl` metadata / register updates at action-system transition sources)

## Out of Scope

- Force control state-model architecture (`OfficeForceProfile`, `OfficeForceState`, contest/controller relations, succession evaluation) — already implemented and not to be reworked here unless belief projection exposes a correctness bug
- AI affordance enumeration and planner ops — E16BFORLEGJURCON-007/008
- Golden E2E tests for belief propagation — E16BFORLEGJURCON-009
- Public order impact from contested offices — deferred to E19
- OfficeForceProfile grace-field semantics (`vacancy_claim_grace_ticks`, `challenger_presence_grace_ticks`) — no active owner in the current ticket set; requires a dedicated follow-up if these fields are to affect behavior
- Historical record integration for force-controller transitions beyond existing holder-register writes — not owned here

## Acceptance Criteria

### Tests That Must Pass

1. `InstitutionalClaim::ForceControl` / `InstitutionalBeliefKey::ForceControllerOf` can be constructed, serialized, and pattern-matched
2. Canonical force-control transition events project into witness `AgentBeliefStore` via the perception pipeline
3. `believed_force_controller()` returns correct `(Some(controller), false)` for uncontested control
4. `believed_force_controller()` returns correct `(None, true)` for contested office and supports `Conflicted` for contradictory claims
5. `believed_force_controller()` returns `Unknown` when agent has no force-control belief
6. `ForceControllerOf` beliefs are included in Tell relay, record consultation, and institutional knowledge trace summarization
7. Remote agents do NOT learn contest outcomes without rumor/report propagation (no omniscient leakage)
8. Existing relevant suite: `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`, plus focused force-control / perception / Tell / record tests

### Invariants

1. Force-control state propagates through institutional belief channels, not omniscient reads (Principle 12/13)
2. No remote agent learns a coup outcome without an actual carrier of information (Principle 7)
3. Belief sources are tracked via `InstitutionalKnowledgeSource` (witnessed vs reported)
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/institutional.rs` test module — new variant / key serde and ordering coverage
2. `crates/worldwake-core/src/belief.rs` test module — `believed_force_controller()` and relay coverage tests
3. `crates/worldwake-sim/src/per_agent_belief_view.rs` and/or `crates/worldwake-sim/src/institutional_knowledge_trace.rs` test modules — force-controller view / summary coverage
4. `crates/worldwake-systems/src/perception.rs` test module — perception extraction test for `ForceControl`
5. `crates/worldwake-systems/src/office_actions.rs` and/or `crates/worldwake-systems/src/offices.rs` test modules — canonical event metadata / record integration tests

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-22
- **What actually changed**:
  - Added `InstitutionalClaim::ForceControl` and `InstitutionalBeliefKey::ForceControllerOf`.
  - Added `AgentBeliefStore::believed_force_controller()` and exposed it through the belief-view layer.
  - Wired force-control institutional claims through perception, Tell relay, record consultation, and institutional knowledge tracing.
  - Added canonical office-register updates for force-controller transitions from the office succession system.
- **Deviations from original plan**:
  - The existing force-control state model (`OfficeForceProfile`, `OfficeForceState`, contest/controller relations, succession evaluation, and claim actions) was already implemented, so this ticket did not rework that architecture.
  - Canonical `ForceControl` projection was finalized in the office succession system rather than inferred from `PressForceClaim` / `YieldForceClaim` action deltas. That is the cleaner source because it has the full authoritative controller/contest outcome for the office.
- **Verification results**:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
