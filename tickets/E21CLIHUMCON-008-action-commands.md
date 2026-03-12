# E21CLIHUMCON-008: Action Commands (actions, do, cancel)

## Summary

Implement the affordance-based action menu: `actions` lists available actions via `get_affordances()`, `do <n>` selects and enqueues an action, `cancel` cancels the current action.

## Depends On

- E21CLIHUMCON-003 (REPL loop — provides `ReplState` with `last_affordances`)
- E21CLIHUMCON-004 (command enum)
- E21CLIHUMCON-005 (display helpers)

## Files to Touch

- `crates/worldwake-cli/src/handlers/actions.rs` — **create**: `handle_actions()`, `handle_do()`, `handle_cancel()`
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `Actions`, `Do`, `Cancel` variants

## Out of Scope

- Other command handlers (006, 007, 009–012)
- Modifying `get_affordances()` or the affordance system
- AI action selection — AI uses the same system independently
- Changes to any crate other than `worldwake-cli`
- Adding special player-only actions (violates invariant 9.12)

## Deliverables

### `handle_actions(sim: &SimulationState, repl_state: &mut ReplState)`
- Requires controlled agent (error if observer mode)
- Call `get_affordances()` for the controlled agent
- Store result in `repl_state.last_affordances`
- Display numbered menu:
  ```
  Available actions:
    1. Eat (Apple) — 3 ticks
    2. Travel to Market Square — 5 ticks
    3. Harvest (Grain) at Field — 4 ticks
  ```
- Each line shows: number, action name, targets/parameters, estimated duration
- If no affordances: print "no actions available"

### `handle_do(n: usize, sim: &mut SimulationState, repl_state: &ReplState)`
- Requires controlled agent
- Validate `n` is within `last_affordances` range (1-indexed)
- Get the selected `Affordance`
- Create `InputEvent::RequestAction(...)` from the affordance
- Enqueue in `sim.input_queue`
- Print confirmation: `"Requested: {action_name}"`
- The action won't execute until the next `tick`

### `handle_cancel(sim: &mut SimulationState)`
- Requires controlled agent
- Create `InputEvent::CancelAction(...)` for the controlled agent
- Enqueue in `sim.input_queue`
- Print confirmation: `"Cancel requested"`
- If no active action: print "no action to cancel"

### Error Cases
- `do 0` or `do N` where N > list length → "invalid action number, run 'actions' first"
- `do` before `actions` (empty last_affordances) → "run 'actions' first to see available actions"
- All three commands in observer mode → "no controlled agent"

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_actions_lists_affordances`: actions returns non-empty list for agent with available actions
  - `test_actions_stores_in_repl_state`: after actions, repl_state.last_affordances is populated
  - `test_do_enqueues_input`: do N → InputQueue contains a RequestAction event
  - `test_do_out_of_range`: do with invalid number → error message
  - `test_do_before_actions`: do with empty last_affordances → error message
  - `test_cancel_enqueues_input`: cancel → InputQueue contains a CancelAction event
  - `test_actions_no_controlled_agent`: actions in observer mode → error

### Invariants That Must Remain True
- Invariant 9.1: commands only enqueue `InputEvent`s — never mutate world directly
- Invariant 9.12: same affordance query as AI agents — no special player actions
- Action execution only happens in `step_tick()` (next tick command)
- `cargo clippy -p worldwake-cli` passes with no warnings
