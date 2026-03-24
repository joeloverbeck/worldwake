# S24TYPINVDOM-001: Define DirtySet newtype and unit tests

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S21 (completed), S22 (completed)

## Problem

The AI invalidation system uses an opaque `dirty: bool` and `Vec<DirtyReason>` to track why agents replan. This ticket introduces the `DirtySet` bitflag newtype that will replace both, enabling typed domain-level replan diagnostics. This is a standalone additive ticket — no existing code is modified.

## Assumption Reassessment (2026-03-24)

1. `DirtyReason` enum exists at `decision_trace.rs:597` with 7 variants: `NoPlan`, `PlanFinished`, `ReplanSignal`, `QueueTransition`, `BlockerCleanup`, `SnapshotChanged`, `QueuePatienceExhausted`. Confirmed via grep.
2. `AgentDecisionRuntime.dirty: bool` exists at `decision_runtime.rs:60`. Confirmed.
3. S22 introduced 5 `dirty = true` sites in `frame.rs` (lines 197, 405), `active_action.rs` (lines 205, 223), and `mod.rs` (line 378) that set boolean but push no `DirtyReason`. Confirmed — these are the frame lifecycle domains the spec adds bits for.
4. No `dirty_set.rs` module exists yet. Confirmed.
5. Single-layer ticket: purely additive type definition. No ordering or heuristic concerns.
11. No mismatch — codebase matches spec assumptions exactly.

## Architecture Check

1. Hand-rolled `u16` bitflags avoids adding an external `bitflags` crate, consistent with the project's minimal-dependency policy (serde, bincode, rand_chacha, blake3).
2. No backwards-compatibility shims — this is a new type with no legacy surface.

## Verification Layers

1. `DirtySet::is_empty()` correctness → focused unit tests
2. `DirtySet::is_snapshot_only()` domain separation → focused unit tests
3. `DirtySet::display_names()` human-readable output → focused unit tests
4. Mask coverage (STRUCTURAL_MASK, SNAPSHOT_MASK, FRAME_MASK) → focused unit tests
5. Single-layer ticket: purely additive, no integration or golden surface needed.

## What to Change

### 1. Create `dirty_set.rs` module in `worldwake-ai/src/`

Define `DirtySet` newtype over `u16` with:
- 6 structural bit constants: `NO_PLAN` (0), `PLAN_FINISHED` (1), `REPLAN_SIGNAL` (2), `QUEUE_TRANSITION` (3), `BLOCKER_CLEANUP` (4), `QUEUE_PATIENCE` (5)
- 6 snapshot bit constants: `POSITION` (6), `NEEDS` (7), `WOUNDS` (8), `COMMODITY` (9), `UNIQUE_ITEMS` (10), `FACILITIES` (11)
- 3 frame lifecycle bit constants: `FRAME_BLOCKAGE` (12), `FRAME_PATIENCE` (13), `ASSUMPTION_FAILED` (14)
- Aggregate masks: `STRUCTURAL_MASK`, `SNAPSHOT_MASK`, `FRAME_MASK`
- Methods: `is_empty()`, `insert()`, `is_snapshot_only()`, `contains()`, `display_names()`
- Trait impls: `Default` (zero), `Display`, `BitOr`, `BitOrAssign`, `Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`

### 2. Register module in `lib.rs`

Add `mod dirty_set;` and `pub use dirty_set::DirtySet;` to `lib.rs`.

## Files to Touch

- `crates/worldwake-ai/src/dirty_set.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module declaration and re-export)

## Out of Scope

- Modifying `AgentDecisionRuntime.dirty` field type (S24TYPINVDOM-002)
- Modifying `observation_snapshot_changed()` return type (S24TYPINVDOM-003)
- Removing `DirtyReason` enum (S24TYPINVDOM-004)
- Modifying any existing test or golden test
- Modifying `decision_trace.rs`, `observation.rs`, `planning.rs`, `frame.rs`, `active_action.rs`, `mod.rs`, `failure_handling.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `dirty_set_default_is_empty` — `DirtySet::default().is_empty()` returns true
2. `dirty_set_insert_sets_bits` — after `insert(NEEDS)`, `is_empty()` is false and `contains(NEEDS)` is true
3. `dirty_set_is_snapshot_only_true_for_snapshot_bits` — `NEEDS | POSITION` returns `is_snapshot_only() == true`
4. `dirty_set_is_snapshot_only_false_with_structural` — `NEEDS | NO_PLAN` returns `is_snapshot_only() == false`
5. `dirty_set_is_snapshot_only_false_with_frame` — `NEEDS | FRAME_BLOCKAGE` returns `is_snapshot_only() == false`
6. `dirty_set_contains_checks_individual_and_combined` — `contains` on single and multi-bit sets
7. `dirty_set_display_names_empty_shows_clean` — empty set displays "CLEAN"
8. `dirty_set_display_names_single` — single bit displays its name
9. `dirty_set_display_names_multiple` — multiple bits pipe-separated
10. `dirty_set_snapshot_mask_covers_six_bits` — `SNAPSHOT_MASK` contains all 6 snapshot bits and no others
11. `dirty_set_frame_mask_covers_three_bits` — `FRAME_MASK` contains all 3 frame bits and no others
12. `dirty_set_structural_mask_covers_six_bits` — `STRUCTURAL_MASK` contains all 6 structural bits and no others
13. `dirty_set_bitor_combines` — `NEEDS | POSITION` contains both
14. `dirty_set_bitor_assign_combines` — `|=` operator works correctly
15. Existing suite: `cargo test -p worldwake-ai` — all existing tests unchanged

### Invariants

1. `DirtySet` uses no external bitflags crate — hand-rolled `u16` only
2. Bit 15 remains reserved (unused)
3. No existing code is modified — purely additive

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/dirty_set.rs` (inline `#[cfg(test)]` module) — 14 unit tests covering all methods, masks, and trait impls as listed above

### Commands

1. `cargo test -p worldwake-ai dirty_set` — targeted new tests
2. `cargo test -p worldwake-ai` — full crate regression
3. `cargo clippy -p worldwake-ai` — no new warnings

## Outcome

- **Completion date**: 2026-03-24
- **What changed**: Created `crates/worldwake-ai/src/dirty_set.rs` with `DirtySet` newtype over `u16` (15 bit constants, 3 aggregate masks, 5 methods, 8 trait impls). Registered module and re-export in `crates/worldwake-ai/src/lib.rs`.
- **Deviations**: None. Implementation matches ticket exactly.
- **Verification**: 14/14 new unit tests pass, full crate regression (591 tests) green, clippy clean.
