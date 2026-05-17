# S151TESRELROU-010: SAVE_FORMAT_VERSION bump 87 → 88

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — single-line `SAVE_FORMAT_VERSION` constant bump and accompanying save/load coverage
**Deps**: archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-003.md, S151TESRELROU-005

## Problem

S151's D13 bumps `SAVE_FORMAT_VERSION` from 87 to 88 to cover the cumulative serialized-state additions across tickets 002 (runtime store fields on `AgentDecisionRuntime`), 003 (universal components on `EntityKind::Agent`), and 005 (payload context fields on `GoalCommittedPayload` / `GoalSuppressedPayload`). Ticket 002 proved current-format bincode round-trip and default initialization, not old bincode save compatibility. This ticket coordinates the single visible save-format boundary and must decide whether a version-87 fixture is still loadable or document the intentional no-migration boundary.

## Assumption Reassessment (2026-05-17)

1. `SAVE_FORMAT_VERSION` lives at `crates/worldwake-sim/src/save_load.rs:6` and is currently `87`. Used at line 101 (`bytes.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes())`) for writes and line 129 (`SAVE_FORMAT_VERSION => load_current_format(payload)`) for reads.
2. Per Step 2 spot-check (b): `SAVE_FORMAT_VERSION` has not changed since the reassessment; bump target is `87 → 88` exactly.
3. The cumulative new serialized state covers:
   - Ticket 002: `AgentDecisionRuntime.testimony_reliability: TestimonyReliability` and `AgentDecisionRuntime.route_preference: RoutePreference` (current-format round-trip and `AgentDecisionRuntime::default()` empty-store behavior proved; old bincode save loading remains ticket-010 scope).
   - Ticket 003: `TestimonyTrustProfile` and `RoutePreferenceProfile` components on `EntityKind::Agent` (bootstrap-seeded by `World::create_agent`; old saves load into Default values via the component-registration replay path).
   - Ticket 005: `GoalCommittedPayload.testimony_trust_context: Vec<_>`, `GoalCommittedPayload.route_preference_context: Vec<_>`, `GoalSuppressedPayload.testimony_trust_context: Vec<_>` (payload omitted-field defaults are explicit, but bincode save-stream compatibility must be verified or documented here).
4. The bump is the official "save format now includes S151 state" boundary. Do not assume Default-derived or `#[serde(default)]` fields make pre-87 bincode save streams loadable without a fixture.
5. Load-migration code is still expected to be unnecessary unless the version-87 fixture proves otherwise. If old bincode bytes cannot deserialize after the shape change, record that as the intentional FND-28 no-backward-compatibility boundary rather than adding a shim.

## Architecture Check

1. Per CLAUDE.md Determinism: bumping the format constant in a single location preserves the single-source-of-truth invariant for save format version.
2. Per FND-28: no shim, no dual-format support. The bump is the natural format boundary after S151's runtime, component, and payload state lands.
3. Per FND-12 (performance compresses computation, never causality): version-88 saves must preserve populated S151 state byte-for-byte through save/load. Version-87 behavior must be either proved loadable into legal default S151 state or documented as intentionally unsupported per FND-28.
4. Single-ticket bump (rather than per-deliverable cascade) keeps the format-version surface coherent — reviewers see one bump for the whole S151 surface, not three independent bumps that would force a cascade discipline.

## Verification Layers

1. Constant value → grep `crates/worldwake-sim/src/save_load.rs:6` for `SAVE_FORMAT_VERSION` and assert value is 88 post-edit.
2. Round-trip → save and reload a `SimulationState` containing populated S151 runtime stores; assert byte equality across the round trip.
3. Version-boundary probe → load a pre-bump (87) save fixture if practical. If it loads, assert all S151 fields are present with default/empty values. If it cannot load because bincode shape changed, document the intentional no-migration result.

## What to Change

### 1. Bump the constant (`crates/worldwake-sim/src/save_load.rs:6`)

```rust
pub const SAVE_FORMAT_VERSION: u32 = 88;
```

### 2. Verify load path

Confirm `load_current_format` at `save_load.rs:129` handles version-88 saves with the new fields. Do not claim version-87 bincode bytes load through default semantics unless the fixture proves it.

### 3. Save/load round-trip coverage

Add a focused unit test in `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` (or its sibling test module):

```rust
#[test]
fn save_load_roundtrip_preserves_s151_state() {
    let mut state = SimulationState::default();
    // Spawn an agent, populate TestimonyReliability and RoutePreference with sample data,
    // populate a GoalCommittedPayload context, then:
    let bytes = state.save();
    let loaded = SimulationState::load(&bytes).expect("loads at version 88");
    assert_eq!(loaded.world_hash(), state.world_hash());
}
```

### 4. Version-boundary regression

Add a sibling test or documented probe for a pre-bump byte fixture (constructed in-test as a version-87 minimal save if practical):

```rust
#[test]
fn save_load_loads_pre_s151_save_with_default_s151_state() {
    let pre_bump_bytes = build_minimal_version_87_save();
    let loaded = SimulationState::load(&pre_bump_bytes).expect("loads pre-S151 save");
    // Assert default/empty S151 state on every agent
}
```

If constructing a version-87 fixture in-test is impractical, document the omission and keep the acceptance claim to version-88 round-trip coverage. If the fixture is practical but fails due to bincode shape changes, document the intentional no-migration boundary.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — single-line constant bump + new tests)

## Out of Scope

- Runtime store additions — ticket 002
- Universal profile additions — ticket 003
- Payload context additions — ticket 005
- Any production-code migration shim unless the user explicitly changes the no-backward-compatibility policy

## Acceptance Criteria

### Tests That Must Pass

1. `SAVE_FORMAT_VERSION == 88` after the bump.
2. Round-trip test: populated S151 state survives save → load → equality check.
3. Version-boundary test/probe: pre-bump save loads into default S151 state, or the no-migration outcome is documented with rationale.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. Save format version is a single integer at a single source location.
2. Saves written by version-88 binaries load by version-88 binaries with byte equality preserved.
3. Version-87 save behavior is explicitly proved or explicitly documented as unsupported under the no-backward-compatibility policy.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` — round-trip and backward-compat tests for the bump.

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`

Merge note: This ticket bumps SAVE_FORMAT_VERSION 87→88 as a single-coordinated step after tickets 002, 003, and 005 land their serialized state. No sibling tickets bump the version; do not infer old bincode save compatibility from `#[serde(default)]` alone.
