# Triage — AI Architecture Consolidation, Third Iteration (2026-05-21)

**Source:** `archive/reports/ai-architecture-consolidation-third-iteration-2026-05-22-exploited.md` (ChatGPT-Pro
hostile AI-architecture audit, third iteration). The author **did not clone the
repo** — it used GitHub code search + targeted file fetches. Every load-bearing
claim was re-verified against the actual tree before acceptance. The decisive lens
was FND-14A: does a belief-view accessor gate a world read on *co-location*
(lawful for physical facts) or merely on `knows_entity` / nothing (unlawful for
social/legal/contention facts)?

## Verdict

Accept the report's recommended **Option B (moderate consolidation)** in narrowed
form. Reject its two heaviest "Critical" proposals as over-engineering not required
by FND-14B. Three deliverables.

## Accepted

- **`archive/specs/S162-belief-view-source-gate-hardening.md`** — closes the confirmed
  FND-14/14A leaks in `PerAgentBeliefView`: `has_control` (no gate), `record_data`/
  `office_data` (live institutional truth on `knows_entity`), the no-gate
  contention reads (`actor_can_claim_extraction_slot`, `has_extraction_queues`,
  `facility_queue_join_tick`, `reservation_conflicts`, `reservation_ranges`),
  `loyalty_to`/`stock_storage_policy` (`knows_entity`→`believed_entity`), and the
  `believed_rights`/`can_control` owner/possessor probes. Adds adversarial
  belief-wall goldens and a snapshot-through-view invariant test. Completes the
  social/control path S158 deferred.
- **`archive/specs/S163-cli-player-pov-boundary.md`** — FND-19: stops the player CLI path
  (`actions.rs` omniscient target names; global `handle_cancel`) from leaking world
  truth; marks `display.rs`/`control.rs` observer/debug-only with an enforceable
  guard; adds a player/AI symmetry test.
- **`archive/tickets/CAUSEVTHON-001-explicit-no-source-event.md`** — replaces the implicit
  `EventId(0)` "no source event" sentinel in `Blocker`/`DiscrepancyEntry` with
  `Option<EventId>`. Reassessment found the report undercounted (60+ production
  sites, not 5; `agent_tick/execution.rs` already branches on `== EventId(0)`).

## Dismissed (verified or over-engineered)

- **Field-level snapshot source typing (`SnapshotFieldSource` on ~50 fields)** —
  report's #1 "Critical." `planning_snapshot.rs` has **zero** direct `world.` reads;
  every field flows through `view.*`. The snapshot is lawful by construction once
  the view is lawful (S162 Deliverable 6 locks this). FND-14B requires preserving
  source classification, not per-field *types*. Same rejection the second iteration
  made.
- **Capability-trait split of `RuntimeBeliefView`** — every confirmed leak closes
  by fixing the accessor body; the split fixes no leak the method fixes don't.
  Option-C churn, not required.
- **`InsertVerification` "incomplete repair"** — honestly staged under S139 with a
  documenting test (`insert_verification_returns_no_epistemic_substrate_without_s139`).
  Not a defect.
- **`merchandise_profile`, `visible_reward_encumbrance`, `factions_of`-for-others
  flagged as leaks** — verified **already correctly belief/self-gated**. No action.
- **HTN "rename to method-guided search" / method-required / portfolio dead-code /
  Floyd-Warshall scaling** — not belief-correctness; the report itself recommends
  deferring. Out of this wave.

## Follow-ups identified, not actioned

- Whether `record_data`/`office_data` consumers need a new believed-record snapshot
  substrate (vs. existing institutional-belief accessors) is an S162 ticket-time
  question; if substantial it becomes its own spec (do not expand S162 into a new
  institutional-belief system).
- A richer `pov_display.rs` / `CharacterPovView` is deferred until a real player UI
  exists (S163 does the minimal POV-safe-label fix only).
