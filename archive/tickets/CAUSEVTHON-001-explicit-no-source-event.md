# CAUSEVTHON-001: Make the "no source event" placeholder explicit in blocker/discrepancy memory

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (`Blocker.source_event` /
`DiscrepancyEntry.source_event` field type + consumers), `worldwake-ai`
(construction sites and the `agent_tick/execution.rs` sentinel branches),
`worldwake-systems` (recording sites), and `worldwake-sim`
(`SAVE_FORMAT_VERSION`).
**Deps**: None. Independent of S162/S163. Source:
`archive/reports/ai-architecture-consolidation-third-iteration-2026-05-22-exploited.md` §11 / Hostile Failure
Inventory ("Causal placeholder IDs"); triage
`docs/triage/2026-05-21-ai-architecture-consolidation-third-iteration-triage.md`.

## Problem

Before this ticket, `Blocker.source_event`
(`crates/worldwake-core/src/blocker_memory.rs`) and
`DiscrepancyEntry.source_event` (`crates/worldwake-core/src/discrepancy.rs`) were
typed `EventId`. Many construction sites that had no causally-linked event at
record time passed the magic value `worldwake_core::EventId(0)` as a "no source
event yet" placeholder. That was an undeclared sentinel: FND-29A wants causal
history to be honest, and a magic `EventId(0)` standing in for "no event" could not
be distinguished from a real event with id 0 except by convention. FND-2 / the
project's "no magic numbers" stance disfavors the implicit value.

The implemented fix makes the placeholder a first-class representation —
`Option<EventId>` (`None` = no source event yet) — so no blocker/discrepancy record
carries a fake causal id and trace/audit code can tell "unsourced" from "sourced by
event 0".

## Assumption Reassessment (2026-05-21)

1. **The third-iteration report undercounts the blast radius.** It names 5 sites in
   `agent_tick/frame.rs` (`:197,765,823,903,926`) plus a test helper in
   `plan_repair.rs:423`. Reassessment against the actual tree shows **60+ production
   `EventId(0)` construction sites** for `source_event`, including
   `failure_handling.rs` (~25), `candidate_generation.rs` (~19),
   `feasibility_probe.rs` (`:772,822`), `feasibility.rs:600`, `agenda_manager.rs:2513`,
   `agent_tick/candidates.rs:95`, `agent_tick/planning.rs:1547`,
   `agent_tick/execution.rs` (`:483,1442`), `agent_tick/observation.rs` (`:433,653`),
   plus the `frame.rs` sites. The ticket scope is the **full** set of
   `source_event` carriers, not just frames. (Verified via
   `grep -rn "EventId(0)" crates/worldwake-ai/src crates/worldwake-core/src`.)
2. **`EventId(0)` is already a live, read sentinel — not dead decoration.**
   `agent_tick/execution.rs` branches on it at `:967`, `:982`, `:1072`, `:1087`
   (`if ... source_event == worldwake_core::EventId(0) { ... }`). These consumers
   were migrated to the `Option` representation (`is_none()`), not just the
   producers. This was the exact "shared data contract under audit" this ticket
   turned on.
3. **Tests assert on the sentinel.** `agent_tick/tests.rs:5162` and `:5373` do
   `assert_ne!(source_event, worldwake_core::EventId(0))` (a real event id is
   expected on those paths); other test fixtures construct `EventId(0)` directly
   (e.g. `tests.rs:5139`, `plan_repair.rs:423`, `search/tests.rs` ~9 sites,
   `partial_plan.rs:502` uses `EventId(88)` as a real fixture). Migrate the
   `assert_ne!` checks to `assert!(source_event.is_some())` and update fixtures to
   `None` / `Some(EventId(..))` per their intent.
4. **Mismatch + correction:** the report frames this as a small `frame.rs` cleanup;
   corrected scope is a core type migration on two record structs with cross-crate
   producer and consumer updates. No new state or behavior — the represented fact
   ("this record has no linked source event") is unchanged; only its encoding moves
   from magic-value to `Option`.
5. **Classification of adjacent contradiction:** whether records *should* eventually
   carry real `EventId`s (rather than `None`) is a separate, larger causal-provenance
   concern (FND-29A). This ticket does **not** attempt to wire real event ids into
   the currently-unsourced records — it only makes the absence honest. Wiring real
   ids where a source event genuinely exists is future cleanup and must become its
   own ticket if pursued.

## Architecture Check

1. `Option<EventId>` is the minimal honest representation: `None` is unambiguous,
   the compiler forces every consumer to handle the unsourced case, and it removes
   a reserved magic id. The alternative — a named `const NO_SOURCE_EVENT:
   EventId = EventId(0)` — keeps a real `EventId` shape that can still be confused
   with a genuine event and does not force consumers to branch; rejected.
2. Per FND-28, the migration replaces the sentinel outright; `EventId(0)` and
   `Option` do not coexist as two representations of "unsourced." All producers,
   the `execution.rs` consumers, and tests move in one change.

## Verified Layers

1. No record carries a fake causal id: focused core unit tests prove blocker and
   discrepancy serialization preserve `source_event == None`.
2. Persistence consumers still fill unsourced records at commit time: existing AI
   persistence tests now assert that persisted blocker/discrepancy memory carries
   `Some(source_event)` and that the event id resolves in the event log.
3. Real-source assertions were preserved by migrating concrete fixture ids to
   `Some(EventId(..))` on blocker/discrepancy records while leaving upstream
   source records as concrete `EventId`.
4. Save/replay shape changed because `BlockerMemory` and `DiscrepancyMemory` are
   persisted components. `SAVE_FORMAT_VERSION` was bumped from 97 to 98, and the
   existing full nondefault save/load roundtrip passed with the new shape.

## Landed Changes

### 1. Core type migration

- `crates/worldwake-core/src/blocker_memory.rs` — `Blocker.source_event:
  Option<EventId>`.
- `crates/worldwake-core/src/discrepancy.rs` — `DiscrepancyEntry.source_event:
  Option<EventId>`.
- Core consumers/tests in `blocker_memory.rs`, `discrepancy.rs`, and
  `test_utils.rs` now use `None` for unsourced records and `Some(EventId(..))` for
  real source events.

### 2. AI producers

- Every production `source_event: worldwake_core::EventId(0)` on blocker/
  discrepancy records in `worldwake-ai` was replaced with `None`.

### 3. Consumers and persistence stamping

- `crates/worldwake-ai/src/agent_tick/execution.rs` now uses `.is_none()` in the
  duplicate-persistence comparison paths and stamps unsourced persisted entries
  with `Some(event_log.next_id())` before committing component memory.
- `partial_plan.rs` wraps concrete barrier-record source ids with `Some(..)` when
  materializing blocker records.
- `worldwake-systems` trade/travel recording fixtures and runtime recording sites
  now use `None` for unavailable causes and `Some(event_id)` for concrete causes.

### 4. Tests

- Added focused core tests for absent-source serialization on blocker and
  discrepancy records.
- Migrated persistence tests from `assert_ne!(.., EventId(0))` to explicit
  `Some(source_event)` extraction and event-log lookup.
- Updated direct-construction fixtures across AI, systems, and save/load tests to
  `None`/`Some(..)` per intent.

## Landed Files

- `crates/worldwake-core/src/blocker_memory.rs` (modify)
- `crates/worldwake-core/src/discrepancy.rs` (modify)
- `crates/worldwake-core/src/test_utils.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — producers + sentinel consumers)
- `crates/worldwake-ai/src/agent_tick/candidates.rs`, `planning.rs`, `observation.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/feasibility.rs`, `feasibility_probe.rs` (modify)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify)
- `crates/worldwake-ai/src/partial_plan.rs`, `plan_repair.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` and associated `#[cfg(test)]` modules (modify)
- `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` and
  scenario test fixtures under `crates/worldwake-ai/tests/scenarios/` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` and
  `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync CAUSEVTHON-001 status)

## Outcome

Completed on 2026-05-21.

- `Blocker.source_event` and `DiscrepancyEntry.source_event` now use
  `Option<EventId>`.
- Unsourced in-memory blockers/discrepancies use `None`; persisted component
  memory still receives a concrete `Some(source_event)` when `agent_tick/execution.rs`
  commits a component update event.
- `SAVE_FORMAT_VERSION` advanced from 97 to 98 because these memories are persisted
  component state.
- The final zero-match scan found no remaining blocker/discrepancy
  `source_event: EventId(0)` construction, `source_event == EventId(0)` sentinel
  branch, or source-event `assert_ne!(.., EventId(0))` assertion under `crates/`.

## Deviations

- The live fallout included `worldwake-systems` and `worldwake-sim`, which were not
  listed in the drafted file surface. `worldwake-systems` had source-event recording
  sites and fixtures; `worldwake-sim` owns the current save-format version and
  save/load fixtures.
- `BarrierFactRecord.source_event` remains a concrete `EventId`; it is an upstream
  record of a real source event, and blocker materialization wraps it as
  `Some(record.source_event)`.

## Verification Result

- Passed `cargo test -p worldwake-core --lib blocker_memory_preserves_explicit_absent_source_event`
- Passed `cargo test -p worldwake-core --lib discrepancy_entry_preserves_explicit_absent_source_event`
- Passed `cargo test -p worldwake-ai --lib persist_blocked_memory_commits_changed_component`
- Passed `cargo test -p worldwake-ai --lib persist_discrepancy_memory_emits_blocker_recorded_for_discrepancy_entries`
- Passed `cargo test -p worldwake-sim --lib save_format_version_is_98_after_causevthon_source_event_option`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py tickets/CAUSEVTHON-001-explicit-no-source-event.md`
- Passed `git diff --check`
