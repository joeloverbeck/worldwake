# Triage — AI Architecture Consolidation, Fourth Iteration (2026-05-22)

**Source:** `reports/ai-architecture-consolidation-fourth-iteration.md` (ChatGPT-Pro
hostile AI-architecture audit, fourth iteration). The author **did not clone the
repo** and ran no tests — the leak inventory's "Evidence" column is empty. Every
load-bearing claim was re-verified against the actual tree using FND-14A as the lens
(co-location-gated physical reads are lawful; `knows_entity`-gated social/legal/
behavioral reads are not).

## Verdict

~85% of the report re-litigates decisions already made and documented in S155/S157/
S158/S162 or already pending as S163. Stripped of re-litigation, **one genuinely new,
confirmed leak** survives (residual live entity-kind reads), plus two latent footguns
closed alongside it. One new spec; reaffirm the pending S163 as higher priority.

## Accepted

- **`archive/specs/S164-belief-view-kind-source-gate.md`** — closed the confirmed residual
  leak: `entity_kind` (`per_agent_belief_view.rs:604-609`) and the last-seen belief
  synthesis (`:1293-1304`, `believed_kind: self.world.entity_kind(*entity)`) had read live
  `world.entity_kind` for remote, non-co-located entities while location/aliveness are
  correctly frozen — an internal inconsistency S158/S162's accessor sweep missed
  because it lives in a belief-construction path, not a named accessor.
  `LastSeenRecord` (`expectation.rs:126-132`) had stored no kind, so the synthesis reached
  for live world. S164 now routes remote kind through stored belief / a last-seen
  observed-kind carrier, gates the former ungated bandit faction-policy accessors
  (`:611-621`) to lawfully known factions (footgun: today's call sites pass own
  factions, so behavior is unchanged), adds a `facility_controller_at` (`:385-401`)
  remote-control-change confirming test, and extends the S162 belief-wall goldens with
  a remote-kind-divergence scenario.

## Reaffirmed (no new spec)

- **`archive/specs/S163-cli-player-pov-boundary.md`** — the report's "Critical: CLI action
  menu inherits the leak" finding is exactly S163, drafted in the third iteration and
  since implemented and archived. It remains the prerequisite boundary before S164;
  no new fourth-iteration spec is needed for the same CLI finding.

## Dismissed (re-litigation or verified lawful)

- **`PerAgentBeliefView` holds `&World` → capability-trait split** (report Critical) —
  rejected in the second and third iterations as Option-C churn. The view lives in
  `worldwake-sim`, the lawful observation/dispatch layer, which is *allowed* `World`.
  No leak the accessor fixes don't already close.
- **Per-field `SnapshotFieldSource` typing** (report Critical/High) —
  `planning_snapshot.rs` has **zero** direct `world.` reads; lawful by construction
  once the view is lawful, locked by the S162 Deliverable-6 snapshot-through-view
  invariant. Same rejection the second and third iterations made.
- **`believed_rights` / `can_control` read live `world.effective_rights` /
  `can_exercise_control`** (report Critical) — S162 Deliverable 5 and its source-class
  table deliberately permit the live read **behind a self/belief-accessibility gate**;
  current code (`:428-445`) matches exactly. Not a regression.
- **`direct_container` / `direct_possessor` remote custody** (report High) — S158
  (line 49) explicitly verified these as "already correctly gated."
- **`merchandise_profile`, reward encumbrance, `factions_of`-for-others** (report
  High/Med) — third-iteration triage verified already correctly belief/self-gated.
- **Bandit faction-policy reads ungated** (report Med) — the accessor lacks an
  internal gate, but every planner-visible call site passes `bandit_factions_of(actor)`
  (own/believed factions), so the reads are lawful self-state today. Closed as a
  *latent footgun* in S164 Deliverable 3, not as an active leak.
- **HTN `RequiredActionLeaf` / portfolio second-ranking / Floyd-Warshall scaling**
  (report Med) — not belief-correctness; the report itself recommends deferring.
- **CI grep-gate banning `World` in `worldwake-ai`** (report enforcement) —
  low-benefit: the AI crate is already proven free of direct world reads by the
  snapshot-through-view invariant test, and the legitimate `world.` reads are in
  `worldwake-sim`'s view (allowed `World`). Re-add only on a demonstrated regression.
- **`BeliefLastSeen` admits physical fields in strategic search**
  (`search/strategic.rs:945-951`) — entity-level `AdmissionSource` is S157's
  deliberate design; the fields a `BeliefLastSeen` entity contributes are bounded by
  what the now-lawful view surfaces (lawful by construction), so no separate fix.

## Follow-ups identified, not actioned

- The richer `CharacterPovView` / `pov_display.rs` capability layer remains deferred
  until a real player UI exists (consistent with S163's scoping note).
- If S164 Deliverable 2 takes the preferred mechanism (an `observed_kind` field on
  `LastSeenRecord`), confirm the perception/observation systems populate it at every
  last-seen write site; if that proves broad it becomes an S164 ticket-time note, not
  a new spec.
