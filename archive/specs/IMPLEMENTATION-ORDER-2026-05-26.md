# Implementation Order

**Status**: ✅ COMPLETED

The post-consolidation AI-architecture wave (S165–S168) completed and was archived to
`archive/specs/IMPLEMENTATION-ORDER-2026-05-25.md`. That order intentionally excluded
the gameplay/world-dynamics specs `S60`–`S66` ("authored, held until core AI
architecture is stabilized"). This file records the **gameplay backlog**, in the same
held disposition, with the addition of the Cluster 1 collision-proof specs derived
from the 2026-05-25 ChatGPT-Pro Cluster 1 report. Implementation of any spec in this
file requires an explicit user directive lifting the gameplay hold; until then, all
specs remain authored-but-deferred.

## Adjunct Wave: Cluster 1 Embodied Self-Care Collision Proof

**Source.** `reports/cluster-1-gameplay-mechanics-improvements-first-iteration.md` —
a ChatGPT-Pro Cluster 1 (homeostatic needs / self-care / facilities) improvement
analysis at `main` SHA `a83cd87617a48e767c2bd53abd66117367cf4b6f`. The author fetched
files directly from the SHA. All 16 load-bearing factual claims verified accurate
against current `main`. The triage turned on benefit, not correctness: 6 of the
report's P0/P1 proposals accepted (split into two specs against the report's
"one-spec/two-slice" recommendation, to match project per-spec-per-concern
convention); P1.3 / P1.4 narrowed; P2.* deferred. Dismissals and rationale:
`docs/triage/2026-05-25-cluster-1-gameplay-mechanics-triage.md`.

```
S172 (Wash discovery + budget closure)         ── completed and archived; planner + scenario contract only
S173 (Self-care interruption + occupancy)      ── completed and archived; depended on archived S172
```

S173 depended on archived S172. Both Cluster 1 adjunct specs are complete.

### Completed

- **S172 — Wash Discovery and Budget Closure** —
  `archive/specs/S172-wash-discovery-budget-closure.md` — *Status: COMPLETED.*
  Closed the known `Wash` budget-exhaustion exclusion in `survival-scattered` and
  `survival-contested`; audits the Wash candidate-enumeration path against FND-14B;
  pinned the four lawful Wash decision-trace branches (Completed, BudgetExhausted,
  BeliefDisconfirmed, NoCandidate); generalized the belief-only Wash regression to
  scattered/contested topologies; added a single CLI player-POV assertion against
  remote-basin-state leak. No new authoritative state. **FND-3/7/8/14/14A/14B/16/19/26/29A/31.**

- **S173 — Self-Care Interruption Contracts and Facility Occupancy** —
  `archive/specs/S173-self-care-interruption-occupancy.md` — *Status: COMPLETED.*
  Defined the interruption contract per self-care action family (eat, drink, sleep,
  toilet, wilderness-relief, wash); introduced `SelfCareOccupancy` on `WashBasin`
  and latrine `Place`; replaced `abort_noop` with `abort_release_self_care_occupancy`
  (Wash, Toilet) and `abort_emit_self_care_interrupted` (Eat, Drink, Wilderness);
  extended `promotable_contention_kind` to classify Wash and Toilet as exclusive use;
  layered `SelfCareInterrupted` trace detail uniformly above all six families; proved
  repeated-interruption deprivation collapse end-to-end via Scenario E.
  **FND-1/3/4/8/9/10/11/19/21/26/28/29/29A/31.**

## Held gameplay backlog (Cluster 0 → world-dynamics deepening)

Held authored gameplay specs from the prior wave remain in `specs/` awaiting
directive. None of the below participate in any active implementation order:

- **S60** — Persistent Site Occupancy (`specs/S60-persistent-site-occupancy.md`)
- **S61** — Predator Ecology and Dens (`specs/S61-predator-ecology-dens.md`)
- **S62** — Boundary Processes and Remote Shocks (`specs/S62-boundary-processes-remote-shocks.md`)
- **S63** — Contested Evidence and Warrants (`specs/S63-contested-evidence-warrants.md`)
- **S64** — Scarcity Response and Debt Rationing (`specs/S64-scarcity-response-debt-rationing.md`)
- **S65** — Social Aftermath Memory (`specs/S65-social-aftermath-memory.md`)
- **S66** — Settlement Decline and Reoccupation (`specs/S66-settlement-decline-reoccupation.md`)

## Activation

When the gameplay hold is lifted, the remaining Cluster 1 adjunct wave work is
**S173**; it depends on archived S172's budget accounting being sound. Ordering
against `S60`–`S66` is a separate decision for the activating directive.

## Outcome

- **Completion date**: 2026-05-26.
- **Cluster 1 adjunct wave finished.** Both adjunct specs completed and archived:
  - S172 — Wash Discovery and Budget Closure → `archive/specs/S172-wash-discovery-budget-closure.md`.
  - S173 — Self-Care Interruption Contracts and Facility Occupancy → `archive/specs/S173-self-care-interruption-occupancy.md`.
- **Gameplay backlog (S60–S66) disposition unchanged.** Those specs remain authored-but-held in `specs/` per the original "authored, held until core AI architecture is stabilized" framing. They are not part of any active implementation order; lifting that hold is a separate directive that will produce its own ordering file when warranted.
- **Deviation from original plan**: none. The file's own activation clause anticipated archival "when the wave completes or is superseded" — the Cluster 1 wave is complete, so archival now matches the documented exit condition.
- **Verification**: `specs/IMPLEMENTATION-ORDER.md` listed S172 and S173 as the only active wave items; both are archived with COMPLETED status. No remaining active directives reference this file. `CLAUDE.md` already cites `archive/specs/IMPLEMENTATION-ORDER-final-2026-05-21.md` as the final implementation-order authority, so no documentation update is required.
