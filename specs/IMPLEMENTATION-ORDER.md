# Implementation Order

**Status**: ACTIVE

The retired phase-gate dependency graph and the prior AI-architecture consolidation
waves (S155–S164) are recorded at
`archive/specs/IMPLEMENTATION-ORDER-final-2026-05-21.md` and the dated archives it
references (most recently `archive/specs/IMPLEMENTATION-ORDER-2026-05-22.md`). This
active file carries the first **AI-architecture improvements** wave — the
post-consolidation work that shifts from belief-boundary hardening to acting better
under lawful uncertainty. Gameplay specs `S60`–`S66` remain authored but
**intentionally excluded** until a future directive reopens them.

## Adjunct Wave: AI Architecture Improvements — First Iteration

**Source.** `reports/ai-architecture-improvements-second-iteration.md` — a ChatGPT-Pro
AI-architecture improvement proposal "redone against current `main`" (SHA `12813246`),
fetching files directly from the SHA rather than relying on stale GitHub code search.
Unlike the first iteration (`reports/ai-architecture-improvements-first-iteration.md`),
which re-proposed the already-rejected capability-trait split / per-field
`SnapshotFieldSource` typing on stale content, every load-bearing claim in the second
iteration **verified accurate** against the actual tree. The triage therefore turned on
benefit, not correctness: 4 of 8 proposals accepted (one narrowed), 4
dismissed/deferred. The accepted set moves the architecture from "fix the belief leak"
(complete: S162/S163/S164) to "act better under stale/false/partial belief, and prove
it." Dismissals and rationale:
`docs/triage/2026-05-22-ai-architecture-improvements-second-iteration-triage.md`.

```
S165 (epistemic verification repair)        ── independent; bridges completed S137 (repair) + S139 (AskWitness goal)
S166 (opportunity compiler source fidelity) ── independent; refines completed S138 compiler
S167 (cognitive archetype behavioral proof) ── independent; builds on completed S152 archetypes
S168 (partial-plan skeleton reuse)          ── independent; activates dead field from completed S149
```

All four are independent and may be implemented in any order or in parallel.
Recommended priority follows benefit: **S165 → S167 → S166 → S168.**

### Completed

- **S165 — Epistemic Verification Repair** —
  `archive/specs/S165-epistemic-verification-repair.md` — *Status: COMPLETED
  2026-05-24.* Replaced the permanently-failing `RepairKind::InsertVerification`
  placeholder with a co-located `AskWitness` verification repair path for
  belief-backed breaches, authoritative witness-anchor recording, payload
  revalidation coverage, and plan-repair golden coverage for the witness and
  no-witness branches.

### Pending

- **S167 — Cognitive Archetype Behavioral Proof Lane** —
  `specs/S167-cognitive-archetype-behavioral-proof.md` — *Status: DRAFT.* Adds the
  missing causal proof (FND-31) that archetypes change *decisions*, not merely
  resolved profile values (which S152 already proves): a behavioral-divergence golden
  for two same-role/same-belief agents differing only by archetype, plus archetype
  activation in a canonical `scenarios/*.ron` so `scenario-coverage.md` no longer shows
  the feature absent. **FND-20, FND-22, FND-22A, FND-29, FND-31.**
- **S166 — Opportunity Compiler Source Fidelity** —
  `specs/S166-opportunity-compiler-source-fidelity.md` — *Status: DRAFT.* Derives the
  compiled opportunity's `source_belief.status` from the real belief (not the
  hard-coded `Probable` at `compile.rs:222`) and `required_actions` from
  `EffectSchemaIndex` (not the hard-coded `MoveCargo` at `compile.rs:127`). Narrowed
  from the report's "canonical substrate" ambition to source fidelity only; absorbs
  Proposal 5. **FND-3, FND-15, FND-16, FND-27, FND-29.**
- **S168 — Partial-Plan Skeleton Reuse** —
  `specs/S168-partial-plan-skeleton-reuse.md` — *Status: DRAFT.* Populates and
  consumes the dead `remaining_skeleton` field (`partial_plan.rs:36,123`) for
  information and search-budget barriers, with mandatory belief revalidation before
  reuse so a skeleton seeds search but never authorizes a stale action. Lowest benefit
  of the wave (an optimization over already-working resume). **FND-20, FND-21, FND-26,
  FND-27, FND-29.**

## Excluded from this order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, held until core AI
  architecture is stabilized. Do not schedule against this wave.
