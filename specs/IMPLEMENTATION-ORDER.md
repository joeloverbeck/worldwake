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

### E15c: Conversation Memory & Recipient Knowledge — COMPLETED
Replaced same-place Tell suppression with explicit conversation memory: `AgentBeliefStore` now tracks told/heard memory with retention-aware reads and deterministic eviction, Tell affordances and AI social candidate generation share listener-aware resend suppression, Tell commit records participant memory and heard dispositions, and golden/trace coverage proves unchanged-repeat suppression plus lawful re-tell after belief change or expiry.

### S14: Conversation Memory Emergence Golden E2E Suites — COMPLETED
Established cross-system golden proof that same-place office facts still require Tell before political planning appears, and that listener-aware resend suppression happens before tell-candidate truncation so untold office facts can still unlock downstream office behavior.

All completed specs are archived under `archive/specs/`.

---

## Dependency Graph

```text
Phase 1-2 + FND-01 + FND-02 + E21 + E14 + E15 + E15b + E15c + E16 + E16d + S01 + S02 + S03 + S07 + S14: COMPLETED

S08 (no unmet deps — bug fix to existing action framework)
S09 (no unmet deps — design fix to defend action duration)
S11 (no unmet deps — investigation of wound lifecycle anomaly)
S12 (no unmet deps — planner prerequisite-aware search heuristic)
S13 (no unmet deps post-E16d — political emergence golden coverage)
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
- **S08**: Action Start Abort Resilience (bug fix, no deps)
  - fixes `AbortRequested` crash during BestEffort action start and medicine conservation leak in heal action
- **S09**: Indefinite Action Re-Evaluation (design fix, no deps)
  - converts defend from indefinite to finite-duration with renewal to prevent agent deadlock
- **S11**: Wound Lifecycle Audit (investigation, no deps)
  - diagnoses and fixes wound disappearance anomaly with zero recovery rate
- **S12**: Planner Prerequisite-Aware Search (planner enhancement, no deps)
  - extends A* heuristic and spatial pruning to consider prerequisite resource locations, enabling 4+ step cross-domain plans
- **S13**: Political Emergence Golden E2E Suites (no unmet deps post-E16d)
  - adds cross-system emergence coverage for combat-driven succession, Tell-driven office claims, and care-vs-politics ordering

**Step 12**:
- **E16c**: Institutional Beliefs & Record Consultation
  - needs E14, E15, E16

**Step 13**:
- **E16b**: Force Legitimacy & Jurisdiction Control
  - needs E16, E16c
- **E17**: Crime, Theft & Justice
  - needs E15, S01, S03, E16c

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
| `S08-action-start-abort-resilience.md` | 3 | 11 | None (bug fix) |
| `S09-indefinite-action-re-evaluation.md` | 3 | 11 | None (design fix) |
| `S11-wound-lifecycle-audit.md` | 3 | 11 | None (investigation) |
| `S12-planner-prerequisite-aware-search.md` | 3 | 11 | None (planner enhancement) |
| `S13-political-emergence-golden-suites.md` | 3 | 11 | E14, S07, E16d (all met) |
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
| 3: Information & Politics | E14–E17, E15b, E15c, E16b, E16c, S01–S03, S07–S09, S11–S13 | Information propagates, offices transfer | IN PROGRESS (E14, E15b, E15c, E16, E16d, S01, S02, S03, S07, S14 complete) |
| 4: Adaptation & Integration | E18–E20, E22 | Full integration, all scenarios | PENDING |
| 4+: Economy Deepening | S04–S06 | Merchant economy depth | PENDING |
