# S103BELCLADED-001: Aspect-source claim deduplication in `record_entity_claim`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief claim storage (worldwake-core)
**Deps**: S101 (completed)

## Problem

`record_entity_claim` unconditionally appends every new claim, causing O(observation_frequency × staleness_window) claim accumulation per entity. At busy locations, agents accumulate 3,000-4,500 claims across ~30 known entities. Since `derive_entity_summary` iterates all claims per entity, per-tick cost scales linearly with claim count, producing a 4.2x slowdown over 500 ticks (extrapolated to 108 minutes for a full 10,080-tick soak vs. 5-6 minute target).

Most accumulated claims are redundant: a fresh direct observation of an entity's location supersedes a stale direct observation of the same aspect from the same source type. The old claim will never win in `derive_entity_summary` but still consumes iteration budget.

## Assumption Reassessment (2026-04-14)

1. `record_entity_claim` at `belief.rs:64` unconditionally appends via `self.entity_claims.entry(claim.subject).or_default().push(claim)` — verified.
2. `EntityBeliefClaim` (`entity_belief_claim.rs:47`) has fields `subject`, `aspect: EntityBeliefAspect`, `source: PerceptionSource`, `acquired_tick`, `confidence` — verified. The dedup key `(subject, aspect, source_kind)` is constructible from existing fields.
3. `PerceptionSource` (`belief.rs:1998`) has variants `DirectObservation`, `Report { from, chain_len }`, `Rumor { chain_len }`, `Inference` — verified. Source kind discrimination: `Direct`, `Report(from)` (distinct per informant), `Rumor`, `Inference`.
4. `derive_entity_summary` (`belief.rs:1860`) picks the highest-ranked claim per aspect using effective confidence derived from `confidence` plus `claimed_event_tick.unwrap_or(acquired_tick)`, then `acquired_tick`, then `claim_id` — verified. Naive "newer replaces older" is not safe for same-informant reports about older events; deduplication must remove only same-key claims that are dominated on confidence, staleness anchor, and acquisition order.
5. Existing tests: `derive_entity_summary_returns_none_for_empty_claims`, `derive_entity_summary_projects_single_claims_into_summary`, `derive_entity_summary_prefers_highest_effective_confidence_per_aspect`, `derive_entity_summary_applies_staleness_before_selecting_winner`, `derive_entity_summary_uses_claimed_event_tick_for_report_staleness`, `derive_entity_summary_breaks_ties_by_newer_tick_then_higher_claim_id` — all in `belief.rs` test module (after line 2278).

## Architecture Check

1. Deduplication by `(subject, aspect, source_kind)` remains the right storage seam, but replacement must be dominance-based rather than "newer always wins." A new claim may replace an existing same-key claim only when it is no worse on stored confidence, staleness anchor (`claimed_event_tick.unwrap_or(acquired_tick)`), and acquisition order. This preserves the direct-observation hot path without collapsing distinct same-informant reports about different event times. Multi-source claims for the same aspect are preserved (FND-16).
2. No backward-compatibility shims. The `record_entity_claim` signature is unchanged; callers are unaffected.

## Verification Layers

1. Same-source same-aspect claims are replaced, not appended → focused unit test asserting claim count after repeated observations
2. Different-source same-aspect claims coexist → focused unit test asserting both claims present
3. `derive_entity_summary` produces identical results → existing `derive_entity_summary_*` tests pass unchanged
4. Golden tests pass unchanged → `cargo test -p worldwake-ai` (summary derivation is identical)

## What to Change

### 1. Add `SourceKind` discriminant type

Add a small enum or derive a discrimination key from `PerceptionSource`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum SourceKind {
    Direct,
    Report(EntityId),
    Rumor,
    Inference,
}

fn source_kind(source: &PerceptionSource) -> SourceKind {
    match source {
        PerceptionSource::DirectObservation => SourceKind::Direct,
        PerceptionSource::Report { from, .. } => SourceKind::Report(*from),
        PerceptionSource::Rumor { .. } => SourceKind::Rumor,
        PerceptionSource::Inference => SourceKind::Inference,
    }
}
```

This is module-private — used only inside `record_entity_claim`.

### 2. Modify `record_entity_claim` to compact only dominated same-key claims

When a new claim arrives for `(subject, aspect, source_kind)`:

1. Drop it if an existing same-key claim already dominates it on stored confidence, staleness anchor, and acquisition order.
2. Remove any existing same-key claims that the new claim dominates by that same comparison.
3. Append the new claim if it is not dominated.

```rust
pub fn record_entity_claim(&mut self, claim: EntityBeliefClaim) {
    self.next_claim_id = ClaimId(claim.claim_id.0.saturating_add(1).max(self.next_claim_id.0));
    let claims = self.entity_claims.entry(claim.subject).or_default();
    let new_kind = source_kind(&claim.source);
    if claims.iter().any(|existing| {
        existing.aspect == claim.aspect
            && source_kind(&existing.source) == new_kind
            && claim_dominates(existing, &claim)
    }) {
        return;
    }
    claims.retain(|existing| {
        existing.aspect != claim.aspect
            || source_kind(&existing.source) != new_kind
            || !claim_dominates(&claim, existing)
    });
    claims.push(claim);
}
```

This keeps the direct-observation steady state at one claim per aspect while preserving same-informant report variants that still matter to `derive_entity_summary`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Changing activation-based entity decay (S101)
- Introducing hard capacity caps on claims
- Changing the confidence-threshold pruning model
- Modifying `derive_entity_summary` logic
- Social observation deduplication (S103BELCLADED-003)
- Amortized pruning optimization (S103BELCLADED-002)

## Acceptance Criteria

### Tests That Must Pass

1. New: same-source same-aspect direct-observation claim replaces an older dominated claim (claim count stays 1)
2. New: different-source same-aspect claims coexist (claim count is 2)
3. New: report from agent A and report from agent B are distinct source kinds (both kept)
4. New: a same-informant report about an older event does not evict a fresher same-key report
5. Existing suite: `cargo test -p worldwake-core` — all existing belief tests pass unchanged
6. Existing suite: `cargo test -p worldwake-ai` — golden tests pass unchanged

### Invariants

1. `derive_entity_summary` produces identical results with or without compaction — only dominated same-key claims are removed
2. Multi-source claims for the same aspect are preserved — FND-16 (uncertainty and contradiction)
3. Repeated direct observations do not accumulate unbounded same-aspect duplicates

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (test module) — `record_entity_claim_replaces_dominated_same_source_same_aspect`: record two DirectObservation claims for Location on the same entity, assert only one claim remains
2. `crates/worldwake-core/src/belief.rs` (test module) — `record_entity_claim_preserves_different_sources`: record a DirectObservation and a Report claim for the same aspect, assert both remain
3. `crates/worldwake-core/src/belief.rs` (test module) — `record_entity_claim_distinguishes_report_informants`: record Report(from=A) and Report(from=B) for the same aspect, assert both remain
4. `crates/worldwake-core/src/belief.rs` (test module) — `record_entity_claim_preserves_nondominated_same_informant_report`: record a fresher same-informant report and then a newer acquisition about an older event, assert both remain

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::record_entity_claim_`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-14.

- Added module-private `SourceKind`, `claim_staleness_anchor`, and `claim_dominates` helpers in `crates/worldwake-core/src/belief.rs`.
- Changed `AgentBeliefStore::record_entity_claim` to compact only dominated same-key claims instead of unconditionally appending, which keeps repeated direct observations from growing without collapsing distinct same-informant reports whose event-time freshness still matters.
- Added focused `record_entity_claim_*` tests covering direct-observation replacement, cross-source coexistence, report-informant separation, and the nondominated same-informant report case.
- Reassessment corrected the original ticket/spec sketch from naive "newer replaces older" behavior to dominance-based compaction because `derive_entity_summary` ranks with `claimed_event_tick.unwrap_or(acquired_tick)` in addition to acquisition order.

## Verification Result

- Passed `cargo test -p worldwake-core --lib belief::tests::record_entity_claim_`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ticket status: untracked active draft
