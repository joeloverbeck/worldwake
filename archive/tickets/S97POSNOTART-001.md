# S97POSNOTART-001: `ArtifactPostingProfile` component + registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS component on `EntityKind::Agent`, component schema, bootstrap seeding
**Deps**: None

## Problem

PostNotice and PostBounty artifacts persist indefinitely because no TTL is attached at creation time. This ticket introduces the per-agent profile that holds TTL defaults, which downstream tickets will read during candidate generation to compute `expires_at`.

## Assumption Reassessment (2026-04-12)

1. `ArtifactPostingProfile` does not yet exist anywhere in the codebase (confirmed: zero grep matches in `crates/worldwake-core`). The `ArtifactPostingContext` struct at `crates/worldwake-core/src/social_artifact.rs:18` already has an `expires_at: Option<Tick>` field — this ticket provides the source values.
2. `component_schema.rs` registers Agent components via `with_component_schema_entries!` macro with closure filter `|kind| kind == EntityKind::Agent`. Macro expands in 4 sites: `delta.rs`, `world.rs`, `component_tables.rs`, `world_txn.rs` — all must import the new type.
3. `World::create_agent()` at `crates/worldwake-core/src/world.rs:152` seeds 15 universal profile defaults. The new component must be seeded there with `insert_component_artifact_posting_profile(entity, ArtifactPostingProfile::default())`.
4. The delta assertion test `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` at `crates/worldwake-core/src/world_txn.rs:2359` asserts the exact delta sequence from `create_agent`. A new `ComponentDelta::Set` entry for `ArtifactPostingProfile` must be added.
5. `Tick` wraps `u64` (`crates/worldwake-core/src/ids.rs:57`) and implements `Add<u64>`. TTL fields must be `u64` to match.

## Architecture Check

1. A dedicated profile component is cleaner than hardcoded TTL constants or embedding TTL in `UtilityProfile` — it separates posting behavior configuration from general utility weights, following the existing pattern of focused profile components (e.g., `CognitiveProfile`, `PerceptionProfile`).
2. No backward-compatibility shims — this is a new component with no prior art to migrate from.

## Verification Layers

1. Component exists and is queryable on agents → focused unit test (`component_schema` round-trip)
2. `create_agent` seeds default → delta assertion test in `world_txn.rs`
3. `Default` impl provides sensible non-zero TTL values → unit test on `ArtifactPostingProfile::default()`
4. Single-layer ticket (ECS component infrastructure only) — no cross-system verification needed.

## What to Change

### 1. Define `ArtifactPostingProfile` struct

In `crates/worldwake-core/src/social_artifact.rs` (or a new `artifact_posting_profile.rs` if the file is large), add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPostingProfile {
    pub threat_warning_ttl: u64,
    pub office_vacancy_ttl: u64,
    pub bounty_ttl: u64,
}

impl Default for ArtifactPostingProfile {
    fn default() -> Self {
        Self {
            threat_warning_ttl: 48,
            office_vacancy_ttl: 96,
            bounty_ttl: 144,
        }
    }
}
```

Re-export from `crates/worldwake-core/src/lib.rs`.

### 2. Register in `component_schema.rs`

Add an entry to the `with_component_schema_entries!` macro invocation with `|kind| kind == EntityKind::Agent` filter. This generates insert/get/clear accessors and `ComponentKind`/`ComponentValue` variants.

### 3. Import at macro expansion sites

Add `use crate::ArtifactPostingProfile;` (or equivalent import) in:
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world_txn.rs`

### 4. Seed in `create_agent`

In `World::create_agent()` at `crates/worldwake-core/src/world.rs:152`, add:
```rust
world.insert_component_artifact_posting_profile(entity, ArtifactPostingProfile::default())?;
```

### 5. Update delta assertion test

In `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` at `crates/worldwake-core/src/world_txn.rs:2359`, add the expected `ComponentDelta::Set` entry for `ArtifactPostingProfile` in the correct position within the delta sequence.

## Files to Touch

- `crates/worldwake-core/src/social_artifact.rs` (modify — add struct + Default impl)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify — add registration entry)
- `crates/worldwake-core/src/delta.rs` (modify — import)
- `crates/worldwake-core/src/world.rs` (modify — import + seed in `create_agent`)
- `crates/worldwake-core/src/component_tables.rs` (modify — import)
- `crates/worldwake-core/src/world_txn.rs` (modify — import + update delta assertion)

## Out of Scope

- GoalBeliefView accessor (ticket 002)
- Candidate generation changes (ticket 003)
- CLI scenario support (ticket 004)
- Golden tests (ticket 005)
- Modifying artifact lifecycle system (already handles `expires_at`)
- Modifying PostNotice action handler (already uses `payload.expires_at`)

## Acceptance Criteria

### Tests That Must Pass

1. `ArtifactPostingProfile::default()` returns non-zero TTL values for all three fields
2. Component round-trip: insert profile on agent, read it back, values match
3. `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` passes with the new delta entry
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Every agent created via `create_agent()` has an `ArtifactPostingProfile` with default values
2. `ArtifactPostingProfile` is registered on `EntityKind::Agent` only — not on other entity kinds
3. TTL fields are `u64` matching `Tick` arithmetic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/social_artifact.rs` (or module test) — unit test for `Default` impl values
2. `crates/worldwake-core/src/world_txn.rs` — modified delta assertion test

### Commands

1. `cargo test -p worldwake-core -- artifact_posting`
2. `cargo test -p worldwake-core -- create_agent`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-04-12.

- Added authoritative `ArtifactPostingProfile` in `crates/worldwake-core/src/social_artifact.rs` with the spec-default TTL values, `Component` registration, and focused default-value coverage.
- Registered the component on `EntityKind::Agent`, re-exported it from `worldwake-core`, seeded it from `World::create_agent()`, and extended the world/component test surface with a direct agent round-trip proof.
- Updated macro-driven component fallout so the new variant is part of `ComponentKind`/`ComponentValue` inventories and the exact `create_agent` delta assertion now includes the new seeded component.

## Deviations

- The component round-trip proof lives in `crates/worldwake-core/src/world.rs`, not `component_schema.rs`; that is the real consumer-facing test boundary for generated component accessors.
- The `create_agent` delta sequence is ordered by component-schema projection, so `ArtifactPostingProfile` appears immediately after `Name` in the created-entity delta batch rather than at the raw insertion call position implied by the draft ticket text.

## Verification Result

- Passed `cargo test -p worldwake-core artifact_posting`
- Passed `cargo test -p worldwake-core create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
