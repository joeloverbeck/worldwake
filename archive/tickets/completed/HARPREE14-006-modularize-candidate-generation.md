# HARPREE14-006: Modularize candidate generation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- internal refactor of candidate_generation.rs
**Deps**: None (Wave 2, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A02

## Problem

`generate_candidates()` is no longer a single monolithic blob, but its top-level orchestration still mixes domain concerns directly. The function already delegates to fine-grained emitters, yet those emitters are wired inline from the public entrypoint rather than through explicit domain-level generators. That leaves the intended extension seams implicit, and it keeps enterprise, combat, need, and production concerns visually interleaved at the orchestration layer.

## Assumption Reassessment (2026-03-11)

1. `generate_candidates()` is at line 50, but it is already partially modularized through private `emit_*` helpers -- corrected
2. The current entrypoint still wires need, production, enterprise, and combat emitters inline -- confirmed
3. `candidate_generation.rs` imports `crate::enterprise::restock_gap`, but not `opportunity_signal` -- corrected
4. Candidate-generation unit tests already exist in `candidate_generation.rs`, and golden e2e coverage already exists in `tests/golden_e2e.rs` -- corrected

## Architecture Check

1. The code does not need a larger rewrite or new modules; the right remaining step is to introduce explicit per-domain generator helpers above the existing fine-grained emitters.
2. No new files are needed; keep the work inside `candidate_generation.rs`.
3. This should remain a behavior-preserving refactor. Enterprise API decoupling stays in `HARPREE14-008`.
4. Do not collapse need/production/combat logic into broad opaque functions if that would hide the existing precise helper boundaries; keep the current low-level emitters and add a thin domain orchestration layer.

## What to Change

### 1. Add `emit_need_candidates()`

Create a private domain-level helper that groups the existing hunger/thirst, sleep, relieve, and wash emitters without changing their internal logic.

### 2. Add `emit_enterprise_candidates()`

Create a private domain-level helper for restock-oriented enterprise candidate generation only. Do not pull additional enterprise analysis into this ticket.

### 3. Add `emit_combat_candidates()`

Create a private domain-level helper that groups danger reduction, healing, and looting emitters.

### 4. Add `emit_production_candidates()`

Create a private domain-level helper that wraps recipe-driven production candidate generation.

### 5. Keep `generate_candidates()` as a thin public orchestrator

The public entrypoint should read agent context, then delegate to the four domain helpers. The existing fine-grained `emit_*` helpers remain the implementation detail beneath those domain helpers.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Changing candidate selection behavior or adding new candidate types
- Removing or rewriting the existing fine-grained emitters beyond what is needed for domain grouping
- Creating new files or modules
- Decoupling enterprise imports or analysis data flow (that remains `HARPREE14-008` / HARDEN-A03)
- Modifying the public API signature of `generate_candidates()`
- Changes outside `candidate_generation.rs` unless tests need targeted reinforcement

## Acceptance Criteria

### Tests That Must Pass

1. Existing candidate-generation unit tests pass
2. Golden e2e hashes remain identical
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` passes with no new warnings

### Invariants

1. `generate_candidates()` public signature unchanged
2. Same candidates generated for identical inputs
3. Golden e2e state hashes identical
4. No behavioral change of any kind

## Test Plan

### New/Modified Tests

1. Add or adjust at most one focused unit test if needed to lock the corrected domain orchestration seam.
2. Keep existing candidate-generation and golden e2e tests as the primary regression coverage.

### Commands

1. `cargo test -p worldwake-ai candidate` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-11
- What actually changed:
  - Corrected the ticket assumptions to match the current codebase: `generate_candidates()` was already decomposed into focused private emitters with existing unit coverage.
  - Added explicit domain-level helpers in `candidate_generation.rs`: `emit_need_candidates()`, `emit_production_candidates()`, `emit_enterprise_candidates()`, and `emit_combat_candidates()`.
  - Kept the existing fine-grained emitters in place under those helpers so the public entrypoint is thinner without losing the current precise boundaries.
  - Added one focused unit test to verify the public orchestrator still reaches all four domain groups together.
- Deviations from original plan:
  - Did not perform a large extraction of candidate logic because that work had already effectively been done before this ticket was executed.
  - Did not decouple enterprise analysis from candidate generation; that remains correctly scoped to `HARPREE14-008`.
  - No new files or modules were introduced.
- Verification results:
  - `cargo test -p worldwake-ai candidate` passed
  - `cargo test -p worldwake-ai --test golden_e2e` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
