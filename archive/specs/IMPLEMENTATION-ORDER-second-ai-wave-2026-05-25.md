# Implementation Order

**Status**: COMPLETED

The retired phase-gate dependency graph and the prior AI-architecture
consolidation waves (S155–S168) are recorded at
`archive/specs/IMPLEMENTATION-ORDER-final-2026-05-21.md` and the dated archives
it references (most recently `archive/specs/IMPLEMENTATION-ORDER-2026-05-25.md`,
which closed out the first AI-architecture-improvements wave S165–S168). This
archived file carried the **second AI-architecture-improvements wave** — the
post-S168 work that broadens lawful verification beyond `AskWitness` and closes
the confirmed learned-state provenance gaps. Gameplay specs `S60`–`S66`
remain authored but **intentionally excluded** until a future directive reopens
them.

## Adjunct Wave: AI Architecture Improvements — Second Iteration

**Source.** `reports/ai-architecture-improvements-third-iteration.md` —
ChatGPT-Pro's third-iteration AI-architecture improvement proposal, run against
current `main` SHA `de0992f3`. Every load-bearing factual claim in the report
verified accurate against the actual tree (12 of 14 claims CONFIRMED; the two
non-confirmations were the *absence* of pre-built abstractions, not refuted
existence claims). The triage therefore turned on benefit, not correctness: 2
of 5 proposals accepted as new specs, 1 reaffirmed as already-shipped (S160),
2 dismissed with cited rationale. The accepted set advances from "the
verification axis exists, but only for AskWitness" (S165 completed) to "the
verification axis is polymorphic across AskWitness, ConsultRecord, and
SearchPlace" + closing the concretely-verified provenance gaps in
learned-state stores. Full triage record:
`docs/triage/2026-05-25-ai-architecture-improvements-third-iteration-triage.md`.

```
S169 (generalized lawful verification substrate)  ── independent; broadens S165's repair axis to ConsultRecord/SearchPlace
S170 (learned-state provenance hardening)         ── independent; closes confirmed gaps in S109/S151 stores
```

Both are independent and may be implemented in any order or in parallel.
Recommended priority follows benefit: **S169 first** (highest-leverage seam
identified in two consecutive iterations of the report), then **S170**
(provenance hygiene, no behavior change).

### Completed

- **S169 — Generalized Lawful Verification Substrate** —
  `archive/specs/S169-generalized-lawful-verification-substrate.md`. Introduced
  a fixed three-provider verification registry (AskWitness, ConsultRecord,
  SearchPlace) at the plan-repair revalidation seam; extended repair decision
  traces and `RepairAppliedPayload` with provider selection; landed
  provider-specific seam proofs, negative omniscience seam proof, and the
  existing S165 AskWitness parity assertion. Goal-level agenda-companion
  polymorphism remains explicitly out of scope. **FND-1, FND-7, FND-14,
  FND-14A, FND-14B, FND-15, FND-16, FND-17, FND-18, FND-20, FND-21, FND-28,
  FND-29, FND-29A, FND-31.**

- **S170 — Learned-State Provenance Hardening** —
  `archive/specs/S170-learned-state-provenance-hardening.md`. Closes four
  provenance gaps: `LearnedOpportunityMemory::OpportunityEntry.source`,
  `RoutePreference::record_safe` event provenance,
  `DiscrepancyEntry::source`, and `Blocker::source`. Pure provenance
  enrichment; no behavior change, no new abstractions, no unified
  `LearnedStateUpdate` trait. **FND-3, FND-22A, FND-26, FND-28, FND-29,
  FND-29A.**

### Pending

No active specs remain in this implementation order.

## Excluded from this order (by directive)

- **S60–S66** (gameplay/world-dynamics specs) — authored, held until core AI
  architecture is stabilized. Do not schedule against this wave.

## Outcome

S169 and S170 completed and archived on 2026-05-25.
This implementation order was archived on 2026-05-25 after its queue reached
`No active specs remain`.
