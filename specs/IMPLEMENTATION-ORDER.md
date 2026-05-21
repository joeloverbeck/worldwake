# Implementation Order

**Status**: ACTIVE

The former phase-gate dependency graph is retired at
`archive/specs/IMPLEMENTATION-ORDER-2026-05-20.md`. The first AI-architecture
consolidation wave (S155–S157) is retired at
`archive/specs/IMPLEMENTATION-ORDER-2026-05-21.md`. This file tracks the **active**
spec waves only. Core AI architecture is being stabilized first; gameplay specs
`S60`–`S66` remain authored but are **intentionally excluded** from the active
order until the AI architecture is solid.

## Adjunct Wave: AI Architecture Consolidation — Second Iteration

**Source.** `reports/ai-architecture-consolidation-second-iteration.md` — the
second hostile AI-architecture audit (ChatGPT-Pro), following the first iteration
that produced S155–S157 (completed). Every load-bearing claim was re-verified
against the codebase before acceptance. The audit's verdict ("keep the
GOAP/action core; harden the belief boundary before adding behavior"; recommended
Option B — Moderate Consolidation) was accepted in narrowed form. The audit's
heavier proposals — `Sourced<T>`/`FieldSource` source-typed view APIs and
per-field snapshot source tags (its own Option C) — were **rejected for now** as
out-of-scope migration risk; the FND-14 safety win is achievable by gating the
leaky view accessors, proven by golden tests.

Findings that did not survive verification were **dismissed** (see
`docs/triage/2026-05-21-ai-architecture-consolidation-second-iteration-triage.md`):
the "container/possessor leak" is already fixed by S155; "per-field snapshot
provenance is absent" is overstated (`build_snapshot_entity` already resolves
fields belief-first); `LEGACY_EXTRACTOR_ORDER` is a naming/authority smell, not a
behavioral leak; "candidate generation mixes emission with anomaly detection" is
already cleanly read/write-phased and is a larger perception-architecture concern
deferred out of this wave.

Accepted work is the genuine, FOUNDATIONS-aligned subset: the confirmed FND-7/14
belief-view remote-truth leaks (the priority), the FND-28 candidate-generation
fossil seam, and the FND-20/29 HTN honesty gaps.

```
S158 (belief-view remote-truth leak closure)        ── completed; extends S155
S159 (candidate-gen schema-owned extractor authority) ── completed; independent of S158/S160
S160 (HTN authority honesty)                          ── extends S156; independent of S159
```

S158 and S159 have landed and are archived. S160 remains an independent cleanup.

### Completed

- **S158 — Belief-View Remote-Truth Leak Closure** —
  `archive/specs/S158-belief-view-remote-truth-leak-closure.md` — *Status:
  COMPLETED.* Closed the confirmed economic, production, physical, and contention
  `PerAgentBeliefView` remote-truth leaks under one source-class rule, restored
  merchant-return coverage through a lawful local-observation rebind, and
  codified the rule in `docs/planner-contracts.md` §2 plus
  `docs/spec-drafting-rules.md`. The social/control rights value path remains
  deferred per S155/S158 scope. Extends S155. **FND-7, FND-14, FND-14A, FND-16,
  FND-19, FND-27, FND-31.**

- **S159 — Candidate-Generation Schema-Owned Extractor Authority** —
  `archive/specs/S159-candidate-generation-schema-owned-extractor-authority.md`
  — *Status: COMPLETED.* Replaced the extractor-order fossil name with
  `CANDIDATE_EXTRACTOR_ORDER`, folded blocked-self-care fallback emission into
  the declared `BlockedSelfCareExploration` post-suppression extractor, preserved
  the phase-local fallback gate, and added transient `CandidateExtractorId`
  provenance diagnostics to guard against out-of-band surviving candidates.
  Behavior-preserving. **FND-20, FND-28, FND-29.**

### Active

- **S160 — HTN Authority Honesty** —
  `specs/S160-htn-authority-honesty.md` — *Status: Draft.* Add
  `MethodSubgoalAuthority::{StageHint, RequiredActionLeaf}` and honest stage-hint
  traces; resolve the fake `fulfill_bounty_group_hunt` method; remove the
  `ActionDefId(u32::MAX)` escort sentinel. No goal becomes method-required.
  Extends S156. **FND-20, FND-29, FND-31.**

## Adjunct Wave: FOUNDATIONS Constitutional Hardening — Gap Audit 2026-05

**Source.** `reports/foundations-gap-audit.md` (ChatGPT-Pro) answered the standing
question *"are the foundations complete?"* during the AI-architecture
consolidation. Verdict: **mostly correct with four targeted strengthenings plus
new canonical scenarios** — no rewrite, no renumbering. Every load-bearing claim
was re-verified against the codebase before acceptance.

The key correction: the audit's headline concern (remote authoritative truth
leaking into planner-visible inputs) is **already closed** by the completed S158;
the source-class rule is shipped in `docs/planner-contracts.md` §2 and
`docs/spec-drafting-rules.md`. The accepted FND-14B addition is therefore
**constitutional anchoring** for that shipped rule (regression-proofing future
planner surfaces), not a leak fix. All five proposals were accepted in narrowed
form; **none rejected**. Forward-looking material with no current backing system
(the FND-12 strengthening, canonical scenarios K/L) is written into the
constitution to set the bar early, while its artifacts
(`docs/causal-equivalence-contracts.md`, K/L goldens) are **deferred** until the
offscreen/boundary/prehistory systems reach the roadmap. The audit's heavier
optional track — splitting FOUNDATIONS into five sub-constitutions — was
**rejected** as churn for no philosophical gain.

```
S161 (FOUNDATIONS constitutional hardening) ── independent of S159/S160; sequence whenever
```

### Active

- **S161 — FOUNDATIONS Constitutional Hardening (Gap Audit 2026-05)** —
  `archive/specs/S161-foundations-constitutional-hardening.md` — *Status:
  Completed and archived.* Strengthen
  FND-12 with explicit causal-equivalence-contract requirements; insert FND-14B
  (planner-visible inputs must be belief-backed or lawful boundary artifacts,
  anchoring the S158 source-class rule); insert an HTN anti-script guard into
  FND-20; replace FND-31 with the systemic-validation doctrine already live in
  `golden-e2e-testing.md` / `scenario-roadmap.md`; add canonical scenarios I–L.
  Anchors planner-contracts/spec-drafting-rules/golden-e2e-testing downstream.
  Doc-only constitutional edit; no new simulation state. K/L artifacts and the
  J / remote-seller HTN-rejection goldens are explicitly deferred to golden-coverage
  work. **FND-12, FND-14, FND-14A, FND-20, FND-27, FND-29A, FND-31.**

## Excluded from the active order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, but held until core AI
  architecture is stabilized. Do not schedule against this wave.
