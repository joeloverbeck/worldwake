# S177WATSRCQUA-004: Quality observation on `SourceReliability` + new `EventTag::ResourceSourceQualityObserved`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core/experience` (field additions to `ReliabilityRecord`), `worldwake-core/event_tag` + payload (new variant), `worldwake-systems/perception` (write `observe_quality` at co-located observation site), `worldwake-ai/source_composite` (new quality factor in composite rank), `worldwake-sim/save_load` (SAVE_FORMAT_VERSION bump)
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-003.md`

## Problem

The spec's D4 deliverable extends the existing capacity-observation pipeline with the quality dimension: when an agent is co-located with a water source, perception writes `last_observed_quality` onto the agent's `SourceReliability` (same site that currently writes `last_observed_capacity`), and emits a new `EventTag::ResourceSourceQualityObserved` for causal-history attribution. The source-rank composite (`source_composite_rank`) gains a quality factor that discounts believed-muddy/stale sources by the agent's `WaterToleranceProfile`. Without this, agents have no learned record of source quality and the ranking layer cannot prefer clean sources over believed-muddy ones — the scarcity ↔ quality emergent tradeoff fails.

## Assumption Reassessment (2026-05-31)

1. `ReliabilityRecord` at `crates/worldwake-core/src/experience.rs:79-98` carries `successful_acquisitions, failed_attempts, last_attempt_tick, provenance_events, average_wait_ticks, wait_observation_count, last_observed_capacity, last_observed_capacity_tick`. Derives `Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize`. `Option<WaterQuality>` is `Copy`-compatible.
2. `ReliabilityRecord` construction sites: 17 total. 13/17 use `..Default::default()` spread syntax (spot-check during reassessment); the remaining 4 explicitly enumerate fields and need `last_observed_quality: None, last_observed_quality_tick: Tick(0)` added.
3. `ReliabilityRecord::observe_capacity(capacity, tick)` at `crates/worldwake-core/src/experience.rs:138-141` is the canonical write API for perception. New method `observe_quality(quality, tick)` follows the same shape.
4. `EventTag` at `crates/worldwake-core/src/event_tag.rs:7-56` has 47 variants today (47-variant assertion at line 68). Derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`. Adding `ResourceSourceQualityObserved` requires updating the variant-count assertion and adding the variant to the exhaustive-match sites if any exist (grep workspace-wide for `match.*EventTag::` patterns).
5. The new event payload follows `SourceExpectationFailurePayload` (`crates/worldwake-ai/src/agent_tick/mod.rs:981-992` for construction). The new payload variant on `DecisionEventPayload` carries source attribution + observed quality + tick.
6. Perception's existing `observe_capacity` call lives at `crates/worldwake-systems/src/perception.rs:174`. Adjacent to this call, the new `observe_quality(source.quality?, tick)` write fires for `CommodityKind::Water` sources only (other commodities have `quality == None`).
7. `source_composite_rank` at `crates/worldwake-ai/src/source_composite.rs:24-95` composes `trust_factor`, `wait_factor`, `capacity_factor` via `compose_factors` (lines 161-167). The new quality factor is added as a fourth composite input, gated on `last_observed_quality.is_some()` and weighted by `WaterToleranceProfile.thirst_relief_factor` (ticket 003).
8. Existing tests in `crates/worldwake-ai/src/source_composite.rs` test module (lines 244+): `trust_factor_neutral_without_failures`, `trust_factor_floors_at_zero_with_total_failures`, `wait_factor_caps_at_floor_200_under_extreme_contention`, `capacity_factor_neutral_for_stale_observation`, `capacity_factor_neutral_for_never_observed`, `capacity_factor_floors_at_500_for_empty_fresh_observation`, `capacity_factor_returns_bonus_for_fresh_full_observation`, `compose_factors_clamps_at_2000_permille`, and others. New quality-factor tests follow the same structure.
9. Existing perception capacity-observation tests at `crates/worldwake-systems/src/perception.rs:5274` confirm `observe_capacity` writes work. Quality-observation tests follow the same shape but seed `ResourceSource { quality: Some(Muddy), … }`.
10. `SourceReliability::enforce_limits` at `experience.rs:180-203` already decays records by `last_attempt_tick` over `memory_retention_ticks`. New quality observation fields decay together with the parent record — no separate decay path needed.
11. Adjacent contradictions: none. Quality observation extends the existing capacity-observation pipeline structurally — it does not introduce a new transport path. FND-14B compliance is preserved (perception is the only writer; ranking reads only via belief-view).

## Architecture Check

1. Single transport path (FND-14A → FND-22A): quality observation rides the existing capacity-observation pipeline. Perception is the only writer; `SourceReliability` is the storage; `source_composite_rank` is the consumer. No parallel quality belief substrate.
2. New `ResourceSourceQualityObserved` event (vs. widening `SourceExpectationFailure`) — quality observation is not a failure; it's a routine perception write. Conflating the two would obscure the causal record (FND-29A). New variant is the FND-26 state-cohesion choice.
3. Quality factor in `compose_factors` (vs. a separate ranking pipeline) — quality belongs in the same composite as trust/wait/capacity per FND-26. The factor is gated on `last_observed_quality.is_some()` so unobserved sources are neutral (no discount).

## Verification Layers

1. Field additions on `ReliabilityRecord` compile and roundtrip via bincode — focused test in `experience.rs`.
2. `observe_quality` write site fires on co-located water-source observation — focused integration test in `perception.rs`.
3. `EventTag::ResourceSourceQualityObserved` emission at the observation site — assertion via decision-trace consumption analogous to `survival_preferences.rs:110` consuming `SourceExpectationFailure`.
4. Quality factor in `source_composite_rank` discounts muddy sources — focused unit tests in `source_composite.rs` cover Clean/Stale/Muddy with explicit relief factor values.
5. Composite-rank ordering preserves: fresh `Clean` > fresh `Stale` > fresh `Muddy` for an agent with default tolerance — focused ordering test.
6. SAVE_FORMAT_VERSION migration — version-gate test.

## What to Change

### 1. Extend `ReliabilityRecord`

`crates/worldwake-core/src/experience.rs:79-98`:

```rust
pub struct ReliabilityRecord {
    pub successful_acquisitions: u16,
    pub failed_attempts: u16,
    pub last_attempt_tick: Tick,
    pub provenance_events: [Option<EventId>; SOURCE_RELIABILITY_PROVENANCE_RING_CAPACITY],
    pub average_wait_ticks: u32,
    pub wait_observation_count: u32,
    pub last_observed_capacity: u16,
    pub last_observed_capacity_tick: Tick,
    #[serde(default)]
    pub last_observed_quality: Option<WaterQuality>,
    #[serde(default)]
    pub last_observed_quality_tick: Tick,
}
```

Update `ReliabilityRecord::new(tick)` to initialize the new fields (`last_observed_quality: None, last_observed_quality_tick: Tick(0)`).

Add `observe_quality` method following the `observe_capacity` precedent:

```rust
pub fn observe_quality(&mut self, quality: WaterQuality, tick: Tick) {
    self.last_observed_quality = Some(quality);
    self.last_observed_quality_tick = tick;
}
```

Update the 4 ReliabilityRecord construction sites that don't use spread syntax to add the new fields.

### 2. Add `EventTag::ResourceSourceQualityObserved` variant

`crates/worldwake-core/src/event_tag.rs`: add the new variant near the existing `SourceExpectationFailure` at line 46. Update the variant-count assertion at line 68 (47 → 48). Grep workspace-wide for exhaustive `match` arms on `EventTag` and add the new arm — most matches use `_ =>` catch-all, but verify.

### 3. Add `DecisionEventPayload::ResourceSourceQualityObserved` variant + payload type

In `crates/worldwake-core/src/decision_event_payload.rs` (or wherever `DecisionEventPayload` lives — verify via grep): add the new payload variant carrying source attribution:

```rust
ResourceSourceQualityObserved(ResourceSourceQualityObservedPayload),

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceSourceQualityObservedPayload {
    pub source: EntityId,
    pub commodity: CommodityKind,
    pub quality: WaterQuality,
    pub tick: Tick,
}
```

Follow the `SourceExpectationFailurePayload` precedent for derive set and field shape.

### 4. Perception write site

`crates/worldwake-systems/src/perception.rs:174` — adjacent to the existing `observe_capacity` call, add:

```rust
if state.commodity == CommodityKind::Water {
    if let Some(quality) = state.quality {
        record.observe_quality(quality, tick);
        // Emit DecisionEventPayload::ResourceSourceQualityObserved at the same site
        // (mirror the SourceExpectationFailure emission pattern from agent_tick/mod.rs:1051-1052)
    }
}
```

Locate the existing emission pattern by grepping for `EventTag::SourceExpectationFailure` in `agent_tick/mod.rs` (around line 1051) and follow the same shape. The emission may live in a sibling function — verify whether perception itself emits or whether a downstream consumer translates the perception write into the event.

### 5. Quality factor in `source_composite_rank`

`crates/worldwake-ai/src/source_composite.rs`: add a new `quality_factor_permille(record, tolerance, current_tick)` function adjacent to `capacity_factor_permille` (line 111). The factor is:

- `1000` (neutral) if `last_observed_quality.is_none()` (no belief).
- `1000` (neutral) if the observation is stale (age > some freshness window — reuse the same freshness window as capacity to keep the model symmetric).
- For fresh observations: read the agent's `WaterToleranceProfile.thirst_relief_factor(quality)` and use it as the factor (Clean → 1000, Stale → 700, Muddy → 450 for default tolerance).

Add the new factor to `compose_factors` (line 161) — extend it to 4 inputs (trust, wait, capacity, quality) and clamp at 2000‰ (existing behavior).

Update the test module to add quality-factor tests following the existing test pattern.

### 6. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:7`: change `113` to `114`.

## Files to Touch

- `crates/worldwake-core/src/experience.rs` (modify — add fields to `ReliabilityRecord`, add `observe_quality` method, update `new` constructor, update non-spread construction sites within this file)
- `crates/worldwake-core/src/test_utils.rs` (modify — 1 site at line 151 if not using spread)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `ResourceSourceQualityObserved` variant, update variant-count assertion)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add `ResourceSourceQualityObservedPayload` + variant)
- `crates/worldwake-systems/src/perception.rs` (modify — write `observe_quality` at co-located observation site; new focused test)
- `crates/worldwake-systems/src/production_actions.rs` (modify — 1 ReliabilityRecord site)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — 1 ReliabilityRecord site)
- `crates/worldwake-ai/src/source_composite.rs` (modify — add `quality_factor_permille`, extend `compose_factors` and `source_composite_rank_from_record`; 1 ReliabilityRecord site; new test module entries)
- `crates/worldwake-ai/src/ranking.rs` (modify — 1 ReliabilityRecord site; verify quality factor flows through `apply_source_reliability_discount`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify if perception emission lives here per existing `SourceExpectationFailure` pattern)
- `crates/worldwake-ai/tests/scenarios/source_composite.rs` (modify — 1 ReliabilityRecord site; extend existing tests to cover quality observation)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 113→114)

## Out of Scope

- Drink action reading `last_observed_quality` — Drink reads `ItemLot.quality` (from ticket 002), not the source-reliability quality belief. Ticket 005 is the Drink ticket.
- Forensic record `SourceAcquisitionFailure` — owned by ticket 007.
- CLI player-POV gating for `last_observed_quality` — owned by ticket 008.
- Authoring tolerance overrides — owned by ticket 003 and exercised by tickets 009-010.

## Acceptance Criteria

### Tests That Must Pass

1. New: `reliability_record_observe_quality_writes_fields` — `observe_quality(Muddy, Tick(100))` sets `last_observed_quality = Some(Muddy)` and `last_observed_quality_tick = Tick(100)`.
2. New: `reliability_record_quality_roundtrip` — bincode roundtrip with both `Some` and `None` quality.
3. New: `perception_writes_quality_on_colocated_water_source` in `crates/worldwake-systems/src/perception.rs` — seed agent at place with `ResourceSource { quality: Some(Stale), … }`, run perception, assert `SourceReliability.sources[key].last_observed_quality == Some(Stale)`.
4. New: `perception_does_not_write_quality_for_non_water_source` — apple source produces no quality observation.
5. New: `quality_factor_floors_at_tolerance_for_muddy_observation` in `source_composite.rs` test module — Muddy observation discounts by tolerance factor (450 for default agent).
6. New: `quality_factor_neutral_for_clean_observation` — Clean → 1000.
7. New: `quality_factor_neutral_for_stale_quality_observation` — observation older than freshness window → 1000.
8. New: `composite_rank_orders_clean_above_muddy_for_default_tolerance` — fresh Clean ranks higher than fresh Muddy.
9. New: `resource_source_quality_observed_event_emitted_at_perception_site` — decision-trace assertion analogous to `survival_preferences.rs:110`.
10. Existing: `cargo test --workspace` passes — the variant-count assertion update, the spread-syntax-dominant construction sites, and the existing capacity-observation tests all hold.

### Invariants

1. Quality observation is only written for `CommodityKind::Water` sources.
2. `last_observed_quality` decays together with the parent `ReliabilityRecord` via `enforce_limits` — no separate decay path exists.
3. `EventTag` variant count is now 48; `SAVE_FORMAT_VERSION` is now 114.
4. `source_composite_rank` quality factor is neutral for unobserved or stale-observation sources; only fresh observations drive the discount.
5. Perception is the only writer of `last_observed_quality`. Ranking reads only via belief-view. No system commands another (FND-26).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/experience.rs` (test module extension) — `observe_quality` write, roundtrip with quality fields.
2. `crates/worldwake-core/src/event_tag.rs` (test module extension) — variant-count assertion updated to 48; new variant exists.
3. `crates/worldwake-systems/src/perception.rs` (test module extension) — quality observation at co-located site (water + non-water cases).
4. `crates/worldwake-ai/src/source_composite.rs` (test module extension) — quality_factor tests; compose_factors with quality input; ordering tests.

### Commands

1. `cargo test -p worldwake-core experience observe_quality` — targeted.
2. `cargo test -p worldwake-core event_tag` — variant-count assertion.
3. `cargo test -p worldwake-systems perception_writes_quality` — targeted.
4. `cargo test -p worldwake-ai source_composite quality` — targeted composite tests.
5. `./scripts/verify.sh` — full workspace.

See Merge-Order Constraints in Step 6 summary — SAVE_FORMAT_VERSION cascade includes this bump (113→114).
