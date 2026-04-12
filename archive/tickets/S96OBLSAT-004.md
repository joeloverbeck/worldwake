# S96OBLSAT-004: Tracker update in obligation commit handlers

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — modifies PostNotice and PostBounty commit handlers
**Deps**: archive/tickets/S96OBLSAT-001.md

## Problem

For satiation to work, the engine must record when obligation actions complete. Without tracker updates in the commit handlers, the `ObligationExecutionTracker` remains empty and satiation never activates.

## Assumption Reassessment (2026-04-12)

1. `commit_post_notice` at `crates/worldwake-systems/src/artifact_actions.rs:1080`. Signature: `fn commit_post_notice(def, instance, _context, _event_log, _rng, txn) -> Result<CommitOutcome, ActionError>`. Has full `WorldTxn` access. Currently returns `CommitOutcome::empty()` at line 1117.
2. `commit_post_bounty` at line 994. Same signature pattern. Returns `CommitOutcome::empty()` at line 1040.
3. Both handlers have access to `instance.actor` (the agent performing the action) and `txn` (the world transaction). The authoritative commit tick is `txn.tick()`, and the generated component accessors on `WorldTxn` are `get_component_obligation_execution_tracker` / `set_component_obligation_execution_tracker`.
4. Cross-system boundary: worldwake-systems writes to worldwake-core component (`ObligationExecutionTracker`). This is state-mediated (FND-26) — no direct call to ranking.
5. `crates/worldwake-systems/src/artifact_actions.rs` already has focused commit tests for both handlers: `post_bounty_commits_social_artifact_with_contention_components` and `post_notice_commits_social_artifact_with_notice_content`. Those are the canonical proof surfaces for this ticket.

## Architecture Check

1. Writing to an agent's own component during action commit is the standard pattern for recording action outcomes (analogous to how wound systems update `WoundList`). State-mediated, not a cross-system call.
2. No backwards-compatibility shims. Additive change to existing commit functions.

## Verification Layers

1. Tracker populated after commit in `commit_post_bounty` → focused artifact-action test
2. Tracker populated after commit in `commit_post_notice` → focused artifact-action test
3. State-mediated interaction (P26) → no direct coupling between systems
4. Single-layer ticket (authoritative action commit); ranking reads are verified in ticket 005.

## What to Change

### 1. Update `commit_post_notice`

After the existing artifact creation logic (before the `Ok(CommitOutcome::empty())`), append the current tick to the agent's `ObligationExecutionTracker.completion_ticks`:

```rust
let mut tracker = txn
    .get_component_obligation_execution_tracker(instance.actor)
    .cloned()
    .unwrap_or_default();
tracker.completion_ticks.push(txn.tick());
txn.set_component_obligation_execution_tracker(instance.actor, tracker)?;
```
Use `txn.tick()` as the authoritative commit tick.

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

1. After committing a PostNotice action, the actor's `ObligationExecutionTracker` contains the commit tick
2. After committing a PostBounty action, the actor's `ObligationExecutionTracker` contains the commit tick
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Only PostNotice and PostBounty commits record to the tracker (no other actions)
2. The tracker is append-only during commit; pruning is deferred to ranking

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_actions.rs::post_bounty_commits_social_artifact_with_contention_components` — prove artifact creation plus tracker append at commit tick
2. `crates/worldwake-systems/src/artifact_actions.rs::post_notice_commits_social_artifact_with_notice_content` — prove artifact creation plus tracker append at commit tick

### Commands

1. `cargo test -p worldwake-systems --lib artifact_actions::tests::post_bounty_commits_social_artifact_with_contention_components -- --exact`
2. `cargo test -p worldwake-systems --lib artifact_actions::tests::post_notice_commits_social_artifact_with_notice_content -- --exact`
3. `cargo test -p worldwake-systems`
4. `cargo build --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Updated `commit_post_bounty` and `commit_post_notice` in `crates/worldwake-systems/src/artifact_actions.rs` to append `txn.tick()` to the actor's `ObligationExecutionTracker` at commit time.
- Reused the existing focused artifact commit tests to prove both artifact creation and tracker append behavior instead of introducing a separate test harness.
- Kept the interaction state-mediated through authoritative component writes on `WorldTxn`, with no direct coupling to ranking logic.

## Deviations

- The original ticket sketch used stale helper names and a vague tick source. The live contract uses `txn.tick()` plus `get_component_obligation_execution_tracker(...).cloned().unwrap_or_default()`.
- The original focused command sketches were too loose for this crate layout: `cargo test -p worldwake-systems <name>` compiled but ran zero tests, so the ticket now records the exact `--lib` module-qualified selectors that actually proved the owned surface.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::post_bounty_commits_social_artifact_with_contention_components -- --exact`
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::post_notice_commits_social_artifact_with_notice_content -- --exact`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ticket file status: archived as untracked file (`archive/tickets/S96OBLSAT-004.md`); original active path removed
