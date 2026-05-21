# CAUSEVTHON-001: Make the "no source event" placeholder explicit in blocker/discrepancy memory

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (`Blocker.source_event` /
`DiscrepancyEntry.source_event` field type + consumers), `worldwake-ai`
(construction sites and the `agent_tick/execution.rs` sentinel branches).
**Deps**: None. Independent of S162/S163. Source:
`reports/ai-architecture-consolidation-third-iteration.md` §11 / Hostile Failure
Inventory ("Causal placeholder IDs"); triage
`docs/triage/2026-05-21-ai-architecture-consolidation-third-iteration-triage.md`.

## Problem

`Blocker.source_event` (`crates/worldwake-core/src/blocker_memory.rs:220`) and
`DiscrepancyEntry.source_event` (`crates/worldwake-core/src/discrepancy.rs`) are
typed `EventId`. Many construction sites that have no causally-linked event at
record time pass the magic value `worldwake_core::EventId(0)` as a "no source event
yet" placeholder. This is an undeclared sentinel: FND-29A wants causal history to
be honest, and a magic `EventId(0)` standing in for "no event" cannot be
distinguished from a real event with id 0 except by convention. FND-2 / the
project's "no magic numbers" stance disfavors the implicit value.

The intended fix is to make the placeholder a first-class representation —
`Option<EventId>` (`None` = no source event yet) — so no record carries a fake
causal id and trace/audit code can tell "unsourced" from "sourced by event 0".

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
   must be migrated to the `Option` representation (`is_none()`), not just the
   producers. This is the exact "shared data contract under audit" this ticket
   turns on.
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

## Verification Layers

1. No record carries a fake causal id → focused unit test: constructing a blocker/
   discrepancy with no source event yields `source_event == None` (no `EventId(0)`).
2. Sentinel consumers still behave identically → `agent_tick/execution.rs` branch
   tests: the `is_none()` path matches the prior `== EventId(0)` path (decision/
   action trace or focused runtime test on the affected blocker/discrepancy
   normalization).
3. Real-source assertions preserved → migrated `assert!(source_event.is_some())`
   tests at the former `assert_ne!(.., EventId(0))` sites.
4. Save/replay: if `Blocker`/`DiscrepancyEntry` participate in the serialized
   state, the format version bumps and a round-trip test covers `None`/`Some`. (Check
   `blocker_memory.rs`/`discrepancy.rs` serde derivation during implementation; if
   serialized, this is a save-format change.)

## What to Change

### 1. Core type migration

- `crates/worldwake-core/src/blocker_memory.rs:220` — `Blocker.source_event:
  Option<EventId>`.
- `crates/worldwake-core/src/discrepancy.rs` — `DiscrepancyEntry.source_event:
  Option<EventId>` (confirm exact line during implementation).
- Update any core consumers/tests in `blocker_memory.rs` (the many
  `source_event: EventId(N)` fixtures → `Some(EventId(N))`) and `test_utils.rs:216,236`.

### 2. AI producers

- Replace every production `source_event: worldwake_core::EventId(0)` in
  `worldwake-ai` with `None` (and any genuine real-id sites — none currently — with
  `Some(..)`). Cover all files from Assumption 1.

### 3. AI consumers (the live sentinel branches)

- `crates/worldwake-ai/src/agent_tick/execution.rs:967,982,1072,1087` — migrate
  `== worldwake_core::EventId(0)` to `.is_none()`.
- `partial_plan.rs:202` (`source_event: record.source_event`) — propagate the
  `Option` type through.

### 4. Tests

- Migrate `agent_tick/tests.rs:5162,5373` `assert_ne!(.., EventId(0))` →
  `assert!(.. .is_some())`.
- Update direct-construction fixtures across `search/tests.rs`, `agent_tick/tests.rs`,
  `plan_repair.rs:423`, `feasibility*.rs`, `candidate_generation.rs` tests to
  `None`/`Some(..)` per intent.

## Files to Touch

- `crates/worldwake-core/src/blocker_memory.rs` (modify)
- `crates/worldwake-core/src/discrepancy.rs` (modify)
- `crates/worldwake-core/src/test_utils.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — producers + sentinel consumers)
- `crates/worldwake-ai/src/agent_tick/candidates.rs`, `planning.rs`, `observation.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/feasibility.rs`, `feasibility_probe.rs` (modify)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify)
- `crates/worldwake-ai/src/partial_plan.rs`, `plan_repair.rs` (modify)
- associated `#[cfg(test)]` modules and `search/tests.rs` (modify)
