# S170LEASTAPRO-002: LearnedOpportunitySource enum + OpportunityEntry migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — agent decision runtime (LearnedOpportunityMemory), save/load
**Deps**: None

## Problem

Before this ticket, `LearnedOpportunityMemory::OpportunityEntry` (`crates/worldwake-core/src/learned_opportunity_memory.rs`) recorded `opportunity`, `observed_tick`, `expires_tick`, and `observed_at` but stored no causal source. The only runtime call site (`record_learned_opportunities_from_read_phase` in `crates/worldwake-ai/src/agent_tick/mod.rs`) is a read-phase candidate-generation inference that synthesizes opportunities from belief state — no discrete triggering event exists. Without a typed sentinel, audits could not distinguish "no event recorded" from "no event possible," and FND-22A's accountable-origin requirement failed.

## Assumption Reassessment (2026-05-25)

1. `OpportunityEntry` in `crates/worldwake-core/src/learned_opportunity_memory.rs` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. The new enum field type (`LearnedOpportunitySource`) must satisfy these. `EventId` is `Copy` and resides in `worldwake-core`, so all derives carry. The bounds test `learned_opportunity_memory_types_satisfy_required_bounds` verifies this contract and remains a passing test after the migration.
2. Spec under audit: `specs/S170-learned-state-provenance-hardening.md` D1. The only runtime call site is `record_learned_opportunities_from_read_phase` in `crates/worldwake-ai/src/agent_tick/mod.rs`. It iterates `generated_keys` from candidate generation with no `event_log` parameter and no per-opportunity event id in scope. The new `LearnedOpportunitySource::ReadPhaseInference` sentinel is the FND-3-honest attribution (model the actual mechanism — read-phase synthesis from belief state — rather than fabricating an event reference).
3. The shared boundary under audit is `OpportunityEntry`'s public field set — adding a new required field on a `Copy` struct used in 11 construction sites across the workspace.
4. Construction sites for `OpportunityEntry { ... }`: 11 total. Runtime: 1 (`crates/worldwake-ai/src/agent_tick/mod.rs`). Tests: `crates/worldwake-core/src/learned_opportunity_memory.rs` helper `opportunity_entry`, inline `record_overwrites_existing_entry_when_revisit_is_fresher` literals, `crates/worldwake-core/src/test_utils.rs`, `crates/worldwake-ai/src/opportunity_compiler/compile.rs`, `crates/worldwake-ai/src/ranking.rs`, and `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs`. No spread-syntax usage (`..OpportunityEntry::default()` — none); no `Default` impl on `OpportunityEntry`. The count is small enough that effort stays Small even without escape hatches.
5. Existing focused tests touching the type in `crates/worldwake-core/src/learned_opportunity_memory.rs`: `learned_opportunity_memory_types_satisfy_required_bounds`, `opportunity_entry_roundtrips_through_bincode`, `learned_opportunity_memory_roundtrips_through_bincode`, `record_overwrites_existing_entry_when_revisit_is_fresher`, `expire_prunes_stale_entries`, `enforce_capacity_evicts_oldest_entries`. All construct `OpportunityEntry` literals and need the new field populated.
6. Save/load: `SAVE_FORMAT_VERSION` was `101` in `crates/worldwake-sim/src/save_load.rs`. Adding a required field to a serialized component schema requires a bump (no `#[serde(default)]` per FND-28). This ticket increments by 1 as part of the cascade with tickets 003 and 004 (see Merge note).
7. Reassessment classification: the adjacent contradiction (read-phase site lacking a triggering event) is a required consequence of the spec's design choice (FND-3-honest sentinel rather than fabricated provenance), not a separate bug. No follow-up ticket needed.

## Architecture Check

1. Domain-specific sentinel naming (`ReadPhaseInference`) instead of a generic `None` or an abstract shared `ProvenanceSource` — per FND-3, models the actual mechanism rather than an opaque absence. Parallel structure to ticket 003's `DiscrepancySource::ReadPhaseInference` but each enum is its own type with its own semantic scope (the third-iteration report's "abstract learning sludge" warning rules out a shared abstract type).
2. No backward-compatibility shim. No `#[serde(default)]`, no `#[serde(alias)]`. Pre-bump saves fail to load — per FND-28's prohibition on backward-compat in live authority paths.

## Verified Layers

1. Accountable origin (FND-22A) → focused unit coverage (round-trip tests assert both `LearnedOpportunitySource::Event(EventId)` and `LearnedOpportunitySource::ReadPhaseInference` variants).
2. Read-phase site truthfully writes sentinel (FND-3, FND-29A) → focused runtime coverage (assert `OpportunityEntry.source == LearnedOpportunitySource::ReadPhaseInference` after `record_learned_opportunities_from_read_phase` invocation).
3. Save/load equivalence (FND-12) → save/load round-trip test for `LearnedOpportunityMemory` with populated `source` field; bincode round-trip preserves the variant exactly.

## Landed Changes

### 1. Define LearnedOpportunitySource enum

In `crates/worldwake-core/src/learned_opportunity_memory.rs`, this ticket defined alongside `OpportunityEntry`:

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

Added `EventId` to the module's `use` statement: `use crate::{Component, EventId, MemoryCapacityProfile, OpportunityKey, Tick};`.

### 2. Extend OpportunityEntry

In `learned_opportunity_memory.rs`, added `pub source: LearnedOpportunitySource,` as a new field after `observed_at`. Existing fields are unchanged.

### 3. Populate runtime call site

In `crates/worldwake-ai/src/agent_tick/mod.rs`, the `OpportunityEntry { … }` construction writes:

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

- `crates/worldwake-core/src/learned_opportunity_memory.rs` test helper `opportunity_entry`: supplied `source: LearnedOpportunitySource::ReadPhaseInference` as the default test variant.
- `crates/worldwake-core/src/learned_opportunity_memory.rs` inline test `record_overwrites_existing_entry_when_revisit_is_fresher`: both `stale` and `fresh` literals add the field.
- `crates/worldwake-core/src/test_utils.rs`: added the field.
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs`: added the field.
- `crates/worldwake-ai/src/ranking.rs`: added the field at both test sites.
- `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs`: added the field.

### 5. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`, incremented `SAVE_FORMAT_VERSION` from 101 to 102. Tickets 003 and 004 own later version bumps if their persisted-shape migrations land.

### 6. Add focused round-trip tests

In `crates/worldwake-core/src/learned_opportunity_memory.rs` test module, added explicit-variant round-trip coverage:

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

In `crates/worldwake-ai/src/agent_tick/tests.rs`, extended the existing `in_transit_read_phase_records_learned_opportunity_memory_entry` runtime test to assert the recorded entry has `source == LearnedOpportunitySource::ReadPhaseInference`.

## Landed Files

- `crates/worldwake-core/src/learned_opportunity_memory.rs` (modify — new enum, field addition, test updates, new round-trip tests)
- `crates/worldwake-core/src/test_utils.rs` (modify — opportunity helper at 264)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — runtime call site at 2582 + new focused test)
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — test at 618)
- `crates/worldwake-ai/src/ranking.rs` (modify — tests at 5899, 7881)
- `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` (modify — test at 300)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION`)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `LearnedOpportunitySource`)

## Out of Scope

- `RoutePreference::record_safe` changes (ticket 001)
- `DiscrepancySource` enum or `DiscrepancyEntry` migration (ticket 003)
- `BlockerSource` enum or `Blocker` migration (ticket 004)
- Restructuring read-phase candidate generation to emit per-opportunity events (per spec Q2 resolution, the sentinel approach is the FND-honest choice — restructure rejected as fabricating provenance)
- Attempting to attribute opportunities to an upstream perception event (per spec Q2 resolution, the learning depends on the full belief state, not one event — attribution would be dishonest)

## Acceptance Result

### Tests Passed

1. Added: `opportunity_entry_with_event_source_roundtrips`.
2. Added: `opportunity_entry_with_read_phase_inference_source_roundtrips`.
3. Extended: `in_transit_read_phase_records_learned_opportunity_memory_entry` asserts `record_learned_opportunities_from_read_phase` writes `source: LearnedOpportunitySource::ReadPhaseInference`.
4. Updated focused core tests continue to pass with the new field populated by the helper.
5. Existing suites passed: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`.

### Invariants

1. Every `OpportunityEntry` written by runtime or test code carries an explicit `source` variant — no field omission is possible (compile error).
2. `LearnedOpportunityMemory` round-trips deterministically with bincode (replay equivalence preserved).
3. The read-phase runtime call site writes `ReadPhaseInference`; no fabricated event id is introduced.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/learned_opportunity_memory.rs` — added `opportunity_entry_with_event_source_roundtrips`, `opportunity_entry_with_read_phase_inference_source_roundtrips`.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — extended a focused test invoking `record_learned_opportunities_from_read_phase` to assert `source == LearnedOpportunitySource::ReadPhaseInference`.

### Commands Run

1. Passed `cargo test -p worldwake-core learned_opportunity_memory`.
2. Passed `cargo test -p worldwake-ai in_transit_read_phase_records_learned_opportunity_memory_entry`.
3. Passed `cargo test -p worldwake-ai`.
4. Passed `cargo test -p worldwake-sim save_load`.
5. Passed `cargo test -p worldwake-core`.
6. Waived `./scripts/verify.sh` for this per-ticket iteration because the `implement-spec-tickets` harness final branch phase owns the full pre-PR verification gate before push.

Merge note: Ticket 002 bumped `SAVE_FORMAT_VERSION` from 101 to 102. Tickets 003 and 004 own the next persisted-shape bumps in the cascade.

## Outcome

Completed on 2026-05-25.

- Added `LearnedOpportunitySource` with `Event(EventId)` and `ReadPhaseInference` variants, plus the required `OpportunityEntry.source` field.
- Re-exported `LearnedOpportunitySource` and updated every live `OpportunityEntry` literal to choose an explicit source.
- Recorded read-phase learned opportunities with `LearnedOpportunitySource::ReadPhaseInference`, preserving the honest "no single event" attribution instead of fabricating provenance.
- Bumped `SAVE_FORMAT_VERSION` from 101 to 102 for the required serialized component-shape change.

## Verification Result

- Passed `cargo test -p worldwake-core learned_opportunity_memory`.
- Passed `cargo test -p worldwake-ai in_transit_read_phase_records_learned_opportunity_memory_entry`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test -p worldwake-sim save_load`.
- Passed `cargo test -p worldwake-core`.
- Waived `./scripts/verify.sh` for this per-ticket iteration because the `implement-spec-tickets` harness final branch phase owns the full pre-PR verification gate before push.
