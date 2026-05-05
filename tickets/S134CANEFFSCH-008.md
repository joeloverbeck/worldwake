# S134CANEFFSCH-008: Social action schemas (communication and epistemic)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in 8 social/epistemic actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating the social/epistemic action family — `tell` (in `tell_actions.rs`), `consult_record` (in `consult_record_actions.rs`), `ask_about_person` (in `ask_about_person_actions.rs`), `ask_witness` (in `epistemic_actions.rs`), `search_place` (in `search_actions.rs`), `investigate` (in `investigate_actions.rs`), and `report_missing` + `report_found` (in `report_actions.rs`) — to declarative `EffectSchema` evaluation. These actions all involve information transfer (belief mutations on the receiving agent) and record creation/query rather than commodity movement. The S127 `CommitTraceData::Tell` shape becomes a typed `EffectFact` output. The planner continues to use the old `apply_hypothetical_transition` path; goldens for these actions must produce bitwise-identical event logs.

## Assumption Reassessment (2026-05-04)

1. Social/epistemic registrations span 7 files in `crates/worldwake-systems/src/`:
   - `tell_actions.rs` — `register_tell_action`
   - `consult_record_actions.rs` — `register_consult_record_action`
   - `ask_about_person_actions.rs` — `register_ask_about_person_action`
   - `epistemic_actions.rs` — `register_ask_witness_action`
   - `search_actions.rs` — `register_search_place_action`
   - `investigate_actions.rs` — `register_investigate_action`
   - `report_actions.rs` — `register_report_missing_action` + `register_report_found_action`
2. After ticket 001, each `ActionDef` literal has `effect_schema: EffectSchema::empty()`. This ticket populates real schemas.
3. These actions mutate belief state on the receiving agent (or record state on a place/institution) rather than mutating commodity inventories. The schema's `EffectStep` language must include belief mutation — likely an `EffectStep::AssertBelief { agent, claim, value }` (or analog) variant. `EffectPrecondition::BeliefHeld` already exists from ticket 001 for the symmetrical query side.
4. `CommitTraceData::Tell` at `crates/worldwake-sim/src/action_handler.rs:39–42` is the existing tell-trace shape (one of two `CommitTraceData` variants, the other being `Harvest`). The schema's `EffectFact` output must surface the same trace data — likely an `EffectFact::BeliefAsserted { agent, claim }` or similar.
5. Existing focused/unit coverage:
   - Per-file `#[cfg(test)]` blocks in each of the 7 source files
   - Goldens — `golden_tell_*.rs`, `golden_witness_*.rs`, `golden_investigate_*.rs`, `golden_search_place_*.rs`, `golden_report_*.rs`. Enumerate during reassessment.
   - Conformance tests: `conformance_tell` (line 1129), `conformance_investigate` (line 1218) at `planner_conformance.rs`.
6. `tell` is bilateral in a different sense than trade: speaker and listener both have belief mutations (speaker's "I told you X" memory, listener's "X (per source: speaker)" claim). Schema must encode both belief-write directions atomically.
7. `consult_record` and `search_place` are read-mostly — they query authoritative records/places and write to the actor's belief state. Schemas: precondition on co-location with record/place, step asserting belief from observed content.
8. `report_missing`/`report_found` create record artifacts (notices) at the place where the report is filed. Ticket 007 did not add a generic `CreateEntity`; this ticket must reassess the live record shape and add a social/record-owned schema step if report artifacts need category-specific creation semantics.
9. Bitwise-identical event-log invariant: every `Tell`/`Witness`/`Investigate`/`SearchPlace`/`Report*` event emission and every belief-store mutation must have identical timing and payload pre- and post-ticket.

## Architecture Check

1. Belief mutation as a first-class `EffectStep` variant aligns the authoritative belief-write path with the planner's hypothetical belief projection — currently both happen but in different code (handler bodies for authoritative, `apply_planner_step` for hypothetical via `GoalModelFallback`). Schema unification eliminates that drift surface.
2. `EffectPrecondition::BeliefHeld` (from ticket 001) and the new `EffectStep::AssertBelief` (added here if needed) make the belief-flow surface explicit in the schema, improving introspection (FND-29) and matching FND-15's "Knowledge Is Acquired Locally and Travels Physically" — every belief mutation has a co-location precondition encoded declaratively.
3. Record-creation semantics through `EffectStep::CreateRecord` or a more specific social/record-owned step align with FND-25 (Social Artifacts Are First-Class) — records, notices, and reports become explicit schema outputs rather than handler-internal mutations.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on tell/witness/investigate/search/report goldens.
2. Belief-mutation invariant → focused runtime test or action trace: each social action's belief-write emits the same `BeliefStore` mutation event with same source-attribution, freshness, and credibility metadata as today.
3. Record-creation invariant → event-log delta: `report_missing`/`report_found` produce identical notice-creation event sequences.
4. Conformance-tests parity → `conformance_tell` and `conformance_investigate` continue to pass.
5. Canonical state hash invariant → soak: identical hashes on the three soak scenarios.

## What to Change

### 1. Add `EffectStep` variants for belief and record mutations

If `effect_schema.rs` doesn't yet have:
- `EffectStep::AssertBelief { agent, claim, value, source, freshness, credibility }` (or analog matching the current `AgentBeliefStore` write API)
- `EffectStep::CreateRecord { kind, place, fields }` or a more specific report/social-record step matching the live artifact shape

…add them in this ticket and implement the corresponding sink methods in both impls.

### 2. Construct `EffectSchema` literals for the 8 social/epistemic actions

Per-action sketches:

- **tell**: preconditions — `CoLocated { actor: speaker, target: listener }`. Steps — `AssertBelief { agent: listener, claim: told_claim, source: speaker, … }`, `EmitEvent { tag: EventTag::Tell }`.
- **consult_record**: preconditions — `CoLocated { actor, target: record_artifact }`. Steps — `AssertBelief { agent: actor, claim: record_content, source: record, … }`, `EmitEvent { tag: EventTag::ConsultRecord }`.
- **ask_about_person** / **ask_witness**: preconditions — `CoLocated`, target-knowledge precondition. Steps — `AssertBelief { agent: asker, claim: target_response, source: target, … }`, `EmitEvent`.
- **search_place**: precondition — `CoLocated { actor, target: place }`. Steps — `AssertBelief { agent: actor, claim: PlaceContents(observed), source: PerceptionAtPlace, … }`, `EmitEvent { tag: EventTag::SearchPlace }`.
- **investigate**: precondition — `CoLocated`, target-evidence-present. Steps — `AssertBelief { agent: actor, claim: investigation_finding, … }`, `EmitEvent { tag: EventTag::Investigate }`.
- **report_missing** / **report_found**: precondition — `CoLocated { actor, target: bulletin_board_or_office }`. Steps — `CreateRecord { kind: Report, place, fields }` (or `CreateEntity` for the report entity), `EmitEvent { tag: EventTag::ReportFiled }`.

### 3. Replace commit handler bodies with `apply_effects` delegation

Each `commit_*` handler in the 7 files shrinks to the standard delegation. Remove imperative bodies. The `CommitTraceData::Tell` shape may need updating to derive from `EffectFact` outputs — confirm during reassessment.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify)
- `crates/worldwake-systems/src/search_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/report_actions.rs` (modify)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs `AssertBelief` or `CreateRecord` variants)
- `crates/worldwake-sim/src/action_handler.rs` (modify if `CommitTraceData::Tell` derivation changes)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-social/epistemic actions (tickets 003, 004, 005, 006, 007, 009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Changing belief-store mutation semantics, source-attribution rules, or freshness/credibility computation.
- Changing record artifact lifecycles (S140 territory) — reports and notices remain whatever shape they are today.

## Acceptance Criteria

### Tests That Must Pass

1. All social/epistemic-touching goldens produce bitwise-identical event logs.
2. Conformance tests `conformance_tell` and `conformance_investigate` continue to pass.
3. `cargo test -p worldwake-systems tell consult_record ask_about_person epistemic search investigate report` — existing inline tests pass.
4. `cargo test -p worldwake-ai golden_survival` — soak goldens produce identical canonical state hashes.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Each social/epistemic action's belief-write produces the same `AgentBeliefStore` mutation with same source/freshness/credibility metadata.
2. Record-creation actions (`report_missing`, `report_found`) produce identical record-artifact entities/components.
3. `CommitTraceData::Tell` continues to surface tell-trace data with same field values.
4. Bitwise-identical canonical state hash on the three soak scenarios.

## Test Plan

### New/Modified Tests

1. Per-file `#[cfg(test)]` blocks — modify existing tests to exercise schema-driven path; add focused tests covering belief-mutation precondition failures (e.g., `tell` with no co-location yields `Discrepancy::NoLegalBinding`).
2. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems tell consult_record ask_about_person epistemic search investigate report`
2. `cargo test -p worldwake-ai conformance_tell conformance_investigate`
3. `cargo test -p worldwake-ai golden_survival`
4. `./scripts/verify.sh`
