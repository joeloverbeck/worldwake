# S170LEASTAPRO-002: LearnedOpportunitySource enum + OpportunityEntry migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — agent decision runtime (LearnedOpportunityMemory), save/load
**Deps**: None

## Problem

`LearnedOpportunityMemory::OpportunityEntry` (`crates/worldwake-core/src/learned_opportunity_memory.rs:5-11`) records `opportunity`, `observed_tick`, `expires_tick`, and `observed_at` but stores no causal source. The only runtime call site (`record_learned_opportunities_from_read_phase` in `crates/worldwake-ai/src/agent_tick/mod.rs:2558-2589`) is a read-phase candidate-generation inference that synthesizes opportunities from belief state — no discrete triggering event exists. Without a typed sentinel, audits cannot distinguish "no event recorded" from "no event possible," and FND-22A's accountable-origin requirement fails.

## Assumption Reassessment (2026-05-25)

1. `OpportunityEntry` at `crates/worldwake-core/src/learned_opportunity_memory.rs:5-11` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. The new enum field type (`LearnedOpportunitySource`) must satisfy these. `EventId` is `Copy` and resides in `worldwake-core`, so all derives carry. The bounds test `learned_opportunity_memory_types_satisfy_required_bounds:87` verifies this contract and remains a passing test after the migration.
2. Spec under audit: `specs/S170-learned-state-provenance-hardening.md` D1. The only runtime call site is `record_learned_opportunities_from_read_phase` at `crates/worldwake-ai/src/agent_tick/mod.rs:2558-2589`. It iterates `generated_keys` from candidate generation with no `event_log` parameter and no per-opportunity event id in scope. The new `LearnedOpportunitySource::ReadPhaseInference` sentinel is the FND-3-honest attribution (model the actual mechanism — read-phase synthesis from belief state — rather than fabricating an event reference).
3. The shared boundary under audit is `OpportunityEntry`'s public field set — adding a new required field on a `Copy` struct used in 11 construction sites across the workspace.
4. Construction sites for `OpportunityEntry { ... }`: 11 total. Runtime: 1 (`crates/worldwake-ai/src/agent_tick/mod.rs:2582`). Tests: `crates/worldwake-core/src/learned_opportunity_memory.rs:78` (helper `opportunity_entry`), inline lines 117-128 (in `record_overwrites_existing_entry_when_revisit_is_fresher`); `crates/worldwake-core/src/test_utils.rs:264`; `crates/worldwake-ai/src/opportunity_compiler/compile.rs:618`; `crates/worldwake-ai/src/ranking.rs:5899, 7881`; `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs:300`. No spread-syntax usage (`..OpportunityEntry::default()` — none); no `Default` impl on `OpportunityEntry`. The count is small enough that effort stays Small even without escape hatches.
5. Existing focused tests touching the type in `crates/worldwake-core/src/learned_opportunity_memory.rs`: `learned_opportunity_memory_types_satisfy_required_bounds:87`, `opportunity_entry_roundtrips_through_bincode:93`, `learned_opportunity_memory_roundtrips_through_bincode:104`, `record_overwrites_existing_entry_when_revisit_is_fresher:115`, `expire_prunes_stale_entries:138`, `enforce_capacity_evicts_oldest_entries:151`. All construct `OpportunityEntry` literals and need the new field populated.
6. Save/load: `SAVE_FORMAT_VERSION` is currently `101` at `crates/worldwake-sim/src/save_load.rs:7`. Adding a required field to a serialized component schema requires a bump (no `#[serde(default)]` per FND-28). This ticket increments by 1 as part of the cascade with tickets 003 and 004 (see Merge note).
7. Reassessment classification: the adjacent contradiction (read-phase site lacking a triggering event) is a required consequence of the spec's design choice (FND-3-honest sentinel rather than fabricated provenance), not a separate bug. No follow-up ticket needed.

## Architecture Check

1. Domain-specific sentinel naming (`ReadPhaseInference`) instead of a generic `None` or an abstract shared `ProvenanceSource` — per FND-3, models the actual mechanism rather than an opaque absence. Parallel structure to ticket 003's `DiscrepancySource::ReadPhaseInference` but each enum is its own type with its own semantic scope (the third-iteration report's "abstract learning sludge" warning rules out a shared abstract type).
2. No backward-compatibility shim. No `#[serde(default)]`, no `#[serde(alias)]`. Pre-bump saves fail to load — per FND-28's prohibition on backward-compat in live authority paths.

## Verification Layers

1. Accountable origin (FND-22A) → focused unit coverage (round-trip tests assert both `LearnedOpportunitySource::Event(EventId)` and `LearnedOpportunitySource::ReadPhaseInference` variants).
2. Read-phase site truthfully writes sentinel (FND-3, FND-29A) → focused runtime coverage (assert `OpportunityEntry.source == LearnedOpportunitySource::ReadPhaseInference` after `record_learned_opportunities_from_read_phase` invocation).
3. Save/load equivalence (FND-12) → save/load round-trip test for `LearnedOpportunityMemory` with populated `source` field; bincode round-trip preserves the variant exactly.

## What to Change

### 1. Define LearnedOpportunitySource enum

In `crates/worldwake-core/src/learned_opportunity_memory.rs`, define alongside `OpportunityEntry`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LearnedOpportunitySource {
    /// The learning update is attributable to a specific world event
    /// (perception, travel completion, transaction, etc.).
    Event(EventId),
    /// The learning update emerged from the agent's read-phase
    /// candidate-generation pass over its current belief state; no
    /// single discrete event produced it.
    ReadPhaseInference,
}
```

Add `EventId` to the module's `use` statement: `use crate::{Component, EventId, MemoryCapacityProfile, OpportunityKey, Tick};`.

### 2. Extend OpportunityEntry

In `learned_opportunity_memory.rs:5-11`, add `pub source: LearnedOpportunitySource,` as a new field after `observed_at`. Existing fields are unchanged.

### 3. Populate runtime call site

In `crates/worldwake-ai/src/agent_tick/mod.rs:2582`, the `OpportunityEntry { … }` construction writes:

```rust
learned_opportunity_memory.record(OpportunityEntry {
    opportunity: *opportunity,
    observed_tick: current_tick,
    expires_tick: Tick(current_tick.0 + u64::from(ttl_ticks)),
    observed_at: in_transit.destination,
    source: LearnedOpportunitySource::ReadPhaseInference,
});
```

The read-phase candidate-generation pass has no per-opportunity event id in scope; the sentinel is the FND-3-honest attribution.

### 4. Update all other construction sites

- `crates/worldwake-core/src/learned_opportunity_memory.rs:78` (test helper `opportunity_entry`): supply `source: LearnedOpportunitySource::ReadPhaseInference` as the default test variant.
- `crates/worldwake-core/src/learned_opportunity_memory.rs:117-128` (inline test `record_overwrites_existing_entry_when_revisit_is_fresher`): both `stale` and `fresh` literals add the new field.
- `crates/worldwake-core/src/test_utils.rs:264`: add the field.
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs:618`: add the field.
- `crates/worldwake-ai/src/ranking.rs:5899, 7881`: add the field at both test sites.
- `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs:300`: add the field.

### 5. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs:7`, increment `SAVE_FORMAT_VERSION` by 1. The cascade across tickets 002/003/004 determines exact target values — the current value at merge time + 1 (see Merge note).

### 6. Add focused round-trip tests

In `crates/worldwake-core/src/learned_opportunity_memory.rs` test module, add explicit-variant round-trip coverage:

```rust
#[test]
fn opportunity_entry_with_event_source_roundtrips() {
    let entry = OpportunityEntry {
        opportunity: opportunity_key(4),
        observed_tick: Tick(12),
        expires_tick: Tick(32),
        observed_at: entity_id(14, 0),
        source: LearnedOpportunitySource::Event(EventId(42)),
    };
    let bytes = bincode::serialize(&entry).unwrap();
    let roundtrip: OpportunityEntry = bincode::deserialize(&bytes).unwrap();
    assert_eq!(roundtrip, entry);
}

#[test]
fn opportunity_entry_with_read_phase_inference_source_roundtrips() {
    let entry = OpportunityEntry {
        opportunity: opportunity_key(4),
        observed_tick: Tick(12),
        expires_tick: Tick(32),
        observed_at: entity_id(14, 0),
        source: LearnedOpportunitySource::ReadPhaseInference,
    };
    let bytes = bincode::serialize(&entry).unwrap();
    let roundtrip: OpportunityEntry = bincode::deserialize(&bytes).unwrap();
    assert_eq!(roundtrip, entry);
}
```

### 7. Add focused runtime test

In `crates/worldwake-ai/src/agent_tick/mod.rs` test module (or a sibling integration target), add a test that invokes `record_learned_opportunities_from_read_phase` and asserts the recorded entry has `source == LearnedOpportunitySource::ReadPhaseInference`.

## Files to Touch

- `crates/worldwake-core/src/learned_opportunity_memory.rs` (modify — new enum, field addition, test updates, new round-trip tests)
- `crates/worldwake-core/src/test_utils.rs` (modify — opportunity helper at 264)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — runtime call site at 2582 + new focused test)
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — test at 618)
- `crates/worldwake-ai/src/ranking.rs` (modify — tests at 5899, 7881)
- `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` (modify — test at 300)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION at line 7)

## Out of Scope

- `RoutePreference::record_safe` changes (ticket 001)
- `DiscrepancySource` enum or `DiscrepancyEntry` migration (ticket 003)
- `BlockerSource` enum or `Blocker` migration (ticket 004)
- Restructuring read-phase candidate generation to emit per-opportunity events (per spec Q2 resolution, the sentinel approach is the FND-honest choice — restructure rejected as fabricating provenance)
- Attempting to attribute opportunities to an upstream perception event (per spec Q2 resolution, the learning depends on the full belief state, not one event — attribution would be dishonest)

## Acceptance Criteria

### Tests That Must Pass

1. New: `opportunity_entry_with_event_source_roundtrips` (per "What to Change" #6).
2. New: `opportunity_entry_with_read_phase_inference_source_roundtrips` (per #6).
3. New: focused runtime test asserting `record_learned_opportunities_from_read_phase` writes `source: LearnedOpportunitySource::ReadPhaseInference`.
4. Updated: `learned_opportunity_memory_types_satisfy_required_bounds`, `opportunity_entry_roundtrips_through_bincode`, `learned_opportunity_memory_roundtrips_through_bincode`, `record_overwrites_existing_entry_when_revisit_is_fresher`, `expire_prunes_stale_entries`, `enforce_capacity_evicts_oldest_entries` all continue to pass with the new field populated by the helper.
5. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`.

### Invariants

1. Every `OpportunityEntry` written by runtime or test code carries an explicit `source` variant — no field omission is possible (compile error).
2. `LearnedOpportunityMemory` round-trips deterministically with bincode (replay equivalence preserved).
3. The read-phase runtime call site writes `ReadPhaseInference`; no fabricated event id is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/learned_opportunity_memory.rs` — add `opportunity_entry_with_event_source_roundtrips`, `opportunity_entry_with_read_phase_inference_source_roundtrips`.
2. `crates/worldwake-ai/src/agent_tick/mod.rs` test module — add a focused test invoking `record_learned_opportunities_from_read_phase` and asserting `source == LearnedOpportunitySource::ReadPhaseInference`.

### Commands

1. `cargo test -p worldwake-core learned_opportunity_memory`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`

Merge note: Ticket 002 bumps `SAVE_FORMAT_VERSION` by 1 as part of the cascade with tickets 003 and 004 — landing order determines exact target values (e.g., 101→102→103→104 if 002 lands first; see the Merge-Order Constraints note in the spec-to-tickets Step 6 summary).
