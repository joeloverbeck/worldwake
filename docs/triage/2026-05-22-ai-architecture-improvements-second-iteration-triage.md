# Triage — AI Architecture Improvements, Second Iteration (2026-05-22)

**Source:** `reports/ai-architecture-improvements-second-iteration.md` (ChatGPT-Pro,
"redone against current `main`" at SHA `12813246`). The author fetched files directly
from the SHA rather than relying on stale code search.

## Verdict

Unlike the **first** iteration (`reports/ai-architecture-improvements-first-iteration.md`,
triaged separately), which re-proposed the already-rejected capability-trait split and
per-field `SnapshotFieldSource` typing on stale content, **every load-bearing factual
claim in the second iteration verified accurate against current `main`.** It correctly
avoids consolidation re-litigation (S162/S163/S164). The triage question was therefore
benefit, not correctness. 4 of 8 proposals accepted (one narrowed); 4 dismissed/deferred.

Decisive finding for Proposal 1: `RepairKind::InsertVerification` unconditionally
returns `NoEpistemicSubstrate` (`plan_repair.rs:131`). S137 (repair) and S139
(AskWitness goal) are both complete/archived, but **no spec or ticket ever owned the
bridge** between them — `S137PLACAULIN-007` (named in the stale "ticket 007" test
comments) wired `RebindTarget`/`ReplaceProvider`, never the verification axis. The gap
is genuine and unclaimed.

## Accepted

- **`archive/specs/S165-epistemic-verification-repair.md`** (Proposal 1, completed
  2026-05-24) — wired
  `InsertVerification` to splice a lawful co-located `AskWitness` verification step
  for belief-backed breaches; plan-repair golden coverage proves the S165 repair seam.
  Substrate (S137 repair context + S139 goal) exists. Highest-leverage.
  **FND-14/15/16/17/20/21/29.**
- **`specs/S166-opportunity-compiler-source-fidelity.md`** (Proposal 2, **narrowed**) —
  derive `source_belief.status` from the real belief (not hard-coded `Probable`,
  `compile.rs:222`) and `required_actions` from `EffectSchemaIndex` (not hard-coded
  `MoveCargo`, `compile.rs:127`). Absorbs Proposal 5's source-faithfulness intent.
  **FND-3/15/16/27/29.**
- **`archive/specs/S167-cognitive-archetype-behavioral-proof.md`** (Proposal 4,
  completed 2026-05-24) — added the missing behavioral-divergence golden
  (different *decisions*, not just S152's
  resolved-profile-value tests) + activate archetypes in a canonical `scenarios/*.ron`
  so coverage no longer shows them absent. **FND-20/22/22A/29/31.**
- **`specs/S168-partial-plan-skeleton-reuse.md`** (Proposal 3, lowest benefit) —
  populate + consume the dead `remaining_skeleton` field (`partial_plan.rs:36,123`) for
  information/budget barriers; revalidate-before-reuse so it never becomes a rail.
  **FND-20/21/26/27/29.**

## Dismissed / deferred

- **Proposal 2's "canonical opportunity substrate / delete parallel emitters"
  ambition** — scope inflation; the report itself warns against overgeneralizing.
  S166 takes only the source-fidelity subset.
- **Proposal 5 (source-reliability trace tightening), standalone** — folded into S166;
  the source-status faithfulness is the actionable core, the rest is trace polish.
- **Proposal 6 (diagnostics as CI gates)** — premature; the report itself says start
  non-failing. Threshold-gating risks overfitting before stable baselines exist.
- **Proposal 7 (HTN method maturity / required leaves)** — matches the standing,
  repeatedly-reaffirmed "no required leaves until a schema contract proves fallback
  invalid" decision. Trace-only maturity is not worth a spec this iteration.
- **Proposal 8 (perf/causal-equivalence guardrails)** — folded into each accepted
  spec's Section H / Validation rather than a standalone spec; instrument as the
  accepted specs land.

## Follow-ups identified, not actioned

- The other unmapped coverage rows (portfolio weights, intention disposition,
  expectation store, last-seen memory, social observations) — a possible sibling to
  S167 if they share one coverage-detector pattern.
- `ConsultRecord` / `InspectContainer` as verification `GoalKind`s — needed to widen
  S165 beyond `AskWitness`/`ExploreLocation`; deferred with S139's own Non-Goals.
