# E21: CLI & Human Control

## Epic Summary
Implement the text-based CLI for human interaction: affordance-based action menus, agent switching, event log viewer, and state inspector.

## Phase
Phase 4: Group Adaptation, CLI & Verification

## Crate
`worldwake-cli`

## Dependencies
- E13 (affordance query for action menus)

## Deliverables

### Text Command Interface
- Interactive CLI loop:
  - Display current state summary
  - Accept commands via stdin
  - Process commands and display results
- Core commands:
  - `tick [n]`: advance simulation by n ticks (default 1)
  - `wait [duration]`: advance until duration elapsed or event occurs
  - `status`: show current agent's status
  - `look`: describe current location and visible entities
  - `actions`: list available actions
  - `do <action> [targets]`: execute an action
  - `cancel`: cancel current action
  - `inventory`: show carried items
  - `needs`: show need levels
  - `beliefs`: show known/believed facts
  - `help`: list commands

### Affordance-Based Action Menu
Per spec section 6.4:
- `actions` command queries affordances for controlled agent
- Shows only legal actions from agent's perceived context
- Each action shows: name, targets, duration, precondition status
- Player selects from menu → creates InputEvent → processed by scheduler
- Same affordance query as AI agents (no special player actions)

### Agent Switching
- `switch <entity_id>` or `switch <agent_name>`: change controlled agent
  - Detach ControlSource::Human from current agent
  - Set current agent to ControlSource::Ai (or None)
  - Attach ControlSource::Human to target agent
  - World simulation continues without reset
  - New agent's affordances immediately available
- `observe`: switch to observer mode (no controlled agent)
- Target must be alive and present in world

### Event Log Viewer
- `events [n]`: show last n events (default 10)
- `event <id>`: show event details including state deltas
- `trace <id>`: show causal chain from event to root cause
- `events at <place>`: show recent events at a location
- `events by <agent>`: show recent events involving an agent

### State Inspector
- `inspect <entity>`: show all components on an entity
- `needs <agent>`: show detailed need levels with urgency
- `inventory <agent>`: show all carried items with quantities
- `location <entity>`: show where entity is
- `beliefs <agent>`: show agent's known and believed facts
- `relations <entity>`: show all relations involving entity

### World Overview
- `world`: show summary of all places with population and notable state
- `places`: list all places with travel connections
- `agents`: list all living agents with location and control source
- `goods`: show global goods summary (total quantities by type)
- `order`: show public order per place

## Invariants Enforced
- 9.1: Simulation authority - CLI only reads state and requests actions, never mutates directly
- 9.12: Player symmetry - no special player actions, same affordance query

## Tests
- [ ] T24: Player replacement - switch control to any living agent, world continues
- [ ] T27: Controlled agent death - world continues, control can transfer
- [ ] T12: No player branching - switching to merchant/guard/bandit shows appropriate affordances
- [ ] Action menu shows only legal actions
- [ ] CLI commands don't mutate world state directly
- [ ] Event viewer shows causal chains correctly
- [ ] Agent switching preserves world state
- [ ] Observer mode: simulation runs without controlled agent

## Acceptance Criteria
- Text interface for full simulation interaction
- Actions only through affordance query (no cheats)
- Agent switching works for any living agent
- Event log navigable with causal tracing
- State inspection for debugging and play
- UI is read-only + action requests (simulation authority)

## Spec References
- Section 3.2 (ControlSource: Human | AI | None, any agent switchable)
- Section 6.4 (human control uses same action query pipeline)
- Section 9.1 (simulation authority: UI only reads or requests)
- Section 9.12 (player symmetry)
- Section 9.21 (controlled-agent mortality)
