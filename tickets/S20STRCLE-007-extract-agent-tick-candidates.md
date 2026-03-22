# S20STRCLE-007: Extract agent_tick/candidates.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S20STRCLE-001

## Problem

The candidate/facility-queue management logic in `agent_tick/mod.rs` (~200 lines) handles facility queue expiry and candidate orchestration. Extracting it into a dedicated sub-module clarifies the "generate candidates" pre-planning phase.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `abandon_expired_facility_queues()` (line 680)
   - `abandon_expired_facility_queues_with_limit()` (line 697)
2. All private.
3–12. N/A — pure structural refactor.

## Architecture Check

1. Facility queue expiry is a self-contained concern that runs before candidate generation. Isolating it reduces `mod.rs` size and clarifies the pre-planning phase.
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `agent_tick/candidates.rs`

Move the listed functions. Add necessary `use` imports.

### 2. Update `agent_tick/mod.rs`

- Add `mod candidates;`
- Add `use candidates::*;`.
- Remove moved function bodies.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/candidates.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)

## Out of Scope

- Extracting other pipeline stages
- Changing function signatures, visibility, or logic
- Modifying `search.rs`
- Any changes outside `worldwake-ai`
- Reorganizing tests (S20STRCLE-013)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Zero behavioral change
2. Function signatures and visibility unchanged

## Test Plan

### New/Modified Tests

1. None — code motion only; verification is command-based.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
