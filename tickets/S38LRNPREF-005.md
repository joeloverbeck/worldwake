# S38LRNPREF-005: Harvest and trade source reliability recording

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — harvest and trade action handlers in worldwake-systems
**Deps**: S38LRNPREF-001, S38LRNPREF-002

## Problem

Agents complete or fail harvest and trade actions without recording source reliability. After this ticket, successful acquisitions and source-intrinsic failures update `SourceReliability`. External interruptions (combat abort) explicitly do NOT penalize source reliability — the source didn't fail, the agent was interrupted.

## Assumption Reassessment (2026-04-02)

1. `commit_harvest` at `crates/worldwake-systems/src/production_actions.rs:543` — signature: `fn commit_harvest(def: &ActionDef, instance: &ActionInstance, _context: &ActionExecutionContext<'_>, _rng: &mut DeterministicRng, txn: &mut WorldTxn<'_>)`.
2. `commit_trade` at `crates/worldwake-systems/src/trade_actions.rs:341` — same signature pattern.
3. `abort_harvest` and `abort_trade` exist in their respective files.
4. Harvest actions are recipe-driven — the commodity is determined by the recipe's output. The source entity is the workstation/facility location.
5. Trade actions use `TradeActionPayload` — the counterparty entity and commodity are in the payload.
6. `StartFailed` handling: harvest start failure (source depleted, no facility) should increment `failed_attempts`. Need to verify where start failures are handled — likely in `start_harvest` returning an error or via the `BestEffort` path in `tick_step.rs`.
7. Trade rejection: need to identify the exact code path for trade rejection vs. successful trade in `commit_trade`.
8. `SourceReliability::enforce_limits` from S38LRNPREF-002 must be called after recording.

## Architecture Check

1. Recording in commit/abort handlers matches the travel experience pattern (S38LRNPREF-004). Keeps experience recording co-located with the action lifecycle.
2. The distinction between source-intrinsic failure (depleted, rejected) and external interruption (combat abort) is architecturally clean — only the source's reliability is tracked, not location danger (that's `RouteExperience`'s domain). This maintains system decoupling (P26).
3. No backward-compatibility shims.

## Verification Layers

1. Successful harvest increments `successful_acquisitions` → focused unit test
2. Failed harvest (depleted) increments `failed_attempts` → focused unit test
3. Harvest abort (external) does NOT update source reliability → focused unit test
4. Successful trade increments `successful_acquisitions` → focused unit test
5. Trade rejection increments `failed_attempts` → focused unit test
6. Trade abort (external) does NOT update source reliability → focused unit test
7. Single-layer ticket (worldwake-systems action handlers); verification via focused tests on authoritative state.

## What to Change

### 1. Modify `commit_harvest`

After existing commit logic:
1. Get agent's `SourceReliability` (or create default if absent).
2. Determine `SourceKey { entity: source_entity, commodity }` from the recipe output and facility.
3. Increment `successful_acquisitions`, update `last_attempt_tick`.
4. If agent has `PreferenceProfile`, call `enforce_limits`.
5. Write updated `SourceReliability` back to world.

### 2. Handle harvest start failure

Where harvest `StartFailed` is handled (the `BestEffort` path or start function error):
1. If the failure is source-intrinsic (depleted resource, no available facility): increment `failed_attempts`.
2. Build `SourceKey` from the attempted recipe/facility.

### 3. Modify `commit_trade`

After existing commit logic:
1. Get agent's `SourceReliability` (or create default).
2. Determine `SourceKey { entity: counterparty, commodity }` from `TradeActionPayload`.
3. On successful trade: increment `successful_acquisitions`.
4. On trade rejection: increment `failed_attempts`.
5. Update `last_attempt_tick`, call `enforce_limits`.

### 4. Explicit no-op on abort

Document in `abort_harvest` and `abort_trade` (via comment) that external aborts intentionally do NOT update source reliability. This makes the design decision visible to future readers.

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify — commit_harvest, start_harvest failure path)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — commit_trade)

## Out of Scope

- Travel experience recording (S38LRNPREF-004)
- Source reliability discount in ranking (S38LRNPREF-007)
- Golden tests (S38LRNPREF-008)

## Acceptance Criteria

### Tests That Must Pass

1. Successful harvest → `successful_acquisitions` incremented for correct `SourceKey`
2. Failed harvest (source depleted) → `failed_attempts` incremented
3. Harvest abort (external interruption) → no `SourceReliability` update
4. Successful trade → `successful_acquisitions` incremented for correct `SourceKey`
5. Trade rejection → `failed_attempts` incremented
6. Trade abort (external interruption) → no `SourceReliability` update
7. `last_attempt_tick` updated on all recording events
8. Eviction called after recording (capacity limit respected)
9. Existing suite: `cargo test --workspace`

### Invariants

1. Source reliability tracks source-intrinsic outcomes only, never external interruptions (P26 — system decoupling)
2. `SourceKey` correctly identifies entity + commodity pair
3. Binary eviction enforced after every record update

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` (new focused tests) — successful harvest recording, failed harvest recording, abort no-op
2. `crates/worldwake-systems/src/trade_actions.rs` (new focused tests) — successful trade recording, trade rejection recording, abort no-op

### Commands

1. `cargo test -p worldwake-systems production`
2. `cargo test -p worldwake-systems trade`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
