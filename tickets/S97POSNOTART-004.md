# S97POSNOTART-004: CLI scenario support for `ArtifactPostingProfile`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — CLI/scenario infrastructure only
**Deps**: archive/tickets/S97POSNOTART-001.md

## Problem

The scenario profile completeness invariant requires every universal agent component to be configurable via `AgentDef` and applied in `spawn_agent()`. Without this, scenario authors cannot customize per-agent TTL values (e.g., guards posting longer-lived warnings than civilians).

## Assumption Reassessment (2026-04-12)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:67` has ~35 optional profile fields. No `ArtifactPostingProfile` or similar field exists (confirmed by grep). The field follows the `Option<ProfileType>` pattern used by all other profiles.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:323` registers components from `AgentDef` fields. Universal components use `unwrap_or_default()` and are always applied. Role-specific components use `if let Some(...)`. The new profile is universal.
3. `ArtifactPostingProfile` contains no `EntityId` references — no `*Def` wrapper type needed. The struct can be used directly in `AgentDef` and RON scenario files.
4. Existing RON scenarios (e.g., `scenarios/cli-evaluation.ron`) do not reference artifact posting — they will simply get `Default` values via `unwrap_or_default()`.

## Architecture Check

1. Universal component with `unwrap_or_default()` is the established pattern — consistent with `CognitiveProfile`, `PerceptionProfile`, `ExplorationProfile`, etc.
2. No backward-compatibility shims — existing scenarios compile without changes since the new field is `Option` and defaults to `None` (triggering `unwrap_or_default()`).

## Verification Layers

1. Scenario with explicit profile loads correctly → scenario loading test
2. Scenario without profile gets default values → `unwrap_or_default()` path test
3. Single-layer ticket (CLI scenario infrastructure) — no simulation-layer verification needed.

## What to Change

### 1. Add field to `AgentDef`

In `crates/worldwake-cli/src/scenario/types.rs`, add:

```rust
pub artifact_posting_profile: Option<ArtifactPostingProfile>,
```

Import `ArtifactPostingProfile` from `worldwake-core`.

### 2. Add registration in `spawn_agent()`

In `crates/worldwake-cli/src/scenario/mod.rs`, add after the existing universal profile registrations:

```rust
txn.set_component_artifact_posting_profile(
    agent,
    agent_def.artifact_posting_profile.clone().unwrap_or_default(),
)?;
```

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add field to `AgentDef`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add `set_component` call in `spawn_agent`)

## Out of Scope

- Updating existing RON scenario files to include explicit posting profiles (they get defaults)
- GoalBeliefView accessor (ticket 002)
- Candidate generation changes (ticket 003)
- Golden tests (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. Scenario with explicit `artifact_posting_profile` field in RON deserializes correctly
2. Scenario without the field uses `ArtifactPostingProfile::default()`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Every agent spawned via `spawn_agent()` has an `ArtifactPostingProfile`
2. Existing scenarios continue to load without modification

## Test Plan

### New/Modified Tests

1. None — verification is through existing scenario loading tests and the golden test in ticket 005 which exercises the full spawn→plan→post pipeline.

### Commands

1. `cargo test -p worldwake-cli -- scenario`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`
