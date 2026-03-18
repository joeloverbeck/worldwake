# GOLDHARN-001: Add Event-Log Assertion Helpers to Golden Harness

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `crates/worldwake-ai/tests/golden_harness/mod.rs`, `crates/worldwake-core/src/event_log.rs`, `crates/worldwake-core/src/delta.rs`

## Problem

Cross-system golden tests currently have to reconstruct authoritative mutation ordering by manually iterating raw event-log records and matching deltas inline. That creates repeated forensic boilerplate and makes mixed-layer assertions harder to read than they need to be.

## Assumption Reassessment (2026-03-18)

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` already owns shared golden-test ergonomics for world setup, belief seeding, office/faction helpers, and action tracing, but it does not currently expose reusable event-log delta search/assertion helpers.
2. `crates/worldwake-core/src/event_log.rs` already exposes the needed raw log traversal surface: `EventLog::len()`, `EventLog::get()`, and `EventLog::events_by_tag()`. `crates/worldwake-core/src/delta.rs` already exposes the typed authoritative delta families: `StateDelta`, `ComponentDelta`, and `RelationDelta`.
3. The current concrete duplication is narrower than originally stated. The strongest existing ad hoc scan is in `crates/worldwake-ai/tests/golden_emergent.rs`, where `golden_combat_death_triggers_force_succession` open-codes:
   - first combat event that sets `DeadAt` on the incumbent
   - first political event that removes `RelationValue::OfficeHolder { office, holder }`
   - first political event that adds `RelationValue::OfficeHolder { office, holder }`
   - append-order assertions between those event ids
4. Other tests do scan the event log directly, but not all of them are the same abstraction target:
   - `crates/worldwake-ai/tests/golden_social.rs` scans discovery evidence inside tagged events.
   - `crates/worldwake-ai/tests/golden_production.rs` scans queue-promotion targets.
   Those sites confirm future reuse potential, but this ticket should not claim they already need the exact same delta helper surface.
5. The gap is test-harness ergonomics only. There is no missing production authority path, and `Engine Changes: None` remains correct.

## Architecture Check

1. A thin helper layer in `golden_harness` is cleaner than repeating raw `EventLog` iteration and `StateDelta` matching inside scenario files. The source of truth stays the append-only event log; the harness only removes repeated forensic boilerplate.
2. This should stay out of production code. Adding convenience APIs to `worldwake-core::EventLog` for test readability would weaken the architecture boundary by teaching authoritative code about test-only assertion patterns.
3. The helper surface should stay generic but small: tag-filtered search, typed component/relation delta predicates, and append-order assertions. Anything more opinionated would start turning the harness into a parallel query DSL.
4. No backward-compatibility layer is needed. Existing ad hoc scans can be replaced directly where the new helper is clearer.

## Verification Layers

1. authoritative mutation presence (`DeadAt` set, `OfficeHolder` removed/added) -> golden harness event-log helper assertions over typed `StateDelta`
2. authoritative mutation ordering (death before vacancy before installation) -> event-id append-order assertions
3. political action-path absence for force succession (`declare_support` must not commit) -> action trace assertions in `golden_emergent`
4. end-to-end scenario outcome (office holder installed, dead rival remains dead) -> authoritative world-state assertions in `golden_emergent`

## What to Change

### 1. Add typed event-log helper functions

Extend `crates/worldwake-ai/tests/golden_harness/mod.rs` with small reusable helpers such as:

- `first_tagged_event_matching(...)` or equivalent
- `event_sets_component(...)`
- `event_adds_relation(...)`
- `event_removes_relation(...)`
- `assert_event_order(...)`

The API should remain generic enough to support future authoritative event-log assertions without introducing a second query abstraction outside typed `EventRecord`/`StateDelta`.

### 2. Add harness-focused tests

Add focused harness tests proving the helpers correctly detect component/relation deltas and preserve append-order semantics.

### 3. Migrate at least one existing golden scenario

Replace the current ad hoc succession-order scan in `golden_combat_death_triggers_force_succession` with the new helpers so the abstraction is proven useful immediately.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify to consume the helpers)

## Out of Scope

- Adding new production event-log APIs
- Adding new runtime/system trace sinks
- Rewriting every existing golden test to the new helper surface

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent`
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Golden tests must be able to assert authoritative mutation ordering without manually open-coding raw event-log scans in each scenario.
2. The helper layer must remain a thin wrapper over typed event-log records and deltas, not a lossy summary cache.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — add helper-focused tests that prove component/relation delta detection and ordering.
2. `crates/worldwake-ai/tests/golden_emergent.rs` — replace the force-succession event-log scan with the helper layer to prove practical use.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession -- --exact`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`

## Outcome

- Completed: 2026-03-18
- Actual changes:
  - Added reusable event-log assertion helpers to `crates/worldwake-ai/tests/golden_harness/mod.rs` for tagged event search, typed component/relation delta matching, and append-order assertions.
  - Added focused harness tests proving those helpers detect `DeadAt` component deltas, `OfficeHolder` relation add/remove deltas, and append-order semantics.
  - Replaced the ad hoc force-succession event-log scan in `crates/worldwake-ai/tests/golden_emergent.rs` with the shared helpers.
- Deviations from original plan:
  - The ticket was corrected before implementation to reflect the actual current duplication: the concrete migration target was `golden_emergent`, not a broader multi-file helper rollout.
  - No production `worldwake-core` API changes were needed or added.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent golden_harness::tests::event_log_helpers_match_component_and_relation_deltas -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent golden_harness::tests::event_log_helpers_preserve_append_order -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
