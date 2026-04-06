# S52EVIDAFT-001: Core evidence types and component registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type, new types in worldwake-core, component schema, save format version
**Deps**: None

## Problem

Actions leave no persistent physical evidence in the world. This ticket adds the foundational types (`EvidenceEntryId`, `EvidenceKind`, `DisturbanceKind`, `SceneEvidence`, `EvidenceEntry`) and registers `SceneEvidence` on `EntityKind::Place` so downstream tickets can emit and perceive evidence.

## Assumption Reassessment (2026-04-05)

1. `EntityKind::Place` exists at `crates/worldwake-core/src/entity.rs`. Currently 3 components registered on Place in `component_schema.rs`: `ResourceSource`, `ProductionOutputOwnershipPolicy`, `BanditCamp`.
2. `component_schema.rs` uses `with_component_schema_entries!` macro with closures filtering by EntityKind — confirmed. Macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) import types by bare name.
3. `Permille` derives Copy at `crates/worldwake-core/src/permille.rs` — used in `BloodTrail.severity`.
4. `ViolationId(pub u64)` pattern at `crates/worldwake-core/src/violation.rs:17-20` — model for `EvidenceEntryId`.
5. `SAVE_FORMAT_VERSION` is 21 at `crates/worldwake-sim/src/save_load.rs:6`.
6. `EventTag::WildernessRelief` exists at `crates/worldwake-core/src/event_tag.rs:30` — referenced by `DisturbanceKind::WildernessRelief`.

### Reassessment Correction (2026-04-05, implementation)

- ticket says: all five evidence types need new bare imports at the macro expansion sites
- live code has: the `forward_authoritative_components` expansion sites in `delta.rs`, `world.rs`, and `component_tables.rs` only mention the registered component type itself. The nested evidence payload types stay inside `SceneEvidence`. `world_txn.rs` also expands component schema today, but through the txn-selector path that uses crate-qualified component types and does not need a new bare import.
- correction applied: narrow the import fallout to `SceneEvidence` only, and keep `world_txn.rs` out of `Files to Touch`
- why safe: this matches the actual selector-macro surface and avoids documenting compile fallout that the current code will not produce

## Architecture Check

1. Evidence types are plain data — no logic, no cross-crate dependencies beyond worldwake-core types. Clean foundation for downstream emission, decay, and perception tickets.
2. `SceneEvidence` on Place (not on individual entities) is correct: evidence is scene-level aftermath at a location, not a property of the actor or victim.
3. No backward-compatibility shims.

## Verification Layers

1. SceneEvidence registered on EntityKind::Place → component schema tests
2. All new types compile with required derives → `cargo build -p worldwake-core`
3. Macro expansion sites compile with new types imported → `cargo build --workspace`
4. Single-layer ticket (worldwake-core types only) — no cross-system verification needed.

## What to Change

### 1. Add evidence types module

Create `crates/worldwake-core/src/evidence.rs`:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EvidenceEntryId(pub u64);
```

`EvidenceKind` enum with variants: `ContainerTampered { container, tampered_at }`, `BloodTrail { from_place, severity, caused_by }`, `DisturbanceMarker { place, kind, created_at }`, `MovementTrace { entity, departed_from, direction, observed_at }`.

`DisturbanceKind` enum: `CombatAftermath`, `ForcedEntry`, `AbandonedGoods`, `WildernessRelief`.

`EvidenceEntry` struct: `id`, `kind`, `created_at`, `decay_ticks`.

`SceneEvidence` component: `evidence: Vec<EvidenceEntry>`, `next_entry_id: u64`.

All types derive Serialize, Deserialize, Clone, Debug, Eq, PartialEq. Copy where possible (EvidenceEntryId, DisturbanceKind). `impl Component for SceneEvidence {}`.

### 2. Register in component_schema.rs

Add `SceneEvidence` with closure `|kind| kind == EntityKind::Place`.

### 3. Re-export from lib.rs

Add `pub mod evidence;` and re-export key types.

### 4. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`: change from 21 to 22.

### 5. Import bare `SceneEvidence` at macro expansion sites

Ensure `SceneEvidence` is imported at `delta.rs`, `world.rs`, and `component_tables.rs`.

## Files to Touch

- `crates/worldwake-core/src/evidence.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — macro expansion site imports)
- `crates/worldwake-core/src/delta.rs` (modify — macro expansion site imports)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro expansion site imports)
- `crates/worldwake-sim/src/save_load.rs` (modify)

## Out of Scope

- Evidence emission from action handlers — ticket 002
- Evidence decay system — ticket 003
- Evidence perception and beliefs — ticket 004
- Golden tests — ticket 005

## Acceptance Criteria

### Tests That Must Pass

1. SceneEvidence registered on EntityKind::Place in component schema
2. EvidenceEntryId, EvidenceKind, DisturbanceKind serialize/deserialize round-trip
3. SceneEvidence with entries serializes/deserializes correctly
4. Existing suite: `cargo test --workspace`

### Invariants

1. SceneEvidence registered ONLY on EntityKind::Place — not on Agent, Item, etc.
2. EvidenceEntryId is Copy
3. All macro expansion sites compile with new types in scope
4. SAVE_FORMAT_VERSION == 22

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/evidence.rs` — Serialize/Deserialize round-trip tests for all new types
2. `crates/worldwake-core/src/component_schema.rs` — Registration test for SceneEvidence on Place

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Added the new evidence substrate in `crates/worldwake-core/src/evidence.rs`: `EvidenceEntryId`, `DisturbanceKind`, `EvidenceKind`, `EvidenceEntry`, and the `SceneEvidence` place component.
  - Re-exported the new evidence types from `crates/worldwake-core/src/lib.rs`.
  - Registered `SceneEvidence` on `EntityKind::Place` in `crates/worldwake-core/src/component_schema.rs`.
  - Added the required bare `SceneEvidence` import fallout in `crates/worldwake-core/src/component_tables.rs`, `crates/worldwake-core/src/world.rs`, and `crates/worldwake-core/src/delta.rs`.
  - Absorbed the expected schema-registry fallout in `crates/worldwake-core/src/delta.rs` so `ComponentKind::ALL` and `component_samples()` still mirror the authoritative component set after the new registration lands.
  - Bumped `SAVE_FORMAT_VERSION` from `21` to `22` in `crates/worldwake-sim/src/save_load.rs`.
- **Deviations from original plan**:
  - Reassessment corrected the ticket’s macro-expansion fallout: only the bare registered `SceneEvidence` type needed imports at the forward-authoritative expansion sites, not all nested evidence payload types.
  - Focused verification exposed one additional owned registry mirror in `crates/worldwake-core/src/delta.rs` (`ComponentKind::ALL` / `component_samples()`), which had to be updated even though the ticket’s original fallout list only named the schema macros.
- **Verification**:
  - `cargo test -p worldwake-core evidence_identifier_and_variants_roundtrip_through_bincode -- --nocapture`
  - `cargo test -p worldwake-core scene_evidence_is_registered_for_places_only -- --nocapture`
  - `cargo test -p worldwake-core`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
