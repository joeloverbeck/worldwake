# S137PLACAULIN-005: RepairMemory shape migration to BreachSignature

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `RepairMemory` shape, save-load, removal of `RepairKey`
**Deps**: archive/tickets/S137PLACAULIN-001.md (BreachSignature), archive/tickets/S137PLACAULIN-003.md (new RepairKind variants populated in RepairEntry), archive/tickets/S137PLACAULIN-004.md (completed save-format baseline at `82`)

## Problem

S137 D7 migrates `RepairMemory.repairs` from `BTreeMap<RepairKey, RepairEntry>` to `BTreeMap<BreachSignature, RepairEntry>` and reshapes `RepairEntry` to discriminate success/failure per repair attempt. The migration is single-truth per FND-28 — `RepairKey` is removed, no parallel `successful_kinds`/`failed_kinds` maps coexist with `repairs`. Ticket 006's `plan_repair` module reads `repairs.get(&signature)` to skip recently-failed `RepairKind` variants for the same breach.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RepairMemory` is defined at `crates/worldwake-core/src/repair_memory.rs:19-22` as `BTreeMap<RepairKey, RepairEntry>` with `success_count: u32` on `RepairEntry`. `RepairKey { goal_key, alternate_target: EntityId }` at lines 5-9; `RepairEntry { repair_key, observed_tick, expires_tick, success_count }` at lines 11-17. Component registration at `crates/worldwake-core/src/component_schema.rs:784-802`. Existing `#[cfg(test)]` tests at lines 55-202 cover bincode roundtrip, `record`, `expire`, `enforce_capacity`. Three test fixtures use `RepairKey` — must be migrated to `BreachSignature` keys.
2. Spec `archive/specs/S137-plan-causal-links-and-repair.md` D7 specifies the new shape including `kind: RepairKind`, `succeeded: bool`, preserved `success_count: u32` field for FND-22A compatibility. `BreachSignature` landed in `archive/tickets/S137PLACAULIN-001.md`. New `RepairKind` variant set landed in `archive/tickets/S137PLACAULIN-003.md`.
3. Shared boundary: the `RepairMemory` component state surface. Live readers/writers were in `crates/worldwake-ai/src/agent_tick/mod.rs` and `crates/worldwake-ai/src/ranking.rs`; `planning.rs` was only a cited adjacent planning surface and needed no source edit. After migration, `RepairKey` is removed entirely (FND-28) — every site referencing the type updates.
4. **Save-format bump cascade**: `SAVE_FORMAT_VERSION` advances from the then-current S137 baseline after tickets 002/003/004. If those tickets land in numeric order, this ticket advances `82→83`. Pre-`83` `RepairMemory` byte streams cannot deserialize because the BTreeMap key type changes. Per the FND-28 single-truth invariant, no migration logic is written — the bump signals the format change.
5. **Adjacent contradiction classification**: removing `RepairKey` requires updating every `RepairKey::` construction or pattern-match site. Grep `rg "RepairKey" crates/` workspace-wide before implementation to enumerate the blast radius. Per current grep, sites are mostly internal to `repair_memory.rs` plus the few `record()` callers in `agent_tick/`. Classified as a required consequence per Divergence Protocol — not deferred.

## Architecture Check

1. **Single-truth migration per FND-28**: `RepairKey` is removed; `BreachSignature` is the sole key for repair memory entries. No alias type, no parallel collection.
2. **FND-22A-aligned discriminated outcome**: `RepairEntry { kind, succeeded, observed_tick, expires_tick, success_count }` preserves the existing `success_count` aggregate for habit-strength semantics while adding per-attempt discriminated outcome. The repair search reads `succeeded == false` to skip recently-failed `RepairKind` variants for the same breach.

## Verified Layers

1. New shape + key migration → focused unit tests in `repair_memory.rs` `#[cfg(test)]` (bincode roundtrip with `BreachSignature`-keyed entries; `record`, `expire`, `enforce_capacity` against new shape).
2. Save-load version bump → focused unit test in `save_load.rs` asserting the next current `SAVE_FORMAT_VERSION` (expected `83` if tickets 002-004 landed in numeric order) and that prior-version `RepairMemory` payloads fail with `UnsupportedVersion`.
3. Cross-crate blast radius → workspace compile/test assertions (`cargo test --workspace --no-run` and `cargo test --workspace`) confirm all `RepairKey` consumers were updated.

## Implemented Changes

### 1. Replace `RepairKey` with `BreachSignature` in `RepairMemory`

In `crates/worldwake-core/src/repair_memory.rs`, removed the `RepairKey` struct. Updated `RepairEntry` to:

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

Updated `RepairMemory` to:

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemory {
    pub repairs: BTreeMap<BreachSignature, RepairEntry>,
}
```

### 2. Updated `record`, `expire`, `enforce_capacity` methods

The method signatures remained semantically the same — `record(&mut self, entry: RepairEntry)` inserts by `entry.signature`; `expire(&mut self, current_tick: Tick)` retains by `expires_tick > current_tick`; `enforce_capacity(&mut self, profile: &MemoryCapacityProfile)` evicts oldest by `observed_tick`. Internal field accesses now use `entry.signature`.

### 3. Updated existing tests

The old `repair_key` helper, the `repair_entry` helper, and all individual test bodies now construct `BreachSignature` instead of `RepairKey`. The helper is:

```rust
fn breach_signature(slot: u32) -> BreachSignature {
    BreachSignature {
        goal_key: sample_goal_key(),
        invalidator: InvalidatorTag::TargetMoved,
        step_target: Some(entity_id(slot, 0)),
    }
}
```

### 4. Removed `RepairKey` from lib re-exports

In `crates/worldwake-core/src/lib.rs`, removed the `RepairKey` re-export. `BreachSignature` remains re-exported from ticket 001.

### 5. Updated cross-crate callers

`rg "RepairKey" crates/` enumerated the live blast radius. The legacy target-rebind memory seam now constructs `BreachSignature { goal_key, invalidator: InvalidatorTag::TargetMoved, step_target: Some(alternate_target) }`. The ranking bonus uses the same signature shape for matching successful target-rebind memory.

### 6. Bumped `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, bumped `SAVE_FORMAT_VERSION` from `82` to `83`.

## Files to Touch

- `crates/worldwake-core/src/repair_memory.rs` (modify — types + methods + tests)
- `crates/worldwake-core/src/lib.rs` (modify — remove `RepairKey` re-export)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump from the then-current S137 baseline; expected 82→83 after tickets 002-004)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — completed-plan repair-memory writer)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — repair-memory writer test)
- `crates/worldwake-ai/src/ranking.rs` (modify — repair-memory ranking bonus reader and test)
- `crates/worldwake-core/src/test_utils.rs` (modify — representative component sample)

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

## Test Plan Result

### New/Modified Tests

1. `crates/worldwake-core/src/repair_memory.rs` `#[cfg(test)]` — migrated existing tests to `BreachSignature`-keyed construction; added `repair_entry_carries_kind_and_succeeded` asserting the new fields roundtrip through bincode.
2. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — renamed the current-version test to `save_format_version_is_83_after_repair_memory_migration`.

### Commands

1. Passed `cargo test -p worldwake-core repair_memory`
2. Passed `cargo test -p worldwake-sim save_load`
3. Passed `cargo test --workspace`
4. Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-13.

- Removed `RepairKey` from `worldwake-core`; `RepairMemory.repairs` is now keyed by `BreachSignature`.
- Added `RepairEntry.signature`, `RepairEntry.kind`, and `RepairEntry.succeeded` while preserving `success_count`.
- Updated core fixtures, component samples, AI repair-memory recording, ranking repair-memory lookup, and focused tests to use `BreachSignature { goal_key, invalidator: TargetMoved, step_target }` for the legacy target-rebind memory seam.
- Bumped `SAVE_FORMAT_VERSION` from `82` to `83`; prior-version saves are rejected by the existing current-format gate.
- Truth-synced `archive/specs/S137-plan-causal-links-and-repair.md` to describe the direct `kind`/`succeeded` fields rather than a nested outcome wrapper, and updated ticket 006's dependency line to point at this completed active ticket.

## Deviations

- `crates/worldwake-ai/src/agent_tick/planning.rs` needed no edit; the live `RepairKey` blast radius was in `agent_tick/mod.rs`, `ranking.rs`, core fixtures, and tests.
- The broad workspace proof used `cargo test --workspace`; the earlier `cargo test --workspace --no-run` pass was also run as an implementation fallout check but is not the final acceptance substitute.

## Verification Result

- Passed `cargo test -p worldwake-core repair_memory`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai completed_alternate_plan_records_repair_memory_entry`
- Passed `cargo test -p worldwake-ai repair_memory_boosts_matching_alternative_only_while_live`
- Passed `rg "RepairKey" crates` with zero matches
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
