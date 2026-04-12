# S96OBLSAT-004: Tracker update in obligation commit handlers

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — modifies PostNotice and PostBounty commit handlers
**Deps**: archive/tickets/S96OBLSAT-001.md

## Problem

For satiation to work, the engine must record when obligation actions complete. Without tracker updates in the commit handlers, the `ObligationExecutionTracker` remains empty and satiation never activates.

## Assumption Reassessment (2026-04-12)

1. `commit_post_notice` at `crates/worldwake-systems/src/artifact_actions.rs:1080`. Signature: `fn commit_post_notice(def, instance, _context, _event_log, _rng, txn) -> Result<CommitOutcome, ActionError>`. Has full `WorldTxn` access. Currently returns `CommitOutcome::empty()` at line 1117.
2. `commit_post_bounty` at line 994. Same signature pattern. Returns `CommitOutcome::empty()` at line 1040.
3. Both handlers have access to `instance.actor` (the agent performing the action) and `txn` (the world transaction). The current tick is available from `instance` or `def`.
4. Cross-system boundary: worldwake-systems writes to worldwake-core component (`ObligationExecutionTracker`). This is state-mediated (FND-26) — no direct call to ranking.

## Architecture Check

1. Writing to an agent's own component during action commit is the standard pattern for recording action outcomes (analogous to how wound systems update `WoundList`). State-mediated, not a cross-system call.
2. No backwards-compatibility shims. Additive change to existing commit functions.

## Verification Layers

1. Tracker populated after commit → focused unit test
2. State-mediated interaction (P26) → no direct coupling between systems
3. Single-layer ticket (authoritative action commit); ranking reads are verified in ticket 005.

## What to Change

### 1. Update `commit_post_notice`

After the existing artifact creation logic (before the `Ok(CommitOutcome::empty())`), append the current tick to the agent's `ObligationExecutionTracker.completion_ticks`:

```rust
let mut tracker = txn.get_obligation_execution_tracker(instance.actor)
    .unwrap_or_default();
tracker.completion_ticks.push(current_tick);
txn.set_component_obligation_execution_tracker(instance.actor, tracker);
```

The current tick is available from the action instance or def context.

### 2. Update `commit_post_bounty`

Same pattern as `commit_post_notice` — append current tick to tracker.

## Files to Touch

- `crates/worldwake-systems/src/artifact_actions.rs` (modify)

## Out of Scope

- Tracker pruning (happens during ranking context construction in ticket 005)
- PostNotice/PostBounty action mechanics, preconditions, or durations — unchanged
- Other action commit handlers — only PostNotice and PostBounty are obligation-class

## Acceptance Criteria

### Tests That Must Pass

1. After committing a PostNotice action, agent's `ObligationExecutionTracker` contains the commit tick
2. After committing a PostBounty action, same behavior
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Only PostNotice and PostBounty commits record to the tracker (no other actions)
2. The tracker is append-only during commit; pruning is deferred to ranking

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_actions.rs` (inline `#[cfg(test)]`) or integration test — verify tracker is populated after obligation action commit

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
