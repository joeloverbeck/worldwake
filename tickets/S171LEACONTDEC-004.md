# S171LEACONTDEC-004: Land lawful source-reliability discount provenance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core` source-reliability carrier shape, source-reliability update sites, `worldwake-ai` ranking discount attribution, save-format versioning, and focused tests
**Deps**: `archive/tickets/S171LEACONTDEC-001.md`; `archive/tickets/S171LEACONTDEC-002.md`

## Problem

`archive/tickets/S171LEACONTDEC-001.md` added `SourceReliabilityDiscount.provenance_event_count` and `most_recent_provenance_event`, and `archive/tickets/S171LEACONTDEC-002.md` reassessment proved the drafted source for those fields was wrong. The live ranking path reads `SourceReliability.sources: BTreeMap<SourceKey, ReliabilityRecord>` in `crates/worldwake-ai/src/ranking.rs::apply_source_reliability_discount`; the matched `ReliabilityRecord` in `crates/worldwake-core/src/experience.rs` has no event provenance ring. `TestimonyReliabilityEntry::provenance_events` is a separate testimony-trust carrier and is not consulted by the source-reliability discount path.

This leaves the two source-reliability provenance fields permanently `0` / `None` unless a lawful source-reliability producer is added. S171 cannot honestly claim all learned-state ranking adjustments are consumption-traceable until the discount axis reads provenance from the same source-reliability entry that produced the discount.

## Assumption Reassessment (2026-05-25)

1. Live source-reliability discounting is driven by `SourceReliability.sources.get(&SourceKey { entity, commodity })?` in `crates/worldwake-ai/src/ranking.rs::apply_source_reliability_discount`, which returns `ReliabilityRecord`, not `TestimonyReliabilityEntry`.
2. `ReliabilityRecord` currently stores `successful_acquisitions`, `failed_attempts`, `last_attempt_tick`, wait observations, and capacity observations. It has no `EventId` field or provenance ring.
3. Source-reliability mutation sites include `crates/worldwake-systems/src/experience_recording.rs::{record_successful_source_acquisition, record_failed_source_attempt}` and `crates/worldwake-ai/src/agent_tick/mod.rs` pending-failure handling. Reassessment must verify whether each site can lawfully name the committed event id at mutation time or needs a different concrete provenance carrier.
4. Shared abstraction boundary under audit: `ReliabilityRecord` as the authoritative learned source-reliability entry, and `SourceReliabilityDiscount` as the derived decision-trace attribution for one ranking-time read of that entry.
5. FND-22A/FND-29 require the discount attribution to identify the actual source-reliability learning path. Synthesizing `EventId`s from testimony reliability or unrelated records would be a false provenance path.

## Architecture Check

1. The clean design must make source-reliability provenance part of the source-reliability carrier itself, or truthfully change the trace field contract to the concrete provenance the carrier can support. It must not join through testimony reliability just because that type already has event provenance.
2. No backwards-compatibility shim: any source-reliability carrier or save-shape change advances the current save format directly.

## Verification Layers

1. Source-reliability mutation provenance -> focused tests at each live mutation site or the smallest shared helper that all mutation sites use.
2. Ranking-time discount attribution -> focused `ranking.rs` test proving `SourceReliabilityDiscount.provenance_event_count` / `most_recent_provenance_event` derive from the matched `ReliabilityRecord`.
3. Save-shape change -> `worldwake-sim` save-format version and non-default roundtrip tests if `ReliabilityRecord` changes.

## What to Change

### 1. Reassess the lawful source-reliability provenance carrier

Decide whether `ReliabilityRecord` can store a ring of event ids, or whether the live mutation timing requires a different concrete provenance shape. Record the selected boundary in this ticket before coding.

### 2. Populate `SourceReliabilityDiscount` from the matched carrier

Replace the placeholder `0` / `None` values in the production discount path with reads from the matched source-reliability entry. Keep pending-failure projection truthful: if a projected in-tick failure has no committed event id yet, the trace must say so through a concrete field or leave the provenance fields empty without pretending an event exists.

### 3. Truth-sync S171 terminal wording

Update `specs/S171-learned-context-decision-trace-edge.md` and `tickets/S171LEACONTDEC-003.md` if the final source-reliability provenance shape differs from the S171 draft’s `TestimonyReliabilityEntry::provenance_events` wording.

## Files to Touch

- `crates/worldwake-core/src/experience.rs` (modify, if `ReliabilityRecord` gains provenance)
- `crates/worldwake-systems/src/experience_recording.rs` (modify, if action-result source reliability records event provenance)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify, if pending source-failure handling records or projects provenance)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify, if save shape changes)
- `specs/S171-learned-context-decision-trace-edge.md` (modify)
- `tickets/S171LEACONTDEC-003.md` (modify if dependency/provenance wording changes)

## Out of Scope

- Learned-opportunity and repair-memory bonus attribution; owned by `archive/tickets/S171LEACONTDEC-002.md`.
- Decision-trace text formatting; owned by `tickets/S171LEACONTDEC-003.md`.
- Changing source-reliability discount arithmetic or ranking order.

## Acceptance Criteria

### Tests That Must Pass

1. Focused source-reliability mutation/provenance test(s) for the selected carrier.
2. Focused `ranking.rs` test proving source-reliability discount provenance comes from the matched source-reliability entry.
3. Save-format tests if the carrier shape changes.
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. `SourceReliabilityDiscount` provenance fields are never populated from unrelated testimony reliability records.
2. The discount arithmetic and candidate ordering remain unchanged.
3. If no lawful source-reliability event provenance exists for a projected discount, the trace remains explicitly empty rather than synthetic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` focused test for discount provenance population from the selected source-reliability carrier.
2. `crates/worldwake-core` / `worldwake-systems` / `worldwake-ai` focused tests as required by the selected mutation carrier.
3. `worldwake-sim` save roundtrip/version tests if the source-reliability carrier shape changes.

### Commands

1. Focused command for the selected source-reliability mutation test.
2. `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`
