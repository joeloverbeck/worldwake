# S146GOASCHGOA-003: `AgentSchemaContextProfile` universal component + belief-view accessor + scenario integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new universal ECS component registered on `EntityKind::Agent`; `GoalBeliefView` accessor; scenario loader integration; `create_agent` bootstrap seeding
**Deps**: 002

## Problem

S146 introduces per-agent extractor opt-out and per-goal budget-override settings via a new universal component `AgentSchemaContextProfile`. The component, its `GoalBeliefView` accessor (so the AI crate can read it without violating FND-7 locality / FND-14 belief-only planning), its scenario-loader integration, and its bootstrap seeding in `create_agent` all land together because the universal-component pattern requires the four pieces to be compile-coherent (a registered component without a `create_agent` seed produces missing-component panics in `expect()` reads; an accessor without an impl is unreachable).

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `create_agent` at `crates/worldwake-core/src/world.rs:183` seeds 21 universal profiles via `insert_component_*_default()` calls (verified: `ArtifactPostingProfile`, `AgentData`, `AgentBeliefStore`, `SurveyMemory`, `ExpectationStore`, `LastSeenMemory`, `PerceptionProfile`, `TellProfile`, `CognitiveProfile`, `AgendaProfile`, `AcquisitionExhaustionTracker`, `ExplorationProfile`, `ObligationSatiationProfile`, `DisposalProfile`, `ExecutionBudget`, `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `CommunicationProfile`, `PreferenceProfile`, `RiskWeightProfile`, `LawAbidingProfile`, `DriveEscalationProfile`). `AgentSchemaContextProfile` must be added here. Existing tests `create_agent_produces_correct_entity:1274`, `create_agent_components_queryable:1296`, `create_agent_attaches_belief_store_perception_profile_and_tell_profile:1324`, and `create_agent_seeds_default_expectation_components` at `crates/worldwake-core/src/expectation.rs:345` exercise this surface.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D5+D11+D12 and `docs/spec-drafting-rules.md` Section 5: universal component pattern requires (a) `component_schema.rs` registration, (b) `AgentDef` field in `crates/worldwake-cli/src/scenario/types.rs`, (c) `spawn_agent` set call in `crates/worldwake-cli/src/scenario/mod.rs`, (d) `Default` impl, (e) runtime `expect()` access. No `*Def` wrapper is needed because `AgentSchemaContextProfile`'s fields (`disabled_extractors: BTreeSet<CandidateExtractorId>`, `budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>`) contain no `EntityId` references.
3. Shared abstraction boundary under audit: `GoalBeliefView` (`crates/worldwake-sim/src/belief_view.rs:317`) and its supertrait `ProfileBeliefView`. Ticket 005 will read the profile through `view.agent_schema_context_profile(agent)`; the accessor lives on `ProfileBeliefView` alongside existing analogs like `cognitive_profile`. Per `tickets/README.md` check #13, the macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) must import the new type because `with_component_schema_entries!` references types via `crate::TypeName`.
4. `CandidateExtractorId` enum (referenced by `disabled_extractors: BTreeSet<CandidateExtractorId>`) is defined by ticket 004. To unblock this ticket from a circular dep, define a stub `CandidateExtractorId` newtype in `crates/worldwake-core/src/agent_schema_context_profile.rs` here as `pub struct CandidateExtractorId(pub u16);` with `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` derives, and ticket 004 extends it to an enum or expands its variant set as needed. **Placeholder, replaced by ticket 004**: the newtype shape here is the foundation; ticket 004 may convert to an enum or add variants/constants as it formalizes the 20 extractor identities.
5. Adjacent contradictions: ticket 003's stub `CandidateExtractorId` is a forward-declared identity type whose ergonomic shape is finalized by ticket 004. Classified as **required consequence** — the universal-component scaffold cannot reference a not-yet-defined type, so a minimal forward declaration here is correct.

## Architecture Check

1. Universal-component-by-default pattern per FND-22 (agent diversity through concrete variation) and FND-26 (state-mediated cross-system interaction): scenarios author per-agent overrides, runtime reads through belief view, no system-to-system calls.
2. No `*Def` wrapper needed because both fields use integer newtypes / primitive-keyed maps (no `EntityId` references) — per `docs/spec-drafting-rules.md` Section 5 the simpler path is correct here.
3. Belief-view-routed read preserves FND-14: ticket 005 reads the profile through `GoalBeliefView::agent_schema_context_profile(actor)`, not through direct world-state access; this is consistent with how existing universal profiles (cognitive, perception, tell) flow into AI logic.

## Verification Layers

1. Component registered + seeded on every agent → `cargo test -p worldwake-core create_agent` (existing tests `create_agent_produces_correct_entity:1274`, `create_agent_components_queryable:1296` extended with `AgentSchemaContextProfile` assertion)
2. `Default` impl yields empty profile (no overrides) → focused unit test in `agent_schema_context_profile.rs`
3. Serde round-trip preserves field values → focused unit test (RON-style scenario load typically uses bincode for save/load; component must survive both)
4. `GoalBeliefView::agent_schema_context_profile` accessor returns the expected component instance → focused unit test in `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` block
5. Scenario loader applies the profile from `AgentDef` when present, defaults to empty when absent → focused test in `crates/worldwake-cli/src/scenario/mod.rs`

## What to Change

### 1. New core component: `crates/worldwake-core/src/agent_schema_context_profile.rs`

```rust
use crate::{GoalDispatchKey, GoalPlanningBudget};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Placeholder shape for ticket 004; expand to enum or registered variant set as needed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CandidateExtractorId(pub u16);

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentSchemaContextProfile {
    pub disabled_extractors: BTreeSet<CandidateExtractorId>,
    pub budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>,
}
```

Note: `GoalDispatchKey` is currently in `crates/worldwake-ai/src/goal_dispatch_key.rs`. To avoid a core→ai dependency, this ticket relocates `GoalDispatchKey` to `worldwake-core` (small move; the enum is discriminant-only, no behavioral coupling). Confirm relocation is safe — the ai-crate import sites switch to `use worldwake_core::GoalDispatchKey`. Likely: 41-variant enum + `ALL` constant + `from_goal_kind` match arm (verify during implementation; if relocation is non-trivial, escalate to user via 1-3-1 before proceeding).

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

Add to the appropriate supertrait (likely `ProfileBeliefView`, following `cognitive_profile`'s pattern):

```rust
fn agent_schema_context_profile(&self, agent: EntityId) -> &AgentSchemaContextProfile;
```

Provide the impl on `RuntimeBeliefView` that reads via `self.world.get_component_agent_schema_context_profile(agent).expect("AgentSchemaContextProfile must be seeded on every Agent")`. Forward through `impl_goal_belief_view!` or the current blanket-impl macro.

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
let schema_profile = agent_def.agent_schema_context_profile.unwrap_or_default();
txn.set_component_agent_schema_context_profile(agent_id, schema_profile)?;
```

### 7. Test updates

Extend `create_agent_components_queryable:1296` (`crates/worldwake-core/src/world.rs`) to assert `get_component_agent_schema_context_profile(agent)` returns `Some(&AgentSchemaContextProfile::default())`. Add a focused unit test for the belief-view accessor.

## Files to Touch

- `crates/worldwake-core/src/agent_schema_context_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module decl + re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — `with_component_schema_entries!` block)
- `crates/worldwake-core/src/delta.rs` (modify — import for macro expansion)
- `crates/worldwake-core/src/world.rs` (modify — `create_agent` seed + import)
- `crates/worldwake-core/src/component_tables.rs` (modify — import)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — relocate to core; if relocation is non-trivial, escalate per 1-3-1)
- `crates/worldwake-ai/src/lib.rs` (modify — re-import `GoalDispatchKey` from core if relocated)
- `crates/worldwake-sim/src/belief_view.rs` (modify — accessor trait method + impl)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `AgentDef.agent_schema_context_profile`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — `spawn_agent` seeding)

## Out of Scope

- `CandidateExtractorId` enum widening / variant set finalization — owned by ticket 004 (this ticket lands the foundational newtype shape only).
- Populating `AgentSchemaContextProfile.disabled_extractors` with non-empty values in any scenario — defaults to empty; scenarios opt agents out as a follow-on authoring choice.
- `enabled_methods: BTreeSet<MethodSchemaId>` field — NOT included; ticket 147 (S147 HTN methods) adds this when HTN decomposition lands.
- Wiring the profile into candidate emission logic — owned by ticket 005's planning.rs migration.
- Budget application from `budget_overrides` in search — owned by ticket 006.

## Acceptance Criteria

### Tests That Must Pass

1. Every agent in every scenario carries an `AgentSchemaContextProfile` after `create_agent` — extended `create_agent_components_queryable:1296`
2. Scenario loader sets the profile from `AgentDef.agent_schema_context_profile` when present, defaults to empty when absent — new `crates/worldwake-cli/src/scenario/mod.rs` focused test
3. `GoalBeliefView::agent_schema_context_profile` returns the seeded profile — new `crates/worldwake-sim/src/belief_view.rs` focused test
4. Existing universal-component test surface remains green — `cargo test -p worldwake-core create_agent`
5. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every entity of `EntityKind::Agent` has exactly one `AgentSchemaContextProfile` component (universal-component pattern, FND-22).
2. `Default::default()` yields empty `disabled_extractors` and `budget_overrides` — no overrides applied unless scenario opts in.
3. Belief-view accessor is the only path AI-crate code reads the profile (FND-14 / FND-7 — no direct world-state access from AI candidate generation or search).
4. Component is registered through `with_component_schema_entries!` and resolves at all three macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`).

## Test Plan

### New/Modified Tests

1. Extended `crates/worldwake-core/src/world.rs::tests::create_agent_components_queryable` — assert new component is present on the created agent
2. `crates/worldwake-core/src/agent_schema_context_profile.rs` `#[cfg(test)]` — `default_is_empty()`, `serde_roundtrip_preserves_overrides()`
3. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — `agent_schema_context_profile_returns_seeded_profile()`
4. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — `spawn_agent_applies_authored_schema_profile()` and `spawn_agent_defaults_empty_schema_profile_when_absent()`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`

Merge note: Ticket 003 likely bumps `SAVE_FORMAT_VERSION 86→87` because a new universal component table is added to the world's serialized shape. Confirm precise mechanics during implementation — the save_load layer may instead tolerate the addition via `#[serde(default)]` on the new component column. If a bump lands, ticket 003 is the single ticket carrying it (no cascade across S146 tickets — ticket 006's `PlanAttemptTrace.goal_budget` field is on a `Clone, Debug`-only trace type with no Serialize derive, so it requires no separate bump).
