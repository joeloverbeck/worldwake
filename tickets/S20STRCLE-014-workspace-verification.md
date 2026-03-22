# S20STRCLE-014: Final workspace verification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S20STRCLE-001 through S20STRCLE-013

## Problem

After all sub-module extractions and test relocations are complete, a full workspace-level verification pass is needed to confirm no cross-crate regressions, no clippy warnings, and that golden test count is unchanged.

## Assumption Reassessment (2026-03-22)

1. All prior S20STRCLE tickets verified at the crate level (`cargo test -p worldwake-ai`). This ticket adds workspace-wide and golden-specific verification.
2. Golden test files: `golden_ai_decisions.rs`, `golden_care.rs`, `golden_combat.rs`, `golden_determinism.rs`, `golden_emergent.rs`, `golden_offices.rs`, `golden_production.rs`, `golden_social.rs`, `golden_supply_chain.rs`, `golden_trade.rs` — 10 files verified via `ls`.
3. No external crates import `agent_tick` or `search` module internals directly — all go through `lib.rs` re-exports (verified via grep).
4–12. N/A — verification-only ticket.

## Architecture Check

1. This ticket produces no code changes — it is a verification gate that confirms the entire S20 spec is complete.
2. No backward-compatibility shims.

## Verification Layers

1. Full workspace test suite → `cargo test --workspace`.
2. Full workspace clippy → `cargo clippy --workspace`.
3. Golden test count → `cargo test -p worldwake-ai --test 'golden_*' -- --list | wc -l` matches pre-split baseline.
4. `git diff --stat` confirms changes are confined to `crates/worldwake-ai/src/`.

## What to Change

### 1. No code changes

This is a verification-only ticket. Run the commands below and confirm all pass.

### 2. Record baseline golden test count

Before starting S20STRCLE-001, record the golden test count. After S20STRCLE-013 is complete, verify the count matches.

## Files to Touch

- None (verification only)

## Out of Scope

- Any code modifications
- Any new features or refactoring beyond S20 scope
- Changes to other crates

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all tests pass
2. `cargo clippy --workspace` — no new warnings
3. Golden test count unchanged (compare pre-split vs post-split `cargo test -p worldwake-ai --test 'golden_*' -- --list | wc -l`)
4. `git diff --stat` shows changes only in `crates/worldwake-ai/src/`

### Invariants

1. Zero behavioral change across entire workspace
2. No new dependencies introduced
3. All `lib.rs` re-exports resolve correctly
4. All golden tests produce identical results

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `cargo test -p worldwake-ai --test 'golden_*' -- --list | wc -l`
4. `git diff --stat`
