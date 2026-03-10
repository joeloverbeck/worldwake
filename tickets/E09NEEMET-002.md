# E09NEEMET-002: DriveThresholds and ThresholdBand shared Phase 2 schema

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new core types, component registration
**Deps**: E08 (Phase 1 complete)

## Problem

Both E09 (deprivation tracking / collapse behavior) and E13 (decision architecture urgency) need per-drive, per-agent threshold bands. The spec explicitly states `DriveThresholds` is shared Phase 2 schema that must not be owned solely by E13. This ticket creates the threshold types and registers `DriveThresholds` as a component.

## Assumption Reassessment (2026-03-10)

1. No `DriveThresholds` or `ThresholdBand` types exist in the codebase — confirmed.
2. IMPLEMENTATION-ORDER.md Step 7a lists "per-drive thresholds" as shared schema.
3. The spec defines 7 drives in `DriveThresholds`: hunger, thirst, fatigue, bladder, dirtiness, pain, danger. Pain and danger are derived values but still need threshold bands for E13 urgency classification.

## Architecture Check

1. Placing in `worldwake-core` allows both `worldwake-systems` (E09) and `worldwake-ai` (E13) to import without circular deps.
2. Per-agent `DriveThresholds` supports Principle 11 (agent diversity) — different agents can have different sensitivity levels.
3. `ThresholdBand` validates ordering (low < medium < high < critical) at construction time.

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

Include a `DriveThresholds::default_human()` constructor that returns reasonable default threshold values for a standard human-like agent.

### 2. Register `DriveThresholds` in `component_schema.rs`

Add macro block for `DriveThresholds` on `EntityKind::Agent`.

### 3. Export from `lib.rs`

Add `pub mod drives;` and re-export.

## Files to Touch

- `crates/worldwake-core/src/drives.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)

## Out of Scope

- How E13 uses thresholds for urgency classification (E13's concern)
- Deprivation exposure tracking (E09NEEMET-003)
- Threshold-driven AI behavior (E13)
- Runtime modification of thresholds

## Acceptance Criteria

### Tests That Must Pass

1. `ThresholdBand::new` succeeds with valid ordered values (low < medium < high < critical).
2. `ThresholdBand::new` returns error when ordering is violated (e.g., low >= medium).
3. `DriveThresholds` can be inserted, retrieved, and removed on Agent entities.
4. `DriveThresholds` insertion is rejected for non-Agent entity kinds.
5. Bincode round-trip for `DriveThresholds`.
6. `DriveThresholds::default_human()` produces valid threshold bands.
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ThresholdBand` enforces strict ordering: low < medium < high < critical.
2. All values are `Permille` — no floats.
3. `DriveThresholds` is per-agent, per-drive — not a global constant.
4. Component kind predicate restricts to `EntityKind::Agent`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/drives.rs` (unit tests) — construction, validation, serialization, trait bounds

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
