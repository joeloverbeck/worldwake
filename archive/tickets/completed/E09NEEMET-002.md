# E09NEEMET-002: DriveThresholds and ThresholdBand shared Phase 2 schema

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new core types, component registration
**Deps**: E08 (Phase 1 complete)

## Problem

Both E09 (deprivation tracking / collapse behavior) and E13 (decision architecture urgency) need per-drive, per-agent threshold bands. The spec explicitly states `DriveThresholds` is shared Phase 2 schema that must not be owned solely by E13. This ticket creates the threshold types and registers `DriveThresholds` as a component.

## Assumption Reassessment (2026-03-10)

1. No `DriveThresholds` or `ThresholdBand` types exist in the codebase — confirmed.
2. `specs/IMPLEMENTATION-ORDER.md` Step 7a lists "per-drive thresholds" as shared schema, and `specs/E09-needs-metabolism.md` explicitly defines `DriveThresholds` as shared Phase 2 schema used by both E09 and E13.
3. The spec defines 7 threshold bands in `DriveThresholds`: hunger, thirst, fatigue, bladder, dirtiness, pain, danger. Pain and danger remain derived pressures, but their threshold bands still belong in the shared schema because E13 classifies urgency against them.
4. Component kind validation is enforced by `World::insert_component_*`, not by raw `ComponentTables`. Tests for agent-only registration must therefore target the `World` API.
5. `worldwake-core` uses macro-driven authoritative component registration. Adding a new component requires updating the schema fanout (`component_schema.rs`, `component_tables.rs`, `delta.rs`, `world.rs`, exports, and coverage tests), not only the three files in the original ticket.
6. No existing per-agent physiology seeding pipeline exists yet. The shared schema still needs a bootstrap baseline, but the cleaner long-term API is a canonical `Default` implementation plus explicit full construction for seeded/custom thresholds.

## Architecture Check

1. Placing in `worldwake-core` allows both `worldwake-systems` (E09) and `worldwake-ai` (E13) to import without circular deps.
2. Per-agent `DriveThresholds` supports Principle 11 (agent diversity) — different agents can have different sensitivity levels.
3. `ThresholdBand` validates ordering (low < medium < high < critical) at construction time.
4. Keeping `ThresholdBand` as a small value type and `DriveThresholds` as the only component avoids polluting the ECS with per-band subcomponents or alias types that would add indirection without improving extensibility.
5. The bootstrap constructor should stay explicitly named (`default_human`) and return concrete shared-schema defaults, but callers must still be free to construct fully custom thresholds for seeded agent diversity.

## What to Change

### 1. New module `crates/worldwake-core/src/drives.rs`

```rust
pub struct ThresholdBand {
    low: Permille,
    medium: Permille,
    high: Permille,
    critical: Permille,
}

impl ThresholdBand {
    pub fn new(low: Permille, medium: Permille, high: Permille, critical: Permille) -> Result<Self, &'static str>
    // Validates: low < medium < high < critical
    // Accessor methods for each level
}

pub struct DriveThresholds {
    pub hunger: ThresholdBand,
    pub thirst: ThresholdBand,
    pub fatigue: ThresholdBand,
    pub bladder: ThresholdBand,
    pub dirtiness: ThresholdBand,
    pub pain: ThresholdBand,
    pub danger: ThresholdBand,
}
impl Component for DriveThresholds {}
```

Expose `DriveThresholds::new(...)` for explicit construction and use `Default` for the baseline bootstrap thresholds.

### 2. Register `DriveThresholds` in the authoritative component schema

Add the macro block for `DriveThresholds` on `EntityKind::Agent`. Because the core component API is schema-generated, this also implies updating the generated surface exercised through:

- `component_tables.rs`
- `delta.rs`
- `world.rs`
- coverage tests that enumerate authoritative components

### 3. Export from `lib.rs`

Add `pub mod drives;` and re-export.

## Files to Touch

- `crates/worldwake-core/src/drives.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (macro expansion + tests)
- `crates/worldwake-core/src/delta.rs` (component enums/round-trip coverage)
- `crates/worldwake-core/src/world.rs` (generated component API + world-level tests)

## Out of Scope

- How E13 uses thresholds for urgency classification (E13's concern)
- Deprivation exposure tracking (E09NEEMET-003)
- Threshold-driven AI behavior (E13)
- Runtime modification of thresholds

## Acceptance Criteria

### Tests That Must Pass

1. `ThresholdBand::new` succeeds with valid ordered values (low < medium < high < critical).
2. `ThresholdBand::new` returns error when ordering is violated (e.g., low >= medium).
3. `DriveThresholds` can be inserted, retrieved, and removed on Agent entities through the `World` component API.
4. `DriveThresholds` insertion is rejected for non-Agent entity kinds through the `World` component API.
5. Bincode round-trip for `DriveThresholds`.
6. `DriveThresholds::default()` produces valid threshold bands.
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ThresholdBand` enforces strict ordering: low < medium < high < critical.
2. All values are `Permille` — no floats.
3. `DriveThresholds` is per-agent, per-drive — not a global constant.
4. Component kind predicate restricts to `EntityKind::Agent`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/drives.rs` (unit tests) — construction, validation, serialization, canonical default baseline, trait bounds
2. `crates/worldwake-core/src/world.rs` (integration-style unit tests) — insert/get/remove on agents, rejection on non-agents
3. `crates/worldwake-core/src/component_tables.rs` / `delta.rs` (coverage updates) — new component participates correctly in authoritative schema fanout

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-10

- Completion date: 2026-03-10
- What actually changed:
  - added `crates/worldwake-core/src/drives.rs` with `ThresholdBand` and agent-scoped `DriveThresholds`
  - registered `DriveThresholds` in the macro-driven authoritative component schema and propagated that through `component_tables.rs`, `delta.rs`, `world.rs`, and `lib.rs`
  - added coverage for threshold validation, explicit full construction, canonical default baseline, serialization, world-level agent-only insertion, and schema fanout
- Deviations from original plan:
  - corrected the spec reference to `specs/E09-needs-metabolism.md`
  - expanded scope beyond the original three files because authoritative component registration is macro-generated and must stay coherent across core tables, delta enums, and world APIs
  - refined the API after the initial archival pass: `DriveThresholds` now uses `Default` as the single canonical baseline entry point and `DriveThresholds::new(...)` for explicit construction, instead of a bespoke `default_human()` constructor
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
