# S30-007: Increase EXHAUSTION_SKIP_TTL to optimal value

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — EXHAUSTION_SKIP_TTL constant value
**Deps**: S30-006 (driver reset workaround removed; save/load parity proven)

## Problem

`EXHAUSTION_SKIP_TTL` is currently `16` ticks (`planning.rs:166`). This was a conservative value chosen because higher values (e.g., 32) broke `golden_save_load_round_trip_under_ai` when the exhaustion cache was lost at save/load boundaries. With save/load parity established (S30-001 through S30-006), the cache now survives boundaries, so the TTL can be increased for better planning performance — agents skip fewer redundant re-searches of goals known to be unsatisfiable.

## Assumption Reassessment (2026-03-27)

1. `EXHAUSTION_SKIP_TTL` is `16` at `planning.rs:166`. Used in `build_candidate_plans()` at line 174.
2. The golden-perf campaign (exp-005, exp-016) showed TTL=32 broke save/load determinism because the cache was lost. That root cause is now fixed by S30-001 through S30-006.
3. S31 will eventually remove the TTL entirely in favor of condition-based invalidation. This is an interim optimization.
4. The optimal value should be determined empirically by re-running golden tests with different TTL values (32, 64, indefinite) and verifying determinism holds.
5. All golden tests must pass at the new TTL, including `golden_save_load_round_trip_under_ai`.

## Architecture Check

1. Changing a constant is the simplest possible change. No structural impact.
2. No shims — the old value is simply replaced.

## Verification Layers

1. Determinism preserved → all golden save/load tests pass at new TTL
2. No behavioral regression → full golden suite passes
3. Single-layer ticket (constant value change) — no cross-layer mapping needed.

## What to Change

### 1. Increase `EXHAUSTION_SKIP_TTL` in `planning.rs`

Change from `16` to the empirically determined optimal value (start with `32`, test `64` and higher if 32 is stable). The spec suggests 32+ as the target.

```rust
const EXHAUSTION_SKIP_TTL: u64 = 32; // Was 16; safe after S30 save/load parity
```

### 2. Re-capture golden hashes

If any golden tests use world-hash assertions that change due to different agent decisions (agents now skip re-searching exhausted goals for longer, potentially choosing different actions), the golden hashes must be re-captured. This is expected and acceptable — the new hashes reflect the improved behavior.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — change `EXHAUSTION_SKIP_TTL` constant)

## Out of Scope

- Removing `EXHAUSTION_SKIP_TTL` entirely (S31 — condition-based invalidation)
- Adding invalidation conditions to `ExhaustionEntry` (S31)
- Any structural changes to the exhaustion cache
- Save/load format changes
- Performance benchmarking infrastructure (use existing golden-perf harness)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_save_load_round_trip_under_ai` passes at new TTL value
2. All golden tests: `cargo test -p worldwake-ai golden`
3. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. Determinism preserved at new TTL — identical seeds produce identical outcomes
2. Save/load parity maintained — uninterrupted and resumed runs match
3. No new ECS components
4. Exponential backoff behavior unchanged (only the skip window duration changes)

## Test Plan

### New/Modified Tests

1. None — existing golden suite is the verification boundary. Golden hashes may need re-capture if agent behavior diverges due to longer skip windows.

### Commands

1. `cargo test -p worldwake-ai golden_save_load`
2. `cargo test -p worldwake-ai golden`
3. `cargo clippy --workspace && cargo test --workspace`
