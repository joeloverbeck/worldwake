# S59EXPOBLSUB-003: Scenario integration for expectation components

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — scenario spawning
**Deps**: S59EXPOBLSUB-002

## Problem

New universal components `ExpectationStore` and `LastSeenMemory` must be spawnable through the scenario system so scenario authors can configure per-agent memory capacity and initial expectations. Core agent creation now guarantees the default baseline, but the scenario layer still needs explicit schema support and override wiring.

## Assumption Reassessment (2026-04-06)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:59` has ~25 optional profile fields. Universal components use `Option<T>` with `unwrap_or_default()` in `spawn_agent()`.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:267` uses `txn.set_component_*` calls. Universal profiles always applied via `unwrap_or_default()`.
3. Neither component contains `EntityId` references that need name resolution, so no `*Def` wrapper type is needed (unlike `MerchandiseProfileDef` or `PatrolRouteDef`).
4. `LastSeenMemory.capacity` is the only scenario-tunable field (u16). `ExpectationStore` starts empty.
5. Ticket says agents spawned from RON scenarios would otherwise lack these components. Live code now differs after `S59EXPOBLSUB-002`: `World::create_agent()` seeds both defaults in core, so scenario spawning already gets the universal baseline. Correction applied: this ticket owns scenario-definability and override parity, not baseline presence. Safe because the architectural direction is unambiguous and narrows the proof surface to the remaining missing slice.

## Architecture Check

1. Follows the established universal-component-with-default pattern. No new spawning patterns needed.
2. No backward compatibility shims. Existing scenarios continue working — both components default to sensible values when omitted from RON.

## Verification Layers

1. Scenario schema accepts both components and `spawn_agent()` preserves defaults plus explicit overrides → focused scenario unit tests
2. Single-layer ticket (scenario wiring only) — additional layer mapping not applicable.

## What to Change

### 1. Add fields to AgentDef

In `crates/worldwake-cli/src/scenario/types.rs`, add to `AgentDef`:

```rust
#[serde(default)]
pub expectation_store: Option<ExpectationStore>,
#[serde(default)]
pub last_seen_memory: Option<LastSeenMemory>,
```

### 2. Apply in spawn_agent

In `crates/worldwake-cli/src/scenario/mod.rs`, in `spawn_agent()`, add:

```rust
txn.set_component_expectation_store(
    agent_id,
    agent_def.expectation_store.clone().unwrap_or_default(),
);
txn.set_component_last_seen_memory(
    agent_id,
    agent_def.last_seen_memory.clone().unwrap_or_default(),
);
```

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add fields)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add set_component calls)

## Out of Scope

- RON scenario files — existing scenarios work without changes (defaults apply)
- LastSeenMemory capacity variation per scenario — works automatically via Optional field

## Acceptance Criteria

### Tests That Must Pass

1. Existing scenario integration test spawns agents with both components set to defaults
2. A scenario with explicit `last_seen_memory: Some(LastSeenMemory { capacity: 50, .. })` overrides the default
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. All agents spawned from scenarios always have both components (universal contract)
2. Omitting fields from RON produces default values (empty store, capacity 20)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — verify new optional fields default to `None`
2. `crates/worldwake-cli/src/scenario/mod.rs` — verify spawned agents retain the universal defaults and accept explicit `LastSeenMemory` override

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- Completed: 2026-04-06
- Added `expectation_store` and `last_seen_memory` optional fields to `AgentDef`, making both components scenario-definable through RON.
- Updated `spawn_agent()` to apply the scenario-provided values when present and preserve the universal defaults when omitted.
- Extended focused scenario tests to prove serde defaults, universal default retention, and explicit `LastSeenMemory.capacity` override behavior.
- Deviation from original plan: the ticket's original problem statement claimed scenario-spawned agents would otherwise lack these components. Live code after `S59EXPOBLSUB-002` already seeded the default baseline in `World::create_agent()`, so this ticket landed only the remaining scenario-definability and override slice.

## Verification Result

- Passed `cargo test -p worldwake-cli test_agent_def_default_optional_fields`
- Passed `cargo test -p worldwake-cli test_spawn_agents_receive_default_universal_profiles`
- Passed `cargo test -p worldwake-cli test_spawn_agent_with_last_seen_memory_override`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
