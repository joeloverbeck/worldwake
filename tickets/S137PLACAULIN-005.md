# S137PLACAULIN-005: RepairMemory shape migration to BreachSignature

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `RepairMemory` shape, save-load, removal of `RepairKey`
**Deps**: archive/tickets/S137PLACAULIN-001.md (BreachSignature), archive/tickets/S137PLACAULIN-003.md (new RepairKind variants populated in RepairEntry), 004 (save-format baseline if it lands before this ticket)

## Problem

S137 D7 migrates `RepairMemory.repairs` from `BTreeMap<RepairKey, RepairEntry>` to `BTreeMap<BreachSignature, RepairEntry>` and reshapes `RepairEntry` to discriminate success/failure per repair attempt. The migration is single-truth per FND-28 — `RepairKey` is removed, no parallel `successful_kinds`/`failed_kinds` maps coexist with `repairs`. Ticket 006's `plan_repair` module reads `repairs.get(&signature)` to skip recently-failed `RepairKind` variants for the same breach.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RepairMemory` is defined at `crates/worldwake-core/src/repair_memory.rs:19-22` as `BTreeMap<RepairKey, RepairEntry>` with `success_count: u32` on `RepairEntry`. `RepairKey { goal_key, alternate_target: EntityId }` at lines 5-9; `RepairEntry { repair_key, observed_tick, expires_tick, success_count }` at lines 11-17. Component registration at `crates/worldwake-core/src/component_schema.rs:784-802`. Existing `#[cfg(test)]` tests at lines 55-202 cover bincode roundtrip, `record`, `expire`, `enforce_capacity`. Three test fixtures use `RepairKey` — must be migrated to `BreachSignature` keys.
2. Spec `specs/S137-plan-causal-links-and-repair.md` D7 specifies the new shape including `kind: RepairKind`, `succeeded: bool`, preserved `success_count: u32` field for FND-22A compatibility. `BreachSignature` landed in `archive/tickets/S137PLACAULIN-001.md`. New `RepairKind` variant set landed in `archive/tickets/S137PLACAULIN-003.md`.
3. Shared boundary: the `RepairMemory` component state surface. Reads from `crates/worldwake-ai/src/agent_tick/planning.rs` and `crates/worldwake-ai/src/agent_tick/mod.rs` (record-write sites). After migration, `RepairKey` is removed entirely (FND-28) — every site referencing the type updates.
4. **Save-format bump cascade**: `SAVE_FORMAT_VERSION` advances from the then-current S137 baseline after tickets 002/003/004. If those tickets land in numeric order, this ticket advances `82→83`. Pre-`83` `RepairMemory` byte streams cannot deserialize because the BTreeMap key type changes. Per the FND-28 single-truth invariant, no migration logic is written — the bump signals the format change.
5. **Adjacent contradiction classification**: removing `RepairKey` requires updating every `RepairKey::` construction or pattern-match site. Grep `rg "RepairKey" crates/` workspace-wide before implementation to enumerate the blast radius. Per current grep, sites are mostly internal to `repair_memory.rs` plus the few `record()` callers in `agent_tick/`. Classified as a required consequence per Divergence Protocol — not deferred.

## Architecture Check

1. **Single-truth migration per FND-28**: `RepairKey` is removed; `BreachSignature` is the sole key for repair memory entries. No alias type, no parallel collection.
2. **FND-22A-aligned discriminated outcome**: `RepairEntry { kind, succeeded, observed_tick, expires_tick, success_count }` preserves the existing `success_count` aggregate for habit-strength semantics while adding per-attempt discriminated outcome. The repair search reads `succeeded == false` to skip recently-failed `RepairKind` variants for the same breach.

## Verification Layers

1. New shape + key migration → focused unit tests in `repair_memory.rs` `#[cfg(test)]` (bincode roundtrip with `BreachSignature`-keyed entries; `record`, `expire`, `enforce_capacity` against new shape).
2. Save-load version bump → focused unit test in `save_load.rs` asserting the next current `SAVE_FORMAT_VERSION` (expected `83` if tickets 002-004 landed in numeric order) and that prior-version `RepairMemory` payloads fail with `UnsupportedVersion`.
3. Cross-crate blast radius → workspace-build assertion (`cargo build --workspace`) confirms all `RepairKey` consumers updated.

## What to Change

### 1. Replace `RepairKey` with `BreachSignature` in `RepairMemory`

In `crates/worldwake-core/src/repair_memory.rs`, remove lines 5-9 (`RepairKey` struct). Update `RepairEntry` (lines 11-17) to:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairEntry {
    pub signature: BreachSignature,
    pub kind: RepairKind,
    pub succeeded: bool,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub success_count: u32,
}
```

Update `RepairMemory` (lines 19-22) to:

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemory {
    pub repairs: BTreeMap<BreachSignature, RepairEntry>,
}
```

### 2. Update `record`, `expire`, `enforce_capacity` methods

The method signatures remain semantically the same — `record(&mut self, entry: RepairEntry)` inserts by `entry.signature`; `expire(&mut self, current_tick: Tick)` retains by `expires_tick > current_tick`; `enforce_capacity(&mut self, profile: &MemoryCapacityProfile)` evicts oldest by `observed_tick`. Update internal field accesses from `entry.repair_key` to `entry.signature`.

### 3. Update existing tests at lines 55-202

The `repair_key` helper (lines 72-77), the `repair_entry` helper (lines 79-86), and all individual test bodies must construct `BreachSignature` instead of `RepairKey`. Use a `breach_signature` helper:

```rust
fn breach_signature(slot: u32) -> BreachSignature {
    BreachSignature {
        goal_key: sample_goal_key(),
        invalidator: InvalidatorTag::TargetMoved,
        step_target: Some(entity_id(slot, 0)),
    }
}
```

### 4. Remove `RepairKey` from lib re-exports

In `crates/worldwake-core/src/lib.rs`, remove the `RepairKey` re-export. `BreachSignature` is already re-exported by `archive/tickets/S137PLACAULIN-001.md`.

### 5. Update cross-crate callers

Grep `rg "RepairKey" crates/` and enumerate every site. Each site that constructed a `RepairKey { goal_key, alternate_target }` to record a repair must now construct a `BreachSignature { goal_key, invalidator, step_target }`. The mapping from legacy `alternate_target: EntityId` to the new `step_target: Option<EntityId>` + `invalidator: InvalidatorTag` requires reading the calling context — most call sites have an in-scope invalidator (the breach reason) or can default to `InvalidatorTag::TargetMoved` if the original `alternate_target` represented an anchor change.

### 6. SAVE_FORMAT_VERSION bump

In `crates/worldwake-sim/src/save_load.rs:6`, bump `SAVE_FORMAT_VERSION` from the then-current S137 baseline to the next value. If tickets 002-004 landed in numeric order, this is expected to be `82→83`. Update load-current-format match.

## Files to Touch

- `crates/worldwake-core/src/repair_memory.rs` (modify — types + methods + tests)
- `crates/worldwake-core/src/lib.rs` (modify — remove `RepairKey` re-export)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump from the then-current S137 baseline; expected 82→83 after tickets 002-004)
- Likely: `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `record` callers using legacy `RepairKey`; grep `RepairKey::` to confirm site list)
- Likely: `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — `record` callers; grep `RepairKey::` to confirm)

## Out of Scope

- The repair search reading `repairs.get(&signature)` to skip failed kinds — ticket 006.
- New `RepairKind` variant set used in `RepairEntry.kind` — already landed in `archive/tickets/S137PLACAULIN-003.md`.
- `BreachSignature` definition — already landed in `archive/tickets/S137PLACAULIN-001.md`.
- Migration logic for prior-version byte streams — none written (FND-28: no backward compatibility); the bump rejects legacy streams.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core repair_memory` — all existing tests pass with `BreachSignature` keys.
2. `cargo test -p worldwake-sim save_load` — `SAVE_FORMAT_VERSION` has the next current value; legacy fixture rejection.
3. `cargo test --workspace` — workspace builds; all `RepairKey` references removed.
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `RepairKey` does not exist anywhere in `crates/` after this ticket — `rg "RepairKey" crates/` returns 0 matches (excluding archived files).
2. `RepairMemory.repairs` is keyed by `BreachSignature`.
3. `SAVE_FORMAT_VERSION` has the next current value; prior-version `RepairMemory` byte streams fail with `UnsupportedVersion`.
4. `RepairEntry.success_count` is preserved for FND-22A habit-strength semantics.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/repair_memory.rs` `#[cfg(test)]` — all 6 existing tests migrated to `BreachSignature`-keyed construction; new test `repair_entry_carries_kind_and_succeeded` asserting the new fields roundtrip through bincode.
2. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — new test named for the actual current version after repair memory migration, expected `save_format_version_is_83_after_repair_memory_migration` if tickets 002-004 landed in numeric order.

### Commands

1. `cargo test -p worldwake-core repair_memory`
2. `cargo test -p worldwake-sim save_load`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
