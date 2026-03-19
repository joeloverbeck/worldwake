# S14TRACEORD-002: Focused Same-Tick Cross-Agent Ordering Coverage

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — focused coverage in `worldwake-sim` and `worldwake-ai`; production changes only if required by `S14TRACEORD-001`
**Deps**: `tickets/S14TRACEORD-001-explicit-intra-tick-ordering-for-action-traces.md`, `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, `docs/golden-e2e-testing.md`

## Problem

The repo now has a golden proving a same-place cross-agent chain (`Tell` enabling `declare_support`) but still lacks focused coverage for the lower-level ordering contract. Without focused tests, future regressions could preserve the high-level golden by accident while still weakening the runtime trace substrate or making same-tick ordering harder to reason about.

## Assumption Reassessment (2026-03-19)

1. Current golden coverage exists in `crates/worldwake-ai/tests/golden_emergent.rs` via `golden_same_place_office_fact_still_requires_tell` and its replay companion. That scenario proves the end-to-end social-to-political chain but is not the right place to define the low-level trace contract alone.
2. Current action-trace focused coverage in `crates/worldwake-sim/src/action_trace.rs` proves storage/query basics only. It does not assert a same-tick cross-actor ordering invariant.
3. Current docs already recommend action traces for same-tick lifecycle visibility in `AGENTS.md` and `docs/golden-e2e-testing.md`, which means focused test coverage should exist for the exact runtime behavior the docs tell developers to rely on.
4. This ticket targets mixed-layer verification. The lower layer is runtime action ordering; the higher layer is one representative emergent golden. The lower layer should become the primary assertion surface for same-tick ordering, with the golden remaining a downstream cross-system proof.
5. Ordering contract is action lifecycle ordering within the same tick. The motivating chain is asymmetric because `Tell` changes another agent's belief state before that agent acts, but the focused coverage should assert general ordering semantics rather than hardcoding social or office behavior into the trace substrate.
6. If `S14TRACEORD-001` lands first, this ticket should assert on the new explicit ordering field. If that ticket is rejected, reassess this ticket rather than cementing vector append position as the permanent public contract.
7. No current focused test names were found for same-tick cross-agent ordering via `rg -n "same-tick|same tick|intra-tick"` across `crates/worldwake-sim`, `crates/worldwake-ai/tests`, docs, and `AGENTS.md`.

## Architecture Check

1. Focused runtime coverage is cleaner than adding more goldens because it pins the real substrate where the contract lives.
2. Keeping one representative golden plus one or two focused runtime tests is more robust than trying to make a single golden carry both semantic emergence and low-level scheduler/trace guarantees.
3. No backward-compatibility shim is appropriate here. If the explicit ordering model from `S14TRACEORD-001` changes the right assertion surface, tests should move to that contract directly.

## Verification Layers

1. Same-tick cross-actor action order is observable and stable at the runtime trace layer -> focused/unit or integration tests in `worldwake-sim`
2. The same-place social-to-political chain continues to use that ordering substrate correctly -> targeted golden in `crates/worldwake-ai/tests/golden_emergent.rs`
3. Later office-holder installation remains a downstream durable consequence and is not used as a proxy for earlier same-tick action order -> authoritative world state in existing golden
4. If explicit ordering metadata is added, consumers assert on that field rather than on vector position -> focused tests first, golden updated only where it materially strengthens clarity

## What to Change

### 1. Add focused runtime coverage for same-tick cross-agent ordering

Add a focused test in `worldwake-sim` that creates a same-tick multi-actor action sequence and proves the runtime trace exposes an inspectable, stable order. Prefer a small integration-style runtime setup over a broad emergent scenario.

### 2. Tighten the motivating golden only where it clarifies the contract

Review `crates/worldwake-ai/tests/golden_emergent.rs` and keep `golden_same_place_office_fact_still_requires_tell` aligned with the focused runtime contract. If `S14TRACEORD-001` adds explicit ordering metadata, update the golden to use it rather than raw trace vector index.

## Files to Touch

- `crates/worldwake-sim/src/action_trace.rs` (modify if focused tests live there)
- `crates/worldwake-sim/src/tick_step.rs` (modify if focused runtime tests live there)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify only if explicit ordering assertions become cleaner)

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

1. `crates/worldwake-sim/src/action_trace.rs` or `crates/worldwake-sim/src/tick_step.rs` — add a focused test for same-tick cross-agent ordering visibility and stability.
2. `crates/worldwake-ai/tests/golden_emergent.rs` — update the existing same-place golden only if the new substrate makes the assertion clearer and less implicit.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
3. `cargo clippy --workspace`
