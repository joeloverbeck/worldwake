# E07ACTFRA-009: Tick/Progress + Commit Validation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines action progression and completion
**Deps**: E07ACTFRA-004 (ActionInstance), E07ACTFRA-005 (Handler Registry), E07ACTFRA-008 (Start Gate)

## Problem

Active actions must progress deterministically each tick and commit when their duration expires. The tick function decrements remaining ticks and delegates to the handler. When `remaining_ticks` reaches 0, commit conditions are re-evaluated on authoritative state — if they fail, the action aborts cleanly with reservation release and replan signal emission.

## Assumption Reassessment (2026-03-09)

1. `ActionInstance.remaining_ticks` is a `u32` that decrements each tick — confirmed in E07ACTFRA-004 design.
2. `ActionDef.commit_conditions` are `Vec<Precondition>` — same type as start preconditions.
3. `EventTag::ActionCommitted` and `EventTag::ActionAborted` exist in core's `event_tag.rs` — confirmed.
4. `WorldTxn` supports reservation release — confirmed from `world_txn.rs`.
5. Commit conditions are checked on authoritative state (not KnowledgeView) — spec says "re-evaluate commit conditions on authoritative state".

## Architecture Check

1. `tick_action` is a free function taking the instance mutably. It returns `ActionProgress` to indicate whether the action continues or completed.
2. Commit validation is triggered automatically when `remaining_ticks == 0` — it is not a separate public API call.
3. On commit failure, the function handles abort internally: release reservations, emit abort event, produce `ReplanNeeded` (defined in E07ACTFRA-010, but the abort path calls into it).

## What to Change

### 1. Create `worldwake-sim/src/tick_action.rs`

Implement:
```rust
pub fn tick_action(
    instance: &mut ActionInstance,
    def: &ActionDef,
    handler_registry: &ActionHandlerRegistry,
    world: &mut World,
    current_tick: Tick,
) -> Result<TickOutcome, ActionError>
```

Where:
```rust
pub enum TickOutcome {
    Continuing,
    Committed,
    Aborted { reason: AbortReason },
}
```

Logic:
1. Assert instance status is `Active`
2. Decrement `remaining_ticks`
3. Call `handler.on_tick()` — if it returns `ActionProgress::Complete`, skip to commit
4. If `remaining_ticks == 0` or handler signals complete:
   a. Evaluate `def.commit_conditions` on authoritative world state
   b. If all pass: call `handler.on_commit()`, set status to `Committed`, emit event with `EventTag::ActionCommitted`, release reservations
   c. If any fail: call `handler.on_abort()`, set status to `Aborted`, emit event with `EventTag::ActionAborted`, release reservations, return `Aborted` with reason
5. If still ticking: return `Continuing`

### 2. Update `worldwake-sim/src/lib.rs`

Declare module, re-export `tick_action` and `TickOutcome`.

## Files to Touch

- `crates/worldwake-sim/src/tick_action.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Interrupt/abort initiated externally (E07ACTFRA-010)
- ReplanNeeded signal struct (E07ACTFRA-010) — this ticket produces the abort reason, E07ACTFRA-010 wraps it
- Scheduler that calls tick_action each tick (E08)
- Concrete handler implementations (later epics)

## Acceptance Criteria

### Tests That Must Pass

1. An active action with `remaining_ticks = 3` decrements to 2 after one tick
2. An action with `remaining_ticks = 1` transitions to `Committed` if commit conditions pass
3. **T06**: An action whose commit conditions fail aborts cleanly — status becomes `Aborted`, reservations are released, abort event is emitted
4. `handler.on_tick()` is called each tick
5. `handler.on_commit()` is called exactly once on successful commit
6. `handler.on_abort()` is called on commit failure
7. Committed actions emit `EventTag::ActionCommitted`
8. Aborted actions emit `EventTag::ActionAborted`
9. Interrupted/aborted actions stop consuming time immediately (status change prevents further ticks)
10. Action effects mutate world state only through `WorldTxn`
11. Existing suite: `cargo test --workspace`

### Invariants

1. Spec 9.9: no action commits unless commit conditions are true at commit time
2. Remaining ticks decrement deterministically
3. Completed actions move to commit validation — no skipping
4. Reservations are always released on commit or abort

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_action.rs` — decrement tests, commit success path, commit failure → abort path, handler callback verification, event emission, reservation release

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
