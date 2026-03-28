# S34GENEPIACT-009: Action traceability — typed ActionTraceDetail for verify_belief and ask_witness

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim: extend `ActionTraceDetail` and summary coverage for epistemic actions
**Deps**: S34GENEPIACT-002 (epistemic payload variants), S34GENEPIACT-003 (verify_belief action exists), S34GENEPIACT-004 (ask_witness action exists)

## Problem

Epistemic actions can exist and commit without becoming first-class action-trace artifacts. Right now `ActionTraceDetail::from_payload()` in `crates/worldwake-sim/src/action_trace.rs` only extracts typed detail for `Tell` and `Investigate`, so `verify_belief` and `ask_witness` would appear in action traces only as generic lifecycle events without their epistemic target/topic identity. That weakens the debugging contract for S34 and forces future golden tests to prove epistemic behavior through weaker downstream assertions instead of the direct action-trace surface.

## Assumption Reassessment (2026-03-28)

1. Existing action-trace typed detail lives in `crates/worldwake-sim/src/action_trace.rs`. `ActionTraceDetail` currently has `Tell` and `Investigate` variants, `ActionTraceDetail::from_payload()` extracts only those two payload families, and the summary tests in the same file only cover those typed cases.
2. `tick_step.rs` already threads `ActionTraceDetail::from_payload()` into start, commit, abort, and start-failure trace emission. The trace plumbing therefore already exists; the gap is specifically the typed-detail enum and extractor coverage, not the lifecycle recorder.
3. The shared abstraction boundary under audit is the sim-layer action-trace contract: `crates/worldwake-sim/src/action_trace.rs` for typed detail/schema plus `crates/worldwake-sim/src/tick_step.rs` for lifecycle emission. This ticket should strengthen that contract for epistemic actions without adding handler-specific ad-hoc instrumentation.
4. `VerifyBeliefPayload` and `AskWitnessPayload` now exist in `crates/worldwake-sim/src/action_payload.rs`, so the trace layer can carry epistemic identity directly from canonical payload types. No aliasing or duplicate trace-only structs are needed.
5. Coverage gap classification after search:
   - focused/unit trace coverage exists for `Tell` and `Investigate` in `crates/worldwake-sim/src/action_trace.rs`
   - no focused/unit trace coverage exists for `verify_belief` or `ask_witness`
   - no remaining S34 ticket currently owns this traceability surface; tickets 003/004 focus on handler semantics, while ticket 008 is golden E2E and should consume a strong trace surface rather than recreate it ad hoc
6. `docs/golden-e2e-testing.md` and `docs/precision-rules.md` both require using action traces when they express the action contract more directly and explicitly say to open a follow-up traceability ticket when provenance matters architecturally. This gap matches that rule exactly.
7. The intended invariant is action-lifecycle provenance, not just outcome: when an epistemic action starts/commits/aborts, traces should expose which verification subject or witness/topic it targeted so later focused tests and goldens can prove the exact epistemic branch without inferring it from side effects alone.
8. Reassessment exposes one adjacent contradiction: S34 can still be implemented functionally without this ticket, but the resulting canonical epistemic path would be harder to explain than existing `tell`/`investigate` paths. This is future cleanup that should become its own ticket now rather than being buried inside goldens or ad-hoc debug output.

## Architecture Check

1. Extending `ActionTraceDetail` with epistemic variants is cleaner than teaching goldens or handlers to inspect raw payloads, names, or downstream state deltas. The trace system is already the canonical lifecycle-proof surface for action identity, so epistemic actions should participate there symmetrically with `Tell` and `Investigate`.
2. Reusing the canonical payload fields directly keeps the architecture robust and extensible. `VerifyBelief` should trace its `VerificationSubject`, and `AskWitness` should trace its exact witness/topic selector fields. No backwards-compatibility aliases, no debug-only shadow schema, and no stringly typed summaries as the source of truth.

## Verification Layers

1. `verify_belief` start/commit/abort traces expose the exact verification subject -> focused `action_trace` unit tests plus focused runtime/action-trace coverage in the handler tickets that consume the trace
2. `ask_witness` start/commit/abort traces expose the exact target/topic identity -> focused `action_trace` unit tests plus focused runtime/action-trace coverage in the handler tickets that consume the trace
3. Human-readable action-trace summaries remain informative for epistemic actions -> focused `action_trace` summary tests
4. Golden E2E epistemic scenarios can prove committed action identity through action trace instead of weaker downstream inference -> consumed by ticket 008 once this ticket lands
5. Single sim-trace layer ticket; no additional event-log or authoritative-world mapping is needed because the contract under audit is the typed lifecycle trace surface itself

## What to Change

### 1. Extend `ActionTraceDetail`

In `crates/worldwake-sim/src/action_trace.rs`, add typed variants for epistemic actions:

```rust
VerifyBelief { subject: VerificationSubject },
AskWitness {
    target: EntityId,
    topic_entity: Option<EntityId>,
    topic_commodity: Option<CommodityKind>,
},
```

Keep the trace contract canonical by embedding the existing core/sim types directly.

### 2. Update payload extraction

Extend `ActionTraceDetail::from_payload()` so:

- `ActionPayload::VerifyBelief(payload)` maps to `ActionTraceDetail::VerifyBelief { subject: payload.subject }`
- `ActionPayload::AskWitness(payload)` maps to `ActionTraceDetail::AskWitness { ... }`

No handler-specific branching should be added elsewhere; `tick_step.rs` should continue to rely on the shared extractor.

### 3. Update summary formatting and focused tests

Extend `ActionTraceDetail::summary()` and the existing `action_trace` tests so epistemic details have stable, human-readable summaries and focused coverage for:

- `from_payload()` extraction
- lifecycle event summaries containing the new typed details
- non-epistemic variants continuing to return `None` where appropriate

## Files to Touch

- `crates/worldwake-sim/src/action_trace.rs` (modify — add epistemic trace-detail variants, extraction, summaries, tests)

## Out of Scope

- Epistemic action handler semantics — tickets 003/004
- Planner ops, candidate generation, ranking — tickets 005/006/007
- Golden scenario setup and end-to-end assertions — ticket 008
- Any new ad-hoc logging, `eprintln!`, or debug-only instrumentation outside the shared trace system

## Acceptance Criteria

### Tests That Must Pass

1. `ActionTraceDetail::from_payload()` extracts `VerifyBelief { subject }` from `ActionPayload::VerifyBelief`
2. `ActionTraceDetail::from_payload()` extracts `AskWitness { target, topic_entity, topic_commodity }` from `ActionPayload::AskWitness`
3. Action-trace summary formatting includes typed `verify_belief` detail
4. Action-trace summary formatting includes typed `ask_witness` detail
5. Existing `Tell` and `Investigate` typed detail coverage remains green
6. Existing suite: `cargo test -p worldwake-sim action_trace`
7. Existing suite: `cargo test -p worldwake-systems` after tickets 003/004 consume the trace surface

### Invariants

1. Epistemic action identity is provable through the shared action-trace surface, not only by downstream belief/violation side effects
2. No trace-only alias schema duplicates canonical payload/core types
3. Existing non-epistemic action-trace detail behavior remains unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_trace.rs` — add focused extraction tests for `VerifyBelief` and `AskWitness`
2. `crates/worldwake-sim/src/action_trace.rs` — add focused summary tests proving epistemic typed detail renders in lifecycle summaries
3. `crates/worldwake-systems/src/epistemic_actions.rs` — when tickets 003/004 implement handlers, consume the shared trace surface rather than relying only on belief-state assertions

### Commands

1. `cargo test -p worldwake-sim action_trace`
2. `cargo test -p worldwake-systems epistemic_actions`
3. `cargo build --workspace`
