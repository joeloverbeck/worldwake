# HARPREE14-006: Modularize candidate generation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- internal refactor of candidate_generation.rs
**Deps**: None (Wave 2, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A02

## Problem

`generate_candidates()` in `candidate_generation.rs` is a monolithic function that mixes need-based, enterprise, combat, and production candidate logic. Adding a new domain (e.g., social goals in E15+) requires editing this single function. This violates Principle 12 (System Decoupling).

## Assumption Reassessment (2026-03-11)

1. `generate_candidates()` is a single monolithic function at line 50 -- confirmed
2. It handles need, enterprise, combat, and production candidates all inline -- confirmed
3. It imports from `crate::enterprise` -- confirmed

## Architecture Check

1. Breaking into per-domain sub-functions within the same module is the minimal refactor that achieves extensibility without over-engineering.
2. No new files needed -- keep sub-generators as private functions in the same module.
3. Pure refactor: identical behavior, just reorganized.

## What to Change

### 1. Extract `generate_need_candidates()`

Move hunger, thirst, sleep, bladder, dirtiness, wash candidate logic into a private function that returns `Vec<GoalCandidate>` (or appends to a mutable vec).

### 2. Extract `generate_enterprise_candidates()`

Move restock, sell, produce candidate logic into a private function.

### 3. Extract `generate_combat_candidates()`

Move danger reduction, looting, healing candidate logic into a private function.

### 4. Extract `generate_production_candidates()`

Move recipe-driven production goal logic into a private function.

### 5. Refactor `generate_candidates()` as orchestrator

The top-level function calls each sub-generator and collects all candidates. It should pass the necessary state (snapshot, belief view, etc.) to each sub-generator.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Changing the candidate generation logic or adding new candidate types
- Creating new files or modules
- Decoupling enterprise imports (that's HARPREE14-008 / HARDEN-A03)
- Modifying the public API signature of `generate_candidates()`
- Changes to any other file

## Acceptance Criteria

### Tests That Must Pass

1. All existing candidate generation tests pass unchanged
2. Golden e2e hashes identical
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. `generate_candidates()` public signature unchanged
2. Same candidates generated in the same order for identical inputs
3. Golden e2e state hashes identical
4. No behavioral change of any kind

## Test Plan

### New/Modified Tests

1. No new tests needed -- this is a pure internal refactor. Existing tests validate identical behavior.

### Commands

1. `cargo test -p worldwake-ai candidate` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
