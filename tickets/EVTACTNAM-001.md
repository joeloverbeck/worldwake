# EVTACTNAM-001: Add action_name to EventPayload for action lifecycle events

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — EventPayload extended with new field
**Deps**: None

## Problem

ActionCommitted events in the CLI display show domain tags (e.g., "Inventory, Transfer, ActionCommitted") but not the action name (e.g., "pick_up"). The CLI's `event <id>` view can show action names for ActionStarted events (by querying the scheduler's active actions), but ActionCommitted events can't use this approach because the action is removed from the scheduler before the committed event is emitted. The CLI workaround (looking up active actions) only works for ActionStarted, creating an asymmetry.

The root fix: store the action name on the event itself at emission time, when the action def is still in scope.

## Assumption Reassessment (2026-04-04)

1. `EventPayload` at `crates/worldwake-core/src/event_record.rs:45-57` has 11 fields. No `action_name` field. Confirmed.
2. `EventTag::ActionStarted`, `ActionCommitted`, `ActionAborted`, `ActionInterrupted` at `event_tag.rs:12-15`. These tag events that should carry the action name.
3. ActionStarted event emitted at `crates/worldwake-sim/src/start_gate.rs:158` via `txn.add_tag(EventTag::ActionStarted)` — the action def is available at this point (`def` variable has `def.name`).
4. ActionCommitted events are emitted by individual action handlers in `worldwake-systems` via event transaction builders. The action def name is available in the handler context.
5. `EventPayload` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Adding `Option<String>` is compatible with all derives.
6. `SAVE_FORMAT_VERSION` at `save_load.rs:6` — must bump because `EventPayload` serialization changes.
7. The CLI's `handlers/events.rs` already resolves action names for ActionStarted events by querying the scheduler. This ticket enables the same for ActionCommitted by reading the field directly from the event.

## Architecture Check

1. Adding `action_name: Option<String>` to `EventPayload` is a clean, non-breaking extension. Events without action names (System, Discovery, etc.) have `None`. Only action lifecycle events populate it.
2. The field is populated at the emission site (where the ActionDef is in scope), not reverse-engineered at display time. This is the right architectural boundary — event data is set at emission, not reconstructed later.
3. No backwards-compatibility shims. Per Principle 28, the save format version bumps and old saves won't load.

## Verification Layers

1. ActionStarted events carry action_name -> focused test: emit ActionStarted, verify event has action_name = Some("tell")
2. ActionCommitted events carry action_name -> focused test: commit action, verify event has action_name
3. Non-action events have None -> existing system events should have action_name = None
4. CLI displays action name for both ActionStarted and ActionCommitted -> smoke test
5. Save format version bumped -> old saves won't silently load with wrong EventPayload shape

## What to Change

### 1. Add action_name to EventPayload

In `crates/worldwake-core/src/event_record.rs`:

```rust
pub struct EventPayload {
    pub tick: Tick,
    pub cause: CauseRef,
    pub actor_id: Option<EntityId>,
    pub target_ids: Vec<EntityId>,
    pub evidence: Vec<EvidenceRef>,
    pub place_id: Option<EntityId>,
    pub state_deltas: Vec<StateDelta>,
    pub observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>,
    pub visibility: VisibilitySpec,
    pub witness_data: WitnessData,
    pub tags: BTreeSet<EventTag>,
    pub action_name: Option<String>,  // NEW
}
```

Add `action_name()` accessor to `EventView` trait and implementations.

### 2. Populate action_name at ActionStarted emission

In `crates/worldwake-sim/src/start_gate.rs`, where `EventTag::ActionStarted` is added — also set `action_name: Some(def.name.clone())` on the pending event.

### 3. Populate action_name at ActionCommitted emission

In action handler commit functions across `crates/worldwake-systems/src/` — the action def name is available via the handler context. Set `action_name: Some(action_name.clone())` when building the commit event.

### 4. Update CLI event display

In `crates/worldwake-cli/src/handlers/events.rs`:
- `handle_event()`: For ActionCommitted events, display `action: {name}` line if `record.action_name()` is `Some`.
- `format_event_summary()`: For ActionCommitted events, append `({name})` to the tag string (like already done for ActionStarted).
- Remove the scheduler-based action name lookup for ActionStarted (it's now on the event directly).

### 5. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`, increment version.

### 6. Update all EventPayload construction sites

Every place that constructs `EventPayload` directly (tests, canonical world builder, verification helpers) needs `action_name: None` added. Grep for `EventPayload {` to find all sites.

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify) — add field, update EventView
- `crates/worldwake-sim/src/start_gate.rs` (modify) — populate at ActionStarted
- `crates/worldwake-systems/src/*.rs` (modify) — populate at ActionCommitted in handler commit fns
- `crates/worldwake-cli/src/handlers/events.rs` (modify) — display from event, remove scheduler lookup
- `crates/worldwake-sim/src/save_load.rs` (modify) — bump version
- Various test files (modify) — add `action_name: None` to EventPayload constructors

## Out of Scope

- Adding action_name to ActionAborted/ActionInterrupted events (can be done as a follow-up)
- Changing EventTag enum
- Changing the action trace infrastructure (ActionTraceSink is separate and orthogonal)

## Acceptance Criteria

### Tests That Must Pass

1. ActionStarted event has `action_name = Some("tell")` (or appropriate name)
2. ActionCommitted event has `action_name = Some("pick_up")` (or appropriate name)
3. System/Discovery events have `action_name = None`
4. CLI `event <id>` for ActionCommitted shows `action: pick_up` line
5. CLI event summary shows `ActionCommitted(pick_up)` format
6. Existing suite: `cargo test --workspace`

### Invariants

1. `action_name` is `None` for all non-action events
2. `action_name` is `Some` for all ActionStarted and ActionCommitted events
3. Save format version is bumped

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — verify EventPayload with action_name serializes/deserializes
2. `crates/worldwake-sim/src/start_gate.rs` — verify ActionStarted event carries name
3. `crates/worldwake-cli/src/handlers/events.rs` — update existing event tests with action_name field

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
