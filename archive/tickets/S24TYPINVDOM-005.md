# S24TYPINVDOM-005: Workspace verification and trace output validation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S24TYPINVDOM-004 (all code changes complete)

## Problem

Final verification gate for S24. Confirms workspace-wide build health, golden test behavioral equivalence, trace output quality, and absence of any `DirtyReason` residue. This is a verification-only ticket with no code changes unless defects are discovered.

## Assumption Reassessment (2026-03-24)

1. After S24TYPINVDOM-001 through -004, `DirtySet` replaces both `dirty: bool` on `AgentDecisionRuntime` and `Vec<DirtyReason>` on traces. All mutation/read/clear sites are migrated. `DirtyReason` enum is removed.
2. Golden tests should pass unchanged because S24 is a representation change with identical behavioral semantics — agents replan under exactly the same conditions as before.
3. `dump_agent()` output should now show typed domain names (e.g., `dirty: NEEDS|POSITION`) instead of `SnapshotChanged`.
4. `summary()` output should include dirty domain names for Planning outcomes.
5. `cargo clippy --workspace` should produce no new warnings.
11. No mismatch — this is a verification-only ticket.

## Architecture Check

1. No code changes — verification only.
2. No backwards-compatibility concerns.

## Verification Layers

1. Workspace build → `cargo build --workspace`
2. Workspace tests → `cargo test --workspace`
3. Workspace lint → `cargo clippy --workspace`
4. `DirtyReason` absence → grep verification across all source files
5. Trace output quality → enable tracing in a golden test, inspect `dump_agent()` output for typed domain names

## What to Change

### 1. Run workspace verification suite

- `cargo build --workspace` — all crates compile
- `cargo test --workspace` — all tests pass (including golden tests)
- `cargo clippy --workspace` — no new warnings

### 2. Verify DirtyReason removal

- `grep -r "DirtyReason" crates/` — returns no hits in source files (only archive/specs/reports)

### 3. Verify trace output quality

Enable tracing in an existing golden test (e.g., one in `golden_ai_decisions.rs` or `golden_emergent.rs`) and inspect `dump_agent()` output:
- Planning outcomes show `dirty: NEEDS|POSITION` (or other specific domain names) instead of generic `SnapshotChanged`
- Frame lifecycle replans show `dirty: FRAME_BLOCKAGE` or `dirty: FRAME_PATIENCE` or `dirty: ASSUMPTION_FAILED`
- Structural replans show `dirty: NO_PLAN` or `dirty: PLAN_FINISHED` etc.

### 4. Verify behavioral equivalence

All golden tests pass without hash changes — the underlying replan triggers are identical, only the diagnostic representation changed.

## Files to Touch

- None (verification-only). If defects are discovered, the fix belongs in this ticket's scope but the specific files depend on the defect.

## Out of Scope

- Adding new golden tests for S24 (the spec does not require new scenarios — S24 is a representation change)
- Modifying any code unless a defect is discovered during verification
- Performance benchmarking of `DirtySet` vs `Vec<DirtyReason>` (the bitflag is trivially faster but measurement is unnecessary)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all tests pass (zero failures)
2. `cargo clippy --workspace` — no new warnings
3. `grep -r "DirtyReason" crates/` — zero hits in Rust source files
4. Manual inspection: `dump_agent()` output shows typed domain names for at least one snapshot-triggered replan and one structural replan

### Invariants

1. All golden tests produce identical behavioral outcomes (same agent actions, same event log entries) — S24 changes representation only
2. No `DirtyReason` enum, variant, import, or reference exists in compiled source code
3. `DirtySet` is the single authoritative and diagnostic representation for invalidation domains

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo build --workspace` — full workspace build
2. `cargo test --workspace` — full workspace regression
3. `cargo clippy --workspace` — lint check
4. `grep -r "DirtyReason" crates/` — residue check (expect zero Rust source hits)

## Outcome

- **Completion date**: 2026-03-25
- **What changed**: Verification-only — no code changes required. All checks passed on first run.
- **Verification results**:
  - `cargo build --workspace` — clean compile
  - `cargo test --workspace` — all tests pass (0 failures)
  - `cargo clippy --workspace` — no warnings
  - `grep -r "DirtyReason" crates/` — zero hits in Rust source files
  - Trace output quality confirmed: `summary()` and `dump_agent()` show typed domain names (e.g., `NEEDS|POSITION`, `NO_PLAN`, `FRAME_BLOCKAGE`) via `DirtySet::display_names()`
- **Deviations**: None — all acceptance criteria met as specified
