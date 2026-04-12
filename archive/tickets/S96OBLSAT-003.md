# S96OBLSAT-003: Scenario contract for ObligationSatiationProfile

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new `AgentDef` field, universal bootstrap wiring in `spawn_agent()`, focused scenario tests, and same-crate `AgentDef` literal fallout
**Deps**: archive/tickets/S96OBLSAT-001.md

## Problem

Per `docs/spec-drafting-rules.md` section 5, every agent component must be exercisable through the scenario system. Without `AgentDef` integration, scenario authors cannot configure per-agent satiation parameters.

## Assumption Reassessment (2026-04-12)

1. `AgentDef` is defined at `crates/worldwake-cli/src/scenario/types.rs:67-131`. It has many optional profile fields, no existing `ObligationSatiationProfile` field, and the type derives only `Deserialize`, so adding a new field also widens manual `AgentDef` literal fallout across CLI tests/helpers.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:323-471`. Universal components are applied with `unwrap_or_default()` (for example `needs`, `drive_thresholds`, `metabolism_profile`, `perception_profile`, `tell_profile`, `cognitive_profile`, `execution_budget`, `expectation_store`, and `last_seen_memory`), while role-specific components use `if let Some(...)`.
3. `ObligationSatiationProfile` is classified as universal (every agent gets `Default` if not in scenario). Because `spawn_agent()` receives `&AgentDef`, the live write path is `agent_def.obligation_satiation_profile.clone().unwrap_or_default()`.
4. The owning focused proof already exists in `crates/worldwake-cli/src/scenario/mod.rs` and `crates/worldwake-cli/src/scenario/types.rs`: scenario tests there already assert default and override behavior for universal/profile fields, so this ticket should add focused spawn/deserialization coverage instead of relying only on later golden coverage.
5. Shared boundary under audit: scenario auth (`AgentDef` deserialization) -> `spawn_agent()` universal bootstrap wiring -> authoritative `ObligationSatiationProfile` component on spawned agents.

## Architecture Check

1. Universal application with `unwrap_or_default()` matches existing scenario/bootstrap patterns and keeps the scenario contract aligned with the authoritative universal bootstrap path.
2. Because `AgentDef` is manually instantiated in many CLI tests/helpers, the clean implementation is still additive but must absorb same-crate constructor fallout instead of pretending the change is only two files.
3. No backwards-compatibility shims. New optional RON field; existing scenarios parse without it.

## Verification Layers

1. Scenario parsing accepts the new optional field and leaves it absent when omitted → focused deserialization test
2. Scenario parsing accepts an explicit `obligation_satiation_profile` override from RON → focused deserialization test
3. `spawn_agent` seeds default `ObligationSatiationProfile` for agents without overrides → focused scenario-spawn test
4. `spawn_agent` preserves an explicit scenario override → focused scenario-spawn test
5. Same-crate constructor fallout stays aligned with the new `AgentDef` shape → `cargo test -p worldwake-cli` / compile fallout

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

### 3. Add focused proof for default + override behavior

- In `crates/worldwake-cli/src/scenario/types.rs`, extend the deserialization tests to prove the new optional field defaults to `None` when omitted.
- In `crates/worldwake-cli/src/scenario/mod.rs`, add focused spawn tests proving default seeding and explicit override wiring for `ObligationSatiationProfile`.

### 4. Fix `AgentDef` literal fallout in CLI tests/helpers

Update manual `AgentDef` literals that do not inherit from `minimal_agent(...)` or another helper so the crate still compiles with the new field.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-cli/src/display.rs` (modify)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify)
- `crates/worldwake-cli/src/handlers/control.rs` (modify)
- `crates/worldwake-cli/src/handlers/events.rs` (modify)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify)

## Out of Scope

- RON scenario files — existing scenarios work unchanged (field is optional, defaults apply)
- ObligationExecutionTracker — runtime-generated, exempt from scenario contract

## Acceptance Criteria

### Tests That Must Pass

1. Scenario deserialization leaves `obligation_satiation_profile` absent when omitted
2. Scenario deserialization accepts an explicit `obligation_satiation_profile` override
3. Spawned agents receive `ObligationSatiationProfile::default()` when no override is provided
4. Spawned agents preserve an explicit `obligation_satiation_profile` override
5. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. `ObligationSatiationProfile` is applied to every agent via `unwrap_or_default()`
2. Existing RON scenarios remain valid (no required field added)
3. Runtime-generated `ObligationExecutionTracker` remains out of the scenario contract

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — omitted-field deserialization proof and explicit-RON override parsing proof
2. `crates/worldwake-cli/src/scenario/mod.rs` — focused default and override spawn proofs

### Commands

1. `cargo test -p worldwake-cli --lib scenario::types::tests::test_agent_def_default_optional_fields -- --exact`
2. `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserialize_full -- --exact`
3. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agents_receive_default_universal_profiles -- --exact`
4. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agents_apply_obligation_satiation_profile_override_when_present -- --exact`
5. `cargo test -p worldwake-cli`
6. `cargo build --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Added `obligation_satiation_profile: Option<ObligationSatiationProfile>` to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` with omitted-field serde default behavior.
- Wired `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs` to seed `ObligationSatiationProfile` universally via `clone().unwrap_or_default()`, matching the authoritative bootstrap contract for universal agent profiles.
- Added focused proof that omitted scenario input stays `None` at deserialization time, that explicit RON scenario overrides parse correctly, that spawned agents receive `ObligationSatiationProfile::default()`, and that explicit scenario overrides are preserved.
- Absorbed same-crate `AgentDef` literal fallout in CLI display/handler test helpers so the crate compiles with the new field.

## Deviations

- The live scope was broader than the original draft’s two-file plan because `AgentDef` is manually instantiated in multiple CLI test/helper modules.
- The original focused command sketches were too loose: `cargo test -p worldwake-cli <name>` compiled but ran zero tests for this crate layout, so the ticket now records the exact `--lib` module-qualified selectors that actually prove the owned surface.

## Verification Result

- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_agent_def_default_optional_fields -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserialize_full -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agents_receive_default_universal_profiles -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agents_apply_obligation_satiation_profile_override_when_present -- --exact`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ticket file status: archived tracked file (`archive/tickets/S96OBLSAT-003.md`); original active path removed
