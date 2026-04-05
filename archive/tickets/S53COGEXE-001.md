# S53COGEXE-001: New CognitiveProfile and ExecutionBudget types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component types, component schema registration, agent constructor + AgentDef extension, save version bump
**Deps**: None

## Problem

`ReasoningProfile` conflates agent psychology (cognitive parameters) with engine compression (search budgets). This ticket adds the two replacement types alongside the existing `ReasoningProfile` so the workspace builds with both old and new types coexisting, enabling incremental consumer migration in ticket 002.

## Assumption Reassessment (2026-04-05)

1. `ReasoningProfile` at `crates/worldwake-core/src/reasoning_profile.rs:8-21` has 12 fields. All field names, types, and defaults verified against the spec's classification table.
2. `component_schema.rs` registers `ReasoningProfile` on `EntityKind::Agent`. New profiles registered alongside it — no removal yet.
3. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs` has `reasoning_profile: Option<ReasoningProfile>`. New fields added alongside — no removal yet.
4. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs` applies `ReasoningProfile`. New profiles applied alongside.
5. Both new types are universal profiles — require Default impls per `docs/spec-drafting-rules.md` section 5.
6. `World::create_agent()` at `crates/worldwake-core/src/world.rs:148-171` seeds universal agent profiles directly. New universal profiles must be inserted there alongside `ReasoningProfile` rather than only through scenario spawning.
7. `SAVE_FORMAT_VERSION` is 23. Adding persisted authoritative components changes the serialized world shape immediately, so this ticket must bump the version now. Old pre-split saves remain out of scope until ticket 003.

## Architecture Check

1. Adding new types alongside the old type enables incremental migration without breaking AI consumers. Both `ReasoningProfile` and `CognitiveProfile` + `ExecutionBudget` coexist temporarily in authoritative world state so consumers can migrate one file at a time in ticket 002.
2. This is a deliberate transitional state, not a backward-compatibility shim. The old type is removed in ticket 003 per P28.
3. Default values for CognitiveProfile match the cognitive fields from ReasoningProfile::default(). Default values for ExecutionBudget match the engine fields.
4. Because the new profiles are authoritative persisted components, the save format version changes in this ticket even though the old `ReasoningProfile` still coexists temporarily.
5. During staged coexistence, explicit `cognitive_profile` / `execution_budget` values win. When they are omitted, setup paths that still provide only `ReasoningProfile` must derive the split profiles from that legacy value so ticket 002 can migrate consumers without silent behavior drift.

## Verification Layers

1. CognitiveProfile and ExecutionBudget compile with correct derives → `cargo build -p worldwake-core`
2. Both registered on EntityKind::Agent → component schema tests
3. Default impls produce values matching ReasoningProfile defaults → focused unit test
4. `AgentDef`, `spawn_agent()`, and `World::create_agent()` accept/seed both new profile types → `cargo build -p worldwake-cli`
5. Save boundary stays honest after the new persisted components land → focused save/load test
6. Single-layer ticket — no AI consumer migration yet.

## What to Change

### 1. Add CognitiveProfile type

Create `crates/worldwake-core/src/cognitive_profile.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CognitiveProfile {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub switch_margin: Permille,
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}
```

Default impl matching ReasoningProfile defaults for the cognitive fields:
- `max_candidates_to_plan: 2`, `max_plan_depth: 8`, `switch_margin: Permille::new_unchecked(100)`
- Block ticks and cooldown values from current ReasoningProfile::default()

### 2. Add ExecutionBudget type

Create `crates/worldwake-core/src/execution_budget.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExecutionBudget {
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub snapshot_travel_horizon: u8,
    pub max_prerequisite_locations: u8,
}
```

Default impl matching ReasoningProfile defaults for the engine fields.

### 3. Register in component_schema.rs

Add both `CognitiveProfile` and `ExecutionBudget` on `EntityKind::Agent` alongside existing `ReasoningProfile`.

### 4. Add to AgentDef and spawn_agent

In `crates/worldwake-cli/src/scenario/types.rs`:
- Add `cognitive_profile: Option<CognitiveProfile>`
- Add `execution_budget: Option<ExecutionBudget>`

In `crates/worldwake-cli/src/scenario/mod.rs` `spawn_agent()`:
- Apply `ReasoningProfile` as before
- If `cognitive_profile` / `execution_budget` are explicitly present, use them
- Otherwise derive them from the staged `ReasoningProfile` value

In `crates/worldwake-core/src/world.rs` `create_agent()`:
- Insert both with `Default::default()` alongside the existing universal profiles

### 5. Re-export from lib.rs

Add `pub mod cognitive_profile;` and `pub mod execution_budget;` and re-export key types.

### 6. Bump save format version

In `crates/worldwake-sim/src/save_load.rs`:
- Bump `SAVE_FORMAT_VERSION` from 23 to 24 to reflect the new persisted component shape

### 7. Import at macro expansion sites

Ensure new types imported at `delta.rs`, `world.rs`, `component_tables.rs`.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (new)
- `crates/worldwake-core/src/execution_budget.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — universal profile seeding + macro expansion imports)
- `crates/worldwake-core/src/delta.rs` (modify — macro expansion imports)
- `crates/worldwake-core/src/world.rs` (modify — macro expansion imports)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro expansion imports)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify — save version bump)

## Out of Scope

- Migrating AI consumers from ReasoningProfile — ticket 002
- Removing ReasoningProfile — ticket 003
- Migration support for pre-split save payloads — ticket 003
- Behavioral validation conformance test — ticket 004

## Acceptance Criteria

### Tests That Must Pass

1. CognitiveProfile::default() matches ReasoningProfile::default() cognitive fields
2. ExecutionBudget::default() matches ReasoningProfile::default() engine fields
3. Both registered on EntityKind::Agent in component schema
4. AgentDef with cognitive_profile and execution_budget deserializes correctly
5. `World::create_agent()` seeds both new universal profiles by default
6. `SAVE_FORMAT_VERSION == 24`
5. Existing suite: `cargo test --workspace`

### Invariants

1. ReasoningProfile still exists and works — no consumers broken
2. Both new types are universal — Default impl required
3. All macro expansion sites compile with new types
4. Saved world shape is versioned honestly the moment the new authoritative components land

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` — Default value tests, serde round-trip
2. `crates/worldwake-core/src/execution_budget.rs` — Default value tests, serde round-trip
3. `crates/worldwake-cli/src/scenario/mod.rs` — spawn/default seeding coverage for the new universal profiles
4. `crates/worldwake-cli/src/handlers/persistence.rs` — round-trip still works after the version bump

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-cli`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-05

What changed:
- Added authoritative `CognitiveProfile` and `ExecutionBudget` component types in `worldwake-core`, registered both on `EntityKind::Agent`, and re-exported them from `lib.rs`.
- Updated universal agent creation in `World::create_agent()` so new agents receive default split profiles alongside the still-live `ReasoningProfile`.
- Extended CLI scenario `AgentDef` and `spawn_agent()` to accept explicit `cognitive_profile` / `execution_budget` values.
- Preserved staged coexistence coherence: when setup paths still provide only `ReasoningProfile`, `spawn_agent()` now derives the split profiles from that legacy value so later consumer migration cannot silently change agent behavior.
- Bumped `SAVE_FORMAT_VERSION` from 23 to 24 because the persisted authoritative world shape changed the moment the new components landed.
- Updated registry/sample fallout and persistence fixtures so the new components are reflected honestly across schema mirrors and round-trip tests.

Deviations from original plan:
- The original ticket boundary assumed save/load could wait until ticket 003. That was corrected before coding because the persisted component shape changed immediately in this ticket.
- The staged coexistence path needed one extra rule beyond the original draft: explicit split profiles win, but omitted split profiles must be derived from `ReasoningProfile` during the migration window.
- Ticket `S53COGEXE-003` was updated in parallel so it now owns coexistence-format cleanup from save version 24 to 25 rather than the first split-version bump.

Verification results:
- `cargo test -p worldwake-core cognitive_profile -- --nocapture`
- `cargo test -p worldwake-core execution_budget -- --nocapture`
- `cargo test -p worldwake-cli test_spawn_agents_receive_default_universal_profiles -- --nocapture`
- `cargo test -p worldwake-cli test_save_load_roundtrip_preserves_agent_reasoning_profile -- --nocapture`
- `cargo test -p worldwake-core`
- `cargo test -p worldwake-cli`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
