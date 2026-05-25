# S170LEASTAPRO-004: BlockerSource enum + Blocker migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — agent decision runtime (BlockerMemory), candidate generation, failure handling, save/load
**Deps**: None

## Problem

`Blocker::source_event: Option<EventId>` at `crates/worldwake-core/src/blocker_memory.rs:220` conflates "no source event recorded" with "no source event possible." Over 90 construction sites across `candidate_generation.rs`, `failure_handling.rs`, `agenda_manager.rs`, `plan_repair.rs`, `feasibility_probe.rs`, and others write `source_event: None`. Most are planning-time inferences with no triggering event; some have real contention/refusal events available. A runtime conditional-promotion pattern at `crates/worldwake-ai/src/agent_tick/execution.rs:1137-1153` opportunistically promotes `None` to `Some(id)`. FND-22A's accountable-origin requirement and FND-29A's queryable-history requirement both fail.

## Assumption Reassessment (2026-05-25)

1. `Blocker` at `crates/worldwake-core/src/blocker_memory.rs:212-221` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. The new `BlockerSource` enum must satisfy these. The migration renames `source_event` → `source` AND changes the type from `Option<EventId>` to the enum — atomic.
2. Spec under audit: `specs/S170-learned-state-provenance-hardening.md` D4. The shared boundary under audit is `Blocker::source_event` → `Blocker::source`: workspace-wide field rename + type change. The rename is scoped to `Blocker` only — `DiscrepancyEntry::source_event` is migrated by ticket 003, and `PartialPlan::source_event` at `crates/worldwake-ai/src/partial_plan.rs:227` (`pub source_event: EventId`, already required and non-Option) is unaffected.
3. Construction sites for `Blocker { ... }`: 93 total. Distribution by file:
   - `crates/worldwake-ai/src/candidate_generation.rs` ~18 sites at lines 11647, 11705, 11769, 11830, 11889, 11958, 12072, 12181, 12312, 12386, 12447, 12555, 13561, 17544, 20348, 21257, 21634, 22157 — all in extractor/candidate-emission paths
   - `crates/worldwake-ai/src/failure_handling.rs` ~17 sites at lines 260, 277, 2957, 2983, 3006, 3029, 3060, 3081, 3111, 3144, 3275, 3307, 3332, 3360, 3955, 3975, 3991
   - `crates/worldwake-ai/src/agenda_manager.rs:2750`
   - `crates/worldwake-ai/src/plan_repair.rs:455`
   - `crates/worldwake-ai/src/feasibility_probe.rs:772, 822`
   - `crates/worldwake-ai/src/agent_tick/observation.rs:653`
   - Plus test sites across the workspace (see Files to Touch)

   No spread-syntax usage; no `Default` impl on `Blocker`. Count is load-bearing — Large effort warranted (per Step 2 sub-check (d) rule: ">100 sites with no spread-syntax → Large").
4. Field-read sites (`.source_event`) for Blocker: `crates/worldwake-ai/src/agent_tick/execution.rs:1137-1153` is the runtime conditional-promotion pattern: `if normalized.source_event.is_none() { normalized.source_event = existing.source_event; }` and `if blocker.source_event.is_none() { blocker.source_event = Some(source_event); }`. Test reads at `crates/worldwake-ai/src/agent_tick/tests.rs:5162, 5169`. Inline tests at `crates/worldwake-core/src/blocker_memory.rs:605, 612, 651, 655` (intent-construction + roundtrip assertions). Scenario test reads at `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs:35, 379, 403, 410-411` (overlap with ticket 003's coverage of the same file at different field accesses — coordinate by editing the specific line ranges).
5. Runtime conditional-promotion at `execution.rs:1137-1153` becomes:
   ```rust
   if matches!(normalized.source, BlockerSource::Inferred) {
       normalized.source = existing.source;
   }
   // ...
   if matches!(blocker.source, BlockerSource::Inferred) {
       blocker.source = BlockerSource::Event(source_event);
   }
   ```
6. Existing focused tests touching `Blocker` in `crates/worldwake-core/src/blocker_memory.rs`: `blocker_types_satisfy_required_bounds:292`, `blocker_clearing_condition_and_baseline_satisfy_required_bounds:306`, `blocker_memory_defaults_empty:359`, `clear_for_removes_matching_blocker_key:505`, `blocker_memory_roundtrips_through_bincode:583`, `blocker_memory_preserves_explicit_absent_source_event:602` (MUST be rewritten — currently asserts `source_event = None` round-trips; new assertion is `source = BlockerSource::Inferred` round-trips), `blocker_memory_roundtrips_non_exact_scope_entries:618` (constructs entries with `source_event: Some(EventId(_))` — update to `BlockerSource::Event(_)`).
7. Save/load: this ticket bumps `SAVE_FORMAT_VERSION` by 1 as part of the cascade with tickets 002 and 003. The save_load.rs test at lines 637-655 constructs two `Blocker` instances with `source_event: Some(EventId(6))` and `source_event: Some(EventId(7))` — update to `BlockerSource::Event(EventId(6))` and `BlockerSource::Event(EventId(7))`.
8. Most planning-time inferences write `BlockerSource::Inferred` (the agent inferred a blocker from belief state without a discrete triggering event — e.g., `NoKnownSeller`, `NoKnownPath`, `MissingInput`, `WorkstationBusy`). Sites with a concrete event id (e.g., a `ReservationConflict` derived from a contention event with `contention_event: Some(EventId)` in its payload, a `BlockingFact::TargetGone` derived from a perception event that confirmed absence) write `BlockerSource::Event(id)`. The audit visits each runtime site at implementation time and picks the appropriate variant.
9. Reassessment classification: the conditional-promotion runtime sites at execution.rs:1137-1153 are required-consequence migrations; their enum-match form preserves the existing semantic intent. The "value-merge" pattern (`existing.source_event` flowing into `normalized.source_event`) becomes `existing.source` flowing into `normalized.source` — straightforward field-name rename for the merge component.

## Architecture Check

1. Sentinel variant name `Inferred` (not `ReadPhaseInference`, the name used by `DiscrepancySource` / `LearnedOpportunitySource`) is deliberate per spec Design Goal 3 — `Blocker` inferences happen during planning broadly (candidate generation, ranking, failure handling), not specifically during read-phase. Per FND-3, the name encodes the right domain semantic. The three sentinel enums (`LearnedOpportunitySource::ReadPhaseInference`, `DiscrepancySource::ReadPhaseInference`, `BlockerSource::Inferred`) deliberately do NOT share a common abstract supertype, per the third-iteration report's "abstract learning sludge" warning.
2. No backward-compatibility shim. Field rename + type change is wholesale; old `source_event` is removed, not aliased. No `#[serde(default)]`, no `#[serde(alias)]`. Per FND-28's prohibition on backward-compat in live authority paths.
3. The conditional-promotion runtime pattern (`is_none() → Some(id)`) translates 1:1 to enum-match form. The semantic intent — "upgrade from inference to authentic event when one becomes available" — is preserved without parallel state.

## Verification Layers

1. Accountable origin (FND-22A) → focused unit coverage (round-trip tests for both `BlockerSource::Event(EventId)` and `BlockerSource::Inferred`).
2. Inferred sentinel at planning-time inference sites (FND-29A) → focused runtime coverage (candidate-generation extractor produces `Blocker` with `source == BlockerSource::Inferred`).
3. Conditional-promotion preserves semantics (FND-28) → focused runtime coverage on `execution.rs:1137-1153` (construct Blocker with `Inferred`, invoke promotion path with a real event id, assert `Event(id)`).
4. Save/load equivalence (FND-12) → save/load round-trip test for `BlockerMemory` with populated `source` field.

## What to Change

### 1. Define BlockerSource enum

In `crates/worldwake-core/src/blocker_memory.rs`, define alongside `Blocker`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockerSource {
    /// The blocker was recorded in response to a specific world event
    /// (a refused trade, a denied access attempt, a contested
    /// reservation that resolved against this agent, etc.).
    Event(EventId),
    /// The blocker is a planning-time inference from the agent's
    /// belief state: candidate generation evaluated a candidate and
    /// determined the blocking fact applies without a discrete
    /// triggering event (e.g., `NoKnownSeller` inferred from absence
    /// of belief-store entries).
    Inferred,
}
```

### 2. Migrate Blocker field

In `blocker_memory.rs:212-221`, replace `pub source_event: Option<EventId>` with `pub source: BlockerSource`. Field order preserved (last field).

### 3. Update runtime construction sites in candidate_generation.rs

Approximately 18 sites (lines listed in Assumption Reassessment #3). All are planning-time inferences from belief state. Each writes `source: BlockerSource::Inferred`. Audit each site at edit time for whether any has a concrete event id in scope (e.g., from a contention-event-carrying `BlockingFact::ReservationConflict { contention_event, .. }` payload); those write `BlockerSource::Event(id)`.

### 4. Update runtime construction sites in failure_handling.rs

Approximately 17 sites (lines listed in Assumption Reassessment #3). Same audit rule.

### 5. Update remaining runtime construction sites

- `crates/worldwake-ai/src/agenda_manager.rs:2750` — audit
- `crates/worldwake-ai/src/plan_repair.rs:455` — audit
- `crates/worldwake-ai/src/feasibility_probe.rs:772, 822` — audit
- `crates/worldwake-ai/src/agent_tick/observation.rs:653` — `apply_pending_facility_intents` infers blocker from observation; write `BlockerSource::Inferred` with a one-line rationale.

### 6. Migrate runtime conditional-promotion in execution.rs:1137-1153

Rewrite both blocks (the "normalized" merge and the "blocker" promotion):

```rust
// "normalized" merge (carries existing source forward)
if matches!(normalized.source, BlockerSource::Inferred) {
    normalized.source = existing.source;
}

// "blocker" promotion (fills in a real event id)
if matches!(blocker.source, BlockerSource::Inferred) {
    blocker.source = BlockerSource::Event(source_event);
}
```

### 7. Update test construction sites

Every `Blocker { ... }` literal in tests across the workspace updates the field name and value. Test files (with per-line guidance below):

- `crates/worldwake-core/src/blocker_memory.rs` inline tests — the `make_intent` helper near line 278 and surrounding construction sites; the round-trip helpers at lines 605, 612, 651, 655 (these are field-write sites in tests).
- `crates/worldwake-ai/src/agent_tick/tests.rs:5162, 5169` — test reads/writes; update to `.source` field name and `BlockerSource::Event(id)` / `BlockerSource::Inferred` per test intent.
- `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs:35, 379, 403, 410-411` — construction sites and field reads. Note overlap with ticket 003's edits in the same file at lines 374, 403, 410-411 — coordinate by line range (ticket 003 edits DiscrepancyEntry construction at 374 + DiscrepancyEntry field reads at 403, 410-411 if those reference DiscrepancyEntry; ticket 004 edits Blocker construction at 35, 379 and any Blocker field reads). Inspect each line precisely at edit time to determine which ticket owns it.
- `crates/worldwake-ai/tests/scenarios/contention_inspectability.rs:249` — update.
- `crates/worldwake-ai/tests/scenarios/portfolio_planning.rs:163` — update.
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs:131, 523` — update.
- `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs:665` — update (this site uses `source_event: EventId(89)` non-Option — investigate; if it's actually a `PartialPlan` site rather than `Blocker`, no change needed).
- `crates/worldwake-ai/tests/scenarios/route_preferences.rs:142` — update.
- `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs:32, 38` — update both construction and assertion sites.
- `crates/worldwake-systems/src/travel_actions.rs:1024, 1034` — test sites, update.
- `crates/worldwake-systems/src/trade_actions.rs:3346, 3356` — test sites, update.

### 8. Rewrite preserves-explicit-absent test

`blocker_memory_preserves_explicit_absent_source_event` at `crates/worldwake-core/src/blocker_memory.rs:602` must be renamed and rewritten as `blocker_memory_preserves_explicit_inferred_source` — assert `BlockerSource::Inferred` round-trips.

### 9. Add focused round-trip test for Event variant

In `crates/worldwake-core/src/blocker_memory.rs` test module, add a parallel test asserting `BlockerSource::Event(EventId(42))` round-trips.

### 10. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs:7`, increment by 1 (cascade with tickets 002 and 003).

### 11. Update save/load tests

`crates/worldwake-sim/src/save_load.rs:637-655` test constructs `Blocker` with `source_event: Some(EventId(6))` and `source_event: Some(EventId(7))`. Update to `source: BlockerSource::Event(EventId(6))` and `source: BlockerSource::Event(EventId(7))`.

### 12. Add focused runtime tests

- In `crates/worldwake-ai/src/agent_tick/execution.rs` test module, add a focused test for the conditional-promotion: construct a `Blocker` with `BlockerSource::Inferred`, invoke the promotion path with a real event id, assert `BlockerSource::Event(source_event)`.
- In `crates/worldwake-ai/src/candidate_generation.rs` test module, add a focused test asserting that an extractor producing a planning-time blocker emits `Blocker { source: BlockerSource::Inferred, … }`.

## Files to Touch

- `crates/worldwake-core/src/blocker_memory.rs` (modify — new enum, field migration, test updates, new round-trip tests)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — ~18 runtime construction sites + new focused test)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — ~17 runtime sites)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — runtime at 2750)
- `crates/worldwake-ai/src/plan_repair.rs` (modify — runtime at 455)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — runtime at 772, 822)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — runtime at 653)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — conditional-promotion at 1137-1153 + new focused test)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test sites at 5162, 5169)
- `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs` (modify — test sites at 35, 379, plus field-read sites; coordinate with ticket 003)
- `crates/worldwake-ai/tests/scenarios/contention_inspectability.rs` (modify — test site at 249)
- `crates/worldwake-ai/tests/scenarios/portfolio_planning.rs` (modify — test site at 163)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify — test sites at 131, 523)
- `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs` (modify — test site at 665 if confirmed Blocker, not PartialPlan)
- `crates/worldwake-ai/tests/scenarios/route_preferences.rs` (modify — test site at 142)
- `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` (modify — test sites at 32, 38)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — test sites at 1024, 1034)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — test sites at 3346, 3356)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION at line 7; update test sites at 637-655)

## Out of Scope

- `RoutePreference::record_safe` changes (ticket 001)
- `LearnedOpportunitySource` or `OpportunityEntry` migration (ticket 002)
- `DiscrepancySource` enum or `DiscrepancyEntry` migration (ticket 003) — `DiscrepancyEntry::source_event` is a separate field on a separate type
- Unifying `BlockerSource`, `DiscrepancySource`, and `LearnedOpportunitySource` into a shared abstract enum (per spec Design Goal 3 — domain-specific sentinel names are intentional)
- Auditing `BlockingFact::ReservationConflict { contention_event, .. }` payload semantics (separate concern; this ticket uses the payload's `contention_event` opportunistically where in scope, but does not restructure `BlockingFact`)

## Acceptance Criteria

### Tests That Must Pass

1. New: `blocker_with_event_source_roundtrips` — bincode round-trip of `Blocker { source: BlockerSource::Event(EventId(42)), … }`.
2. Rewritten: `blocker_memory_preserves_explicit_inferred_source` (was `blocker_memory_preserves_explicit_absent_source_event:602`) — assert `BlockerSource::Inferred` round-trips.
3. New: focused runtime test for the conditional-promotion at `execution.rs:1137-1153`. When initial source is `Inferred` and a real event id is later in scope, the field is promoted to `Event(id)`.
4. New: focused candidate-generation test asserting that an extractor producing a planning-time blocker emits `Blocker { source: BlockerSource::Inferred, … }`.
5. Updated: `blocker_memory_roundtrips_through_bincode`, `blocker_memory_roundtrips_non_exact_scope_entries` pass with new field shape.
6. Existing suite: `cargo test -p worldwake-core blocker`, `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`.

### Invariants

1. Every `Blocker` constructed by runtime or test code has an explicit `source` variant; the type system enforces this (no `Option`-style escape hatch).
2. The conditional-promotion semantic ("upgrade from inference to authentic event when one becomes available") is preserved by the enum-match form.
3. `BlockerMemory` round-trips deterministically with bincode.
4. The set of blocker entries an agent holds at tick T is unchanged by this migration (per spec Validation invariant) — only the provenance representation changes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocker_memory.rs` — add `blocker_with_event_source_roundtrips`; rewrite `blocker_memory_preserves_explicit_absent_source_event` as `blocker_memory_preserves_explicit_inferred_source`.
2. `crates/worldwake-ai/src/agent_tick/execution.rs` test module — add a focused test for the conditional-promotion (mirrors ticket 003's parallel `DiscrepancyEntry` test).
3. `crates/worldwake-ai/src/candidate_generation.rs` test module — add a focused test asserting an extractor produces `Blocker { source: BlockerSource::Inferred, … }`.

### Commands

1. `cargo test -p worldwake-core blocker`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-systems`
5. `./scripts/verify.sh`

Merge note: Ticket 004 bumps `SAVE_FORMAT_VERSION` by 1 as part of the cascade with tickets 002 and 003 — landing order determines exact target values.
