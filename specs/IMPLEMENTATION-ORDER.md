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

All completed specs are archived under `archive/specs/`.

---

## Dependency Graph

```text
Phase 1-2 + FND-01 + FND-02 + E21 + E14: COMPLETED

E15 ──→ E17 (crime needs discovery + ownership claims + planner binding)
E16 ──→ E18 (bandits need faction system)
E16 ──→ E19 (guards need public order)
S01, S03 ──→ E17 (crime needs discovery + ownership claims + planner binding)
S02, E16 ──→ E18, E19, E20
S04 ──→ S05 (stock storage needs selling + ownership)
S04 ──→ S06 (opportunity valuation needs market presence)
E14 provides the prerequisite belief boundary for E15, E16, S01, S02, S03, S04, and S07.
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

**Step 10** (parallel after completed E14):
- **E16**: Offices, Succession & Factions
- **S01**: Production Output Ownership Claims
- **S02**: Goal Decision Policy Unification
- **S03**: Planner Target Identity & Affordance Binding
- **S07**: Care Intent & Treatment Targeting

**Step 11** (needs E15, S01, S03):
- **E17**: Crime, Theft & Justice

#### Phase 3 Gate
- [ ] `OmniscientBeliefView` fully replaced — no code path uses it
- [ ] Information propagates through explicit channels (witnesses, rumors, records)
- [ ] Offices transfer through succession
- [ ] All FND-02 tickets verified closed
- [ ] T10: Belief isolation — agent does not react to unseen theft, death, or camp migration
- [ ] T11: Office uniqueness
- [ ] T25: Unseen crime discovery

---

### Phase 4: Adaptation & Integration

**Step 12** (parallel, needs S02 + E16):
- **E18**: Bandit Camp Dynamics
- **E19**: Guard & Patrol Adaptation
- **E20**: Companion Behaviors

**Step 13** (needs E18–E20):
- **E22**: Scenario Integration & Soak Tests

#### Phase 4 Gate
- [ ] All T20–T32 pass
- [ ] 100-seed soak test with zero invariant violations
- [ ] Replay consistency verified
- [ ] Causal depth ≥ 4 across ≥ 3 subsystems for all 4 exemplar scenarios

---

### Phase 4+: Economy Deepening

**Step 14** (parallel after E22):
- **S04**: Merchant Selling Market Presence (needs E14)
- **S05**: Merchant Stock Storage & Stalls (needs S04, S01)
- **S06**: Commodity Opportunity Valuation (needs S04)

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
| `E16-offices-succession-factions.md` | 3 | 10 | E14 |
| `S01-production-output-ownership-claims.md` | 3 | 10 | E14 |
| `S02-goal-decision-policy-unification.md` | 3 | 10 | E14 |
| `S03-planner-target-identity-and-affordance-binding.md` | 3 | 10 | E14 |
| `S07-care-intent-and-treatment-targeting.md` | 3 | 10 | E14 |
| `E17-crime-theft-justice.md` | 3 | 11 | E15, S01, S03 |
| `E18-bandit-dynamics.md` | 4 | 12 | E16, S02 |
| `E19-guard-patrol.md` | 4 | 12 | E16, S02 |
| `E20-companion-behaviors.md` | 4 | 12 | S02 |
| `E22-integration-soak-tests.md` | 4 | 13 | E18, E19, E20 |
| `S04-merchant-selling-market-presence.md` | 4+ | 14 | E14 |
| `S05-merchant-stock-storage-and-stalls.md` | 4+ | 14 | S04, S01 |
| `S06-commodity-opportunity-valuation.md` | 4+ | 14 | S04 |

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
| 3: Information & Politics | E14–E17, S01–S03 | Information propagates, offices transfer | PENDING |
| 4: Adaptation & Integration | E18–E20, E22 | Full integration, all scenarios | PENDING |
| 4+: Economy Deepening | S04–S06 | Merchant economy depth | PENDING |
