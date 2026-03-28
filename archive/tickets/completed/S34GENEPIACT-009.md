# S34GENEPIACT-009: Action traceability — typed ActionTraceDetail for verify_belief and ask_witness

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim`: typed `ActionTraceDetail` support and summary coverage for epistemic actions landed
**Deps**: S34GENEPIACT-002 (epistemic payload variants), S34GENEPIACT-003 (verify_belief action exists), S34GENEPIACT-004 (ask_witness action exists)

## Problem

Epistemic actions needed to become first-class action-trace artifacts so tests and future goldens could prove exact `verify_belief` / `ask_witness` identity through the shared trace surface instead of inferring it from weaker downstream side effects.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the sim-layer action-trace contract:
   - typed detail extraction and summaries in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
   - lifecycle emission in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs)
2. The live code now already contains the intended typed epistemic trace variants in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs):
   - `ActionTraceDetail::VerifyBelief { subject }`
   - `ActionTraceDetail::AskWitness { target, topic_entity, topic_commodity }`
3. `ActionTraceDetail::from_payload()` now already extracts typed epistemic identity from canonical payloads in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs), and `tick_step.rs` continues to consume that shared extractor for lifecycle events rather than adding handler-specific instrumentation.
4. Focused coverage already exists and is green:
   - `action_trace::tests::detail_from_payload_extracts_verify_belief_identity`
   - `action_trace::tests::detail_from_payload_extracts_ask_witness_identity`
   - `action_trace::tests::summary_includes_verify_belief_detail_when_present`
   - `action_trace::tests::summary_includes_ask_witness_detail_when_present`
5. Focused runtime/handler consumers are also live and green in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs), confirming the trace surface is usable alongside the action semantics.
6. This is a single-layer traceability ticket. No additional event-log or authoritative-world mapping is required because the contract under audit is the typed lifecycle trace surface itself.
7. Mismatch + correction: the original ticket narrative is stale because the implementation has already landed. This cleanup pass archives the ticket as completed instead of leaving a finished change represented as pending work.

## Architecture Check

1. The landed architecture is the clean one this ticket wanted: epistemic actions participate in the same canonical typed trace system as `Tell` and `Investigate`.
2. The implementation reuses canonical payload/core types directly. No trace-only alias schema or debug-only shadow representation was introduced.

## Verification Layers

1. `verify_belief` typed lifecycle identity -> focused `action_trace` unit tests in `worldwake-sim`
2. `ask_witness` typed lifecycle identity -> focused `action_trace` unit tests in `worldwake-sim`
3. Human-readable action-trace summaries remain informative -> focused `action_trace` summary tests
4. Focused epistemic handler/runtime coverage consumes the shared trace surface without ad-hoc instrumentation -> `worldwake-systems` focused `epistemic_actions` tests
5. Single sim-trace layer ticket; broader event-log or authoritative-world mapping is not applicable

## What Changed

### 1. Extended the shared trace schema

`ActionTraceDetail` now includes typed `VerifyBelief` and `AskWitness` variants in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs).

### 2. Added canonical payload extraction

`ActionTraceDetail::from_payload()` now extracts typed epistemic identity directly from `ActionPayload::VerifyBelief` and `ActionPayload::AskWitness`.

### 3. Added focused trace summary coverage

The trace summary and extraction tests in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) now cover the epistemic variants directly.

## Files Touched

- `crates/worldwake-sim/src/action_trace.rs` (already implemented in repo)

## Out of Scope

- Epistemic action handler semantics
- Planner/candidate-generation/ranking changes
- Golden E2E deliberate-verification scenarios

## Acceptance Criteria

### Tests That Must Pass

1. `ActionTraceDetail::from_payload()` extracts `VerifyBelief { subject }` from `ActionPayload::VerifyBelief`
2. `ActionTraceDetail::from_payload()` extracts `AskWitness { target, topic_entity, topic_commodity }` from `ActionPayload::AskWitness`
3. Action-trace summary formatting includes typed `verify_belief` detail
4. Action-trace summary formatting includes typed `ask_witness` detail
5. Existing `Tell` and `Investigate` typed detail coverage remains green
6. `cargo test -p worldwake-sim action_trace`
7. `cargo test -p worldwake-systems epistemic_actions`
8. `cargo build --workspace`

### Invariants

1. Epistemic action identity is provable through the shared action-trace surface, not only by downstream belief/violation side effects
2. No trace-only alias schema duplicates canonical payload/core types
3. Existing non-epistemic action-trace detail behavior remains unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_trace.rs` — focused extraction tests for `VerifyBelief` and `AskWitness`
2. `crates/worldwake-sim/src/action_trace.rs` — focused summary tests proving epistemic typed detail renders in lifecycle summaries
3. `crates/worldwake-systems/src/epistemic_actions.rs` — focused handler/runtime tests consume the shared trace surface

### Commands

1. `cargo test -p worldwake-sim action_trace`
2. `cargo test -p worldwake-systems epistemic_actions`
3. `cargo build --workspace`

## Outcome

- Date: 2026-03-28
- What actually changed: typed `ActionTraceDetail` support for `verify_belief` and `ask_witness` is already live in `worldwake-sim`, with focused extraction and summary coverage, and focused `worldwake-systems` epistemic tests consuming the surface.
- Deviation from original plan: none material; the cleanup work here was to reassess and archive the already-landed ticket instead of leaving it pending.
- Verification results: `cargo test -p worldwake-sim action_trace`, `cargo test -p worldwake-systems epistemic_actions`, and `cargo build --workspace` all passed on 2026-03-28.
