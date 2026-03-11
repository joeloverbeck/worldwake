# HARPREE14-008: Decouple enterprise module from candidate generation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- API change between enterprise and candidate_generation
**Deps**: HARPREE14-006 (HARDEN-A02 must be done first)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A03

## Problem

`candidate_generation.rs` imports `crate::enterprise::restock_gap` and `crate::enterprise::opportunity_signal` directly. The enterprise module's internal API becomes a coupling point -- changes to enterprise internals force changes in candidate generation.

## Assumption Reassessment (2026-03-11)

1. `candidate_generation.rs` imports from `crate::enterprise` -- confirmed
2. HARPREE14-006 will have extracted `generate_enterprise_candidates()` as a sub-function -- assumed (hard dep)
3. Enterprise functions `restock_gap` and `opportunity_signal` exist -- confirmed

## Architecture Check

1. The decoupling follows Principle 12 (System Decoupling): enterprise analysis produces data, candidate generation consumes it, with an explicit data boundary.
2. An intermediate data struct (`EnterpriseSignal` or equivalent) makes the contract explicit and testable.
3. The orchestrator in `generate_candidates()` bridges the two: calls enterprise functions, converts to signals, passes to sub-generator.

## What to Change

### 1. Define `EnterpriseSignal` data type

A struct (or set of structs) representing the outputs of enterprise analysis: restock gaps, opportunity signals, etc. This can live in `enterprise.rs` or `candidate_generation.rs`.

### 2. Refactor `generate_enterprise_candidates()` to accept data, not call functions

Change the sub-generator (from HARPREE14-006) to accept `&[EnterpriseSignal]` or equivalent pre-computed data instead of calling `restock_gap()` / `opportunity_signal()` directly.

### 3. Move enterprise function calls to the orchestrator

The top-level `generate_candidates()` calls enterprise functions and passes results to `generate_enterprise_candidates()`.

### 4. Remove direct enterprise imports from candidate generation internals

The only enterprise import should be the signal type, not the analysis functions (unless the orchestrator needs them).

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/enterprise.rs` (modify -- add signal type or adjust exports)

## Out of Scope

- Changing enterprise analysis logic
- Adding new enterprise signals or goal types
- Modifying `generate_candidates()` public signature
- Changes outside the two listed files

## Acceptance Criteria

### Tests That Must Pass

1. All existing candidate generation tests pass unchanged
2. All existing enterprise tests pass unchanged
3. Golden e2e hashes identical
4. `cargo test --workspace` passes
5. `cargo clippy --workspace` -- no new warnings

### Invariants

1. `generate_candidates()` public signature unchanged
2. Same candidates generated for identical inputs
3. Enterprise analysis logic unchanged
4. Golden e2e state hashes identical

## Test Plan

### New/Modified Tests

1. Optionally add a unit test for `generate_enterprise_candidates()` with mock signals to verify it produces correct candidates from signal data alone.

### Commands

1. `cargo test -p worldwake-ai candidate` (targeted)
2. `cargo test -p worldwake-ai enterprise` (targeted)
3. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
4. `cargo test --workspace`
5. `cargo clippy --workspace`
