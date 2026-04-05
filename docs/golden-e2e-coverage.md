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

1. **S58 Artifact-issuance remaining E2E gap** — [specs/S58-golden-gaps-S51.md](../specs/S58-golden-gaps-S51.md). The suite now proves manual notice downstream effects through Scenario 107 and autonomous institutional bounty issuance through Scenario 112, but it still lacks an end-to-end golden showing that an AI agent autonomously posts a high-danger `ThreatWarning` notice and that the posted artifact later reroutes another agent's travel choice.

### Recommended Implementation Order

- `S58` — closes the remaining S51 autonomous-notice golden gap.

---

## Evaluated and Rejected Scenarios

The following scenarios were considered during the 2026-03-14 coverage review and rejected with architectural justification:

1. **Fatigue/Bladder/Dirtiness as interrupt** — `interrupts.rs` branches on `GoalPriorityClass`, not need type. `is_critical_survival_goal()` treats Sleep/Relieve/Wash identically to hunger/thirst interrupts. Same code path as Scenario 2; a fatigue-specific interrupt golden test would exercise no additional logic.

2. **Multi-attacker danger escalation (2v1)** — `attackers.len() >= 2 → CRITICAL` is already unit-tested in `pressure.rs`. The behavioral consequence (defensive response under danger) is already golden-tested via Scenario 7f. The gap between unit coverage and golden coverage is too narrow to justify a high-setup multi-agent combat scenario.

3. **Journey abandonment (vs suspension)** — `AbandonsCommitment` classification is already unit-tested in `decision_runtime.rs`. High setup complexity (must engineer a scenario where the original destination becomes permanently unreachable or irrelevant mid-journey) for limited code path difference from Scenario 3c's suspension/reactivation path.

4. ~~**SellCommodity**~~ — (removed 2026-04-01: S04 implemented full merchant selling market presence. `SellCommodity` candidate generation, `StaffMarket` planner op, and `SaleListing` lifecycle are now golden-tested in `golden_merchant_selling.rs` with 12 scenarios.)

5. **Self-treatment through ordinary `heal`** — (2026-03-14: rejected. 2026-03-18: implemented.) S07 unified care model made self-treatment lawful via `TreatWounds { patient: self }`. Now golden-tested in Scenario 2c (`golden_self_care_with_medicine`, `golden_self_care_acquires_ground_medicine`).

---

## Removed Backlog Items

Items removed from the golden backlog with rationale (prevents duplicate coverage proposals):

- **S49 Unified social artifact remaining E2E gaps** (removed 2026-04-05) — Implemented and archived as [archive/specs/S49-golden-gaps-S45.md](../archive/specs/S49-golden-gaps-S45.md). Scenario 108 in `golden_integration.rs` now proves delivery-bounty fulfillment and later claim with deterministic replay, and Scenario 109 in `golden_offices.rs` now proves office-vacancy notice uptake into political action without `consult_record`.

- **S57 Rights lattice remaining E2E gap** (removed 2026-04-05) — Implemented and archived as [archive/specs/S57-golden-gaps-S50.md](../archive/specs/S57-golden-gaps-S50.md). Scenario 111 in `golden_emergent.rs` now proves punishment at a secondary jurisdiction place distinct from the office seat, with deterministic replay.

- **S48 Learned source reliability reroutes later acquisition after real failure** (removed 2026-04-05) — Implemented and archived as [archive/specs/S48-golden-gaps-S38.md](../archive/specs/S48-golden-gaps-S38.md). The delivered golden now lives in `golden_trade.rs` as Scenario 94, where a real seller rejection records durable `SourceReliability` and reroutes later acquisition to a lawful sibling seller, matching the corrected archived proof boundary.

- **S47 Hungry Merchant Eats Own Listed Sale Stock** (removed 2026-04-01) — Implemented as Scenario 87 in `golden_merchant_selling.rs` (`hungry_merchant_eats_listed_stock` plus deterministic replay). Also removed the blanket `sale_kinds` suppression in `candidate_generation.rs` that prevented merchants from considering consumption of their own sale stock. The ranking system now handles the survival-vs-enterprise tradeoff through `GoalPriorityClass`: `ConsumeOwnedCommodity` escalates with hunger pressure while `SellCommodity` stays at Medium, so merchants only eat sale stock when survival urgency exceeds enterprise value (High or Critical hunger bands).

- **S33OPPSCOGOAIDE-009 Opportunity-scoped source switching goldens** (removed 2026-03-28) — Implemented across `golden_production.rs` and `golden_ai_decisions.rs`. The suite now closes the remaining S33 golden gap at the right ownership boundaries: the blocked-source branch is proven by the strengthened `golden_contested_harvest_start_failure_recovers_via_remote_fallback`, which now asserts the first fresh post-failure selected opportunity is the remote sibling after the local authoritative `StartFailed`/blocker path, and the exhausted-opportunity branch is proven by `golden_exhausted_opportunity_switches_to_sibling_source` plus deterministic replay, which shows a seeded exhausted `OpportunityKey` is suppressed while its sibling loose-lot opportunity is still generated and selected.

- **E17CRITHEJUS-012 Theft creates owner-local `EntityMissing` -> `SuspectedTheft` discovery** (removed 2026-03-26) — Implemented as Scenario 37 in `golden_emergent.rs` (`golden_theft_leads_owner_to_local_suspected_theft_discovery` plus deterministic replay). The suite now covers the canonical owner-local crime-discovery chain end to end: thief steals and departs with the lot -> owner later returns lawfully to the stash -> stale place belief produces `EntityMissing` -> `investigate` resolves into typed `SuspectedTheft` evidence with `suspect: None`. This removes the golden gap for the owner-discovery branch while leaving witness-tell, accusation, and punishment coverage to later E17 tickets.

- **E17CRITHEJUS-013 Witnessed theft enables accusation chain** (removed 2026-03-27) — Implemented as Scenario 38 in `golden_emergent.rs` (`golden_witnessed_theft_accusation_chain` plus deterministic replay). The suite now covers the witness-driven justice route end to end: witnessed hidden theft -> `TellTopic::SocialObservation` relay of typed `SuspectedTheft` evidence -> authority-local `ViolationMemory` case -> `CrimeRegister` accusation -> later `PunishAccused` follow-through.

- **E17CRITHEJUS-018 / E17CRITHEJUS-022 stale-fine traceability closeout** (removed 2026-03-27) — Implemented as Scenario 39 in `golden_emergent.rs` (`golden_traceability_explains_stale_fine_branch_without_source_diving`). The suite now covers the mixed-layer debugging contract for justice punishment: decision traces record why `Fine` was selected from the recorded accusation, then action traces show the later concrete accessibility contradiction without collapsing the proof into a generic downstream failure.

- **S27-006 Supply depletion enables ShareBelief** (removed 2026-03-25) — Implemented as Scenario 40 in `golden_emergent.rs`. The suite now covers the local depletion-reporting chain end to end: refreshed `resource_source.available_quantity` belief on the speaker -> same-tick `ShareBelief { listener, subject: source }` plus `InvestigateViolation { violation_id, place }` candidate coexistence -> committed `tell` -> listener learns the depleted source through report rather than direct perception.

- **S13: Political Emergence Golden Suites** (removed 2026-03-28) — Implemented as Scenarios 44-46 in `golden_emergent.rs`: `golden_wounded_politician_pain_first`/`golden_wounded_politician_enterprise_first` (Scenario 44), `golden_combat_death_triggers_force_succession` (Scenario 45), and `golden_tell_propagates_political_knowledge` (Scenario 46), each with deterministic replay companions. The suite proves three cross-system political emergence chains: utility-weight-driven care-vs-enterprise priority ordering under wounds (P3, P20), combat death cascading into force-law vacancy and succession through shared state (P1, P9, P24), and autonomous Tell transferring institutional office beliefs to unlock political planning at a remote jurisdiction (P1, P7, P13).

- **S32: Crime Emergence Golden Suites** (removed 2026-03-27) — Implemented as Scenarios 41-43 in `golden_emergent.rs`: `golden_witness_deterrence_suppresses_theft_candidate`, `golden_exile_punishment_when_fine_is_not_locally_collectible`, and `golden_dual_discovery_converges_without_double_accusation`, each with deterministic replay companions. The suite now proves three previously unclosed E17 crime/justice emergence paths at their live ownership boundaries: witness-count deterrence suppresses `StealItem` candidate emission before any theft starts, Fine-infeasibility falls through to lawful `PunishAccused(Exile)` based on local collectibility plus governed faction membership, and dual discovery paths converge on one institutional accusation rather than duplicating case state.

- **S27-005 Entity missing triggers investigation** (removed 2026-03-25) — Implemented as Scenario 36 in `golden_emergent.rs`. The suite now covers the baseline single-incident expectation-violation path: local stale belief mismatch -> `InvestigateViolation { violation_id, place }` candidate -> `investigate` commit -> `WitnessedAbsence` aftermath -> exact `ViolationMemory` resolution. The harder same-place sibling-isolation case remains covered separately by Scenario 35.

- **Scenario 10: Belief Isolation** (removed 2026-03-14) — Already covered by focused runtime tests in `agent_tick.rs` (`same_place_perception_seeds_seller_belief_for_runtime_candidates`, `unseen_seller_relocation_preserves_stale_acquisition_belief`, `unseen_death_does_not_create_corpse_reaction_without_reobservation`).

- **Scenario 11: Memory Retention Decay** (removed 2026-03-14) — Retention enforcement is applied during perception refresh, not by standalone forgetting sweep. Focused tests in `agent_tick.rs` cover both halves.

- **P-NEW-11 Loot/Bury Suppression Under Self-Care Pressure** (removed 2026-03-13) — Implemented as Scenario 8c.

- **P-NEW-3 Goal-Switch Margin Boundary** (removed 2026-03-13) — Already covered by focused tests in `goal_switching.rs`, `interrupts.rs`, `plan_selection.rs`, and `journey_switch_policy.rs`.

- **P-NEW-8 Blocked Facility Use Avoidance in Planner** (removed 2026-03-13) — Already proven by Scenario 9b.

- **P15 Put-Down Action** (removed 2026-03-13) — Stale premise; current AI cargo architecture treats destination-local controlled stock as sufficient for `MoveCargo`.

- **SellCommodity base coverage** (removed 2026-04-01) — S04 implemented full merchant selling market presence. 12 golden scenarios in `golden_merchant_selling.rs` cover listing lifecycle, buyer discovery, trade against listed lots, dampening, remote travel-to-sell, demand memory ranking, and deterministic replay. The original rejection (#4) was based on missing emission logic that no longer applies.

- **P16 BuryCorpse Goal** (removed 2026-03-13) — Implemented as Scenario 8b.

- **P-NEW-9 Dead Agent Pruned from Facility Queue** (removed 2026-03-13) — Implemented as Scenario 9d.

- **P18 Save/Load Round-Trip Under AI** (removed 2026-03-13) — Implemented as Scenario 6e.

- **P-NEW-10 Wound Bleed → Clotting → Natural Recovery** (removed 2026-03-13) — Implemented as Scenario 7g.
