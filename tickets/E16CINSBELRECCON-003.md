# E16CINSBELRECCON-003: Extend AgentBeliefStore + PerceptionProfile for Institutional Knowledge

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new fields on existing components in worldwake-core
**Deps**: E16CINSBELRECCON-001 (institutional types must exist)

## Problem

Agents need storage for institutional beliefs and per-agent consultation parameters. `AgentBeliefStore` must gain an `institutional_beliefs` field, and `PerceptionProfile` must gain consultation-related fields (`institutional_memory_capacity`, `consultation_speed_factor`, `contradiction_tolerance`).

## Assumption Reassessment (2026-03-21)

1. `AgentBeliefStore` (belief.rs:12-17) currently has: `known_entities`, `social_observations`, `told_beliefs`, `heard_beliefs`. No institutional beliefs field exists.
2. `PerceptionProfile` (belief.rs:453-458) currently has: `memory_capacity`, `memory_retention_ticks`, `observation_fidelity`, `confidence_policy`. No consultation fields exist.
3. Both types derive `Serialize`/`Deserialize`. Adding fields will break existing serialized data (save files) — but save/load compatibility is explicitly not maintained across breaking changes per Principle 26.
4. `AgentBeliefStore::new()` returns `Self::default()`. Default for `BTreeMap` is empty, which is correct for institutional_beliefs.
5. `delta.rs` has a `ComponentValue::AgentBeliefStore(...)` sample and a `ComponentValue::PerceptionProfile(...)` sample — both must be updated.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Extending existing components is cleaner than creating separate `InstitutionalBeliefStore` or `RecordConsultationProfile` components — avoids schema proliferation and keeps agent state unified.
2. No backward-compatibility shims. Old save files will not deserialize (acceptable per Principle 26).

## Verification Layers

1. `AgentBeliefStore` with institutional beliefs roundtrips → bincode test
2. `PerceptionProfile` with new fields roundtrips → bincode test
3. Default values are sensible → unit tests check defaults
4. Single-layer ticket — component field additions only.

## What to Change

### 1. Extend `AgentBeliefStore` in `belief.rs`

Add field:
```rust
pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
```

Update `Default` impl to include `institutional_beliefs: BTreeMap::new()`.

### 2. Extend `PerceptionProfile` in `belief.rs`

Add fields:
```rust
pub institutional_memory_capacity: u32,       // default: 20
pub consultation_speed_factor: Permille,      // default: Permille(500)
pub contradiction_tolerance: Permille,        // default: Permille(300)
```

Update `Default` impl with the defaults above.

### 3. Update delta test samples in `delta.rs`

Update `ComponentValue::AgentBeliefStore(...)` and `ComponentValue::PerceptionProfile(...)` samples to include the new fields.

### 4. Update any test helpers that construct these types

Grep for `AgentBeliefStore {` and `PerceptionProfile {` struct literals across the workspace and add the new fields.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add fields to both structs + update defaults)
- `crates/worldwake-core/src/delta.rs` (modify — update test samples)
- Various test files across crates that construct `AgentBeliefStore` or `PerceptionProfile` struct literals (modify — add new fields)

## Out of Scope

- Derivation helpers on institutional beliefs (ticket -009)
- WorldTxn projection helpers (ticket -004)
- AI reading institutional beliefs (tickets -010 through -014)
- Capacity enforcement logic (part of projection in ticket -004)
- Any action definitions or handlers

## Acceptance Criteria

### Tests That Must Pass

1. `AgentBeliefStore` with populated `institutional_beliefs` roundtrips through bincode
2. `PerceptionProfile` with new fields roundtrips through bincode
3. Default `PerceptionProfile` has `institutional_memory_capacity == 20`, `consultation_speed_factor == Permille(500)`, `contradiction_tolerance == Permille(300)`
4. Default `AgentBeliefStore` has empty `institutional_beliefs`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `institutional_beliefs` uses `BTreeMap` (deterministic iteration)
2. `consultation_speed_factor` and `contradiction_tolerance` use `Permille` (no floats)
3. All existing tests pass with new fields added to struct literals

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — roundtrip tests for extended structs, default value assertions
2. `crates/worldwake-core/src/delta.rs` — updated sample still roundtrips

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace && cargo test --workspace`
