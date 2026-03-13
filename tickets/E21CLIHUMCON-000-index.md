# E21CLIHUMCON — CLI & Human Control Tickets

Epic: E21 (CLI & Human Control)
Crate: `worldwake-cli`
Spec: `specs/E21-cli-human-control.md`

## Ticket Index

| Ticket | Title | Depends On |
|--------|-------|------------|
| E21CLIHUMCON-001 | Scenario types (RON structs) | — |
| E21CLIHUMCON-002 | Scenario spawning (`spawn_scenario()`) | 001 |
| E21CLIHUMCON-003 | CLI args, bootstrap, and REPL loop | 002 |
| E21CLIHUMCON-004 | Command enum (clap subcommands) | — |
| E21CLIHUMCON-005 | Display helpers (entity resolution, formatting) | — |
| E21CLIHUMCON-006 | Tick and status commands | 003, 004, 005 |
| E21CLIHUMCON-007 | Inspection commands (look, inspect, inventory, needs, relations) | 003, 004, 005 |
| E21CLIHUMCON-008 | Action commands (actions, do, cancel) | 003, 004, 005 |
| ~~E21CLIHUMCON-009~~ | ~~Event commands (events, event, trace)~~ ✅ | 003, 004, 005 |
| E21CLIHUMCON-010 | Control commands (switch, observe) | 003, 004, 005 |
| E21CLIHUMCON-011 | World overview commands (world, places, agents, goods) | 003, 004, 005 |
| E21CLIHUMCON-012 | Persistence commands (save, load) | 003, 004 |
| E21CLIHUMCON-013 | Default scenario and integration tests | 002, all handlers |

## Dependency Graph

```
001 (scenario types)
 └→ 002 (spawn_scenario)
      └→ 003 (bootstrap + REPL)
           ├→ 006 (tick/status)
004 ────────┤→ 007 (inspect)
005 ────────┤→ 008 (actions)
           ├→ 009 (events)
           ├→ 010 (control)
           ├→ 011 (world overview)
           └→ 012 (persistence)

All handlers → 013 (default scenario + integration tests)
```

## Parallelization

- **Wave 1**: 001, 004, 005 (independent)
- **Wave 2**: 002 (depends on 001)
- **Wave 3**: 003 (depends on 002)
- **Wave 4**: 006–012 (all depend on 003+004+005, can be parallelized)
- **Wave 5**: 013 (depends on all handlers)
