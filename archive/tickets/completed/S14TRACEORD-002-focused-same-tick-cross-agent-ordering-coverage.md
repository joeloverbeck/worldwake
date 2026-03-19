# S14TRACEORD-002: Focused Same-Tick Cross-Agent Ordering Coverage

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — focused coverage in `worldwake-sim`; no production changes expected unless reassessment reveals a real runtime gap
**Deps**: `archive/tickets/completed/S14TRACEORD-001-explicit-intra-tick-ordering-for-action-traces.md`, `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, `docs/golden-e2e-testing.md`

## Problem

The repo now has a golden proving a same-place cross-agent chain (`Tell` enabling `declare_support`) but still lacks focused coverage for the lower-level ordering contract. Without focused tests, future regressions could preserve the high-level golden by accident while still weakening the runtime trace substrate or making same-tick ordering harder to reason about.

## Assumption Reassessment (2026-03-19)

1. Current golden coverage exists in `crates/worldwake-ai/tests/golden_emergent.rs` via `golden_same_place_office_fact_still_requires_tell` and its replay companion. That scenario already asserts ordering via the explicit `(tick, sequence_in_tick)` key on `ActionTraceEvent`, so it is aligned with the post-`S14TRACEORD-001` trace substrate rather than raw vector position.
2. Current focused action-trace coverage is broader than this ticket originally claimed. `crates/worldwake-sim/src/action_trace.rs` proves explicit per-tick sequence assignment on the sink, and `crates/worldwake-sim/src/tick_step.rs` proves runtime ordering across mixed lifecycle events (`Started`, `StartFailed`, `Committed`) for one actor in one tick. The remaining gap is narrower: there is still no focused `step_tick` test proving same-tick ordering across two distinct actors.
3. Current docs already recommend action traces for same-tick lifecycle visibility in `AGENTS.md` and `docs/golden-e2e-testing.md`, which means focused test coverage should exist for the exact runtime behavior the docs tell developers to rely on.
4. This ticket now targets a narrower mixed-layer verification cleanup. The lower layer is still runtime action ordering, but the higher-layer golden already uses the correct explicit ordering contract and does not need further tightening unless the focused runtime reassessment exposes a mismatch.
5. Ordering contract is action lifecycle ordering within the same tick. In the focused runtime test, the compared branches are symmetric and the divergence is driven by input sequence order across two actors. In the motivating golden, the chain remains asymmetric because `Tell` mutates another agent's belief state before that agent later commits `declare_support`.
6. `S14TRACEORD-001` is already completed and archived. This ticket must build directly on the explicit ordering field it introduced rather than restating the old pre-field gap.
7. Search results on 2026-03-19 found no focused runtime test specifically naming or proving same-tick multi-actor ordering in `crates/worldwake-sim/src/tick_step.rs`, `crates/worldwake-sim/src/action_trace.rs`, or `crates/worldwake-ai/tests/golden_emergent.rs`, even though adjacent single-actor and sink-level ordering coverage now exists.

## Architecture Check

1. A narrow multi-actor runtime test is cleaner than expanding the golden because the architectural substrate already exists and the remaining risk is specifically in runtime emission order, not in social or political behavior.
2. Keeping the emergent golden as a downstream consumer and adding one focused multi-actor runtime proof is more robust than making the golden carry both semantic emergence and low-level scheduler guarantees.
3. No backward-compatibility shim is appropriate here. The test should assert directly on `ActionTraceEvent.sequence_in_tick` and the ordered trace surface that already exists.

## Verification Layers

1. Same-tick cross-actor action order is observable and stable at the runtime trace layer -> focused `step_tick` test in `crates/worldwake-sim/src/tick_step.rs`
2. The explicit ordering substrate remains inspectable independent of actor identity -> focused sink tests in `crates/worldwake-sim/src/action_trace.rs`
3. The same-place social-to-political chain continues to consume that ordering substrate correctly -> existing targeted golden in `crates/worldwake-ai/tests/golden_emergent.rs`
4. Later office-holder installation remains a downstream durable consequence and is not used as a proxy for earlier action order -> authoritative world state in the existing golden

## What to Change

### 1. Add focused runtime coverage for same-tick cross-agent ordering

Add a focused test in `worldwake-sim` that creates a same-tick multi-actor action sequence and proves the runtime trace exposes an inspectable, stable order across two actors. Prefer a small integration-style runtime setup over a broad emergent scenario.

### 2. Reconfirm the motivating golden, but do not broaden it unnecessarily

Review `crates/worldwake-ai/tests/golden_emergent.rs` and keep `golden_same_place_office_fact_still_requires_tell` aligned with the focused runtime contract. No golden edit is needed if it already asserts on the explicit `(tick, sequence_in_tick)` ordering key.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify if focused runtime tests live there)
- `crates/worldwake-ai/tests/golden_emergent.rs` (review only; modify only if explicit ordering assertions are still stale)

## Out of Scope

- Changing the semantics of same-tick propagation
- Adding new social or political mechanics
- Rewriting unrelated goldens or broadening this into a full trace-provenance project

## Acceptance Criteria

### Tests That Must Pass

1. New focused test proving same-tick cross-agent action ordering at the runtime trace layer
2. Existing suite: `cargo test -p worldwake-sim`
3. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
4. Existing suite: `cargo clippy --workspace`

### Invariants

1. Same-tick cross-agent ordering is asserted at the runtime trace layer, not only inferred from an end-to-end golden.
2. The golden remains responsible for cross-system causal proof, not for defining the low-level runtime trace contract by itself.
3. No test reintroduces a brittle “must be on a strictly later tick” assumption unless that becomes an explicit engine rule.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — add a focused test for same-tick cross-agent ordering visibility and stability across two actors using the explicit trace ordering key.
   Rationale: existing focused coverage proves sink-level sequencing and single-actor mixed lifecycle ordering, but not multi-actor runtime emission in one tick.
2. `crates/worldwake-ai/tests/golden_emergent.rs` — review only; no change if the existing same-place golden already uses `(tick, sequence_in_tick)`.
   Rationale: the golden should stay a downstream consumer of the runtime contract rather than duplicate focused runtime assertions.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
3. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-19
- Actual changes:
  - Corrected the ticket assumptions to reflect that `S14TRACEORD-001` is already completed, `ActionTraceEvent.sequence_in_tick` already exists, and `golden_same_place_office_fact_still_requires_tell` already consumes the explicit ordering key.
  - Added focused runtime coverage in `crates/worldwake-sim/src/tick_step.rs` for same-tick cross-actor commit ordering using the explicit `(tick, sequence_in_tick)` trace key.
  - Reconfirmed that no `worldwake-ai` golden change was needed because the existing golden was already aligned with the explicit ordering substrate.
- Deviations from original plan:
  - No production code changes were required.
  - No `worldwake-ai` test changes were required because the motivating golden was already updated by earlier work.
  - The remaining gap was narrower than originally stated: multi-actor runtime coverage, not introduction of explicit ordering semantics.
- Verification results:
  - `cargo test -p worldwake-sim` ✅
  - `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell` ✅
  - `cargo clippy --workspace` ✅
