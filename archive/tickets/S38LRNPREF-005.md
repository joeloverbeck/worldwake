# S38LRNPREF-005: Harvest and trade source reliability recording

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — sim start-failure callback boundary plus harvest/trade recording in worldwake-systems
**Deps**: S38LRNPREF-001, S38LRNPREF-002

## Problem

Agents complete or fail harvest and trade actions without recording source reliability. After this ticket, successful acquisitions and source-intrinsic failures update `SourceReliability`. External interruptions (combat abort) explicitly do NOT penalize source reliability — the source didn't fail, the agent was interrupted.

## Assumption Reassessment (2026-04-02)

1. `commit_harvest` at `crates/worldwake-systems/src/production_actions.rs:543` — signature: `fn commit_harvest(def: &ActionDef, instance: &ActionInstance, _context: &ActionExecutionContext<'_>, _rng: &mut DeterministicRng, txn: &mut WorldTxn<'_>)`.
2. `commit_trade` at `crates/worldwake-systems/src/trade_actions.rs:341` — same signature pattern.
3. `abort_harvest` and `abort_trade` exist in their respective files.
4. Harvest actions are recipe-driven — the commodity is determined by the recipe's output. The source entity is the workstation/facility location.
5. Trade actions use `TradeActionPayload` — the counterparty entity and commodity are in the payload.
6. Harvest `StartFailed` cannot be implemented in `production_actions.rs` alone. Live start failures can occur in `start_gate.rs` before `start_harvest` runs, so source-intrinsic failure recording needs a dedicated sim start-failure callback boundary.
7. Trade rejection: need to identify the exact code path for trade rejection vs. successful trade in `commit_trade`.
8. `SourceReliability::enforce_limits` from S38LRNPREF-002 must be called after recording.

## Architecture Check

1. Successful acquisitions and trade-rejection aftermath still belong in commit/abort handlers, but harvest start failure needs a sim-owned start-failure hook because the live authoritative rejection can happen before `start_harvest` is entered.
2. The distinction between source-intrinsic failure (depleted, rejected) and external interruption (combat abort) remains architecturally clean — only the source's reliability is tracked, not location danger (that's `RouteExperience`'s domain).
3. No backward-compatibility shims.

## Verification Layers

1. Successful harvest increments `successful_acquisitions` → focused unit test
2. Failed harvest (depleted) increments `failed_attempts` → focused unit test
3. Harvest abort (external) does NOT update source reliability → focused unit test
4. Successful trade increments `successful_acquisitions` → focused unit test
5. Trade rejection increments `failed_attempts` → focused unit test
6. Trade abort (external) does NOT update source reliability → focused unit test
7. Mixed-layer ticket: focused sim proof for start-failure aftermath plus focused authoritative systems proof for commit/abort recording.

## What to Change

### 1. Modify `commit_harvest`

After existing commit logic:
1. Get agent's `SourceReliability` (or create default if absent).
2. Determine `SourceKey { entity: source_entity, commodity }` from the recipe output and facility.
3. Increment `successful_acquisitions`, update `last_attempt_tick`.
4. If agent has `PreferenceProfile`, call `enforce_limits`.
5. Write updated `SourceReliability` back to world.

### 2. Add a start-failure aftermath hook at the sim boundary

In the live start gate / handler boundary:
1. Add a dedicated optional start-failure callback on `ActionHandler` that receives the action definition, actor, resolved targets, effective payload, execution context, and authoritative `ActionError` that caused start failure.
2. Invoke that callback from `start_gate.rs` before returning recoverable start failures so harvest can persist source-intrinsic failure aftermath even when the action never starts.
3. Use that hook for harvest only in this ticket.

### 3. Handle harvest start failure

In the harvest start-failure callback:
1. If the failure is source-intrinsic (resource source missing/depleted or facility gone for the attempted harvest source): increment `failed_attempts`.
2. Do NOT penalize reservation contention or other non-source failures.
3. Build `SourceKey` from the attempted facility + output commodity, update `last_attempt_tick`, and enforce limits.

### 4. Modify `commit_trade`

After existing commit logic:
1. Get agent's `SourceReliability` (or create default).
2. Determine `SourceKey { entity: counterparty, commodity }` from `TradeActionPayload`.
3. On successful trade: increment `successful_acquisitions`.
4. On trade rejection: increment `failed_attempts`.
5. Update `last_attempt_tick`, call `enforce_limits`.

### 5. Explicit no-op on external abort

Document in `abort_harvest` and the external-interruption branch of `abort_trade` (via comment or structure) that external aborts intentionally do NOT update source reliability. Trade rejection still counts as source failure.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify — add start-failure callback surface)
- `crates/worldwake-sim/src/start_gate.rs` (modify — invoke start-failure callback before returning recoverable start failures)
- `crates/worldwake-systems/src/production_actions.rs` (modify — commit_harvest, harvest start-failure callback, abort comment/no-op)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — commit_trade)

## Out of Scope

- Travel experience recording (S38LRNPREF-004)
- Source reliability discount in ranking (S38LRNPREF-007)
- Golden tests (S38LRNPREF-008)

## Acceptance Criteria

### Tests That Must Pass

1. Successful harvest → `successful_acquisitions` incremented for correct `SourceKey`
2. Failed harvest start (source depleted / source gone) → `failed_attempts` incremented
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

1. `crates/worldwake-sim/src/start_gate.rs` or nearby focused sim proof — harvest start-failure callback records source-intrinsic failure and ignores non-source failures
2. `crates/worldwake-systems/src/production_actions.rs` (new focused tests) — successful harvest recording, abort no-op
3. `crates/worldwake-systems/src/trade_actions.rs` (new focused tests) — successful trade recording, trade rejection recording, abort no-op

### Commands

1. `cargo test -p worldwake-sim start_gate`
2. `cargo test -p worldwake-systems production`
3. `cargo test -p worldwake-systems trade`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

Completed: 2026-04-02

What changed:
- Added a new sim-layer `on_start_failure` callback surface on `ActionHandler` and invoked it from `start_gate.rs`, so recoverable start failures can persist aftermath before action instantiation.
- Kept that new start-failure path aligned with live start semantics by only committing the failure transaction when the hook actually records state or tags.
- Added shared source-reliability recording helpers in `crates/worldwake-systems/src/experience_recording.rs`.
- Wired harvest source reliability updates in `production_actions.rs` for successful harvests and source-intrinsic harvest start failures, with explicit no-op behavior for external aborts.
- Wired trade source reliability updates in `trade_actions.rs` for successful trades and explicit trade rejection, with external interruptions left as no-op for source reliability.
- Added focused tests covering successful acquisition recording, source-intrinsic failure recording, capacity enforcement, and external-abort no-op behavior.

Deviations from original plan:
- The ticket was corrected before implementation from a systems-only change to a mixed-layer sim + systems ticket, because harvest `StartFailed` aftermath could not be implemented lawfully inside `production_actions.rs` alone.
- Source reliability recording helpers were factored into a small shared systems module instead of duplicating nearly identical update logic in harvest and trade handlers.

Verification results:
- `cargo test -p worldwake-systems harvest_commit_records_successful_source_reliability_and_enforces_capacity -- --nocapture`
- `cargo test -p worldwake-systems successful_trade_transfers_goods_and_coin_with_trade_tags_and_provenance -- --nocapture`
- `cargo test -p worldwake-systems negotiation_walkaway_records_failed_trade_observations -- --nocapture`
- `cargo test -p worldwake-systems harvest_start_failure_records_source_intrinsic_reliability_failure -- --nocapture`
- `cargo test -p worldwake-systems harvest_external_abort_does_not_update_source_reliability -- --nocapture`
- `cargo test -p worldwake-systems explicit_external_trade_abort_does_not_update_source_reliability -- --nocapture`
- `cargo test -p worldwake-sim start_gate -- --nocapture`
- `cargo test -p worldwake-sim`
- `cargo test -p worldwake-systems`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
