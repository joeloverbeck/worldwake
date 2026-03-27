# Golden E2E Suite: Coverage Dashboard

**Scope**: `crates/worldwake-ai/tests/golden_*.rs`
**Purpose**: Interpretive coverage analysis for planning new spec coverage.
**Conventions**: For assertion patterns, trace usage, and scenario authoring, see [golden-e2e-testing.md](golden-e2e-testing.md).

---

## Generated Artifacts

The mechanical coverage data is generated from structured source annotations. Do not duplicate it here.

- **Test inventory**: [generated/golden-e2e-inventory.md](generated/golden-e2e-inventory.md) — per-file counts and `golden_*` name lists.
- **Scenario map**: [generated/golden-scenario-map.md](generated/golden-scenario-map.md) — scenario identifiers, titles, metadata, Setup/Proves/Chain prose, and owning tests.
- **Coverage matrix**: [generated/golden-coverage-matrix.md](generated/golden-coverage-matrix.md) — GoalKind, ActionDomain, Systems, Topology, and Foundation Principles tables.

Regenerate/validate all with `python3 scripts/golden_inventory.py --write --check-docs`.

---

## Pending Backlog Summary

**S02c: Multi-Role Emergent Supply Chain** (3 tests: main + replay + conservation) — still blocked on `specs/S10-bilateral-trade-negotiation.md` for the full producer→merchant→consumer combined chain only. The craft-restock prerequisite segment is no longer a gap: `golden_supply_chain.rs` now covers both the harvest-restock segment and the prerequisite-aware craft-restock segment, while the ignored blocked full-chain cases remain the unresolved pricing/negotiation gap.

**S32: Crime Emergence Golden Suites** (6 tests: 3 main + 3 replay companions) — covers three E17-crime mechanics with zero golden coverage: (1) Exile punishment fallback when Fine is infeasible (Scenario 41), (2) witness deterrence suppressing theft candidates via witness_risk_penalty (Scenario 42), (3) dual discovery convergence with duplicate accusation prevention (Scenario 43). See `specs/S32-crime-emergence-golden-suites.md`. Tickets: GOLDE2E-017, GOLDE2E-018, GOLDE2E-019, GOLDE2E-020.

### Recommended Implementation Order

1. S32 crime emergence golden suites
2. S02c multi-role emergent supply chain

---

## Evaluated and Rejected Scenarios

The following scenarios were considered during the 2026-03-14 coverage review and rejected with architectural justification:

1. **Fatigue/Bladder/Dirtiness as interrupt** — `interrupts.rs` branches on `GoalPriorityClass`, not need type. `is_critical_survival_goal()` treats Sleep/Relieve/Wash identically to hunger/thirst interrupts. Same code path as Scenario 2; a fatigue-specific interrupt golden test would exercise no additional logic.

2. **Multi-attacker danger escalation (2v1)** — `attackers.len() >= 2 → CRITICAL` is already unit-tested in `pressure.rs`. The behavioral consequence (defensive response under danger) is already golden-tested via Scenario 7f. The gap between unit coverage and golden coverage is too narrow to justify a high-setup multi-agent combat scenario.

3. **Journey abandonment (vs suspension)** — `AbandonsCommitment` classification is already unit-tested in `decision_runtime.rs`. High setup complexity (must engineer a scenario where the original destination becomes permanently unreachable or irrelevant mid-journey) for limited code path difference from Scenario 3c's suspension/reactivation path.

4. **SellCommodity** — `GoalKind::SellCommodity` variant exists but `candidate_generation.rs` lacks sell-specific emission logic. Not testable as a golden scenario without first implementing new system code to generate sell candidates.

5. **Self-treatment through ordinary `heal`** — (2026-03-14: rejected. 2026-03-18: implemented.) S07 unified care model made self-treatment lawful via `TreatWounds { patient: self }`. Now golden-tested in Scenario 2c (`golden_self_care_with_medicine`, `golden_self_care_acquires_ground_medicine`).

---

## Removed Backlog Items

Items removed from the golden backlog with rationale (prevents duplicate coverage proposals):

- **E17CRITHEJUS-012 Theft creates owner-local `EntityMissing` -> `SuspectedTheft` discovery** (removed 2026-03-26) — Implemented as Scenario 37 in `golden_emergent.rs` (`golden_theft_leads_owner_to_local_suspected_theft_discovery` plus deterministic replay). The suite now covers the canonical owner-local crime-discovery chain end to end: thief steals and departs with the lot -> owner later returns lawfully to the stash -> stale place belief produces `EntityMissing` -> `investigate` resolves into typed `SuspectedTheft` evidence with `suspect: None`. This removes the golden gap for the owner-discovery branch while leaving witness-tell, accusation, and punishment coverage to later E17 tickets.

- **E17CRITHEJUS-013 Witnessed theft enables accusation chain** (removed 2026-03-27) — Implemented as Scenario 38 in `golden_emergent.rs` (`golden_witnessed_theft_accusation_chain` plus deterministic replay). The suite now covers the witness-driven justice route end to end: witnessed hidden theft -> `TellTopic::SocialObservation` relay of typed `SuspectedTheft` evidence -> authority-local `ViolationMemory` case -> `CrimeRegister` accusation -> later `PunishAccused` follow-through.

- **E17CRITHEJUS-018 / E17CRITHEJUS-022 stale-fine traceability closeout** (removed 2026-03-27) — Implemented as Scenario 39 in `golden_emergent.rs` (`golden_traceability_explains_stale_fine_branch_without_source_diving`). The suite now covers the mixed-layer debugging contract for justice punishment: decision traces record why `Fine` was selected from the recorded accusation, then action traces show the later concrete accessibility contradiction without collapsing the proof into a generic downstream failure.

- **S27-006 Supply depletion enables ShareBelief** (removed 2026-03-25) — Implemented as Scenario 40 in `golden_emergent.rs`. The suite now covers the local depletion-reporting chain end to end: refreshed `resource_source.available_quantity` belief on the speaker -> same-tick `ShareBelief { listener, subject: source }` plus `InvestigateViolation { violation_id, place }` candidate coexistence -> committed `tell` -> listener learns the depleted source through report rather than direct perception.

- **S27-005 Entity missing triggers investigation** (removed 2026-03-25) — Implemented as Scenario 36 in `golden_emergent.rs`. The suite now covers the baseline single-incident expectation-violation path: local stale belief mismatch -> `InvestigateViolation { violation_id, place }` candidate -> `investigate` commit -> `WitnessedAbsence` aftermath -> exact `ViolationMemory` resolution. The harder same-place sibling-isolation case remains covered separately by Scenario 35.

- **Scenario 10: Belief Isolation** (removed 2026-03-14) — Already covered by focused runtime tests in `agent_tick.rs` (`same_place_perception_seeds_seller_belief_for_runtime_candidates`, `unseen_seller_relocation_preserves_stale_acquisition_belief`, `unseen_death_does_not_create_corpse_reaction_without_reobservation`).

- **Scenario 11: Memory Retention Decay** (removed 2026-03-14) — Retention enforcement is applied during perception refresh, not by standalone forgetting sweep. Focused tests in `agent_tick.rs` cover both halves.

- **P-NEW-11 Loot/Bury Suppression Under Self-Care Pressure** (removed 2026-03-13) — Implemented as Scenario 8c.

- **P-NEW-3 Goal-Switch Margin Boundary** (removed 2026-03-13) — Already covered by focused tests in `goal_switching.rs`, `interrupts.rs`, `plan_selection.rs`, and `journey_switch_policy.rs`.

- **P-NEW-8 Blocked Facility Use Avoidance in Planner** (removed 2026-03-13) — Already proven by Scenario 9b.

- **P15 Put-Down Action** (removed 2026-03-13) — Stale premise; current AI cargo architecture treats destination-local controlled stock as sufficient for `MoveCargo`.

- **P16 BuryCorpse Goal** (removed 2026-03-13) — Implemented as Scenario 8b.

- **P-NEW-9 Dead Agent Pruned from Facility Queue** (removed 2026-03-13) — Implemented as Scenario 9d.

- **P18 Save/Load Round-Trip Under AI** (removed 2026-03-13) — Implemented as Scenario 6e.

- **P-NEW-10 Wound Bleed → Clotting → Natural Recovery** (removed 2026-03-13) — Implemented as Scenario 7g.
