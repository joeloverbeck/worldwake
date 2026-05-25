# S170LEASTAPRO-004: BlockerSource enum + Blocker migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `BlockerMemory` persisted shape, AI blocker producers/consumers, source-event promotion, save/load
**Deps**: `archive/tickets/S170LEASTAPRO-003.md`

## Problem

Before this ticket, `Blocker::source_event: Option<EventId>` at `crates/worldwake-core/src/blocker_memory.rs` conflated "no source event recorded" with "no source event possible." Explicit `Blocker` construction sites across production and tests wrote `source_event: None` for inferred blockers, while event-backed blocker paths wrote `Some(id)`. A runtime conditional-promotion pattern in `crates/worldwake-ai/src/agent_tick/execution.rs` opportunistically promoted `None` to `Some(id)`. FND-22A's accountable-origin requirement and FND-29A's queryable-history requirement both failed.

## Assumption Reassessment (2026-05-25)

1. `Blocker` at `crates/worldwake-core/src/blocker_memory.rs:212-221` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. The new `BlockerSource` enum must satisfy these. The migration renames `source_event` → `source` AND changes the type from `Option<EventId>` to the enum — atomic.
2. Spec under audit: `specs/S170-learned-state-provenance-hardening.md` D4. The shared boundary under audit is `Blocker::source_event` → `Blocker::source`: workspace-wide field rename + type change. The rename is scoped to `Blocker` only — `DiscrepancyEntry::source_event` is migrated by ticket 003, and `PartialPlan::source_event` at `crates/worldwake-ai/src/partial_plan.rs:227` (`pub source_event: EventId`, already required and non-Option) is unaffected.
3. Construction sites for `Blocker { ... }`: 93 total before implementation. Live reassessment corrected the draft distribution: `candidate_generation.rs`, `search/tests.rs`, and several scenario files were test fixture fallout, not runtime extractor producers. Production-owned sites included `failure_handling.rs`, `agent_tick/candidates.rs`, `agent_tick/frame.rs`, `agent_tick/observation.rs`, `agent_tick/execution.rs`, `feasibility_probe.rs`, `partial_plan.rs`, and `trade_actions.rs`. All explicit literals still required migration because the shared field was removed.
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
7. Save/load: ticket 002 has already bumped `SAVE_FORMAT_VERSION` from 101 to 102 (`archive/tickets/S170LEASTAPRO-002.md`), and ticket 003 bumped it to 103 (`archive/tickets/S170LEASTAPRO-003.md`). This ticket owns the following persisted-shape bump. The save_load.rs test at lines 637-655 constructs two `Blocker` instances with `source_event: Some(EventId(6))` and `source_event: Some(EventId(7))` — update to `BlockerSource::Event(EventId(6))` and `BlockerSource::Event(EventId(7))`.
8. Most planning-time inferences write `BlockerSource::Inferred` (the agent inferred a blocker from belief state without a discrete triggering event — e.g., `NoKnownSeller`, `NoKnownPath`, `MissingInput`, `WorkstationBusy`). Sites with a concrete event id (e.g., a `ReservationConflict` derived from a contention event with `contention_event: Some(EventId)` in its payload, a `BlockingFact::TargetGone` derived from a perception event that confirmed absence) write `BlockerSource::Event(id)`. The audit visits each runtime site at implementation time and picks the appropriate variant.
9. Reassessment classification: the conditional-promotion runtime sites at execution.rs:1137-1153 are required-consequence migrations; their enum-match form preserves the existing semantic intent. The "value-merge" pattern (`existing.source_event` flowing into `normalized.source_event`) becomes `existing.source` flowing into `normalized.source` — straightforward field-name rename for the merge component.

## Architecture Check

1. Sentinel variant name `Inferred` (not `ReadPhaseInference`, the name used by `DiscrepancySource` / `LearnedOpportunitySource`) is deliberate per spec Design Goal 3 — `Blocker` inferences happen during planning broadly (candidate generation, ranking, failure handling), not specifically during read-phase. Per FND-3, the name encodes the right domain semantic. The three sentinel enums (`LearnedOpportunitySource::ReadPhaseInference`, `DiscrepancySource::ReadPhaseInference`, `BlockerSource::Inferred`) deliberately do NOT share a common abstract supertype, per the third-iteration report's "abstract learning sludge" warning.
2. No backward-compatibility shim. Field rename + type change is wholesale; old `source_event` is removed, not aliased. No `#[serde(default)]`, no `#[serde(alias)]`. Per FND-28's prohibition on backward-compat in live authority paths.
3. The conditional-promotion runtime pattern (`is_none() → Some(id)`) translates 1:1 to enum-match form. The semantic intent — "upgrade from inference to authentic event when one becomes available" — is preserved without parallel state.

## Verified Layers

1. Accountable origin (FND-22A) → focused unit coverage (round-trip tests for both `BlockerSource::Event(EventId)` and `BlockerSource::Inferred`).
2. Inferred sentinel at planning-time inference sites (FND-29A) → compile-enforced explicit `BlockerSource::Inferred` at all no-event blocker construction sites plus focused `cargo test -p worldwake-ai`.
3. Conditional-promotion preserves semantics (FND-28) → focused runtime coverage on `execution.rs:1137-1153` (construct Blocker with `Inferred`, invoke promotion path with a real event id, assert `Event(id)`).
4. Save/load equivalence (FND-12) → save/load round-trip test for `BlockerMemory` with populated `source` field.

## Landed Changes

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

### 3. Updated explicit `Blocker` construction sites

Production and test literals now write either `source: BlockerSource::Inferred` or `source: BlockerSource::Event(id)`. The draft claim that `candidate_generation.rs` had runtime blocker producers was corrected: those were tests compiled by the shared field migration.

### 4. Updated runtime blocker producers

Runtime sites in AI failure handling, active-action/frame/candidate observation paths, coordination-barrier blocker recording, feasibility probing, and trade no-buyer recording now write the explicit source variant. Event-backed paths preserve the event id; planning or read-state inference paths use `BlockerSource::Inferred`.

### 5. Updated remaining constructor fallout

Test fixtures, golden helpers, save/load fixtures, core sample builders, and route/trade/travel tests were updated to the new field shape.

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

### 7. Updated test construction sites

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

### 8. Rewrote preserves-explicit-absent test

`blocker_memory_preserves_explicit_absent_source_event` at `crates/worldwake-core/src/blocker_memory.rs:602` must be renamed and rewritten as `blocker_memory_preserves_explicit_inferred_source` — assert `BlockerSource::Inferred` round-trips.

### 9. Added focused round-trip test for Event variant

In `crates/worldwake-core/src/blocker_memory.rs` test module, add a parallel test asserting `BlockerSource::Event(EventId(42))` round-trips.

### 10. Bumped SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`, `SAVE_FORMAT_VERSION` is now 104.

### 11. Updated save/load tests

`crates/worldwake-sim/src/save_load.rs:637-655` test constructs `Blocker` with `source_event: Some(EventId(6))` and `source_event: Some(EventId(7))`. Update to `source: BlockerSource::Event(EventId(6))` and `source: BlockerSource::Event(EventId(7))`.

### 12. Added focused runtime tests

- In `crates/worldwake-ai/src/agent_tick/execution.rs` test module, add a focused test for the conditional-promotion: construct a `Blocker` with `BlockerSource::Inferred`, invoke the promotion path with a real event id, assert `BlockerSource::Event(source_event)`.
- No new candidate-generation runtime test was added because live reassessment showed no candidate-generation runtime blocker producer; the cited sites were test fixture fallout.

## Landed Files

- `crates/worldwake-core/src/blocker_memory.rs` (modify — new enum, field migration, test updates, new round-trip tests)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — test fixture constructor fallout)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — runtime and test constructor fallout)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — runtime at 772, 822)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — runtime at 653)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — runtime exclusive-facility blocker)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — runtime patience/frame blockers)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — conditional-promotion at 1137-1153 + new focused test)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test sites at 5162, 5169)
- `crates/worldwake-ai/src/feasibility.rs` (modify — test helper fallout)
- `crates/worldwake-ai/src/partial_plan.rs` (modify — coordination-barrier event-backed blocker)
- `crates/worldwake-ai/src/search/tests.rs` (modify — test fixture fallout)
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
- `LearnedOpportunitySource` or `OpportunityEntry` migration (`archive/tickets/S170LEASTAPRO-002.md`, completed)
- `DiscrepancySource` enum or `DiscrepancyEntry` migration (ticket 003) — `DiscrepancyEntry::source_event` is a separate field on a separate type
- Unifying `BlockerSource`, `DiscrepancySource`, and `LearnedOpportunitySource` into a shared abstract enum (per spec Design Goal 3 — domain-specific sentinel names are intentional)
- Auditing `BlockingFact::ReservationConflict { contention_event, .. }` payload semantics (separate concern; this ticket uses the payload's `contention_event` opportunistically where in scope, but does not restructure `BlockingFact`)

## Acceptance Result

### Tests Passed Or Waived

1. New: `blocker_with_event_source_roundtrips` — bincode round-trip of `Blocker { source: BlockerSource::Event(EventId(42)), … }`.
2. Rewritten: `blocker_memory_preserves_explicit_inferred_source` (was `blocker_memory_preserves_explicit_absent_source_event:602`) — assert `BlockerSource::Inferred` round-trips.
3. New: focused runtime test for the conditional-promotion at `execution.rs:1137-1153`. When initial source is `Inferred` and a real event id is later in scope, the field is promoted to `Event(id)`.
4. Waived: no candidate-generation runtime blocker-producer test was added because reassessment showed the candidate-generation `Blocker` literals were tests, not extractor output.
5. Updated: `blocker_memory_roundtrips_through_bincode`, `blocker_memory_roundtrips_non_exact_scope_entries` pass with new field shape.
6. Existing suite: `cargo test -p worldwake-core blocker`, `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`.

### Invariants

1. Every `Blocker` constructed by runtime or test code has an explicit `source` variant; the type system enforces this (no `Option`-style escape hatch).
2. The conditional-promotion semantic ("upgrade from inference to authentic event when one becomes available") is preserved by the enum-match form.
3. `BlockerMemory` round-trips deterministically with bincode.
4. The set of blocker entries an agent holds at tick T is unchanged by this migration (per spec Validation invariant) — only the provenance representation changes.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/blocker_memory.rs` — add `blocker_with_event_source_roundtrips`; rewrite `blocker_memory_preserves_explicit_absent_source_event` as `blocker_memory_preserves_explicit_inferred_source`.
2. `crates/worldwake-ai/src/agent_tick/execution.rs` test module — add a focused test for the conditional-promotion (mirrors ticket 003's parallel `DiscrepancyEntry` test).
3. No candidate-generation runtime test added; the drafted target did not exist as a live runtime producer.

### Commands Run

1. `cargo test -p worldwake-core blocker`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-systems`
5. `cargo test --workspace --no-run`
6. Waived `./scripts/verify.sh` for this per-ticket closeout because the `implement-spec-tickets` harness owns the full pre-push gate after final spec archival.

Merge note: Ticket 002 bumped `SAVE_FORMAT_VERSION` from 101 to 102. Ticket 003 bumped it from 102 to 103, and ticket 004 bumped it to 104.

## Outcome

Completed on 2026-05-25.

- Added `BlockerSource::{Event, Inferred}` and migrated `Blocker::source_event: Option<EventId>` to `Blocker::source: BlockerSource`.
- Replaced runtime and test `Blocker` construction sites with explicit source variants; event-backed paths preserve real `EventId`s and inferred paths use `BlockerSource::Inferred`.
- Preserved the existing source-event promotion semantics in `agent_tick/execution.rs` using enum matching instead of `Option` mutation.
- Bumped `SAVE_FORMAT_VERSION` to 104 and updated save/load fixtures to round-trip `BlockerSource::Event` values.
- Corrected drafted scope drift: `candidate_generation.rs` and search fixture references were constructor fallout in tests, not runtime blocker producers.

## Deviations

- The drafted candidate-generation runtime proof was waived because the live branch has no candidate-generation runtime `Blocker` producer. The invariant is covered by type-enforced constructor migration, `cargo test --workspace --no-run`, focused core/source-promotion tests, and the full `worldwake-ai` crate suite.
- `./scripts/verify.sh` is deferred to the final `implement-spec-tickets` pre-push phase.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core blocker`
- Passed `cargo test -p worldwake-ai blocker_memory_with_source_events_promotes_inferred_source`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai`
- Waived `./scripts/verify.sh` because the harness final branch phase owns the pre-push verification gate.
