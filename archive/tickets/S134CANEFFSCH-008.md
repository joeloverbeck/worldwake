# S134CANEFFSCH-008: Social action schemas (communication and epistemic)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real category-owned `EffectSchema` steps in 8 social/epistemic actions and switches their commit handler bodies to `apply_effects_with_context(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating the social/epistemic action family — `tell` (in `tell_actions.rs`), `consult_record` (in `consult_record_actions.rs`), `ask_about_person` (in `ask_about_person_actions.rs`), `ask_witness` (in `epistemic_actions.rs`), `search_place` (in `search_actions.rs`), `investigate` (in `investigate_actions.rs`), and `report_missing` + `report_found` (in `report_actions.rs`) — to declarative `EffectSchema` evaluation. These actions all involve information transfer (belief mutations on the receiving agent) and record creation/query rather than commodity movement. The landed path preserves the existing `CommitTraceData::Tell` shape through the local authoritative sink. The planner continues to use the old `apply_hypothetical_transition` path; goldens for these actions must produce bitwise-identical event logs.

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
3. These actions mutate belief state on the receiving agent, actor memory, expectation state, violation memory, or record state rather than mutating commodity inventories. Live reassessment showed the effect language needs category-owned social/epistemic commit steps, not a generic belief-write primitive.
4. `CommitTraceData::Tell` at `crates/worldwake-sim/src/action_handler.rs:39–42` is the existing tell-trace shape (one of two `CommitTraceData` variants, the other being `Harvest`). The landed schema path preserves that trace by carrying the existing tell `CommitOutcome` through the local authoritative sink.
5. Existing focused/unit coverage:
   - Per-file `#[cfg(test)]` blocks in each of the 7 source files
   - Goldens — `golden_tell_*.rs`, `golden_witness_*.rs`, `golden_investigate_*.rs`, `golden_search_place_*.rs`, `golden_report_*.rs`. Enumerate during reassessment.
   - Conformance tests: `conformance_tell` (line 1129), `conformance_investigate` (line 1218) at `planner_conformance.rs`.
6. `tell` is bilateral in a different sense than trade: speaker and listener both have belief mutations (speaker's "I told you X" memory, listener's "X (per source: speaker)" claim). Schema must encode both belief-write directions atomically.
7. `consult_record` and `search_place` are read-mostly — they query authoritative records/places and write to the actor's belief state. Schemas: precondition on co-location with record/place, step asserting belief from observed content.
8. `report_missing`/`report_found` update report and office-register state at the place where the report is filed. Ticket 007 did not add a generic `CreateEntity`; live reassessment showed these actions need category-owned report steps instead of generic record-artifact creation semantics.
9. Bitwise-identical event-log invariant: every `Tell`/`Witness`/`Investigate`/`SearchPlace`/`Report*` event emission and every belief-store mutation must have identical timing and payload pre- and post-ticket.

## Live Reassessment Update (2026-05-05)

1. The drafted generic `AssertBelief` / `CreateRecord` schema sketch is not representable as a faithful generic effect on the live branch. These commits depend on action payloads, local validation helpers, RNG-driven tell acceptance, record-entry projection, expectation/violation/last-seen memory updates, and report/office-register side effects.
2. The landed boundary follows the established S134 category-owned pattern from tickets 003-007: add typed `EffectStep` variants for the social/epistemic action commits, default unsupported sinks to `Discrepancy::ImproperPlanningState`, and override only the local authoritative sink that owns each action family.
3. `CommitTraceData::Tell` remains the public trace carrier. The tell schema step stores the existing `CommitOutcome` from the old tell commit helper rather than introducing a separate generic `EffectFact` for belief assertion. That keeps the trace payload bitwise-compatible with the existing scheduler/event-log shape.
4. No persisted runtime state shape changed. `ActionDef.effect_schema` is registry data, and these new steps are not serialized into `SimulationState`; `SAVE_FORMAT_VERSION` remains unchanged.

## Architecture Check

1. Social/epistemic commits as first-class category-owned `EffectStep` variants align the authoritative write path with the S134 schema migration without flattening payload-driven belief, memory, expectation, and record aftermath into a lossy generic primitive.
2. The action definitions still carry the existing authoritative preconditions and commit conditions for co-location, target shape, actor liveness, and payload validation. The new schema steps make the commit owner visible to the S134 registry while preserving the live handler semantics.
3. Record/report semantics remain the exact live report and office-register mutations. No generic record-creation abstraction was introduced because the live branch's reports update violation memory, missing-person status claims, and last-seen/expectation state rather than creating one uniform report artifact.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on tell/witness/investigate/search/report goldens.
2. Belief-mutation invariant → focused runtime test or action trace: each social action's belief-write emits the same `BeliefStore` mutation event with same source-attribution, freshness, and credibility metadata as today.
3. Report-state invariant → event-log delta: `report_missing`/`report_found` produce identical report and office-register aftermath.
4. Conformance-tests parity → `conformance_tell` and `conformance_investigate` continue to pass.
5. Canonical state hash invariant → soak: identical hashes on the three soak scenarios.

## What to Change

### 1. Add category-owned social/epistemic `EffectStep` variants

Add `EffectStep::CommitTell`, `EffectStep::ConsultRecord`, `EffectStep::AskAboutPerson`, `EffectStep::AskWitness`, `EffectStep::SearchPlace`, `EffectStep::Investigate`, `EffectStep::ReportMissing`, and `EffectStep::ReportFound`.

Add matching default-rejecting `EffectSink` methods. Unsupported sinks reject these steps with `Discrepancy::ImproperPlanningState`; each action module supplies the local authoritative sink for the step it owns.

### 2. Construct `EffectSchema` literals for the 8 social/epistemic actions

Each action definition carries a category-owned single-step schema. The old commit bodies are extracted behind local authoritative sinks so payload validation, RNG, belief mutation, record projection, violation memory, report status claims, and `CommitTraceData::Tell` remain identical.

- **tell**: `EffectStep::CommitTell`
- **consult_record**: `EffectStep::ConsultRecord`
- **ask_about_person**: `EffectStep::AskAboutPerson`
- **ask_witness**: `EffectStep::AskWitness`
- **search_place**: `EffectStep::SearchPlace`
- **investigate**: `EffectStep::Investigate`
- **report_missing**: `EffectStep::ReportMissing`
- **report_found**: `EffectStep::ReportFound`

### 3. Replace commit handler bodies with `apply_effects` delegation

Each `commit_*` handler in the 7 files delegates through `apply_effects_with_context(..., EffectMode::Authoritative)`. The previous imperative bodies are retained as private helper functions behind the local sinks so the live payload validation, RNG, trace, belief, record, and report aftermath remains unchanged. `CommitTraceData::Tell` remains unchanged.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify)
- `crates/worldwake-systems/src/search_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/report_actions.rs` (modify)
- `crates/worldwake-sim/src/effect_schema.rs` (modify — add category-owned social/epistemic steps and default-rejecting sink methods)
- `crates/worldwake-sim/src/action_handler.rs` (no change — `CommitTraceData::Tell` remains the existing trace carrier)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (no change — local action-file sinks override the category-owned steps)

## Out of Scope

- Migrating non-social/epistemic actions (tickets 003, 004, 005, 006, 007, 009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Changing belief-store mutation semantics, source-attribution rules, or freshness/credibility computation.
- Changing record artifact lifecycles (S140 territory) — reports and notices remain whatever shape they are today.

## Acceptance Criteria

### Tests That Must Pass

1. All social/epistemic-touching goldens produce bitwise-identical event logs.
2. Conformance tests `conformance_tell` and `conformance_investigate` continue to pass.
3. `cargo test -p worldwake-systems --lib` — existing inline tests pass, including the 7 social/epistemic action modules.
4. Exact social/epistemic goldens and the three ignored survival soak goldens pass.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Each social/epistemic action's belief-write produces the same `AgentBeliefStore` mutation with same source/freshness/credibility metadata.
2. Report actions (`report_missing`, `report_found`) produce identical violation memory, missing-person status, last-seen, expectation, and office-register state as applicable.
3. `CommitTraceData::Tell` continues to surface tell-trace data with same field values.
4. Bitwise-identical canonical state hash on the three soak scenarios.

## Test Plan

### New/Modified Tests

1. Existing per-file `#[cfg(test)]` blocks exercise the schema-driven commit path because the public commit entrypoints now delegate through `apply_effects_with_context`.
2. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems --lib`
2. `cargo test -p worldwake-ai --test planner_conformance conformance_tell -- --exact`
3. `cargo test -p worldwake-ai --test planner_conformance conformance_investigate -- --exact`
4. `cargo test -p worldwake-ai --test golden_survival_tell -- --ignored`
5. `cargo test -p worldwake-ai --test golden_survival_ask_consult -- --ignored`
6. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
7. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
8. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
9. `cargo test -p worldwake-ai`
10. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-05.

- Added category-owned social/epistemic `EffectStep` variants in `crates/worldwake-sim/src/effect_schema.rs`: `CommitTell`, `ConsultRecord`, `AskAboutPerson`, `AskWitness`, `SearchPlace`, `Investigate`, `ReportMissing`, and `ReportFound`.
- Added default-rejecting sink methods for those steps. Unsupported sinks, including the staged hypothetical path, still fail with `Discrepancy::ImproperPlanningState` until ticket 010 owns planner parity.
- Replaced empty schemas in `tell_actions.rs`, `consult_record_actions.rs`, `ask_about_person_actions.rs`, `epistemic_actions.rs`, `search_actions.rs`, `investigate_actions.rs`, and `report_actions.rs` with the corresponding category-owned schema steps.
- Switched the 8 social/epistemic commit entrypoints to `apply_effects_with_context(..., EffectMode::Authoritative)` through local authoritative sinks that delegate to the existing commit helper bodies.
- Preserved `CommitTraceData::Tell` by carrying the existing tell `CommitOutcome` through the local tell sink.

## Deviations

- No generic `AssertBelief`, generic `CreateRecord`, or generic belief/record `EffectFact` was added. The live handler semantics require category-owned payload/RNG/state aftermath, matching the S134 category-migration pattern already used by earlier tickets.
- The drafted command `cargo test -p worldwake-systems tell consult_record ask_about_person epistemic search investigate report` is not valid Cargo filter syntax for independent families. The implementation used the affected crate suite plus exact AI conformance and golden commands instead.
- `./scripts/verify.sh` was run directly after inspecting its live gate list.
- No `SAVE_FORMAT_VERSION` bump was made because the new schema steps live on the action-definition registry, not saved runtime state.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_tell -- --exact`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_investigate -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_tell`
- Passed `cargo test -p worldwake-ai --test golden_survival_ask_consult`
- Passed `cargo test -p worldwake-ai --test golden_survival_tell -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_ask_consult -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
