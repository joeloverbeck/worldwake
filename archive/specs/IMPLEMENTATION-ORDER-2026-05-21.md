# Implementation Order

**Status**: COMPLETED

The former phase-gate dependency graph is retired at
`archive/specs/IMPLEMENTATION-ORDER-2026-05-20.md`. This file tracks the **active** spec waves
only. Core AI architecture is being stabilized first; gameplay specs `S60`–`S66` remain authored
but are **intentionally excluded** from the active order until the AI architecture is solid.

## Adjunct Wave: AI Architecture Consolidation

**Source.** `reports/ai-architecture-consolidation-first-iteration.md` — a hostile AI-architecture
audit (ChatGPT-Pro). Every load-bearing claim was re-verified against the codebase before
acceptance. The audit's verdict ("keep the core, tighten authority boundaries"; recommended
Option B — Moderate Reshaping) was accepted in narrowed form. Findings that did not survive
verification were dismissed (see `docs/triage/2026-05-20-ai-architecture-consolidation-triage.md`):
candidate-generation "concern-mixing" is already cleanly read/write-phased; ranking "magic
numbers" are profile-driven and documented (not FND-2 violations); goal-semantics consolidation
is a smell, not a correctness bug, and is the audit's own deferred Option C.

Accepted work is the genuine, FOUNDATIONS-aligned subset: the confirmed FND-14A belief-boundary
leak, and the FND-28/FND-20/FND-29 HTN honesty cleanup.

```
S155 (belief-view boundary correctness)   ── COMPLETED
S156 (HTN authority honesty)              ── COMPLETED
S157 (snapshot admission provenance)      ── COMPLETED
```

### Completed

- **S155 — Belief-View Boundary Correctness** —
  `archive/specs/S155-belief-view-boundary-correctness.md` — *Status: COMPLETED.* Fixed the
  confirmed FND-14A remote-truth leak in `PerAgentBeliefView::effective_place()` and the
  un-gated belief-facing `can_control()` path, then landed belief-boundary golden coverage and
  the planner-contract documentation.

- **S156 — HTN Authority Honesty** —
  `archive/specs/S156-htn-authority-honesty.md` — *Status: COMPLETED.* Stripped the
  `GoalSchema.methods` fossil (FND-28), the no-op `AgentRole` gate, the two dead
  methods + unused `EntityCriterion` variants, and the three unenforced `MethodSchema` fields;
  made strategic fallback explicit and traced; folded an HTN drafting checklist into
  `docs/spec-drafting-rules.md`; and documented the method-trace fallback/rejection contract in
  `docs/planner-contracts.md`.

- **S157 — Planner Snapshot Admission Provenance** —
  `archive/specs/S157-planner-snapshot-admission-provenance.md` — *Status: COMPLETED.*
  Landed bounded snapshot admission provenance: per-entity admission-source tagging,
  source-restricted strategic scans, and opportunity-scoped snapshot-admission decision traces.
  This remains a defense-in-depth/provenance slice, not the audit's heavier `PlannerVisible<T>` +
  four-trait-split + "never pass `&World`" proposal.

## Excluded from the active order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, but held until core AI architecture is
  stabilized. Do not schedule against this wave.

## Outcome

Completed: 2026-05-21

What changed:
- The final active AI architecture consolidation wave completed: S155, S156, and S157 are all
  archived as completed specs.
- This active implementation-order artifact is retired and archived as
  `archive/specs/IMPLEMENTATION-ORDER-2026-05-21.md`.

Deviations from original plan:
- None in this closeout pass. The file already recorded the accepted narrowed audit scope and the
  completed S155-S157 outcomes before archival.

Verification:
- Manual archival check: `specs/IMPLEMENTATION-ORDER.md` moved to the dated archive path and the
  original active path no longer exists.
