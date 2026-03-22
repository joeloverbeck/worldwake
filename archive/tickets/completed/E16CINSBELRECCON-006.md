# E16CINSBELRECCON-006: Institutional Event Projection for Witnesses

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend political-event witness projection in `worldwake-systems`
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003, E16CINSBELRECCON-004

## Problem

When an agent witnesses a visible institutional event, the agent should gain corresponding institutional beliefs without needing to consult a record. This is the witnessing acquisition path required by `specs/E16c-institutional-beliefs-and-record-consultation.md` §7. Current code already stores institutional beliefs and record-consultation beliefs, but `crates/worldwake-systems/src/perception.rs` still only projects entity snapshots and social observations from witnessed events.

## Assumption Reassessment (2026-03-22)

1. `crates/worldwake-systems/src/perception.rs` is the correct owner for witnessed event projection. It batches `AgentBeliefStore` mutations in-memory and commits them once at the end of the tick via `set_component_agent_belief_store`; this ticket should extend that batching path rather than performing per-witness transactional writes.
2. Visible political events already exist, but not as a dedicated institutional payload API. The live mutation surface is `EventRecord::state_deltas()`, especially `RelationDelta::{Added, Removed}` for `RelationKind::{OfficeHolder, SupportDeclaration}` emitted by `crates/worldwake-systems/src/offices.rs` and `crates/worldwake-systems/src/office_actions.rs`.
3. `AgentBeliefStore`, `InstitutionalClaim`, `InstitutionalBeliefKey`, and `BelievedInstitutionalClaim` already exist in `worldwake-core`, and `AgentBeliefStore::record_institutional_belief` is the right store-level insertion surface for perception-side projection.
4. N/A — not a planner ticket.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. The closure boundary under test is event-lifecycle to witness-belief projection: a committed political event with visible relation deltas must yield the correct institutional claim in witness belief state. The authoritative surfaces to check are the event-log deltas and the resulting `AgentBeliefStore::institutional_beliefs`.
9. N/A.
10. Isolation choice: focused perception tests should synthesize or replay only the political event and local witnesses needed for the claim under test, excluding unrelated AI/planner behavior and record consultation.
11. Mismatch + correction: the original ticket assumed a generic “event payload/tag to institutional claim” mapping and included faction membership witness events in scope. Current code has no visible faction-membership event producer in `worldwake-systems`, so this ticket is narrowed to the live visible political relation surfaces: office vacancy/installation and support declaration events. Faction membership witness projection should land only when an actual witnessed induction/removal event exists.
12. N/A.

## Architecture Check

1. Extending `perception.rs` is cleaner than creating a parallel institutional-perception system because witnessed event routing already lives there. The robust implementation is to derive normalized institutional claims from political relation deltas, grouped by institutional key, so one event projects its final semantic claim instead of leaking intermediate remove/add churn from transactional mutation details.
2. No backward-compatibility shims.

## Verification Layers

1. Political relation delta normalization -> focused `perception.rs` tests over committed political events and resulting institutional-belief projection
2. Information locality -> focused `perception.rs` negative test proving remote agents do not receive witness beliefs
3. Provenance / learned tick / learned place -> direct belief-store inspection in focused tests
4. Single-layer ticket: decision trace / action trace are not applicable because the contract is witness projection inside the perception system, not AI selection or action lifecycle ordering
5. Event shape compatibility with live office/support emitters -> `cargo test -p worldwake-systems` so office/perception tests run together against the real mutation surfaces

## What to Change

### 1. Extend perception system in `perception.rs`

After the existing entity and social-observation projection for witnesses, add institutional belief projection for visible political events:

- Inspect `record.state_deltas()` for `RelationKind::OfficeHolder` and `RelationKind::SupportDeclaration`.
- Normalize deltas per institutional key so a remove+add overwrite in one event projects the final claim for that key rather than both intermediate states.
- Project normalized claims into each witness store using `AgentBeliefStore::record_institutional_belief(...)` with `InstitutionalKnowledgeSource::WitnessedEvent`, `learned_tick = record.tick()`, and `learned_at = record.place_id()`.

### 2. Define mapping from political relation deltas to institutional claims

Add a helper in `perception.rs` that derives normalized `(InstitutionalBeliefKey, InstitutionalClaim)` pairs from a political event’s relation deltas:

- `OfficeHolder Added` -> `InstitutionalClaim::OfficeHolder { holder: Some(holder) }`
- `OfficeHolder Removed` with no later add for the same office in the event -> `InstitutionalClaim::OfficeHolder { holder: None }`
- `SupportDeclaration Added` -> `InstitutionalClaim::SupportDeclaration { candidate: Some(candidate) }`
- `SupportDeclaration Removed` with no later add for the same supporter/office in the event -> `InstitutionalClaim::SupportDeclaration { candidate: None }`

This keeps the mapping local, testable, and decoupled from specific action/system call sites.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add institutional belief projection for political event witnesses)

## Out of Scope

- ConsultRecord action (ticket -005)
- Tell propagation of institutional claims (ticket -008)
- Record entry mutation when political events fire (ticket -007)
- AI reading institutional beliefs (Phase B2 tickets)
- Force-claim / contested events (deferred to E16b)
- Faction membership witness projection until a visible faction induction/removal event exists in live systems
- Non-political events (combat, trade, etc.) — no institutional claims to extract here

## Acceptance Criteria

### Tests That Must Pass

1. Agent at same place as office installation gains `InstitutionalClaim::OfficeHolder { holder: Some(...) }` with `WitnessedEvent` source
2. Agent at same place as office vacancy event gains `InstitutionalClaim::OfficeHolder { holder: None }` with `WitnessedEvent` source
3. Agent at same place as support declaration gains `InstitutionalClaim::SupportDeclaration { candidate: Some(...) }` with `WitnessedEvent` source
4. Support overwrite/remove+add churn in one event projects only the final support claim for that supporter/office key
5. Agent at different place does NOT gain institutional belief from the political event
6. `learned_tick` matches the event tick and `learned_at` matches the event place
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Information locality (Principle 7): only same-place witnesses gain beliefs
2. Provenance traceability: every projected belief has `WitnessedEvent` source
3. Perception system remains stateless — no stored state, only event-driven projection
4. Event-delta normalization must not manufacture contradictory claims from one overwrite event

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — office installation witness projects `OfficeHolder(Some(holder))`
2. `crates/worldwake-systems/src/perception.rs` — office vacancy witness projects `OfficeHolder(None)`
3. `crates/worldwake-systems/src/perception.rs` — support overwrite event projects only the final `SupportDeclaration`
4. `crates/worldwake-systems/src/perception.rs` — remote non-witness does not receive the institutional belief

### Commands

1. `cargo test -p worldwake-systems perception::tests::political_event_projects_office_installation_claim_for_witness`
2. `cargo test -p worldwake-systems perception::tests::political_event_projects_office_vacancy_claim_for_witness`
3. `cargo test -p worldwake-systems perception::tests::political_event_support_overwrite_projects_only_final_claim`
4. `cargo test -p worldwake-systems perception::tests::political_event_does_not_project_institutional_claim_to_remote_agent`
5. `cargo test -p worldwake-systems`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - corrected the ticket scope to match the live architecture: political witness projection now derives institutional claims from committed political relation deltas in `crates/worldwake-systems/src/perception.rs`
  - implemented witness projection for `OfficeHolder` install/vacancy and `SupportDeclaration` events using the existing batched `AgentBeliefStore` update path
  - normalized same-event remove/add overwrite churn so a witness records the final support declaration instead of contradictory intermediate claims
  - added focused perception tests for installation, vacancy, overwrite normalization, and remote non-witness isolation
- Deviations from original plan:
  - did not use `WorldTxn::project_institutional_belief()` inside the perception loop; the existing batched store-update architecture was cleaner and preserved perception’s stateless commit pattern
  - removed faction membership witness projection from scope because there is no live visible faction-membership event producer yet
- Verification results:
  - `cargo test -p worldwake-systems perception::tests::political_event_projects_office_installation_claim_for_witness` ✅
  - `cargo test -p worldwake-systems perception::tests::political_event_projects_office_vacancy_claim_for_witness` ✅
  - `cargo test -p worldwake-systems perception::tests::political_event_support_overwrite_projects_only_final_claim` ✅
  - `cargo test -p worldwake-systems perception::tests::political_event_does_not_project_institutional_claim_to_remote_agent` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace` ✅
  - `cargo test --workspace` ✅
