# Implementation Order & Dependency Graph

## Completed Work

### Phase 1: World Legality (E01–E08) — COMPLETED
Established core and sim crates: ECS, topology graph, item/container model, conservation invariants, relation system, append-only event log with causal linking, transactional world mutations, canonical state hashing, action framework with preconditions, tick-driven scheduler, deterministic replay, and save/load persistence.

### Phase 2: Emergent Economy (E09–E13) — COMPLETED
Established systems and ai crates: homeostatic needs and deprivation wounds, resource regeneration and recipe-based crafting, merchant trade with valuation, combat with wound tracking, and pressure-based GOAP decision architecture with goal ranking, plan search, failure handling, and per-tick autonomous agent control.

### FND-01: Phase 1 Foundations Alignment — COMPLETED
Removed route scores, banned zero-tick actions, replaced load match-arms with physical profiles, renamed KnowledgeView→BeliefView, constrained loyalty mutations.

### E21: CLI & Human Control — COMPLETED
Pulled forward from Phase 4 post-Phase 2 as the primary manual testing interface.

All completed specs are archived under `archive/specs/`.

---

## Dependency Graph

```text
Phase 1-2 + FND-01 + E21: COMPLETED

FND-02 gate tickets ──→ E14
E14 ──→ E15 (rumors build on perception)
E14 ──→ E16 (succession needs beliefs/loyalty)
E14 ──→ S01 (ownership claims need belief-mediated disputes)
E14 ──→ S02 (goal policy unification needs belief-based ranking)
E14 ──→ S03 (planner target identity needs belief view)
E14 ──→ S07 (care intent must be belief-mediated and patient-anchored)
E15, S01, S03 ──→ E17 (crime needs discovery + ownership claims + planner binding)
E16 ──→ E18 (bandits need faction system)
E16 ──→ E19 (guards need public order)
S02, E16 ──→ E18, E19, E20
E14 ──→ S04 (merchant selling needs belief-based market awareness)
S04, S01 ──→ S05 (stock storage needs selling + ownership)
S04 ──→ S06 (opportunity valuation needs market presence)
E18, E19, E20 ──→ E22 (integration tests need everything)
```

---

## Active Execution Steps

### FND-02 Gate (specs/FND-02-foundations-alignment-phase2.md)

**Step 8.5a** (parallel, spec/analysis work):
- **FND02-001**: Fix E14 spec — `f32`→`Permille`, `HashMap`→`BTreeMap`, add Section H analysis, define the E14/E16 loyalty evidence boundary, require full `OmniscientBeliefView` replacement
- **FND02-004**: Dampening audit across Phase 2 systems — document all amplifying loops and their physical dampeners

**Step 8.5b** (parallel, code work):
- **FND02-002**: Preserve `SellCommodity` deferral until S04 — keep seller-side selling out of FND-02 and cover the invariant with regression tests
- **FND02-003**: Wire `AcquireCommodity(Treatment)` emission in `candidate_generation.rs` — wounds → medicine-seeking
- **FND02-005**: Debuggability APIs — `explain_goal()` in ai crate, `trace_event_cause()` in sim crate

**FND02-006** (already done as part of FND-02 creation): DRAFT specs promoted to S01–S06.

#### FND-02 Gate Criteria
- [ ] E14 spec determinism-safe (no `f32`, no `HashMap` in authoritative state)
- [ ] Phase 2 / FND-02 candidate coverage complete for currently supported, satisfiable goal variants; `SellCommodity` remains explicitly deferred to S04
- [ ] Dampening audit documented (`docs/dampening-audit-phase2.md`)
- [ ] Debuggability APIs (`explain_goal`, `trace_event_cause`) exist and tested
- [ ] 6 DRAFTs promoted to S01–S06 ✅
- [ ] `cargo test --workspace` passes

---

### Phase 3: Information & Politics

**Step 9** (needs FND-02 gate):
- **E14**: Perception & Belief System
  - Replaces `OmniscientBeliefView` entirely
  - Implements FND02-001 belief-side requirements
  - Establishes the belief/evidence inputs later social systems use for loyalty/support modeling
  - Satisfies FND-01 Section B deferred information pipeline requirements

**Step 10** (parallel after E14):
- **E15**: Rumor, Witness & Discovery
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
| `FND-02-foundations-alignment-phase2.md` | FND-02 Gate | 8.5a/8.5b | Phase 2 complete |
| `E14-perception-beliefs.md` | 3 | 9 | FND-02 gate |
| `E15-rumor-witness-discovery.md` | 3 | 10 | E14 |
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
| FND-02 | FND02-001–006 | Phase 2 foundations alignment | ACTIVE |
| 3: Information & Politics | E14–E17, S01–S03 | Information propagates, offices transfer | PENDING |
| 4: Adaptation & Integration | E18–E20, E22 | Full integration, all scenarios | PENDING |
| 4+: Economy Deepening | S04–S06 | Merchant economy depth | PENDING |
