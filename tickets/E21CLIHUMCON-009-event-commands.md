# E21CLIHUMCON-009: Event Commands (events, event, trace)

## Summary

Implement the event log viewer: `events [n]` shows recent events, `event <id>` shows full details, `trace <id>` walks the causal chain backward via `CauseRef`.

## Depends On

- E21CLIHUMCON-003 (REPL loop)
- E21CLIHUMCON-004 (command enum)
- E21CLIHUMCON-005 (display helpers)

## Files to Touch

- `crates/worldwake-cli/src/handlers/events.rs` — **create**: `handle_events()`, `handle_event()`, `handle_trace()`
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `Events`, `Event`, `Trace` variants

## Out of Scope

- Other command handlers (006–008, 010–012)
- Modifying `EventLog`, `EventRecord`, or `CauseRef` types
- Forward causal tracing (spec explicitly says backward only, line 116)
- Changes to any crate other than `worldwake-cli`

## Deliverables

### `handle_events(sim: &SimulationState, n: usize)`
- Default `n` to 10 if not specified
- Get last `n` events from `EventLog`
- Display each as a summary line:
  ```
  [E42] tick 15 — Harvest completed by Kael (Grain ×3)
  [E41] tick 14 — Travel started: Kael → Market Square
  ```
- Show: event ID, tick, event tag/description, involved entities (using `entity_display_name()`)

### `handle_event(sim: &SimulationState, id: u64)`
- Look up `EventRecord` by `EventId(id)` in the event log
- Print full details:
  - Event ID, tick, tag
  - Cause (`CauseRef` — print referenced event ID or "none")
  - Witnesses (list of entity names)
  - Component deltas (list each `ComponentDelta` in human-readable form)
  - Relation deltas (list each `RelationDelta`)
- If event ID not found → error message

### `handle_trace(sim: &SimulationState, id: u64)`
- Start at `EventId(id)`
- Walk backward via `EventRecord.cause` (`CauseRef`)
- For each event in the chain, print a summary line (like `handle_events` format)
- Stop when reaching an event with no cause (root event)
- Print the chain with indentation showing depth:
  ```
  [E42] tick 15 — Harvest completed by Kael
    ← [E38] tick 12 — Harvest started by Kael
      ← [E35] tick 10 — Action requested: Harvest
  ```
- If event ID not found → error message
- Guard against unreasonably long chains (cap at 100 hops with warning)

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_events_shows_recent`: after ticking, events shows non-empty list
  - `test_events_default_count`: events with no arg defaults to 10 (or fewer if log is shorter)
  - `test_events_custom_count`: events 3 → shows at most 3 events
  - `test_event_details`: event with valid ID → shows tag, tick, cause
  - `test_event_not_found`: event with invalid ID → error message
  - `test_trace_walks_backward`: trace from a caused event → shows chain of causes
  - `test_trace_root_event`: trace from root event → shows single event (no cause)
  - `test_trace_not_found`: trace with invalid ID → error message

### Invariants That Must Remain True
- All handlers are read-only — zero world mutation
- Trace walks backward only (via `CauseRef`), never forward
- Event IDs displayed consistently as `[E{id}]`
- `cargo clippy -p worldwake-cli` passes with no warnings
