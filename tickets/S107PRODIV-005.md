# S107PRODIV-005: CLI wiring — AgentDef + spawn_agent for DiversificationProfile and LastProactiveExplorationTick

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — CLI/scenario layer only
**Deps**: S107PRODIV-001

## Problem

Scenario authors need to configure `DiversificationProfile` on agents to enable proactive exploration. This requires adding the profile to `AgentDef` and wiring `spawn_agent()` to set both `DiversificationProfile` (scenario-configured, role-specific) and `LastProactiveExplorationTick` (runtime-generated, always None at spawn for agents with the profile).

## Assumption Reassessment (2026-04-17)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:70`. Currently has ~31 optional profile fields. Role-specific pattern: `exploration_profile: Option<ExplorationProfileDef>` at line 115. DiversificationProfile has no EntityId fields, so no `*Def` wrapper is needed — use `Option<DiversificationProfile>` directly.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:328`. Role-specific conditional pattern: `if let Some(ref profile) = agent_def.theft_disposition { txn.set_component_...(agent_id, profile.clone())?; }` (lines 431-432).
3. `LastProactiveExplorationTick` should be set to `LastProactiveExplorationTick(None)` at spawn only for agents that have `DiversificationProfile` — it's meaningless without the profile.

## Architecture Check

1. Follows established role-specific component wiring pattern. No `*Def` wrapper overhead since all fields are primitive types (Permille, u32, u16).
2. No backward-compatibility shims. Existing scenarios without `diversification_profile` continue to work — the field is `Option` and agents without it behave identically to today.

## Verification Layers

1. Agent with DiversificationProfile in scenario → component present after spawn → focused test
2. Agent without DiversificationProfile → component absent → focused test
3. LastProactiveExplorationTick set to None at spawn for agents with profile → focused test
4. Single-layer ticket: CLI wiring only

## What to Change

### 1. Add to AgentDef

In `crates/worldwake-cli/src/scenario/types.rs`, add field:
```rust
pub diversification_profile: Option<DiversificationProfile>,
```

### 2. Wire in spawn_agent

In `crates/worldwake-cli/src/scenario/mod.rs`, add role-specific conditional:
```rust
if let Some(ref dp) = agent_def.diversification_profile {
    txn.set_component_diversification_profile(agent_id, *dp)?;
    txn.set_component_last_proactive_exploration_tick(
        agent_id,
        LastProactiveExplorationTick(None),
    )?;
}
```

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify) — add field to AgentDef
- `crates/worldwake-cli/src/scenario/mod.rs` (modify) — add spawn_agent wiring

## Out of Scope

- DiversificationProfile Default impl (already in ticket 001)
- Scenario file creation (ticket 007)
- Proactive exploration logic (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. RON scenario with `diversification_profile: Some(DiversificationProfile { ... })` deserializes correctly
2. spawn_agent sets DiversificationProfile component when present in AgentDef
3. spawn_agent sets LastProactiveExplorationTick(None) when DiversificationProfile present
4. spawn_agent does NOT set either component when diversification_profile is None
5. Existing suite: `cargo test -p worldwake-cli`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Agents without DiversificationProfile have no LastProactiveExplorationTick component
2. DiversificationProfile is role-specific — conditional `if let Some` pattern, not `unwrap_or_default`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` — spawn test with and without diversification_profile in AgentDef

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`
