# S96OBLSAT-003: Scenario contract for ObligationSatiationProfile

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new field on AgentDef, new set_component call in spawn_agent
**Deps**: archive/tickets/S96OBLSAT-001.md

## Problem

Per `docs/spec-drafting-rules.md` section 5, every agent component must be exercisable through the scenario system. Without `AgentDef` integration, scenario authors cannot configure per-agent satiation parameters.

## Assumption Reassessment (2026-04-12)

1. `AgentDef` is defined at `crates/worldwake-cli/src/scenario/types.rs:67-131`. It has ~25 optional profile fields. No existing `ObligationSatiationProfile` field.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:323-471`. Universal components are applied with `unwrap_or_default()` (e.g., `homeostatic_needs` at line 338). Role-specific components use `if let Some(...)` (e.g., `combat_profile` at line 385).
3. `ObligationSatiationProfile` is classified as universal (every agent gets `Default` if not in scenario). Following the universal pattern: `agent_def.obligation_satiation_profile.clone().unwrap_or_default()`.

## Architecture Check

1. Universal application with `unwrap_or_default()` matches existing universal component patterns. Agents without explicit config get harmless defaults (satiation only applies when `notice_posting_weight > 0`, which defaults to 0).
2. No backwards-compatibility shims. New optional RON field; existing scenarios parse without it.

## Verification Layers

1. Scenario parsing accepts new field → focused test or existing scenario load
2. `spawn_agent` sets component on all agents → verified by golden tests in ticket 006
3. Single-layer ticket (scenario wiring only).

## What to Change

### 1. Add field to `AgentDef`

In `crates/worldwake-cli/src/scenario/types.rs`, add to `AgentDef`:

```rust
pub obligation_satiation_profile: Option<ObligationSatiationProfile>,
```

With `#[serde(default)]` if not already applied at struct level.

### 2. Apply in `spawn_agent()`

In `crates/worldwake-cli/src/scenario/mod.rs`, add after the universal component block:

```rust
txn.set_component_obligation_satiation_profile(
    agent_id,
    agent_def.obligation_satiation_profile.clone().unwrap_or_default(),
);
```

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)

## Out of Scope

- RON scenario files — existing scenarios work unchanged (field is optional, defaults apply)
- ObligationExecutionTracker — runtime-generated, exempt from scenario contract

## Acceptance Criteria

### Tests That Must Pass

1. Existing scenarios load without error
2. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. `ObligationSatiationProfile` is applied to every agent via `unwrap_or_default()`
2. Existing RON scenarios remain valid (no required field added)

## Test Plan

### New/Modified Tests

1. None — scenario-loading coverage by existing tests; golden test in ticket 006 exercises the full path.

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
