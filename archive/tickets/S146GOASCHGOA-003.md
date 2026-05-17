# S146GOASCHGOA-003: `AgentSchemaContextProfile` universal component + belief-view accessor + scenario integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new universal ECS component registered on `EntityKind::Agent`; `GoalBeliefView` accessor; scenario loader integration; `create_agent` bootstrap seeding
**Deps**: archive/tickets/S146GOASCHGOA-002.md

## Problem

S146 introduces per-agent extractor opt-out and per-goal budget-override settings via a new universal component `AgentSchemaContextProfile`. The component, its `GoalBeliefView` accessor (so the AI crate can read it without violating FND-7 locality / FND-14 belief-only planning), its scenario-loader integration, and its bootstrap seeding in `create_agent` all land together because the universal-component pattern requires the four pieces to be compile-coherent (a registered component without a `create_agent` seed produces missing-component panics in `expect()` reads; an accessor without an impl is unreachable).

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `create_agent` at `crates/worldwake-core/src/world.rs` seeds universal profiles via `insert_component_*_default()` calls. `AgentSchemaContextProfile` belongs in that same bootstrap block, and `create_agent_components_queryable` is the focused current test surface for the seed.
2. Per `archive/specs/S146-goal-schema-and-per-goal-budgets.md` D5+D11+D12 and `docs/spec-drafting-rules.md` Section 5: universal component pattern requires (a) `component_schema.rs` registration, (b) `AgentDef` field in `crates/worldwake-cli/src/scenario/types.rs`, (c) `spawn_agent` set call in `crates/worldwake-cli/src/scenario/mod.rs`, (d) `Default` impl, and (e) belief-view-routed runtime access. No `*Def` wrapper is needed because `AgentSchemaContextProfile`'s fields (`disabled_extractors: BTreeSet<CandidateExtractorId>`, `budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>`) contain no `EntityId` references.
3. Shared abstraction boundary under audit: `GoalBeliefView` and its supertrait `ProfileBeliefView` in `crates/worldwake-sim/src/belief_view.rs`. Existing profile methods return `Option<T>` by value, so this ticket adds `agent_schema_context_profile(agent) -> Option<AgentSchemaContextProfile>` and implements it on `PerAgentBeliefView` by cloning the seeded component for the viewed agent. Ticket 005 can unwrap or default according to its planner contract.
4. `CandidateExtractorId` is now a minimal core newtype in `crates/worldwake-core/src/agent_schema_context_profile.rs`: `pub struct CandidateExtractorId(pub u16);` with `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` derives. Ticket 004 owns extractor catalog finalization; this ticket only lands the stable identity type required by the component shape.
5. `GoalDispatchKey` relocation from `worldwake-ai` to `worldwake-core` is safe but required by the component map key. The former inherent `GoalDispatchKey::declaration()` method could not remain in `worldwake-ai` after relocation, so it became the AI-local extension trait `GoalDispatchKeySchemaExt`.

## Architecture Check

1. Universal-component-by-default pattern per FND-22 (agent diversity through concrete variation) and FND-26 (state-mediated cross-system interaction): scenarios author per-agent overrides, runtime reads through belief view, no system-to-system calls.
2. No `*Def` wrapper needed because both fields use integer newtypes / primitive-keyed maps (no `EntityId` references) — per `docs/spec-drafting-rules.md` Section 5 the simpler path is correct here.
3. Belief-view-routed read preserves FND-14: ticket 005 reads the profile through `GoalBeliefView::agent_schema_context_profile(actor)`, not through direct world-state access; this is consistent with how existing universal profiles (cognitive, perception, tell) flow into AI logic.

## Verified Layers

1. Component registered + seeded on every agent → `cargo test -p worldwake-core create_agent` (existing tests `create_agent_produces_correct_entity:1274`, `create_agent_components_queryable:1296` extended with `AgentSchemaContextProfile` assertion)
2. `Default` impl yields empty profile (no overrides) → focused unit test in `agent_schema_context_profile.rs`
3. Serde round-trip preserves field values → focused unit test (RON-style scenario load typically uses bincode for save/load; component must survive both)
4. `GoalBeliefView::agent_schema_context_profile` accessor returns the expected component instance through `PerAgentBeliefView` → focused unit test in `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` block
5. Scenario loader applies the profile from `AgentDef` when present, defaults to empty when absent → focused test in `crates/worldwake-cli/src/scenario/mod.rs`

## Landed Changes

### 1. Landed core component: `crates/worldwake-core/src/agent_schema_context_profile.rs`

```rust
use crate::{GoalDispatchKey, GoalPlanningBudget};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CandidateExtractorId(pub u16);

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentSchemaContextProfile {
    pub disabled_extractors: BTreeSet<CandidateExtractorId>,
    pub budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>,
}
```

Before this ticket, `GoalDispatchKey` lived in `crates/worldwake-ai/src/goal_dispatch_key.rs`. To avoid a core→ai dependency, this ticket relocated `GoalDispatchKey` to `worldwake-core` (small move; the enum is discriminant-only, no behavioral coupling). The ai-crate import sites now use `worldwake_core::GoalDispatchKey`; AI-local schema declarations remain available through `GoalDispatchKeySchemaExt`.

### 2. Register the component in `crates/worldwake-core/src/component_schema.rs`

Add a `with_component_schema_entries!` block for `AgentSchemaContextProfile` with the `|kind| kind == EntityKind::Agent` filter, mirroring existing universal profiles (e.g., `CognitiveProfile`'s registration as reference). The macro generates `insert_component_agent_schema_context_profile`, `get_component_agent_schema_context_profile`, `set_component_agent_schema_context_profile`, etc.

Per `tickets/README.md` check #13: ensure `AgentSchemaContextProfile` is imported into `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/world.rs`, and `crates/worldwake-core/src/component_tables.rs` so the macro's generated code resolves the bare type name.

### 3. Seed default in `create_agent`

In `crates/worldwake-core/src/world.rs:183` `create_agent` body, add:

```rust
world.insert_component_agent_schema_context_profile(
    entity,
    AgentSchemaContextProfile::default(),
)?;
```

Place alongside other universal `insert_component_*::default()` calls (the seeding block runs lines ~190–235).

### 4. `GoalBeliefView` accessor in `crates/worldwake-sim/src/belief_view.rs`

Add to `ProfileBeliefView`, following existing copy-returning profile accessors:

```rust
fn agent_schema_context_profile(&self, agent: EntityId) -> Option<AgentSchemaContextProfile>;
```

Provide the impl on `PerAgentBeliefView` that reads the viewed agent's component and returns a cloned value. Forward through the existing blanket `GoalBeliefView for T where T: ProfileBeliefView` implementation.

### 5. `AgentDef` field in `crates/worldwake-cli/src/scenario/types.rs`

Add to `AgentDef`:

```rust
#[serde(default)]
pub agent_schema_context_profile: Option<AgentSchemaContextProfile>,
```

No `*Def` wrapper needed — primitive-keyed fields only.

### 6. `spawn_agent` integration in `crates/worldwake-cli/src/scenario/mod.rs`

In `spawn_agent` near the existing universal-profile application block (around line 590+, alongside `metabolism_profile.unwrap_or_default()` etc.):

```rust
let schema_profile = agent_def.agent_schema_context_profile.clone().unwrap_or_default();
txn.set_component_agent_schema_context_profile(agent_id, schema_profile)?;
```

### 7. Test updates

Extend `create_agent_components_queryable` (`crates/worldwake-core/src/world.rs`) to assert `get_component_agent_schema_context_profile(agent)` returns `Some(&AgentSchemaContextProfile::default())`. Add a focused unit test for the belief-view accessor.

## Landed Files

- `crates/worldwake-core/src/agent_schema_context_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module decl + re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — `with_component_schema_entries!` block)
- `crates/worldwake-core/src/delta.rs` (modify — import for macro expansion)
- `crates/worldwake-core/src/world.rs` (modify — `create_agent` seed + import)
- `crates/worldwake-core/src/world_txn.rs` (modify — expected create-agent delta coverage)
- `crates/worldwake-core/src/component_tables.rs` (modify — import)
- `crates/worldwake-core/src/goal_dispatch_key.rs` (moved from `crates/worldwake-ai/src/goal_dispatch_key.rs`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `GoalDispatchKey` from core and `GoalDispatchKeySchemaExt`)
- `crates/worldwake-ai/src/goal_schema.rs` (modify — extension trait for `declaration()`)
- `crates/worldwake-ai/src/{agent_tick/planning.rs,decision_trace.rs,exhaustion.rs,feasibility.rs,goal_model.rs,goal_policy.rs,interrupts.rs}` (modify — import extension trait where needed)
- `crates/worldwake-sim/src/belief_view.rs` (modify — accessor trait method + impl)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — concrete accessor impl + focused test)
- `crates/worldwake-sim/src/save_load.rs` (modify — save-format bump and current-format roundtrip coverage)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `AgentDef.agent_schema_context_profile`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — `spawn_agent` seeding)
- CLI scenario helper/test modules containing exhaustive `AgentDef` literals (modify — add `agent_schema_context_profile: None`)
- `crates/worldwake-cli/src/bin/scenario_coverage.rs` (modify — count authored schema profile field)

## Out of Scope

- `CandidateExtractorId` enum widening / variant set finalization — owned by ticket 004 (this ticket lands the foundational newtype shape only).
- Populating `AgentSchemaContextProfile.disabled_extractors` with non-empty values in any scenario — defaults to empty; scenarios opt agents out as a follow-on authoring choice.
- `enabled_methods: BTreeSet<MethodSchemaId>` field — NOT included; ticket 147 (S147 HTN methods) adds this when HTN decomposition lands.
- Wiring the profile into candidate emission logic — owned by ticket 005's planning.rs migration.
- Budget application from `budget_overrides` in search — owned by ticket 006.

## Acceptance Result

### Tests Passed

1. Every agent in every scenario carries an `AgentSchemaContextProfile` after `create_agent` — extended `create_agent_components_queryable:1296`
2. Scenario loader sets the profile from `AgentDef.agent_schema_context_profile` when present, defaults to empty when absent — focused `crates/worldwake-cli/src/scenario/mod.rs` test
3. `GoalBeliefView::agent_schema_context_profile` returns the seeded profile — focused `crates/worldwake-sim/src/per_agent_belief_view.rs` test
4. Existing universal-component test surface remains green — `cargo test -p worldwake-core create_agent`
5. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every entity of `EntityKind::Agent` has exactly one `AgentSchemaContextProfile` component (universal-component pattern, FND-22).
2. `Default::default()` yields empty `disabled_extractors` and `budget_overrides` — no overrides applied unless scenario opts in.
3. Belief-view accessor is the only path AI-crate code reads the profile (FND-14 / FND-7 — no direct world-state access from AI candidate generation or search).
4. Component is registered through `with_component_schema_entries!` and resolves at all three macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`).

## Test Plan Result

### Modified Tests

1. Extended `crates/worldwake-core/src/world.rs::tests::create_agent_components_queryable` — asserts the component is present on the created agent
2. `crates/worldwake-core/src/agent_schema_context_profile.rs` `#[cfg(test)]` — `default_is_empty()`, `serde_roundtrip_preserves_overrides()`
3. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — `agent_schema_context_profile_returns_seeded_profile()`
4. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — `test_spawn_agent_applies_authored_schema_context_profile()` and extended `test_spawn_agents_receive_default_universal_profiles()`

### Command Result

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh` — waived for this ticket iteration; package tests, workspace build, all-target clippy, and AI tests covered the landed seam.

## Outcome

Ticket 003 landed the universal component and its current-format persistence boundary:

1. Added `AgentSchemaContextProfile` plus `CandidateExtractorId` to core, registered it as an agent-only component, seeded it in `create_agent`, and extended delta/txn/component-table coverage.
2. Relocated `GoalDispatchKey` to core so `AgentSchemaContextProfile.budget_overrides` can use it without a core→AI dependency; AI keeps schema declarations through `GoalDispatchKeySchemaExt`.
3. Added belief-view access through `ProfileBeliefView`/`GoalBeliefView` and `PerAgentBeliefView`.
4. Added `AgentDef.agent_schema_context_profile` scenario authoring, `spawn_agent` application, scenario-coverage field reporting, and default/authored scenario tests.
5. Bumped `SAVE_FORMAT_VERSION` from 86 to 87 and added non-default `AgentSchemaContextProfile` save/load roundtrip proof. No legacy save compatibility shim was added, consistent with this repo's no-backward-compatibility rule.

## Verification Result

Focused checks passed during implementation:

1. Passed: `cargo test -p worldwake-core agent_schema_context_profile`
2. Passed: `cargo test -p worldwake-core create_agent`
3. Passed: `cargo test -p worldwake-sim agent_schema_context_profile_returns_seeded_profile`
4. Passed: `cargo test -p worldwake-cli --no-run`
5. Passed: `cargo test -p worldwake-cli test_spawn_agent_applies_authored_schema_context_profile`
6. Passed: `cargo test -p worldwake-cli test_spawn_agents_receive_default_universal_profiles`
7. Passed: `cargo test -p worldwake-sim save_format_version_is_87_after_agent_schema_context_profile_registration`
8. Passed: `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_full_nondefault_state`

Broader checks passed before closeout:

1. Passed: `cargo test -p worldwake-core`
2. Passed: `cargo test -p worldwake-sim`
3. Passed: `cargo test -p worldwake-cli`
4. Passed: `cargo build --workspace`
5. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
6. Passed: `cargo test -p worldwake-ai`
7. Waived: `scripts/verify.sh` for this single-ticket iteration; the package tests, workspace build, all-target clippy, and AI tests covered the landed seam.

`cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` both reported Cargo's existing future-incompatibility warning for `ashpd v0.8.1`; `-D warnings` did not reject the code.
