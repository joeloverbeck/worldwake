# S171LEACONTDEC-001: Foundation attribution types and trace-struct field extensions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` trace/runtime carrier shape plus `worldwake-sim` save-format version bump
**Deps**: spec `archive/specs/S171-learned-context-decision-trace-edge.md`

## Problem

Before this ticket, the decision-trace surface had no way to record which learned-state entry produced a ranking-time bonus or discount. `RankedGoalSummary` (`crates/worldwake-ai/src/decision_trace.rs:691-715`) carried `source_reliability_discount` and `competition_discount` attribution structs for the two discount axes but no equivalent attribution for the two bonus axes (`learned_opportunity_bonus` / `repair_memory_bonus`). `SourceReliabilityDiscount` (`decision_trace.rs:773-780`) recorded `failure_ratio_permille` but no lawful source-reliability provenance carrier. This ticket added the foundation attribution types and field extensions to the existing trace structs so the threading work in S171LEACONTDEC-002 had somewhere to land. Per the spec's FND-22A "experience path" requirement, S170 made the stored update inspectable; S171LEACONTDEC-002 made bonus consumption at ranking time inspectable, and `archive/tickets/S171LEACONTDEC-004.md` later landed the remaining source-reliability provenance producer.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RankedGoalSummary` derives only `Clone, Debug` (no `Serialize`/`Deserialize`) per `decision_trace.rs:692`; its `Default` impl at `decision_trace.rs:717` exists, so existing literal construction sites using `..Default::default()` spread will pick up new `None` fields automatically. `SourceReliabilityDiscount` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` per `decision_trace.rs:773`; no `Default` impl exists, so all construction sites must enumerate the new fields explicitly. Live reassessment corrected the draft save premise: `SourceReliabilityDiscount` is embedded in serialized `AgentDecisionRuntime.agenda_state` through `AgendaEntry`, so this ticket must also bump `SAVE_FORMAT_VERSION` and update the non-default runtime roundtrip fixture.
2. `LearnedOpportunitySource` (`crates/worldwake-core/src/learned_opportunity_memory.rs:5-20`) and `BreachSignature` (`crates/worldwake-core/src/repair_memory.rs:8`) both derive `Copy, Clone, Eq, Hash, Serialize, Deserialize` — they satisfy the bounds the new attribution structs require. `EventId`, `Tick`, `OpportunityKey` are already imported in `decision_trace.rs` (per Verification Agent finding at lines 10-17).
3. Shared abstraction boundary under audit: `RankedGoalSummary` as the per-decision trace record consumed by both internal trace formatters (at `decision_trace.rs:317-325`, 1794-1802, 2110) and by `observer.rs:1207-1213` (which reads `motive_source_contributions` only and is unaffected by the new attribution fields).

## Architecture Check

1. Two domain-specific attribution structs (not a unified `BonusAttribution` trait) keep the trace surface symmetric with the existing `SourceReliabilityDiscount` / `CompetitionDiscount` sibling pattern — concrete typed fields per FND-3, no abstract-score-bag, no shared trait coupling the two unrelated learned stores. Per the spec's Design Goal 4: domain-specific attribution types over unified abstraction.
2. No backwards-compatibility shim: new fields are added directly to existing structs; existing literal construction sites either inherit `None` via `..Default::default()` (RankedGoalSummary) or get explicit `None`/`0`/`None` values (SourceReliabilityDiscount). Because the discount is serialized inside current runtime save state, the current save format advances rather than adding `#[serde(default)]` compatibility for old runtime bytes.

## Verified Layers

1. Type definitions and field additions exist with the spec's exact shapes -> compile-time check (`cargo build --workspace`).
2. Existing tests continue to pass -> the spec's V2 contract that motive_score/priority_class/agenda order are byte-identical pre/post is satisfied by the absence of runtime data-flow changes in this ticket; `cargo test -p worldwake-ai` proves the foundation lands cleanly.
3. Current save/runtime shape advanced, not backward-compatible -> `SAVE_FORMAT_VERSION` assertions and `agent_decision_runtime_bincode_round_trip_preserves_all_fields` prove the new serialized discount fields roundtrip with non-default values.
4. Single-layer ticket — no mixed-layer mapping applies. Bonus-attribution threading landed in S171LEACONTDEC-002, source-reliability provenance later landed in `archive/tickets/S171LEACONTDEC-004.md`, and trace-text rendering later landed in `archive/tickets/S171LEACONTDEC-003.md`.

## Landed Changes

### 1. Add `LearnedOpportunityBonusAttribution` and `RepairMemoryBonusAttribution`

In `crates/worldwake-ai/src/decision_trace.rs`, near the existing `SourceReliabilityDiscount` / `CompetitionDiscount` block (around line 763-780), add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LearnedOpportunityBonusAttribution {
    pub opportunity: OpportunityKey,
    pub entry_source: LearnedOpportunitySource,
    pub entry_observed_tick: Tick,
    pub entry_expires_tick: Tick,
    pub pre_bonus_motive: u32,
    pub post_bonus_motive: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemoryBonusAttribution {
    pub signature: worldwake_core::BreachSignature,
    pub entry_success_count: u32,
    pub entry_expires_tick: Tick,
    pub pre_bonus_motive: u32,
    pub post_bonus_motive: u32,
}
```

If `LearnedOpportunitySource` is not already re-exported through `worldwake_core`, import it directly from `worldwake_core::learned_opportunity_memory::LearnedOpportunitySource` (verify with a single grep at implementation time).

### 2. Extend `SourceReliabilityDiscount` with provenance fields

In `crates/worldwake-ai/src/decision_trace.rs` at lines 773-780:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceReliabilityDiscount {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub failure_ratio_permille: u32,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
    // NEW
    pub provenance_event_count: u32,
    pub most_recent_provenance_event: Option<EventId>,
}
```

All 8 construction sites must populate the new fields with `0` and `None` respectively in this ticket. S171LEACONTDEC-002 later proved that the drafted source-reliability provenance read was aimed at the wrong carrier; `archive/tickets/S171LEACONTDEC-004.md` later landed the lawful source-reliability producer. Sites identified by Step 2 codebase validation:

- `crates/worldwake-ai/src/decision_trace.rs:3132` (`sample_source_reliability_discount`)
- `crates/worldwake-ai/src/ranking.rs:657` (production `apply_source_reliability_discount` return)
- `crates/worldwake-ai/src/ranking.rs:6577, 6871, 6993` (focused tests)
- `crates/worldwake-ai/src/decision_runtime.rs:639` (committed-state path)
- `crates/worldwake-ai/src/agent_tick/planning.rs:5548` (test fixture)
- `crates/worldwake-ai/src/goal_model.rs:2787` (focused test)

Placeholder for source-reliability provenance: production sites at `ranking.rs:657` and `decision_runtime.rs:639` initially write `0` / `None`; `archive/tickets/S171LEACONTDEC-004.md` later landed the lawful source-reliability provenance reads.

### 3. Extend `RankedGoalSummary` with two new bonus-attribution fields

In `crates/worldwake-ai/src/decision_trace.rs` at lines 691-715:

```rust
pub struct RankedGoalSummary {
    // existing fields unchanged…
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    // NEW
    pub learned_opportunity_bonus: Option<LearnedOpportunityBonusAttribution>,
    pub repair_memory_bonus: Option<RepairMemoryBonusAttribution>,
    // remaining fields unchanged…
}
```

Updated `impl Default for RankedGoalSummary` to set the two new fields to `None`. Sites using `..Default::default()` spread picked up `None` automatically; sites that enumerate fields explicitly received the two new field lines. Live compile fallout expanded the drafted list to include additional explicit trace-test and survival-forensics constructors; those were current-ticket shared-shape fallout.

- `crates/worldwake-ai/src/decision_trace.rs:3497, 3510, 3943, 3956, 4956, 4969` (test fixtures)
- `crates/worldwake-ai/src/agent_tick/planning.rs:342`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:1424`
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs:176`
- `crates/worldwake-ai/tests/scenarios/motive_sources.rs:78`

All sites get `learned_opportunity_bonus: None, repair_memory_bonus: None` (or are unchanged if they use `..Default::default()` spread — verify per-site at implementation time).

### 4. Add new sample helpers and extend existing one (D8 subsumed)

In `crates/worldwake-ai/src/decision_trace.rs:3121-3140`:

- Add `sample_learned_opportunity_bonus_attribution()` returning a `LearnedOpportunityBonusAttribution` constructed with representative values (analogous to `sample_competition_discount` at line 3121).
- Add `sample_repair_memory_bonus_attribution()` returning a `RepairMemoryBonusAttribution`.
- Update `sample_source_reliability_discount` (line 3131) to populate the two new fields with representative values (e.g., `provenance_event_count: 3, most_recent_provenance_event: Some(EventId(42))`).

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs` (modify) — new type definitions, struct extensions, Default impl update, sample helpers
- `crates/worldwake-ai/src/ranking.rs` (modify) — update 4 `SourceReliabilityDiscount` construction sites
- `crates/worldwake-ai/src/decision_runtime.rs` (modify) — update 1 `SourceReliabilityDiscount` construction site at line 639
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — update 1 `SourceReliabilityDiscount` construction site at line 5548 + 1 `RankedGoalSummary` construction at line 342
- `crates/worldwake-ai/src/goal_model.rs` (modify) — update 1 `SourceReliabilityDiscount` construction site at line 2787
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify) — update 1 `RankedGoalSummary` construction site at line 1424
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify) — update 1 `RankedGoalSummary` construction at line 176
- `crates/worldwake-ai/tests/scenarios/motive_sources.rs` (modify) — update 1 `RankedGoalSummary` construction at line 78
- `crates/worldwake-sim/src/save_load.rs` (modify) — bump `SAVE_FORMAT_VERSION` and version assertions because `SourceReliabilityDiscount` is serialized through `AgentDecisionRuntime.agenda_state`

## Out of Scope

- Threading attribution into `RankedGoalSummary` at ranking time (D5+D6 — bonus fields landed by S171LEACONTDEC-002; source-reliability provenance later landed in `archive/tickets/S171LEACONTDEC-004.md`).
- Decision-trace formatter additions (D7 — landed by `archive/tickets/S171LEACONTDEC-003.md`).
- Any new behavior or score arithmetic change — this is type-definition-and-construction-site migration only.

## Acceptance Criteria Result

### Tests Passed

1. `cargo build --workspace` succeeds after type additions and all construction-site migrations.
2. Existing focused tests at `ranking.rs:6418-6805` (the 7 `source_reliability_discount_*` tests), `ranking.rs:5826 repair_memory_boosts_matching_alternative_only_while_live`, and `ranking.rs:5883 learned_opportunity_memory_boosts_matching_opportunity_only_while_live` all continue to pass without modification.
3. Existing suite: `cargo test -p worldwake-ai`.
4. Save-format assertions in `worldwake-sim` passed with the current version advanced to 105.

### Invariants

1. `RankedGoalSummary` derives `Clone, Debug` only — no Serialize/Deserialize is added in this ticket.
2. `SourceReliabilityDiscount` continues to derive `Serialize, Deserialize`; the new fields satisfy the derives (u32 and Option<EventId> are both Serialize/Deserialize-compatible).
3. All `RankedGoalSummary` and `SourceReliabilityDiscount` construction sites compile cleanly; no site falls back to silent default through unintended elision.
4. No old-save compatibility shim is added; the current save format advances once.

## Test Plan Result

### Test Changes

1. No new assertion tests were added; the new sample helpers and the updated `sample_source_reliability_discount` are test infrastructure for the later `archive/tickets/S171LEACONTDEC-003.md`. Existing runtime coverage named in Assumption Reassessment item 1 proved the foundation lands without behavior change.

### Commands Run

1. `cargo build --workspace`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-sim save_load::tests::save_format_version_is_105_after_learned_context_trace_fields`
4. `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state`
5. Waived `./scripts/verify.sh` for this ticket iteration; the harness reserves that full gate for final branch push after the S171 family lands.

## Verification Result

1. Passed `cargo build --workspace` (2026-05-25).
2. Passed `cargo test -p worldwake-ai` (2026-05-25).
3. Passed `cargo test -p worldwake-sim save_load::tests::save_format_version_is_105_after_learned_context_trace_fields` (2026-05-25).
4. Passed `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state` (2026-05-25).
5. Waived `./scripts/verify.sh` for this ticket iteration; the full gate is reserved for final branch push after the S171 family lands.

## Outcome

Completed 2026-05-25.

Changed:
- Added `LearnedOpportunityBonusAttribution` and `RepairMemoryBonusAttribution` to `decision_trace.rs`.
- Extended `RankedGoalSummary` with optional learned-opportunity and repair-memory attribution fields defaulting to `None`.
- Extended `SourceReliabilityDiscount` with `provenance_event_count` and `most_recent_provenance_event`.
- Updated explicit `RankedGoalSummary` and `SourceReliabilityDiscount` constructors across AI trace/runtime fixtures and tests.
- Bumped `SAVE_FORMAT_VERSION` from 104 to 105 because `SourceReliabilityDiscount` is serialized through `AgentDecisionRuntime.agenda_state`.

Deviations:
- Live reassessment disproved the draft's no-save-shape premise. The ticket absorbed the required current save-format bump instead of adding compatibility shims.
- S171LEACONTDEC-002 populated the new bonus attribution fields in ranking and extended the `AgendaEntry` carrier before `summarize_ranked_goal` projected those fields into `RankedGoalSummary`. `archive/tickets/S171LEACONTDEC-004.md` landed the remaining source-reliability provenance producer.
