# S170LEASTAPRO-003: DiscrepancySource enum + DiscrepancyEntry migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — agent decision runtime (DiscrepancyMemory), save/load
**Deps**: None

## Problem

`DiscrepancyEntry::source_event: Option<EventId>` at `crates/worldwake-core/src/discrepancy.rs:74` conflates "no source event was recorded" with "there is no source event possible." `apply_pending_discrepancies` (`crates/worldwake-ai/src/agent_tick/observation.rs:416-434`) hardcodes `source_event: None` for all read-phase inferences, and a runtime conditional-promotion pattern at `crates/worldwake-ai/src/agent_tick/execution.rs:1242-1258` opportunistically promotes `None` to `Some(id)`. The `Option` shape loses the FND-29-required distinction between "inference-without-event" and "missed-recording," and the conditional-promotion semantics are obscured.

## Assumption Reassessment (2026-05-25)

1. `DiscrepancyEntry` at `crates/worldwake-core/src/discrepancy.rs:67-75` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. The new `DiscrepancySource` enum must satisfy these. The migration renames `source_event` → `source` AND changes the type from `Option<EventId>` to the new enum — both changes happen atomically.
2. Spec under audit: `specs/S170-learned-state-provenance-hardening.md` D3. `apply_pending_discrepancies` at `crates/worldwake-ai/src/agent_tick/observation.rs:416-434` invokes `discrepancy_memory.record(DiscrepancyEntry { … })` with `source_event: None`. `PendingDiscrepancyRecord` (`crates/worldwake-ai/src/candidate_generation.rs:253-258`) carries `scope`, `discrepancy`, `observed_tick`, `clearing_condition` — no event id. The migration writes `source: DiscrepancySource::ReadPhaseInference` at this site.
3. The shared boundary under audit is `DiscrepancyEntry::source_event` → `DiscrepancyEntry::source`: a workspace-wide field rename + type change. The rename is scoped to `DiscrepancyEntry` only — `Blocker::source_event` (`crates/worldwake-core/src/blocker_memory.rs:220`) is a separate field on a separate type, migrated by ticket 004. A third same-named field exists at `crates/worldwake-ai/src/partial_plan.rs:227` (`pub source_event: EventId`, already required and non-Option) and is unaffected.
4. Construction sites for `DiscrepancyEntry { ... }`: 32 total. Runtime (non-`#[cfg(test)]`): `crates/worldwake-ai/src/agent_tick/observation.rs:422` (`apply_pending_discrepancies`), `frame.rs:733, 871, 894`, `planning.rs:1666`, `execution.rs:612` (`discrepancy_entry_for_repair` helper), `failure_handling.rs:267`. Test/helper: `crates/worldwake-core/src/discrepancy.rs:163, 313`, `crates/worldwake-ai/src/agent_tick/tests.rs:5345, 5436, 8932, 8940, 9038, 9046`, `crates/worldwake-ai/src/feasibility_probe.rs:767`, `crates/worldwake-ai/src/agent_tick/execution.rs:1614`, `crates/worldwake-ai/src/plan_repair.rs:449-450`, `crates/worldwake-ai/src/agenda_manager.rs:2745`, `crates/worldwake-ai/src/candidate_generation.rs:12544`, `crates/worldwake-ai/src/failure_handling.rs:4225`, `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs:374`, `crates/worldwake-ai/tests/scenarios/plan_repair.rs:119`, `crates/worldwake-sim/src/save_load.rs:611`. No spread-syntax; no `Default` impl. Count is load-bearing for effort.
5. Field-read sites (`.source_event`) for DiscrepancyEntry: `crates/worldwake-core/src/discrepancy.rs:299, 304, 337, 341` (inline tests asserting `source_event == None` / `Some(EventId(7))`); `crates/worldwake-ai/src/agent_tick/execution.rs:1242-1258` (runtime conditional-promotion pattern: `if normalized.source_event.is_none() { normalized.source_event = existing.source_event; }` and `if entry.source_event.is_none() { entry.source_event = Some(source_event); }`); `crates/worldwake-ai/src/agent_tick/tests.rs:5373, 5381` (test reads); `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs:410-411` (overlaps with ticket 004's read coverage of the same file at different field accesses — coordinate by referencing the field by exact line during edits).
6. Runtime conditional-promotion at `execution.rs:1242-1258` becomes (preserving semantic intent):
   ```rust
   if matches!(entry.source, DiscrepancySource::ReadPhaseInference) {
       entry.source = DiscrepancySource::Event(source_event);
   }
   ```
   The "upgrade from inference to authentic event when one becomes available" pattern translates 1:1 to enum-match form.
7. Existing focused tests touching `DiscrepancyEntry`: `discrepancy_types_satisfy_required_bounds:174`, `discrepancy_entry_roundtrips_through_bincode:273`, `discrepancy_entry_preserves_explicit_absent_source_event:290` (MUST be rewritten — currently asserts `source_event = None` round-trips; new assertion is `source = DiscrepancySource::ReadPhaseInference` round-trips), `discrepancy_memory_roundtrips_non_exact_scope_entries:308` (constructs entries with `source_event: Some(EventId(7))` / `Some(EventId(8))` — update to `DiscrepancySource::Event(EventId(7))` / `DiscrepancySource::Event(EventId(8))`).
8. Save/load: this ticket bumps `SAVE_FORMAT_VERSION` by 1 as part of the cascade with tickets 002 and 004 (see Merge note). The save_load.rs test at 611-626 constructs a `DiscrepancyEntry` with `source_event: Some(worldwake_core::EventId(5))` — update to `source: DiscrepancySource::Event(worldwake_core::EventId(5))`.
9. Reassessment classification: the runtime conditional-promotion at execution.rs:1242-1258 is a required-consequence migration — its enum-match form preserves the existing semantic intent. The "value-merge" patterns (e.g., `existing.source_event` flowing into `normalized.source_event`) become `existing.source` flowing into `normalized.source` — straightforward field-name rename for the merge.

## Architecture Check

1. Replaces ambiguous `Option<EventId>` with a typed enum carrying explicit semantics. The compiler now forces every construction site to decide whether the discrepancy has an attributable event or is a read-phase inference — no silent `None`.
2. No backward-compatibility shim. The field rename + type change is wholesale; old `source_event` is removed, not aliased. No `#[serde(default)]`, no `#[serde(alias)]`. Per FND-28's prohibition on backward-compat in live authority paths.
3. The conditional-promotion runtime pattern (`is_none() → Some(id)`) translates 1:1 to enum-match form, preserving the semantic intent ("upgrade from inference to authentic event when one becomes available") without introducing parallel state. No FND-28 violation.

## Verification Layers

1. Accountable origin (FND-22A) → focused unit coverage (round-trip tests for both `DiscrepancySource::Event(EventId)` and `DiscrepancySource::ReadPhaseInference`).
2. Explicit read-phase sentinel at inference sites (FND-29) → focused runtime coverage (assert `apply_pending_discrepancies` writes `source == DiscrepancySource::ReadPhaseInference`).
3. Conditional-promotion preserves semantics (FND-28 — no parallel authority) → focused runtime coverage on `execution.rs:1242-1258` (construct entry with `ReadPhaseInference`, invoke promotion path with a real event id, assert `Event(id)` after).
4. Save/load equivalence (FND-12) → save/load round-trip test for `DiscrepancyMemory` with populated `source` field.

## What to Change

### 1. Define DiscrepancySource enum

In `crates/worldwake-core/src/discrepancy.rs`, define alongside `DiscrepancyEntry`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscrepancySource {
    /// The discrepancy was triggered by a specific world event (a
    /// perception that contradicted prior belief, a travel completion
    /// that invalidated a route assumption, etc.).
    Event(EventId),
    /// The discrepancy emerged from a read-phase inference over the
    /// agent's belief state during candidate generation (e.g., a
    /// `PendingDiscrepancyRecord` produced by an extractor). No
    /// single triggering event exists.
    ReadPhaseInference,
}
```

### 2. Migrate DiscrepancyEntry field

In `discrepancy.rs:67-75`, replace `pub source_event: Option<EventId>` with `pub source: DiscrepancySource`. Field order preserved (last field).

### 3. Update apply_pending_discrepancies

In `crates/worldwake-ai/src/agent_tick/observation.rs:422-434`, change `source_event: None` to `source: DiscrepancySource::ReadPhaseInference`. Add a one-line rationale comment: `// read-phase inference from PendingDiscrepancyRecord; no triggering event in scope`.

### 4. Audit and update other runtime construction sites

For each site, choose the variant based on whether a triggering event id is in scope:

- `crates/worldwake-ai/src/agent_tick/frame.rs:733, 871, 894` — audit each; write `DiscrepancySource::Event(id)` where a triggering event id is in scope (typically from the frame-failure context), `DiscrepancySource::ReadPhaseInference` with a one-line rationale comment otherwise.
- `crates/worldwake-ai/src/agent_tick/planning.rs:1666` — audit per same rule.
- `crates/worldwake-ai/src/agent_tick/execution.rs:612` (`discrepancy_entry_for_repair` helper) — extend the helper's signature with `source: DiscrepancySource` or derive the source from the existing `broken_link` / `signature` parameters; update both callers.
- `crates/worldwake-ai/src/failure_handling.rs:267` — audit per same rule.

### 5. Migrate runtime conditional-promotion sites

In `crates/worldwake-ai/src/agent_tick/execution.rs:1242-1258`, rewrite both conditional-promotion blocks (the "normalized" merge and the "entry" promotion):

```rust
// "normalized" merge (the one carrying existing source forward)
if matches!(normalized.source, DiscrepancySource::ReadPhaseInference) {
    normalized.source = existing.source;
}

// "entry" promotion (the one filling in a real event id)
if matches!(entry.source, DiscrepancySource::ReadPhaseInference) {
    entry.source = DiscrepancySource::Event(source_event);
}
```

### 6. Update test construction sites

Every `DiscrepancyEntry { ... }` literal in tests across the workspace updates the field name and value:

- Inline tests in `crates/worldwake-core/src/discrepancy.rs:163` (`discrepancy_entry` helper) and `:313` (`discrepancy_memory_roundtrips_non_exact_scope_entries` literals) — set `source: DiscrepancySource::Event(EventId(7))` and `source: DiscrepancySource::Event(EventId(8))` matching the original `Some(EventId(7))` / `Some(EventId(8))` intent.
- `crates/worldwake-ai/src/agent_tick/tests.rs:5345, 5436, 8932, 8940, 9038, 9046` — audit each test's intent; use `DiscrepancySource::Event(id)` where the test exercises an event-attributed path, `DiscrepancySource::ReadPhaseInference` otherwise.
- `crates/worldwake-ai/src/feasibility_probe.rs:767` — use `DiscrepancySource::ReadPhaseInference` (test exercises a feasibility probe with no event context).
- `crates/worldwake-ai/src/agent_tick/execution.rs:1614` (`repair_discrepancy_entry` test helper) — same rule.
- `crates/worldwake-ai/src/plan_repair.rs:449-450` (`discrepancy_entry` test helper) — same rule.
- `crates/worldwake-ai/src/agenda_manager.rs:2745` (test) — same rule.
- `crates/worldwake-ai/src/candidate_generation.rs:12544` (test) — same rule.
- `crates/worldwake-ai/src/failure_handling.rs:4225` (test) — same rule.
- `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs:374` — update `source_event: None` to `source: DiscrepancySource::ReadPhaseInference`. Update the field-read sites at lines 403, 410-411 to use the new field name (`stored_intent.source = DiscrepancySource::Event(source_event); assert_eq!(recorded.source, DiscrepancySource::Event(source_event));`).
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs:119` (`discrepancy_entry` test helper) — same rule.

### 7. Update field-read sites in tests.rs

In `crates/worldwake-ai/src/agent_tick/tests.rs:5373, 5381`, change `.source_event` field reads/writes to `.source` and adjust the value pattern (e.g., `for_each(|entry| entry.source = DiscrepancySource::Event(source_event))`).

### 8. Rewrite preserves-explicit-absent test

`discrepancy_entry_preserves_explicit_absent_source_event` at `crates/worldwake-core/src/discrepancy.rs:290` must be renamed and rewritten:

```rust
#[test]
fn discrepancy_entry_preserves_explicit_read_phase_inference_source() {
    let mut entry = discrepancy_entry(
        blocker_key(),
        Discrepancy::RouteUnknown,
        Tick(19),
        DiscrepancyClearing::ReobservationOf {
            target: entity_id(4, 0),
        },
    );
    entry.source = DiscrepancySource::ReadPhaseInference;

    let bytes = bincode::serialize(&entry).unwrap();
    let roundtrip: DiscrepancyEntry = bincode::deserialize(&bytes).unwrap();

    assert_eq!(roundtrip.source, DiscrepancySource::ReadPhaseInference);
}
```

### 9. Add focused round-trip test for Event variant

Add a parallel test asserting `DiscrepancySource::Event(EventId(42))` round-trips.

### 10. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs:7`, increment by 1 (cascade with tickets 002 and 004).

### 11. Update save/load test in save_load.rs

`crates/worldwake-sim/src/save_load.rs:611-626` test constructs a `DiscrepancyEntry` with `source_event: Some(worldwake_core::EventId(5))`. Update to `source: worldwake_core::DiscrepancySource::Event(worldwake_core::EventId(5))`.

### 12. Add focused runtime tests

- In `crates/worldwake-ai/src/agent_tick/observation.rs` test module, add a test asserting `apply_pending_discrepancies` produces entries with `source == DiscrepancySource::ReadPhaseInference`.
- In `crates/worldwake-ai/src/agent_tick/execution.rs` test module, add a test for the conditional-promotion: construct an entry with `ReadPhaseInference`, invoke the promotion path with a real event id, assert `source == DiscrepancySource::Event(source_event)` after.

## Files to Touch

- `crates/worldwake-core/src/discrepancy.rs` (modify — new enum, field migration, test updates, new round-trip tests)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — runtime site at 422 + new focused test)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — runtime sites at 733, 871, 894)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — runtime site at 1666)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — runtime helper at 612, conditional-promotion at 1242-1258, test helper at 1614 + new focused test)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test sites at 5345, 5373, 5381, 5436, 8932, 8940, 9038, 9046)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — runtime at 267, test at 4225)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — test site at 2745)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — test site at 12544)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — test site at 767)
- `crates/worldwake-ai/src/plan_repair.rs` (modify — test helper at 449-450)
- `crates/worldwake-ai/tests/scenarios/cross_goal_blocker_scoping.rs` (modify — test construction at 374 and field-read sites at 403, 410-411)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify — test helper at 119)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION at line 7; update test construction at 611-626)

## Out of Scope

- `RoutePreference::record_safe` changes (ticket 001)
- `LearnedOpportunitySource` or `OpportunityEntry` migration (ticket 002)
- `BlockerSource` enum or `Blocker` migration (ticket 004) — `Blocker::source_event` is a separate field on a separate type
- Unifying `LearnedOpportunitySource` and `DiscrepancySource` into a shared abstract enum (per spec Design Goal 3 — domain-specific sentinel names are intentional)

## Acceptance Criteria

### Tests That Must Pass

1. New: `discrepancy_entry_with_event_source_roundtrips` — bincode round-trip of `DiscrepancyEntry { source: DiscrepancySource::Event(EventId(42)), … }`.
2. Rewritten: `discrepancy_entry_preserves_explicit_read_phase_inference_source` (was `discrepancy_entry_preserves_explicit_absent_source_event:290`) — assert `DiscrepancySource::ReadPhaseInference` round-trips.
3. New: focused runtime test on `apply_pending_discrepancies` asserting written entries have `source == DiscrepancySource::ReadPhaseInference`.
4. New: focused runtime test for the conditional-promotion at `execution.rs:1242-1258`. When the initial source is `ReadPhaseInference` and a real event id later comes into scope, the field is promoted to `Event(id)`.
5. Updated: `discrepancy_memory_roundtrips_non_exact_scope_entries:308` passes with new field shape.
6. Existing suite: `cargo test -p worldwake-core discrepancy`, `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim`.

### Invariants

1. Every `DiscrepancyEntry` constructed by runtime or test code has an explicit `source` variant; the type system enforces this (no `Option`-style escape hatch).
2. The conditional-promotion semantic ("upgrade from inference to authentic event when one becomes available") is preserved by the enum-match form.
3. `DiscrepancyMemory` round-trips deterministically with bincode.
4. The set of discrepancy entries an agent holds at tick T is unchanged by this migration (per spec Validation invariant) — only the provenance representation changes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/discrepancy.rs` — add `discrepancy_entry_with_event_source_roundtrips`; rewrite `discrepancy_entry_preserves_explicit_absent_source_event` as `discrepancy_entry_preserves_explicit_read_phase_inference_source`.
2. `crates/worldwake-ai/src/agent_tick/observation.rs` test module — add a focused test asserting `apply_pending_discrepancies` writes `ReadPhaseInference`.
3. `crates/worldwake-ai/src/agent_tick/execution.rs` test module — add a focused test for the conditional-promotion (mirroring ticket 004's parallel Blocker test).

### Commands

1. `cargo test -p worldwake-core discrepancy`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-sim`
4. `./scripts/verify.sh`

Merge note: Ticket 003 bumps `SAVE_FORMAT_VERSION` by 1 as part of the cascade with tickets 002 and 004 — landing order determines exact target values.
