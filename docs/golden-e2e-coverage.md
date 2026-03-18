# Golden E2E Suite: Coverage Dashboard

**Date**: 2026-03-12 (updated 2026-03-18)
**Scope**: `crates/worldwake-ai/tests/golden_*.rs` (split across domain files, shared harness in `golden_harness/mod.rs`)
**Purpose**: Quick-reference coverage status for planning new spec coverage. For detailed scenario descriptions, see [golden-e2e-scenarios.md](golden-e2e-scenarios.md).

---

## File Layout

```
crates/worldwake-ai/tests/
  golden_harness/
    mod.rs                    — GoldenHarness, helpers, recipe builders, world setup
  golden_ai_decisions.rs      — 11 tests (scenarios 1, 2, 3b, 3c, 5, 7, 7a, 7b, 7d, 7e, S02b)
  golden_care.rs              — 12 tests (third-party care + self-care + ground medicine acquisition + indirect-report gate + care goal invalidation + replays)
  golden_production.rs        — 17 tests (scenarios 3, 3d, 3f, 4, 6a, 6b, 6c, 6d, 9, 9b, 9c, 9d + replays)
  golden_combat.rs            — 19 tests (living combat + wound recovery + defensive mitigation + death/loot/burial/suppression + multi-corpse binding + bury suppression + combined suppression-binding scenarios + replays)
  golden_determinism.rs       — 4 tests (scenarios 6, 6e, S02 + replay)
  golden_trade.rs             — 4 tests (scenarios 2b, 2d + replays)
  golden_social.rs            — 10 tests (autonomous tell, suppression under survival pressure, rumor relay degradation, stale-belief correction, skeptical-listener rejection, bystander locality, entity-missing discovery, chain-length filtering, agent diversity, rumor-wasted-trip-discovery)
  golden_emergent.rs          — 9 tests (cross-system emergence: wound-vs-hunger priority S07a/S07b, care-weight divergence S07c, care-travel-to-remote-patient S07d, loot-corpse-self-care S07e + replays)
```

---

## Coverage Matrix

### GoalKind Coverage

| GoalKind | Tested? | Scenarios |
|----------|---------|-----------|
| ConsumeOwnedCommodity | Yes | 1, 2, 3, 4, 5, 6b, 7, 7a |
| AcquireCommodity (SelfConsume) | Yes | 1, 2b, 4, 5 |
| AcquireCommodity (Restock) | Yes | 2d |
| AcquireCommodity (RecipeInput) | Yes | 6a |
| TreatWounds (self) | Yes | 2c |
| TreatWounds (other) | Yes | 2c |
| Sleep | Yes | 2 |
| Relieve | Yes | 7b |
| Wash | Yes | 7e |
| EngageHostile | Yes | 7c |
| ReduceDanger | Yes | 7f |
| ProduceCommodity | Yes | 6b |
| SellCommodity | **No** | — |
| RestockCommodity | Yes | 2d |
| MoveCargo | Yes | 2d |
| LootCorpse | Yes | 8 |
| BuryCorpse | Yes | 8b |
| ShareBelief | Yes | 2e |

**Coverage: 17/18 GoalKinds tested (94.4%).**

### ActionDomain Coverage

| Domain | Tested? | How |
|--------|---------|-----|
| Generic | Implicit | — |
| Needs (eat, drink, sleep, relieve, wash) | Yes | eat + drink + sleep + relieve + wash |
| Production (harvest, craft) | Yes | 4, 5, 6b |
| FacilityQueue (queue_for_facility_use) | Yes | 9 |
| Trade | Yes | 2b |
| Travel | Yes | 1, 3 (implicit) |
| Transport | Yes | pick-up/materialization (4, 6c) + destination-local cargo delivery semantics (2d) |
| Combat (attack, defend) | Yes | 7c, 7f |
| Care (heal, self-care) | Yes | 2c |
| Corpse (`loot`, `bury`) | Yes | 8, 8b |
| Social (`tell`) | Yes | 2e |

**Coverage: 11/11 domains fully tested.**

### Needs Coverage

| Need | Tested as driver? | Tested as interrupt? |
|------|-------------------|---------------------|
| Hunger | Yes (all scenarios) | Yes (2) |
| Thirst | Yes (7a, 3c) | Yes (3c) |
| Fatigue | Yes (2, initial) | **No** |
| Bladder | Yes (7b) | **No** |
| Dirtiness | Yes (7e) | **No** |

### Topology Coverage

| Place | Used? | Scenarios |
|-------|-------|-----------|
| VillageSquare | Yes | All |
| OrchardFarm | Yes | 1, 2d, 3, 3b, 4, 5 |
| GeneralStore | Yes | 2d |
| CommonHouse | **No** | — |
| RulersHall | **No** | — |
| GuardPost | **No** | — |
| PublicLatrine | Yes | 7b |
| NorthCrossroads | Yes | 3b |
| ForestPath | Yes | 3b |
| BanditCamp | Yes | 3b |
| SouthGate | Yes | 2d |
| EastFieldTrail | Yes | 3b |

**9/12 places are now used. Multi-hop travel is explicitly tested via both the BanditCamp→OrchardFarm route and the GeneralStore→OrchardFarm merchant restock route.**

### Cross-System Interaction Coverage

| Interaction | Tested? |
|-------------|---------|
| Needs → AI goal generation | Yes |
| Metabolism → need escalation → eating | Yes |
| Metabolism → thirst escalation → drinking | Yes |
| Bladder pressure → travel → relief | Yes |
| Production → materialization → transport → consumption | Yes |
| Resource depletion → regeneration → re-harvest | Yes |
| Deprivation → wounds → death | Yes |
| Death → loot | Yes |
| Corpse burial → containment-based inaccessibility | Yes |
| Trade negotiation between two agents | Yes |
| Multi-hop travel to distant acquisition source | Yes |
| Combat between two living agents | Yes |
| Healing a wounded agent with medicine | Yes |
| Merchant restock → travel → acquire → return stock to home market | Yes |
| Goal switching during multi-leg travel | Yes |
| Carry capacity exhaustion forcing inventory decisions | Yes |
| Multi-agent reservation-backed resource exhaustion | Yes |
| Multiple competing needs (hunger + thirst + fatigue) | Yes |
| Penalty-interrupt threshold (subcritical continue, critical interrupt) | Yes |
| Dirtiness pressure → wash → water consumption | Yes |
| Active attack pressure → ReduceDanger → defensive mitigation | Yes |
| Death after departure on multi-hop travel | Yes |
| Multi-agent exclusive facility queue rotation → grant promotion → harvest | Yes |
| Queue patience timeout → authoritative dequeue → alternative facility recovery | Yes |
| Death while waiting in exclusive facility queue → authoritative prune → next-waiter promotion | Yes |
| Materialized output theft → fresh replanning to distant fallback | Yes |
| Save/load round-trip with reconstructed AI runtime → identical continuation | Yes |
| Wound bleed → clotting → natural recovery | Yes |
| Loot/bury suppression under self-care pressure → relief → suppression lift | Yes |
| Multi-corpse loot binding → sequential target selection via matches_binding | Yes |
| Bury suppression under hunger stress → eat → suppression lift → burial | Yes |
| Suppression prevents loot on multiple targets → eat → binding selects correct target → sequential loot | Yes |
| Bystander witnesses telling without receiving belief payload | Yes |
| Entity-missing discovery from violated local expectation | Yes |
| Stale belief → travel to depleted source → passive re-observation → replan | Yes |
| Memory retention decay → belief eviction → changed candidate generation | Focused runtime coverage |
| Pain pressure → treatment acquisition → pick-up → heal | Yes |
| Self-care: wound → TreatWounds{self} → medicine consumption → wound reduction | Yes |
| Self-care supply path: wound → TreatWounds{self} → ground pick-up → heal | Yes |
| Direct-observation gate: Report-sourced wound belief does NOT trigger TreatWounds | Yes |
| Care goal invalidation: patient self-heals → healer's TreatWounds satisfied | Yes |
| Speaker-side chain-length filtering prevents infinite gossip propagation | Yes |
| social_weight diversity → distinct social behavior (Principle 20) | Yes |
| Zero-motive filter prevents execution of unmotivated goals | Yes |
| Rumor → travel → passive observation → discovery → belief source upgrade → replan | Yes |
| 200-tick multi-agent world with conservation + deterministic replay (Principle 6) | Yes |
| UtilityProfile weight divergence → different goal selection (Principle 20, survival vs enterprise) | Yes |
| Wound vs hunger priority resolved by concrete utility weights (Principle 3, 20) | Yes |
| care_weight divergence → different agents make different care decisions for same patient | Yes |
| Care + Travel: healer travels to remote patient, travel time dampens healing (Principle 10) | Yes |
| Loot corpse → acquire medicine → self-care chain (Principle 1 maximal emergence) | Yes |

---

## Summary Statistics

| Metric | Current | Pending Backlog |
|--------|---------|-----------------|
| Proven tests | 87 | 90 |
| GoalKind coverage | 17/18 (94.4%) | 17/18 (94.4%) |
| ActionDomain coverage | 11/11 full | 11/11 full |
| Needs tested | 5/5 | 5/5 |
| Places used | 9/12 | 9/12 |
| Cross-system chains | 48 | 51 |

### Pending Backlog Summary

**S02c: Multi-Role Emergent Supply Chain** (3 tests: main + replay + conservation) — blocked on `specs/S08-ai-decision-traceability.md`. The producer→merchant→consumer end-to-end test could not be debugged to completion without AI decision traces. Will be re-implemented after S08 lands.

### Recommended Implementation Order

No remaining golden backlog items.

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
