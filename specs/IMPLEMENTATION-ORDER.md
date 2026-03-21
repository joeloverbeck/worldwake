# Implementation Order & Dependency Graph

## Completed Work

### Phase 1: World Legality (E01–E08) — COMPLETED
Established core and sim crates: ECS, topology graph, item/container model, conservation invariants, relation system, append-only event log with causal linking, transactional world mutations, canonical state hashing, action framework with preconditions, tick-driven scheduler, deterministic replay, and save/load persistence.

### Phase 2: Emergent Economy (E09–E13) — COMPLETED
Established systems and ai crates: homeostatic needs and deprivation wounds, resource regeneration and recipe-based crafting, merchant trade with valuation, combat with wound tracking, and pressure-based GOAP decision architecture with goal ranking, plan search, failure handling, and per-tick autonomous agent control.

### FND-01: Phase 1 Foundations Alignment — COMPLETED
Removed route scores, banned zero-tick actions, replaced load match-arms with physical profiles, renamed KnowledgeView→BeliefView, constrained loyalty mutations.

### FND-02: Phase 2 Foundations Alignment — COMPLETED
Aligned the E14/E15 specs with determinism and Section H requirements, preserved `SellCommodity` deferral until S04, wired treatment-oriented acquisition candidates, documented Phase 2 dampening, and added goal/event debuggability APIs.

### E21: CLI & Human Control — COMPLETED
Pulled forward from Phase 4 post-Phase 2 as the primary manual testing interface.

### E14: Perception & Belief System — COMPLETED
Established per-agent belief stores, passive/direct perception, social observation capture, `PerAgentBeliefView`, belief-only planner reads, public route/place structure reads, and belief-mediated remote facility/resource discovery.

### E15: Rumor, Witness & Discovery — COMPLETED
Established Tell-based social transmission, per-agent Tell profiles, discovery events for violated expectations, social observation capture for telling, and transaction-owned metadata finalization for transaction-built E15 event payloads.

### E15b: Social AI Goals + Golden E2E Tests — COMPLETED
Established GoalKind::ShareBelief, PlannerOpKind::Tell, social candidate generation with chain-length filtering, social_weight-driven ranking, 10 golden social E2E tests covering autonomous Tell, rumor relay, discovery correction, agent diversity, and the full information lifecycle. Added system-wide zero-motive filter in rank_candidates() and treatment_pain() helper for treatment acquisition scoring.

### E16: Offices, Succession & Factions — COMPLETED
Established offices and factions as first-class institutions, support declarations, courage-driven coercion handling, office actions, succession resolution, public-order aggregation, and political AI integration through the belief/runtime-view boundary. The force branch remains intentionally conservative; explicit contested control stays in active follow-up spec E16b.

### E16d: Political Planning Fix & Golden E2E Coverage — COMPLETED
Established planner semantics and verification coverage for political coalition-building and locality: `Bribe`/`Threaten` now participate in planning, office goldens cover claim/coalition/travel/suppression/eligibility/force/locality behavior, and the political golden coverage docs are up to date. The original “incumbent defense” sub-plan was corrected out of scope rather than forced into the support-law architecture.

### S01: Production Output Ownership Claims — COMPLETED
Established explicit ownership for production output: `ProductionOutputOwnershipPolicy` component with Actor/ProducerOwner/Unowned variants, `create_item_lot_with_owner()` atomic helper, `can_exercise_control()` extended with faction/office delegation, harvest and craft commit ownership resolution, `believed_owner_of()` belief query, ownership-gated pickup validation, and GOAP planner adaptations for actor-owned output.

### S02: Goal Decision Policy Unification — COMPLETED
Unified goal-family decision policy into a single `goal_policy.rs` module with `GoalFamilyPolicy`, `DecisionContext`, and `evaluate_suppression()`. Ranking and interrupts both consume the shared policy surface. Removed all four legacy functions (`is_suppressed`, `is_critical_survival_goal`, `is_reactive_goal`, `no_medium_or_above_self_care_or_danger`). All 16 goal families migrated.

### S03: Planner Target Identity & Affordance Binding — COMPLETED
Added `matches_binding()` to `GoalKindPlannerExt` with auxiliary-pass/terminal-check dispatch across all 17 `GoalKind` variants. Wired `.retain()` binding filter in search candidates. Added `BindingRejection` trace struct for debuggability. Exact-bound goals (`LootCorpse`, `EngageHostile`, `TreatWounds`, `ShareBelief`, `BuryCorpse`) now reject wrong-target affordances before successor construction; flexible goals remain unaffected.

### S07: Care Intent & Treatment Targeting — COMPLETED
Replaced split care model (`GoalKind::Heal` + `CommodityPurpose::Treatment`) with unified patient-anchored `GoalKind::TreatWounds { patient }`. Added `care_weight` to `UtilityProfile` for self/other ranking split. Made self-care lawful. Established `DirectObservation` gate for third-party care via `emit_care_goals()`. 12 golden tests prove self-care, third-party care, observation gate, and care goal invalidation.

### S08: Action Start Abort Resilience — COMPLETED
Established recoverable BestEffort start-abort handling, structured authoritative start-failure handoff into AI/runtime reconciliation, validation-only `start_heal()`, first-effect Medicine spending with explicit heal-local state, and golden care coverage for the real pre-start wound-disappearance race.

### E15c: Conversation Memory & Recipient Knowledge — COMPLETED
Replaced same-place Tell suppression with explicit conversation memory: `AgentBeliefStore` now tracks told/heard memory with retention-aware reads and deterministic eviction, Tell affordances and AI social candidate generation share listener-aware resend suppression, Tell commit records participant memory and heard dispositions, and golden/trace coverage proves unchanged-repeat suppression plus lawful re-tell after belief change or expiry.

### S14: Conversation Memory Emergence Golden E2E Suites — COMPLETED
Established cross-system golden proof that same-place office facts still require Tell before political planning appears, and that listener-aware resend suppression happens before tell-candidate truncation so untold office facts can still unlock downstream office behavior.

### S15: Start-Failure Emergence Golden E2E Suites — COMPLETED
Established production, trade, and political golden proof that the shared S08 `StartFailed` contract survives outside care: lawful authoritative start rejection now has end-to-end coverage across contested harvest, vanished local trade, and remote office-claim races, including next-tick AI reconciliation and deterministic replay.

### S16: S09 Golden Validation Suite — COMPLETED
Established the delivered S09 behavioral golden coverage and aligned the archival trail to the shipped ownership boundaries: `golden_defend_changed_conditions` now proves defend re-evaluation under changed conditions in `golden_combat.rs`, and `golden_spatial_multi_hop_plan` plus its deterministic replay companion prove default-budget VillageSquare hub reachability in `golden_ai_decisions.rs`. The later S16 follow-up work also strengthened the spatial decision-trace boundary with winning-search provenance and refactored the helper structure without duplicating the scenario.

### S11: Wound Lifecycle Audit — COMPLETED
Established the wound lifecycle corrections and hardening planned for Phase 3: deprivation harm now worsens a stable same-kind wound instead of duplicating entries, clotted-wound recovery gates are reflected in AI self-care ranking, wound progression/pruning contracts have focused regression coverage, and the golden/workspace verification boundary passed without requiring additional hash recapture.

### S17: Wound Lifecycle Golden E2E Suites — COMPLETED
Established the missing golden proof for the two S11 wound-lifecycle invariants: Scenario 29 in `golden_emergent.rs` now proves deprivation wound worsening consolidates through live needs dispatch without duplicating wounds, with a deterministic replay companion; Scenario 30 in `golden_combat.rs` now proves recovery-aware priority promotion makes the actor eat before wash so the recovery gate opens and wound severity decreases, also with deterministic replay coverage. Golden coverage docs and generated inventory were kept aligned at 133 `golden_*` tests.

### S12: Planner Prerequisite-Aware Search — COMPLETED
Established dynamic per-node prerequisite-aware place guidance for planner search, focused trace/budget coverage, and canonical remote-care plus remote-production golden proof. The final completion also aligned `ProduceCommodity` ranking and candidate generation with the planner architecture so remote self-consume crafting stays truthful as `ProduceCommodity` instead of falling back to stale acquire proxies.

### S18: Prerequisite-Aware Emergent Chain Goldens — COMPLETED
Established the missing Phase 3 golden proof for craft-restock and stale prerequisite-belief recovery, and closed the archival loop on the live planner contract after the stale `ProduceCommodity` narrative was corrected to the lawful `RestockCommodity` surface.

All completed specs are archived under `archive/specs/`.

---

## Dependency Graph

```text
Phase 1-2 + FND-01 + FND-02 + E21 + E14 + E15 + E15b + E15c + E16 + E16d + S01 + S02 + S03 + S07 + S08 + S14: COMPLETED

S09 ✅ (design fix to defend action duration completed)
S11 ✅ (wound lifecycle audit completed)
S11 ──→ S17 ✅ (wound lifecycle golden E2E coverage completed)
S12 ✅ (planner prerequisite-aware search heuristic completed)
E16c ──→ S13 (political emergence golden coverage needs institutional beliefs)
S15 ✅ (cross-system start-failure emergence golden coverage)
S16 ✅ (S09 behavioral golden validation coverage)
E15 ──→ E15b (social AI goals need Tell mechanics + belief system) ✅
E15, E15b ──→ E15c ✅ (conversation memory and recipient knowledge completed)
E15c, E16d ──→ S14 ✅ (cross-system golden proof for same-place Tell and listener-aware pre-truncation)
S01 ──→ ✅ COMPLETED (production output ownership claims)
S02 ──→ ✅ COMPLETED (goal decision policy unification)
E16 ──→ E16c (institutional beliefs need offices/factions/support substrate)
E16c ──→ E16b (force legitimacy needs institutional records and belief propagation)
E16c ──→ E17 (justice records and institutional knowledge should reuse one record/belief architecture)
E16 ──→ E16b (explicit force legitimacy needs offices, factions, and succession substrate)
E15, S03 ✅ ──→ E17 (crime needs discovery + ownership claims + planner binding)
E16 ──→ E18 (bandits need faction system)
E16 ──→ E19 (guards need public order)
E16b ──→ E19 (guards need contested-office control state)
E16c ──→ E19 (guards need institutional belief/record pathways)
S01 ✅, S03 ✅, E16c ──→ E17 (crime needs discovery + ownership claims + planner binding + record architecture)
S02 ✅, E16 ──→ E18, E20
S02 ✅, E16, E16b, E16c ──→ E19
E16c ──→ S05 (institutional stock ledgers should reuse record architecture)
S04 ──→ S05 (stock storage needs selling + ownership)
S04 ──→ S06 (opportunity valuation needs market presence)
S10 (no unmet deps — E11 trade + E14 perception both completed; can be scheduled anytime)
S10 ──→ S06 (opportunity valuation benefits from variable pricing)
E14 provides the prerequisite belief boundary for E15, ~~E15c~~, E16, E16c, ~~S01~~, ~~S02~~, ~~S03~~, S04, ~~S07~~, and S10.
E18, E19, E20 ──→ E22 (integration tests need everything)
```

---

## Active Execution Steps

### Phase 3: Information & Politics

**Step 9**: COMPLETED
- **E14**: Perception & Belief System
  - Replaced `OmniscientBeliefView`
  - Established the belief/evidence inputs later social systems use for loyalty/support modeling
  - Satisfied FND-01 Section B deferred information pipeline requirements

**Step 10** (parallel after completed E14/E15):
- **E15b**: Social AI Goals — ✅ COMPLETED
- **E16**: Offices, Succession & Factions — ✅ COMPLETED
- **S01**: Production Output Ownership Claims — ✅ COMPLETED
- **S02**: Goal Decision Policy Unification — ✅ COMPLETED
- **S03**: Planner Target Identity & Affordance Binding — ✅ COMPLETED
- **S07**: Care Intent & Treatment Targeting — ✅ COMPLETED

**Step 11** (parallel):
- **E15c**: Conversation Memory & Recipient Knowledge — ✅ COMPLETED
  - established explicit conversation memory and lawful resend suppression
- **S08**: Action Start Abort Resilience — ✅ COMPLETED
  - recoverable start-failure classification, structured AI/runtime failure handoff, first-effect Medicine spending, and care golden regression are in place
- **S09**: Indefinite Action Re-Evaluation — ✅ COMPLETED
  - removed indefinite action duration paths; defend now uses profile-driven finite `ActorDefendStance` duration and re-enters normal replanning after commit
- **S11**: Wound Lifecycle Audit — ✅ COMPLETED
  - delivered deprivation-wound identity preservation, recovery-aware self-care ranking, wound hardening coverage, and closeout verification
- **S12**: Planner Prerequisite-Aware Search — ✅ COMPLETED
  - dynamic combined-place search guidance, focused trace/budget coverage, and canonical remote-care plus remote-production goldens are in place
- **S15**: Start-Failure Emergence Golden E2E Suites — ✅ COMPLETED
  - production/trade/politics goldens now prove `StartFailed` plus next-tick AI recovery outside the care domain
- **S16**: S09 Golden Validation Suite — ✅ COMPLETED
  - shipped defend changed-conditions and VillageSquare spatial-validation goldens at their canonical ownership boundaries, with deterministic replay and decision-trace-first route proof
- **S17**: Wound Lifecycle Golden E2E Suites — ✅ COMPLETED
  - Scenario 29 in `golden_emergent.rs` proves deprivation worsening consolidates without duplicating wounds, and Scenario 30 in `golden_combat.rs` proves recovery-aware need promotion opens the wound-recovery gate; both shipped with deterministic replay companions and aligned golden docs/inventory
- **S18**: Prerequisite-Aware Emergent Chain Goldens — ✅ COMPLETED
  - craft-restock and stale prerequisite-belief recovery goldens are shipped, and the stale planner-surface narrative was corrected to the live `RestockCommodity` contract before archival

**Step 12**:
- **E16c**: Institutional Beliefs & Record Consultation
  - needs E14, E15, E16

**Step 13**:
- **E16b**: Force Legitimacy & Jurisdiction Control
  - needs E16, E16c
- **E17**: Crime, Theft & Justice
  - needs E15, S01, S03, E16c
- **S13**: Political Emergence Golden E2E Suites
  - needs E16c (institutional beliefs for proper belief-based political knowledge paths)
  - adds cross-system emergence coverage for combat-driven succession, Tell-driven office claims, and care-vs-politics ordering

#### Phase 3 Gate
- [ ] `OmniscientBeliefView` fully replaced — no code path uses it
- [ ] Information propagates through explicit channels (witnesses, rumors, records)
- [ ] Offices transfer through succession
- [x] Redundant Tell suppression uses explicit conversation memory rather than same-place listener-knowledge shortcuts
- [x] Political planning gives explicit `Bribe`/`Threaten` outcomes instead of falling through unchanged planning state
- [x] Political golden coverage proves claim, coalition, threat, travel, eligibility, suppression, force-succession, and locality scenarios
- [ ] Institutional facts propagate through records and consultation rather than live helper shortcuts
- [ ] Force succession uses explicit contest/control state rather than presence-only installation
- [ ] All FND-02 tickets verified closed
- [ ] T10: Belief isolation — agent does not react to unseen theft, death, or camp migration
- [ ] T11: Office uniqueness
- [ ] T25: Unseen crime discovery

---

### Phase 4: Adaptation & Integration

**Step 14** (parallel):
- **E18**: Bandit Camp Dynamics
  - needs ~~S02~~, E16
- **E19**: Guard & Patrol Adaptation
  - needs ~~S02~~, E16, E16b, E16c
- **E20**: Companion Behaviors
  - needs ~~S02~~ (all deps met)

**Step 15** (needs E18–E20):
- **E22**: Scenario Integration & Soak Tests

#### Phase 4 Gate
- [ ] All T20–T32 pass
- [ ] 100-seed soak test with zero invariant violations
- [ ] Replay consistency verified
- [ ] Causal depth ≥ 4 across ≥ 3 subsystems for all 4 exemplar scenarios

---

### Phase 4+: Economy Deepening

**Step 16** (parallel after E22):
- **S04**: Merchant Selling Market Presence (needs E14)
- **S05**: Merchant Stock Storage & Stalls (needs S04, S01, E16c)
- **S06**: Commodity Opportunity Valuation (needs S04, benefits from S10)
- **S10**: Bilateral Trade Negotiation (all deps met — E11, E14 completed; can be scheduled earlier)

#### Final Acceptance
- All Phase 4 gate criteria plus:
- [ ] Merchants autonomously sell at markets with stock storage
- [ ] Commodity opportunity valuation drives trade route decisions
- [ ] Economy sustains 100+ tick soak without conservation violations

---

## Active Spec Inventory

All specs in `specs/` must appear exactly once in this order. Completed/archived specs live in `archive/specs/`.

| Spec | Phase | Step | Dependencies |
|------|-------|------|-------------|
| `S13-political-emergence-golden-suites.md` | 3 | 13 | E16c, E16d, E12, S07, E14 |
| `E16c-institutional-beliefs-and-record-consultation.md` | 3 | 12 | E14, E15, E16 |
| `E16b-force-legitimacy-and-jurisdiction-control.md` | 3 | 13 | E16, E16c, E14, E15 |
| `E17-crime-theft-justice.md` | 3 | 13 | E15, ~~S01~~, ~~S03~~, E16c |
| `E18-bandit-dynamics.md` | 4 | 14 | E16, S02 |
| `E19-guard-patrol.md` | 4 | 14 | E16, E16b, E16c, S02 |
| `E20-companion-behaviors.md` | 4 | 14 | S02 |
| `E22-integration-soak-tests.md` | 4 | 15 | E18, E19, E20 |
| `S04-merchant-selling-market-presence.md` | 4+ | 16 | E14 |
| `S05-merchant-stock-storage-and-stalls.md` | 4+ | 16 | S04, S01, E16c |
| `S06-commodity-opportunity-valuation.md` | 4+ | 16 | S04 |
| `S10-bilateral-trade-negotiation.md` | 4+ | 16 | E11, E14 (all met) |

## Crate Dependency Graph

```text
worldwake-core:    (no internal deps)
worldwake-sim:     depends on worldwake-core
worldwake-systems: depends on worldwake-core, worldwake-sim
worldwake-ai:      depends on worldwake-core, worldwake-sim, worldwake-systems
worldwake-cli:     depends on worldwake-core, worldwake-sim, worldwake-systems, worldwake-ai
```

## Phase Summary

| Phase | Specs | Goal | Status |
|-------|-------|------|--------|
| 1: World Legality | E01–E08 | Deterministic world with conservation | ✅ COMPLETED |
| FND-01 | FND01-001–005 | Phase 1 foundations alignment | ✅ COMPLETED |
| 2: Emergent Economy | E09–E13 | Agents autonomously survive | ✅ COMPLETED |
| E21 | E21 | CLI & human control | ✅ COMPLETED |
| FND-02 | FND02-001–006 | Phase 2 foundations alignment | ✅ COMPLETED |
| 3: Information & Politics | E14–E17, E15b, E15c, E16b, E16c, S01–S03, S07–S09, S11–S18 | Information propagates, offices transfer | IN PROGRESS (E14, E15b, E15c, E16, E16d, S01, S02, S03, S07, S08, S09, S11, S12, S14, S15, S16, S17, S18 complete) |
| 4: Adaptation & Integration | E18–E20, E22 | Full integration, all scenarios | PENDING |
| 4+: Economy Deepening | S04–S06 | Merchant economy depth | PENDING |
