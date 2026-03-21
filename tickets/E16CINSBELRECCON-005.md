# E16CINSBELRECCON-005: ConsultRecord Action Definition and Handler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action in worldwake-sim + handler in worldwake-systems
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-002, E16CINSBELRECCON-003, E16CINSBELRECCON-004

## Problem

Agents must be able to consult records as an explicit action. The `ConsultRecord` action is how `VisibilitySpec::PublicRecord` becomes concrete gameplay — agents travel to a record's location and spend time reading it to gain institutional beliefs. Without this, records are inert world artifacts with no gameplay surface.

## Assumption Reassessment (2026-03-21)

1. `ActionPayload` in `action_payload.rs` currently has 10 variants (Tell through Loot). No `ConsultRecord` variant exists.
2. `ActionDomain` in `action_domain.rs` has 10 domains including `Social`. The spec assigns ConsultRecord to `ActionDomain::Social`.
3. Action definitions are registered in `action_def_registry` (worldwake-sim). Handlers are registered in `action_handler_registry` (worldwake-sim) with implementations in worldwake-systems.
4. `action_registry.rs` in worldwake-systems registers all system action handlers. The new handler must be added there.
5. ConsultRecord preconditions: actor and record co-located, record accessible, actor alive and not in transit. Duration: `RecordData.consultation_ticks` (modified by `PerceptionProfile.consultation_speed_factor`).
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Follows existing action pattern: payload variant → action def → handler → effect on commit. No novel patterns needed.
2. No backward-compatibility shims.

## Verification Layers

1. ConsultRecord action starts when actor and record are co-located → focused integration test
2. ConsultRecord action fails to start when actor and record are in different places → precondition test
3. On commit, entries are projected into actor's institutional beliefs → action trace + belief state check
4. max_entries_per_consult is respected → unit test on handler logic
5. consultation_speed_factor modifies duration → unit test

## What to Change

### 1. `ConsultRecordActionPayload` in `action_payload.rs`

```rust
pub struct ConsultRecordActionPayload {
    pub record: EntityId,
}
```

Add `ConsultRecord(ConsultRecordActionPayload)` variant to `ActionPayload` enum. Add accessor method `as_consult_record()`.

### 2. Action definition in `action_def_registry`

Register `consult_record` action def:
- domain: `ActionDomain::Social`
- name: `"consult_record"`
- preconditions: actor alive, not in transit, co-located with record entity, record is `EntityKind::Record`
- duration: computed from `RecordData.consultation_ticks` × `PerceptionProfile.consultation_speed_factor`

### 3. New handler in `crates/worldwake-systems/src/consult_record_actions.rs`

On commit:
- Read record's `RecordData`
- Take up to `max_entries_per_consult` entries (newest first)
- For each entry, call `WorldTxn::project_institutional_belief()` with `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`
- Emit event with `EventTag::Social` (or `Political` if appropriate)

### 4. Register handler in `action_registry.rs`

Add the consult_record handler to the action handler registry.

### 5. Register module in `crates/worldwake-systems/src/lib.rs`

Add `pub mod consult_record_actions;`.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add variant + payload struct + accessor)
- `crates/worldwake-sim/src/action_def_registry.rs` (modify — register consult_record def)
- `crates/worldwake-systems/src/consult_record_actions.rs` (new — handler implementation)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register handler)
- `crates/worldwake-systems/src/lib.rs` (modify — add module declaration)

## Out of Scope

- AI candidate generation for ConsultRecord (ticket -012)
- PlannerOpKind::ConsultRecord (ticket -011)
- S12 prerequisite integration (ticket -011)
- Affordance generation for ConsultRecord (covered by existing affordance_query if action def is registered)
- Record access/custody rules beyond co-location (future spec)
- Tell propagation of consulted beliefs (ticket -008)
- Record mutation by political handlers (ticket -007)

## Acceptance Criteria

### Tests That Must Pass

1. ConsultRecord starts successfully when actor and record are co-located
2. ConsultRecord fails precondition when actor and record are in different places
3. On commit, actor gains institutional beliefs from record entries with `RecordConsultation` provenance
4. Only `max_entries_per_consult` entries are read (newest first)
5. Duration is `RecordData.consultation_ticks` adjusted by `PerceptionProfile.consultation_speed_factor`
6. ConsultRecord is interruptible before commit (no partial belief projection on abort)
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. ConsultRecord is a pure read of the record — it does not mutate `RecordData`
2. Belief projection uses `WorldTxn::project_institutional_belief()` (respects capacity)
3. Action follows the standard action lifecycle (start → active → commit/abort)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/consult_record_actions.rs` (inline tests) — co-location precondition, entry limit, belief projection, duration calculation
2. Integration test with full action framework — start, tick, commit cycle

### Commands

1. `cargo test -p worldwake-systems consult_record`
2. `cargo clippy --workspace && cargo test --workspace`
