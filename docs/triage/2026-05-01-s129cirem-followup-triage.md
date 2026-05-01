# Triage: S129CIREM follow-ups (2026-05-01)

## Source

`/brainstorm` triage of commits `2188b23a..f4480a9b` (S129CIREM-001..004),
which remediated post-S129 CI failures across `golden_survival_drive_escalation`,
`golden_survival_baseline`, `golden_survival_contested`,
`golden_survival_scattered`, `golden_survival_tell`, and
`golden_survival_patrol`.

Question: are the CIREM fixes hacks/workarounds masking architectural
gaps?

Verdict: **no on the per-ticket fixes; yes on three substrate patterns
they revealed.** Each CIREM ticket fixed a genuine root cause without
weight-knob bypasses, contract relaxations, or force-X-when-Y
carve-outs. But three of the four expose deeper architectural patterns
that the per-ticket scope did not generalize, and that will keep
producing one-off CIREM-style fixes if not addressed.

## Accepted (deliverables created)

- **S132** — Frontier-Exhaustion Strategy as Goal-Kind Property →
  `specs/S132-frontier-exhaustion-strategy.md`. Smell: CIREM-002 added
  self-consume `AcquireCommodity` to a hand-maintained allow-list in
  `frontier_exhaustion_entry`; CIREM-004 added `Patrol`. Default is
  permanent suppression. Substrate should be a goal-kind property,
  not an enumerated switch. FND-21 / FND-22A.

- **BELASPCOV-001** — `BelievedEntityState` ↔ `EntityBeliefAspect`
  coverage audit → `archive/tickets/BELASPCOV-001-believed-entity-state-claim-aspect-coverage.md`.
  Smell: CIREM-003 had to add `WashBasinState` as a missing
  `EntityBeliefAspect` and bump save-format to 58 because the field
  existed on the summary without claim backing, causing chronic
  stale-claim decay loss. Audit looks for other fields with the same
  gap.

- **INFRARET-001** — Generalize direct-observed concrete-opportunity
  retention →
  `tickets/INFRARET-001-generalize-direct-observed-infrastructure-retention.md`.
  Smell: CIREM-003's `state_salience_boost` hardcodes two shape pairs
  (wash-basin-with-state, resource-source-with-workstation-tag).
  Future opportunity infrastructure inherits no retention until added
  to the switch. Soft depends on BELASPCOV-001.

- **RELIEFACT-001** — Extract per-need relief-actionability predicate
  → `tickets/RELIEFACT-001-per-need-relief-actionability-predicate.md`.
  Smell: CIREM-002 added `if need_id == HomeostaticNeedId::Dirtiness`
  branch in `emit_exploration_candidates`. Per-need relief substrate
  should be declarative, not enumerated.

- **LOCROOT-001** — Audit direct-root synthesis for `EntityAtActorPlace` /
  `ActorPlace`-precondition arms →
  `tickets/LOCROOT-001-direct-root-synthesis-locality-audit.md`.
  Smell: CIREM-003 found that `PlannerOpKind::Wash` synthesis emitted
  non-local roots; fixed with an explicit locality gate. Trade and
  Harvest arms have the same target-spec but no synthesizer-side
  locality gate. Audit determines per-arm whether upstream filtering
  is the canonical guard or whether a synthesizer gate is missing.

## Dismissed

- **CIREM-001 scenario re-narrative**: drive-escalation scenario shifted
  from "all-needs envelope" to "self-care-family exercise + repeated
  wash" with hunger/thirst critical-run overrides. Rejected as a
  follow-up because the ticket's reassessment proved the original
  arithmetic could not satisfy `Drink` under any escalation path
  (`thirst_weight: 100 * pressure 1000 * max_multiplier 3000 / 1000 = 300 < hunger 750`).
  Renarrating an internally-inconsistent contract is correct, not a
  relaxation.

- **Diagnostics-driven candidate emission feedback (CIREM-002)**:
  `fully_blocked_desires` → fallback exploration is a clean two-pass
  structure that doesn't yet need formalization. Revisit if a third
  use case emerges.

## Placement

Added as `### Adjunct Wave: S129 Post-Remediation Architectural Audit`
under `## Phase 10: Survival Mechanic Depth` in
`specs/IMPLEMENTATION-ORDER.md`. Phase 10 is the home of S128/S129;
this wave is the follow-up to that work.
