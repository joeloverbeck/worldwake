# Golden E2E Suite: Coverage Dashboard

**Date**: 2026-03-12 (updated 2026-03-18, offices/locality added 2026-03-18, inventory grounded 2026-03-18, S13-002 social-political emergence added 2026-03-18, S13-003 wounded-politician ordering added 2026-03-19, E15c social coverage aligned 2026-03-19, S14 conversation-memory emergence added 2026-03-19, S08 care start-abort regression added 2026-03-19, S15 start-failure emergence inventory aligned 2026-03-19, S16 spatial multi-hop coverage added 2026-03-21, inventory generation added 2026-03-21, S17 wound-lifecycle coverage aligned 2026-03-21)
**Scope**: `crates/worldwake-ai/tests/golden_*.rs`
**Purpose**: Quick-reference coverage status for planning new spec coverage. For detailed scenario descriptions, see [golden-e2e-scenarios.md](golden-e2e-scenarios.md).
**Conventions**: For assertion patterns and trace usage, see [golden-e2e-testing.md](golden-e2e-testing.md).
**Inventory source**: The canonical mechanical inventory now lives in [generated/golden-e2e-inventory.md](generated/golden-e2e-inventory.md) and is regenerated/validated with `python3 scripts/golden_inventory.py --write --check-docs`. That command cross-checks the current `golden_*.rs` declarations against `cargo test -p worldwake-ai -- --list`.

---

## File Layout

See [generated/golden-e2e-inventory.md](generated/golden-e2e-inventory.md) for the current per-file counts and the full `golden_*` name inventory. Keep this dashboard focused on coverage interpretation rather than duplicating the mechanical inventory by hand.

---

## Coverage Matrix

### GoalKind Coverage

| GoalKind | Tested? | Scenarios |
|----------|---------|-----------|
| ConsumeOwnedCommodity | Yes | 1, 2, 3, 4, 5, 6b, 7, 7a, 30 |
| AcquireCommodity (SelfConsume) | Yes | 1, 2b, 4, 5 |
| AcquireCommodity (Restock) | Yes | 2d |
| AcquireCommodity (RecipeInput) | Yes | 6a |
| TreatWounds (self) | Yes | 2c, 23 |
| TreatWounds (other) | Yes | 2c |
| Sleep | Yes | 2 |
| Relieve | Yes | 7b |
| Wash | Yes | 7e, 30 |
| EngageHostile | Yes | 7c |
| ReduceDanger | Yes | 7f |
| ProduceCommodity | Yes | 6b |
| SellCommodity | **No** | — |
| RestockCommodity | Yes | 2d |
| MoveCargo | Yes | 2d |
| LootCorpse | Yes | 8 |
| BuryCorpse | Yes | 8b |
| ShareBelief | Yes | 2e, 22, 24, 25 |
| ClaimOffice | Yes | 11, 12, 13, 14, 15, 16, 17, 18, 19, 22, 23, 24, 25, 28 |
| SupportCandidateForOffice | Yes | 12, 13, 14 |

**Coverage: 19/19 GoalKinds tested (100%).**

### ActionDomain Coverage

| Domain | Tested? | How |
|--------|---------|-----|
| Generic | Implicit | — |
| Needs (eat, drink, sleep, relieve, wash) | Yes | eat + drink + sleep + relieve + wash |
| Production (harvest, craft) | Yes | 4, 5, 6b, 26 |
| FacilityQueue (queue_for_facility_use) | Yes | 9 |
| Trade | Yes | 2b, 27 |
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

**9/12 places are now used. Multi-hop travel is explicitly tested via the BanditCamp→OrchardFarm route, the VillageSquare→OrchardFarm branchy-hub route, and the GeneralStore→OrchardFarm merchant restock route.**

### Cross-System Interaction Coverage

| Interaction | Tested? |
|-------------|---------|
| Needs → AI goal generation | Yes |
| Metabolism → need escalation → eating | Yes |
| Metabolism → thirst escalation → drinking | Yes |
| Bladder pressure → travel → relief | Yes |
| Production → materialization → transport → consumption | Yes |
| Remote recipe-input procurement → multi-leg travel → craft → output pickup → consume | Yes |
| Resource depletion → regeneration → re-harvest | Yes |
| Deprivation → wounds → death | Yes |
| Death → loot | Yes |
| Corpse burial → containment-based inaccessibility | Yes |
| Trade negotiation between two agents | Yes |
| Multi-hop travel to distant acquisition source | Yes |
| Default-budget branchy-hub travel selection from VillageSquare | Yes |
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
| Critical deprivation fires consolidate into one persistent starvation wound instead of duplicating wounds | Yes |
| Recovery-aware promotion raises eat above a higher wash motive and opens the wound-recovery gate | Yes |
| Loot/bury suppression under self-care pressure → relief → suppression lift | Yes |
| Multi-corpse loot binding → sequential target selection via matches_binding | Yes |
| Bury suppression under hunger stress → eat → suppression lift → burial | Yes |
| Suppression prevents loot on multiple targets → eat → binding selects correct target → sequential loot | Yes |
| Bystander witnesses telling without receiving belief payload | Yes |
| Entity-missing discovery from violated local expectation | Yes |
| Stale belief → travel to depleted source → passive re-observation → replan | Yes |
| Unchanged repeat tell → explicit told-memory suppression | Yes |
| Material belief change → lawful re-tell → updated listener belief | Yes |
| Conversation-memory expiry → lawful re-tell without belief-content change | Yes |
| Decision traces expose social omission and re-enablement via recipient-knowledge status | Yes |
| Pain pressure → treatment acquisition → pick-up → heal | Yes |
| Self-care: wound → TreatWounds{self} → medicine consumption → wound reduction | Yes |
| Self-care supply path: wound → TreatWounds{self} → ground pick-up → heal | Yes |
| Direct-observation gate: Report-sourced wound belief does NOT trigger TreatWounds | Yes |
| Care goal invalidation: patient self-heals → healer's TreatWounds satisfied | Yes |
| Care pre-start wound disappearance: lawful TreatWounds selection → wounds disappear before authoritative input drain → `StartFailed` → blocked intent persisted next tick | Yes |
| Contested harvest start failure: two agents lawfully select the same local harvest → loser records `StartFailed` at authoritative start → next AI tick clears stale branch → travel to remote orchard → harvest → eat | Yes |
| Local trade opportunity vanishes: two buyers lawfully target one edible stock unit → loser records `StartFailed` on stale trade start → next AI tick abandons dead local trade branch → distant production fallback → eat | Yes |
| Speaker-side chain-length filtering prevents infinite gossip propagation | Yes |
| social_weight diversity → distinct social behavior (Principle 20) | Yes |
| Zero-motive filter prevents execution of unmotivated goals | Yes |
| Rumor → travel → passive observation → discovery → belief source upgrade → replan | Yes |
| Enterprise weight → ClaimOffice → DeclareSupport → succession resolution → office installation | Yes |
| Loyalty → SupportCandidateForOffice → DeclareSupport(other) → multi-agent support competition → decisive installation | Yes |
| Bribe → commodity transfer → loyalty → SupportCandidateForOffice → coalition majority → office installation | Yes |
| Threaten → courage diversity → yield/resist divergence → coalition building → office installation (Principle 20) | Yes |
| Remote ClaimOffice belief → multi-hop travel planning (Principle 7 locality) → sequential travel → DeclareSupport → succession installation | Yes |
| Unknown remote office fact → no political candidate generation or travel → explicit reported belief update → ClaimOffice emerges → travel → succession installation | Yes |
| Hunger self-care pressure suppresses ClaimOffice → eat → suppression lift → DeclareSupport → succession installation | Yes |
| Faction membership eligibility gate → ClaimOffice candidate generation allowed for member and denied for non-member → only eligible claimant installs | Yes |
| Combat death → authoritative vacancy mutation → delayed force-law succession installation | Yes |
| Remote office Tell → reported office belief update → ClaimOffice emerges → travel → DeclareSupport → succession installation | Yes |
| Same-place office fact still requires Tell: co-location alone does not create office knowledge, but Tell unlocks ClaimOffice and DeclareSupport at the same place | Yes |
| Already-told recent subject omitted before truncation → older untold office fact still told → remote ClaimOffice travel and succession still occur | Yes |
| Remote office claim race: two informed claimants travel toward the same vacancy → winner installs first → loser records political `StartFailed` at authoritative start → next AI tick clears stale claim path and stops re-attempting the occupied office | Yes |
| Wounded politician ordering: medium pain commits `heal` before `declare_support`, low pain commits `declare_support` before `heal`, and both converge to office-holder + reduced wounds | Yes |
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
| Proven tests | 137 | 137 |
| GoalKind coverage | 19/19 (100%) | 19/19 (100%) |
| ActionDomain coverage | 11/11 full | 11/11 full |
| Needs tested | 5/5 | 5/5 |
| Places used | 9/12 | 9/12 |
| Cross-system chains | 70 | 70 |

### Pending Backlog Summary

**S02c: Multi-Role Emergent Supply Chain** (3 tests: main + replay + conservation) — blocked on `specs/S10-bilateral-trade-negotiation.md`. The remaining full producer→merchant→consumer golden chain is no longer blocked on S08 tracing; the archived S09 outcome confirmed the unresolved gap is trade valuation/pricing architecture, and `golden_supply_chain.rs` still contains only trace-segment tests plus ignored blocked full-chain cases.

### Recommended Implementation Order

1. S02c multi-role emergent supply chain

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
