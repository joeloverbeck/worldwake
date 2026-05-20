# Implementation Order

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
S156 (HTN authority honesty)              ── priority 2, independent of S155 (parallelizable)
S157 (snapshot admission provenance)      ── DEFERRED, depends on S155, NOT scheduled
```

### Completed

- **S155 — Belief-View Boundary Correctness** —
  `archive/specs/S155-belief-view-boundary-correctness.md` — *Status: COMPLETED.* Fixed the
  confirmed FND-14A remote-truth leak in `PerAgentBeliefView::effective_place()` and the
  un-gated belief-facing `can_control()` path, then landed belief-boundary golden coverage and
  the planner-contract documentation.

### Active

- **S156 — HTN Authority Honesty** — `specs/S156-htn-authority-honesty.md` — *Status: DRAFT.*
  Strips the `GoalSchema.methods` fossil (FND-28), the no-op `AgentRole` gate, the two dead
  methods + unused `EntityCriterion` variants, and the three unenforced `MethodSchema` fields;
  makes strategic fallback explicit and traced; folds an HTN drafting checklist into
  `docs/spec-drafting-rules.md`. Independent of S155 (different files) and may proceed in
  parallel.

### Deferred (written, NOT in active order)

- **S157 — Planner Snapshot Admission Provenance** —
  `specs/S157-planner-snapshot-admission-provenance.md` — *Status: DRAFT — DEFERRED.*
  Defense-in-depth + provenance/debuggability (FND-15, FND-29): admission-source tagging for
  snapshot entities and source-restricted strategic scans. Depends on S155 (which removes the
  underlying leak at the source). Schedule deliberately only after S155 lands and the team
  chooses to invest in snapshot-level provenance. A bounded alternative to the audit's heavier
  `PlannerVisible<T>` + four-trait-split + "never pass `&World`" proposal, which is not adopted.

## Excluded from the active order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, but held until core AI architecture is
  stabilized. Do not schedule against this wave.
