# S171LEACONTDEC-004: Land lawful source-reliability discount provenance

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core` source-reliability carrier shape, source-reliability update sites, `worldwake-ai` ranking discount attribution, save-format versioning, and focused tests
**Deps**: `archive/tickets/S171LEACONTDEC-001.md`; `archive/tickets/S171LEACONTDEC-002.md`

## Problem

`archive/tickets/S171LEACONTDEC-001.md` added `SourceReliabilityDiscount.provenance_event_count` and `most_recent_provenance_event`, and `archive/tickets/S171LEACONTDEC-002.md` reassessment proved the drafted source for those fields was wrong. The live ranking path reads `SourceReliability.sources: BTreeMap<SourceKey, ReliabilityRecord>` in `crates/worldwake-ai/src/ranking.rs::apply_source_reliability_discount`; the matched `ReliabilityRecord` in `crates/worldwake-core/src/experience.rs` has no event provenance ring. `TestimonyReliabilityEntry::provenance_events` is a separate testimony-trust carrier and is not consulted by the source-reliability discount path.

Before this ticket, the two source-reliability provenance fields stayed permanently `0` / `None` because no lawful source-reliability producer existed. S171 could not honestly claim all learned-state ranking adjustments were consumption-traceable until the discount axis read provenance from the same source-reliability entry that produced the discount.

## Assumption Reassessment (2026-05-25)

1. Live source-reliability discounting is driven by `SourceReliability.sources.get(&SourceKey { entity, commodity })?` in `crates/worldwake-ai/src/ranking.rs::apply_source_reliability_discount`, which returns `ReliabilityRecord`, not `TestimonyReliabilityEntry`.
2. `ReliabilityRecord` stores `successful_acquisitions`, `failed_attempts`, `last_attempt_tick`, wait observations, capacity observations, and after this ticket an 8-entry optional `EventId` provenance ring.
3. Source-reliability mutation sites include `crates/worldwake-systems/src/experience_recording.rs::{record_successful_source_acquisition, record_failed_source_attempt}`, facility queue wait observations, direct-local perception capacity observations, and `crates/worldwake-ai/src/agent_tick/mod.rs` pending-failure handling. Only seams that can lawfully name the event-log event id write a `Some(EventId)`; start-time/projection-only seams keep provenance empty instead of synthesizing an event.
4. Shared abstraction boundary under audit: `ReliabilityRecord` as the authoritative learned source-reliability entry, and `SourceReliabilityDiscount` as the derived decision-trace attribution for one ranking-time read of that entry.
5. FND-22A/FND-29 require the discount attribution to identify the actual source-reliability learning path. Synthesizing `EventId`s from testimony reliability or unrelated records would be a false provenance path.

## Architecture Check

1. The clean design must make source-reliability provenance part of the source-reliability carrier itself, or truthfully change the trace field contract to the concrete provenance the carrier can support. It must not join through testimony reliability just because that type already has event provenance.
2. No backwards-compatibility shim: any source-reliability carrier or save-shape change advances the current save format directly.

## Verified Layers

1. Source-reliability mutation provenance -> focused tests at each live mutation site or the smallest shared helper that all mutation sites use.
2. Ranking-time discount attribution -> focused `ranking.rs` test proving `SourceReliabilityDiscount.provenance_event_count` / `most_recent_provenance_event` derive from the matched `ReliabilityRecord`.
3. Save-shape change -> `worldwake-sim` save-format version and non-default roundtrip tests if `ReliabilityRecord` changes.

## Landed Changes

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

### Required Tests Result

1. Focused source-reliability mutation/provenance test(s) for the selected carrier.
2. Focused `ranking.rs` test proving source-reliability discount provenance comes from the matched source-reliability entry.
3. Save-format tests if the carrier shape changes.
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. `SourceReliabilityDiscount` provenance fields are never populated from unrelated testimony reliability records.
2. The discount arithmetic and candidate ordering remain unchanged.
3. If no lawful source-reliability event provenance exists for a projected discount, the trace remains explicitly empty rather than synthetic.

## Test Plan Result

### Covered Tests

1. `crates/worldwake-ai/src/ranking.rs` focused test for discount provenance population from the selected source-reliability carrier.
2. `crates/worldwake-core` / `worldwake-systems` / `worldwake-ai` focused tests as required by the selected mutation carrier.
3. `worldwake-sim` save roundtrip/version tests if the source-reliability carrier shape changes.

### Planned Commands Result

1. Focused command for the selected source-reliability mutation test.
2. `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount`
3. `cargo test -p worldwake-ai`
4. Full pre-PR verification deferred to the final S171 branch push, matching the family workflow.

### Commands Run

1. `cargo test -p worldwake-core experience::tests::reliability_record_provenance_keeps_bounded_recent_event_ring`
2. `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount_applies_failure_ratio_proportionally`
3. `cargo test -p worldwake-systems harvest_commit_records_successful_source_reliability_and_enforces_capacity`
4. `cargo test -p worldwake-systems successful_trade_transfers_goods_and_coin_with_trade_tags_and_provenance`
5. `cargo test -p worldwake-systems negotiation_walkaway_records_failed_trade_observations`
6. `cargo test -p worldwake-systems --lib perception::tests::perception_writes_capacity_observation_for_co_located_resource_source -- --exact`
7. `cargo test -p worldwake-systems --lib perception::tests::perception_overwrites_capacity_observation_on_subsequent_tick -- --exact`
8. `cargo test -p worldwake-ai -- agent_tick::tests::apply_source_reliability_failure_observations_coalesces_duplicates_and_enforces_limits`
9. `cargo test -p worldwake-sim save_load::tests::save_format_version_is_106_after_source_reliability_provenance`
10. `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state`
11. `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount`
12. `cargo test -p worldwake-systems --lib source_reliability`
13. `cargo test -p worldwake-core`
14. `cargo test -p worldwake-systems`
15. `cargo test -p worldwake-sim`
16. `cargo test -p worldwake-ai`
17. Waived `./scripts/verify.sh` for this ticket iteration; the full gate is reserved for final branch push after the S171 family lands.

## Verification Result

1. Passed `cargo test -p worldwake-core experience::tests::reliability_record_provenance_keeps_bounded_recent_event_ring` (2026-05-25).
2. Passed `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount_applies_failure_ratio_proportionally` (2026-05-25).
3. Passed `cargo test -p worldwake-systems harvest_commit_records_successful_source_reliability_and_enforces_capacity` (2026-05-25).
4. Passed `cargo test -p worldwake-systems successful_trade_transfers_goods_and_coin_with_trade_tags_and_provenance` (2026-05-25).
5. Passed `cargo test -p worldwake-systems negotiation_walkaway_records_failed_trade_observations` (2026-05-25).
6. Passed `cargo test -p worldwake-systems --lib perception::tests::perception_writes_capacity_observation_for_co_located_resource_source -- --exact` (2026-05-25).
7. Passed `cargo test -p worldwake-systems --lib perception::tests::perception_overwrites_capacity_observation_on_subsequent_tick -- --exact` (2026-05-25).
8. Passed `cargo test -p worldwake-ai -- agent_tick::tests::apply_source_reliability_failure_observations_coalesces_duplicates_and_enforces_limits` (2026-05-25).
9. Passed `cargo test -p worldwake-sim save_load::tests::save_format_version_is_106_after_source_reliability_provenance` (2026-05-25).
10. Passed `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state` (2026-05-25).
11. Passed `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount` (2026-05-25).
12. Passed `cargo test -p worldwake-systems --lib source_reliability` (2026-05-25).
13. Passed `cargo test -p worldwake-core` (2026-05-25).
14. Passed `cargo test -p worldwake-systems` (2026-05-25).
15. Passed `cargo test -p worldwake-sim` (2026-05-25).
16. Passed `cargo test -p worldwake-ai` (2026-05-25).
17. Waived `./scripts/verify.sh` for this ticket iteration; the full gate is reserved for final branch push after the S171 family lands.

## Outcome

Completed 2026-05-25.

Changed:
- Added a bounded 8-entry optional `EventId` provenance ring to `ReliabilityRecord`, exported its capacity, and covered ring rollover/default/roundtrip behavior.
- Threaded lawful provenance ids from harvest commits, trade commits/aborts, facility queue wait observations, perception capacity observations, and AI pending source-failure observation commits into the source-reliability entry.
- Updated source-reliability discount attribution in `ranking.rs` so `SourceReliabilityDiscount.provenance_event_count` and `most_recent_provenance_event` are read from the matched `ReliabilityRecord`.
- Bumped `SAVE_FORMAT_VERSION` to 106 and updated non-default save/runtime fixtures for the new serialized carrier shape.

Deviations:
- Start-time harvest failure recording and craft/staff-market effect application do not synthesize event provenance because those seams either do not receive an `EventLog` or do not mutate source reliability for a concrete source-reliability result. They keep provenance empty.
- AI pending source-failure handling records the component-commit event id shared by the coalesced source-reliability update, not one distinct event per source key. This matches the append-only event-log boundary for that transaction.
