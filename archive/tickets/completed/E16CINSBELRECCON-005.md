# E16CINSBELRECCON-005: ConsultRecord Action Definition and Handler

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action payload/runtime duration surface in worldwake-sim + consult handler in worldwake-systems
**Deps**: archive/tickets/E16CINSBELRECCON-001.md, archive/tickets/E16CINSBELRECCON-002.md, archive/tickets/E16CINSBELRECCON-003.md, archive/tickets/completed/E16CINSBELRECCON-004.md

## Problem

Agents must be able to consult records as an explicit action. The `ConsultRecord` action is how `VisibilitySpec::PublicRecord` becomes concrete gameplay: agents travel to a record's location and spend time reading it to gain institutional beliefs. Without this, records remain inert world artifacts with no gameplay surface.

## Assumption Reassessment (2026-03-22)

1. `ActionPayload` in [action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) currently has 11 concrete payload variants after `None` (`Tell`, `Bribe`, `Threaten`, `DeclareSupport`, `Transport`, `Harvest`, `Craft`, `Trade`, `Combat`, `Loot`, `QueueForFacilityUse`). No `ConsultRecord` payload or typed accessor exists yet.
2. `ActionDomain` in [action_domain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_domain.rs) already includes `Social`, and spec section 6 in [E16c-institutional-beliefs-and-record-consultation.md](/home/joeloverbeck/projects/worldwake/specs/E16c-institutional-beliefs-and-record-consultation.md) still places `ConsultRecord` in that domain. No new domain is needed.
3. The original ticket named the wrong registration boundary. `ActionDefRegistry` and `ActionHandlerRegistry` in worldwake-sim are generic containers, but concrete action definitions are assembled in worldwake-systems through `register_*_action` functions and `register_all_actions()` in [action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs). `ConsultRecord` should follow that existing architecture.
4. `RecordData`, `EntityKind::Record`, `WorldTxn::append_record_entry`, `WorldTxn::supersede_record_entry`, and `WorldTxn::project_institutional_belief` are already implemented in core by the archived dependency tickets. This ticket should consume those live surfaces rather than restating core record/belief infrastructure.
5. The original duration assumption was incomplete. `start_action()` in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs) resolves `def.duration` before handler `on_start` runs, and `DurationExpr` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) has no consultation-specific variant. A handler-only implementation cannot cleanly honor `RecordData.consultation_ticks * PerceptionProfile.consultation_speed_factor`. A small runtime extension to `DurationExpr` and duration estimation/resolution is therefore in scope.
6. No heuristic removal is involved. The correct architecture is still an explicit action, not an instant belief helper.
7. N/A.
8. N/A.
9. N/A.
10. Scope isolation: this ticket must stay on the action lifecycle and authoritative belief projection. It must not claim the broader E16c migration is complete because `PerAgentBeliefView` in [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) still uses the pre-E16c live institutional helper seam, which is covered later by ticket `-014`.
11. Mismatch + correction: the current code does not support the original "handler-only new action" narrative. The correct scope is: add `ConsultRecord` payload plumbing, add the minimal new runtime duration expression needed for consultation speed scaling, implement the worldwake-systems action definition/handler, and cover start/commit/abort behavior with focused sim/systems tests.
12. Current narrow verification command names are real. `cargo test -p worldwake-sim -- --list` and `cargo test -p worldwake-systems -- --list` confirm the existing test targets and registration surfaces this ticket will extend.
13. Additional live-code correction discovered during implementation: once `consult_record` is registered in the production action catalog, `worldwake-ai`'s planner semantics table must classify it explicitly because `build_semantics_table()` intentionally expects every registered action def to have a planner semantics entry. This ticket should update that registry-integrity seam without claiming consult-goal generation is implemented.

## Architecture Check

1. The clean architecture remains the existing action pattern: payload variant -> action def -> handler -> commit-time world mutation. The only extension worth making is a narrow `DurationExpr` branch for record consultation because the action framework, not the handler, owns authoritative duration resolution.
2. Extending `DurationExpr` is more robust than burying consultation timing inside handler state or silently ignoring `consultation_speed_factor`. It keeps dynamic duration in the same shared runtime surface already used by travel, combat, metabolism, trade, and treatment.
3. This is more beneficial than the current architecture because records become usable through the standard action lifecycle rather than via a special-case read helper. That preserves locality, duration, interruptibility, and traceability.
4. No backward-compatibility aliasing or parallel consult paths.

## Verification Layers

1. payload + duration runtime plumbing resolves consult duration from authoritative record/profile state -> focused sim unit tests
2. action definition and start-time preconditions (alive, has control, not in transit, co-located record, record kind) -> focused systems unit tests
3. action lifecycle ordering (start -> active -> commit or abort) -> action-trace assertions in focused systems tests
4. commit projects institutional beliefs with `RecordConsultation` provenance and does not mutate `RecordData` -> authoritative world-state assertions in focused systems tests
5. abort before commit leaves institutional beliefs unchanged -> action-trace plus authoritative world-state assertions
6. Single ticket, mixed sim/systems layers. No AI decision-trace mapping is applicable because autonomous consult planning is out of scope here.

## What to Change

### 1. Add `ConsultRecordActionPayload` in `action_payload.rs`

```rust
pub struct ConsultRecordActionPayload {
    pub record: EntityId,
}
```

Add `ConsultRecord(ConsultRecordActionPayload)` to `ActionPayload` and add `as_consult_record()`.

### 2. Add runtime duration support for consultation in worldwake-sim

Add a `DurationExpr` variant dedicated to record consultation and wire it through:

- `DurationExpr::resolve_for()` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
- `estimate_duration_from_beliefs()` in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)
- any related payload/type exports and focused tests

Resolution rule:

- read the consulted record from the bound target or payload
- read the actor's `PerceptionProfile.consultation_speed_factor`
- compute `max(1, consultation_ticks * factor / 1000)` with integer arithmetic

### 3. Add the consult action definition and handler in worldwake-systems

Register `consult_record` using the existing `register_*_action` pattern:

- domain: `ActionDomain::Social`
- actor constraints: actor alive, has control, not in transit
- target: `TargetSpec::EntityAtActorPlace { kind: EntityKind::Record }`
- preconditions: actor alive, record exists, co-located, target kind is `EntityKind::Record`
- duration: the new consultation-specific `DurationExpr`
- interruptibility: freely interruptible

On commit:

- read the record's `RecordData`
- take up to `max_entries_per_consult` entries newest-first
- project each entry into the actor's belief store via `WorldTxn::project_institutional_belief()`
- use `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`
- do not mutate the record itself

### 4. Register the action in the production action catalog

Add `register_consult_record_action()` to [action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) and export the module from [lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs).

### 5. Add focused tests before broad verification

Cover:

- payload accessor and duration-expression resolution
- registration includes `consult_record`
- start succeeds for co-located actor/record and fails when separated
- commit reads newest entries first, respects `max_entries_per_consult`, and projects provenance-tagged beliefs
- abort before commit does not leak partial belief projection

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add variant + payload struct + accessor)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — add consultation duration expression)
- `crates/worldwake-sim/src/belief_view.rs` (modify — belief-side duration estimation support)
- `crates/worldwake-sim/src/lib.rs` (modify — export new payload type if needed)
- `crates/worldwake-sim/src/action_trace.rs` (modify only if typed payload detail coverage is extended)
- `crates/worldwake-systems/src/consult_record_actions.rs` (new — handler implementation)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register handler)
- `crates/worldwake-systems/src/lib.rs` (modify — add module declaration)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — keep registered-action semantics classification exhaustive)
- `crates/worldwake-ai/src/agent_tick.rs`, `crates/worldwake-ai/src/failure_handling.rs`, `crates/worldwake-ai/src/goal_model.rs` (modify only as needed for exhaustive `PlannerOpKind` coverage after adding `ConsultRecord`)

## Out of Scope

- AI candidate generation for `ConsultRecord` (ticket `-011` / later consult-goal tickets)
- planner operators or prerequisite-place integration for consult planning beyond the minimal registry-integrity semantics entry required to keep every registered action classified
- belief-derived public institutional query migration in `PerAgentBeliefView` / AI (ticket `-014`)
- record access/custody policy beyond the current live action-validation surface (future spec/ticket)
- Tell propagation of consulted beliefs (ticket `-008`)
- record mutation by political handlers (ticket `-007`)

## Acceptance Criteria

### Tests That Must Pass

1. ConsultRecord starts successfully when actor and record are co-located
2. ConsultRecord fails precondition when actor and record are in different places
3. On commit, actor gains institutional beliefs from record entries with `RecordConsultation` provenance
4. Only `max_entries_per_consult` entries are read (newest first)
5. Duration is `RecordData.consultation_ticks` adjusted by `PerceptionProfile.consultation_speed_factor`
6. ConsultRecord is interruptible before commit (no partial belief projection on abort)
7. Full action catalog registration still succeeds with `consult_record` included
8. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-systems`

### Invariants

1. ConsultRecord is a pure read of the record; it does not mutate `RecordData`
2. Belief projection uses `WorldTxn::project_institutional_belief()` and therefore respects institutional memory capacity
3. Action follows the standard action lifecycle (start -> active -> commit/abort)
4. Consultation duration is owned by the shared action runtime surface, not bespoke handler timing logic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` — add payload accessor coverage for `ConsultRecord`.
Rationale: locks the typed-payload surface used by request resolution, tracing, and handler validation.
2. `crates/worldwake-sim/src/action_semantics.rs` and/or `crates/worldwake-sim/src/belief_view.rs` — add consultation duration resolution/estimation tests.
Rationale: proves the new runtime extension computes duration from `RecordData` plus `PerceptionProfile.consultation_speed_factor`, which the old architecture could not express.
3. `crates/worldwake-systems/src/consult_record_actions.rs` — add focused action-definition and handler tests covering start success/failure, belief projection, entry limit, and abort-without-commit.
Rationale: proves the action behaves correctly at the authoritative action lifecycle boundary without overclaiming AI integration.
4. `crates/worldwake-systems/src/action_registry.rs` — extend full catalog coverage to require `consult_record`.
Rationale: proves the new action is actually registered in the production system action catalog.

### Commands

1. `cargo test -p worldwake-sim action_payload::tests::`
2. `cargo test -p worldwake-sim action_semantics::tests::`
3. `cargo test -p worldwake-systems consult_record_actions::tests::`
4. `cargo test -p worldwake-systems action_registry::tests::build_full_action_registries_returns_complete_action_catalog -- --exact`
5. `cargo test -p worldwake-sim`
6. `cargo test -p worldwake-systems`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - added `ActionPayload::ConsultRecord` plus typed accessors and serialization coverage in `worldwake-sim`
  - added `DurationExpr::ConsultRecord` plus authoritative/belief-side consultation duration resolution from `RecordData.consultation_ticks` and `PerceptionProfile.consultation_speed_factor`
  - added the `consult_record` action definition/handler in `worldwake-systems`, including co-location validation, newest-first bounded record reading, and institutional belief projection with `RecordConsultation` provenance
  - registered `consult_record` in the production action catalog
  - updated `worldwake-ai` planner semantics classification and exhaustive `PlannerOpKind` matches so the new registered action remains a first-class engine action without adding consult-goal generation
- Deviations from original plan:
  - the original ticket understated the need for shared runtime duration support; handler-only timing was not architecturally valid because duration resolves before `on_start`
  - the live code also required a small AI semantics-table integration to preserve the invariant that every registered action is classified; this did not expand scope into consult planning
- Verification results:
  - focused consult payload/duration/action/registry tests passed in `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo fmt --all --check` passed
  - `cargo clippy --workspace` passed
