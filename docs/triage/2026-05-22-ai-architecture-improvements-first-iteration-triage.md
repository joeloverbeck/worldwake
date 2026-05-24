# Triage — AI Architecture Improvements, First Iteration (2026-05-22)

**Source:** `reports/ai-architecture-improvements-first-iteration.md` (ChatGPT-Pro
deep-research proposal on AI-architecture alignment with `docs/FOUNDATIONS.md`).
The report **explicitly declined to read archived specs** ("Archive search hits
appeared during discovery, but I did not fetch or use archived files as
evidence"). Every load-bearing claim was re-verified against the actual tree
(verification: belief-view accessors, snapshot construction, CLI affordance path,
`EntityBeliefAspect` variants).

## Verdict

**No new specs warranted.** Despite a different title, this is effectively a fifth
iteration of the hostile AI-architecture audit series. Its 8 proposals map ~1:1
onto the completed S155→S164 belief-boundary wave (S158 landed 2026-05-21, one day
before this report) and onto the four prior consolidation-iteration triages
(2026-05-20 → 2026-05-22). Code claims were technically accurate as descriptions
but missed the documented design rationale — the same blind spot the report's
no-archive methodology guarantees. The one genuinely-open item (believed-rights
aspect) stays deferred per the consistent prior position; the user confirmed this
outcome at triage.

## Accepted

- None.

## Reaffirmed (no new spec)

- **P1 control/rights live read** (`believed_rights`/`can_control` call
  `world.effective_rights`/`can_exercise_control`, `per_agent_belief_view.rs:427-445`)
  — verified present, but **behind a self/belief-accessibility gate**: a
  deliberate, documented S155/S158/S162-D5 decision, dismissed as "not a
  regression" in the 2nd/3rd/4th triages. The stricter FND-14A reading (rights
  *values* belief-backed) requires a net-new `Rights`/`Control`/`Jurisdiction`
  `EntityBeliefAspect` — confirmed absent (`entity_belief_claim.rs:17` carries
  `Owner`/`Holder`/`ContentionState`/`Activity`/`Inventory` only). S158 named this
  as legitimate future work; repeatedly deferred. **Remains deferred.**
- **P3 unified player/AI POV affordance** — exactly `archive/specs/S163`
  (implemented + archived). CLI uses the controlled agent's actual belief store and
  shares the gated view with AI; not omniscient.
- **P8 formalism responsibility boundaries** — `archive/specs/S156`/`S160`
  (HTN authority honesty) + `docs/planner-contracts.md` already codify this.

## Dismissed (re-litigation or verified lawful)

- **P1 economic/production/physical/contention leaks** — closed by `S158`
  (all five tickets); accessors gate on co-location or belief.
- **P1 capability-trait split (remove `&World` from the view)** — rejected as
  Option-C churn in the 2nd/3rd/4th triages. The view lives in `worldwake-sim`, the
  lawful observation/dispatch layer that *is* allowed `World`.
- **P1 `direct_container`/`direct_possessor`** — `S158` line 49 explicitly verified
  these as already correctly gated.
- **P2 per-field `PlannerField<T>` / `SnapshotFieldSource` typing** — `S157`
  declined the static source-typing refactor; `planning_snapshot.rs` has zero
  direct `world.` reads, lawful-by-construction via the S162 snapshot-through-view
  invariant. Dismissed in 2nd/3rd/4th triages.
- **P4 false/stale/contradicted belief discipline** — exists via the
  `belief_wall_trap` golden family and S158's no-leak goldens.
- **P5 agent diversity / learning / habits / doctrine** — substrate exists (the
  report concedes this); `S151` (testimony reliability + route preferences) and
  `S152` (cognitive archetypes, seeded diversity) cover it. Remaining "scenario
  coverage" concern is governed by the existing scenario-profile-completeness
  invariant (`docs/spec-drafting-rules.md` §5), not a spec.
- **P6 field-source / affordance-legality traces** — depends on P2; deferred with it.
- **P7 perf/scaling guards** — no confirmed defect; the report itself recommends
  deferring (consistent with the 4th triage's Floyd-Warshall dismissal).

## Follow-ups identified, not actioned

- The believed-rights/control/jurisdiction `EntityBeliefAspect` (stricter FND-14A
  rights-*value* belief-backing) remains the single real future option. If a future
  iteration revisits it, scope it as net-new belief infrastructure + a believed-
  rights surface — not as a bug fix, and not via the rejected capability-trait split.
- Future external audits should be instructed to read `archive/specs/` and
  `docs/triage/` first, to stop re-litigating the settled S155→S164 wave.
