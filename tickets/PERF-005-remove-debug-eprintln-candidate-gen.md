# PERF-005: Remove debug `eprintln!` in `emit_report_found_candidates`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` candidate generation
**Deps**: None

## Problem

`emit_report_found_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:3355` contains a debug `eprintln!` that fires for every expectation record whose state is not `Resolved { outcome: Found* }`. In soak runs with agents accumulating expectation records, this produces massive stderr output. On WSL2, stderr pipe saturation can block the process entirely, causing apparent hangs.

The line was identified during initial investigation of the CI performance regression. It was already removed in the working tree during this session but has not been committed.

## Assumption Reassessment (2026-04-07)

1. Line 3355 confirmed removed in working tree: `git diff -- crates/worldwake-ai/src/candidate_generation.rs` shows the `eprintln!` deletion.
2. No other `eprintln!` calls exist in hot production paths — confirmed via grep. Remaining `eprintln!` calls are in `decision_trace.rs` (diagnostic dump, not called during simulation) and `soak_seed_perf.rs` (binary error exit).
3. The `continue` on the following line is preserved — control flow unchanged.

## Architecture Check

1. Debug prints in hot paths violate the general principle that performance may compress computation but not causality (FND-12) — the I/O cost of formatting and writing debug output on every record is pure waste with no causal value.
2. No backwards-compatibility shims.

## Verification Layers

1. No behavioral change — the `eprintln!` was pure side-effect debug output
2. Single-layer ticket; no cross-system verification required.

## What to Change

### 1. Delete the `eprintln!` at candidate_generation.rs:3355

The line `eprintln!("[ReportFound] record {:?} state={:?} — not Found*", record.id, record.state);` is removed. The `else { continue; }` block remains.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — already done in working tree)

## Out of Scope

- Adding structured logging or tracing infrastructure
- Auditing other debug prints

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai`
2. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate generation produces identical candidates (no behavioral change)

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; the change is a one-line deletion with no behavioral effect.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
