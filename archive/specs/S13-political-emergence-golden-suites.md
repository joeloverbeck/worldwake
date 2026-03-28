**Status**: COMPLETED

# S13: Political Emergence Golden E2E Suites

## Summary

Add 3 cross-system emergence golden tests to `golden_emergent.rs` that prove the political system (E16/E16d) participates in emergent multi-system chains. Currently all political tests live in `golden_offices.rs` and exercise politics in relative isolation. These new scenarios prove that combat, social, and care systems interact with politics through shared state (Principle 24) to produce outcomes no single system orchestrates (Principle 1).

## Phase

Phase 3: Information & Politics (post-E16c)

## Crate

`worldwake-ai` (golden tests only — no new system code)

## Dependencies

- E16c (institutional beliefs) — COMPLETED
- E16d (political planning, bribe/threaten semantics, golden harness office helpers) — COMPLETED
- E12 (combat, wounds, death) — COMPLETED
- S07 (care golden tests — establishes the emergent test patterns) — COMPLETED
- E14 (perception/belief system — belief boundary, social observation, Tell) — COMPLETED

## Scenarios

### Scenario 44: Wounded Politician — Enterprise vs Care Priority Resolution

**File**: `golden_emergent.rs` (Suite 2)
**Test functions**: `golden_wounded_politician_pain_first`, `golden_wounded_politician_enterprise_first`, `golden_wounded_politician_replays_deterministically`
**Systems exercised**: Care (self-treatment, wound state), Politics (ClaimOffice, DeclareSupport), AI (utility-weight-driven ranking across care and enterprise domains), Conservation
**Principles proven**: P3 (concrete state — wounds and utility weights, not abstract priority tiers), P20 (agent diversity — same office, different weights → different behavior), P24 (care and politics coordinate through state, not cross-system calls)

**Setup**: Two sub-variants sharing the `run_wounded_politician(seed, wound_severity, pain_weight, enterprise_weight)` driver:

**Variant A (pain-first)**:
- Agent with wound_severity=400, pain_weight=pm(800), enterprise_weight=pm(400)
- Wounded (stable clotted wound, `no_recovery_combat_profile()`), has 1 Medicine + institutional belief about a vacant Support-law office ("Village Elder") at VillageSquare with succession_period=5

**Variant B (enterprise-first)**:
- Agent with wound_severity=200, pain_weight=pm(300), enterprise_weight=pm(800)
- Same wound type, same medicine, same institutional belief about the vacant office

**E16c institutional belief path (validated)**: The agent's office knowledge is seeded via `seed_office_holder_belief()` with `InstitutionalKnowledgeSource::WitnessedEvent`. Political candidate generation reads `believed_office_holder()` from the agent's institutional belief store, not from any runtime seam.

**Emergent behavior proven**:
- Variant A: Agent self-heals first (heal commits before declare_support), then claims office
- Variant B: Agent claims office first (declare_support commits before heal), then self-heals
- Both agents eventually both heal and become office holder — but the ordering differs based on concrete utility weights
- This is the political extension of the wound-vs-hunger pattern (S07 Suites 1/2)

**Assertion surface**:
1. Decision traces: both `TreatWounds { patient: self }` and `ClaimOffice { office }` are generated
2. Action traces: heal and declare_support commit ticks recorded, ordering asserted per variant
3. Authoritative: agent becomes office holder AND wound load decreases AND medicine consumed
4. Conservation: medicine totals never increase
5. Determinism: replay companion

---

### Scenario 45: Combat Death Triggers Office Vacancy and Autonomous Succession

**File**: `golden_emergent.rs` (Suite 5)
**Test functions**: `golden_combat_death_triggers_force_succession`, `golden_combat_death_triggers_force_succession_replays_deterministically`
**Systems exercised**: Combat (attack, wound infliction, death), Politics (vacancy detection via `DeadAt`, force-law succession resolution), action tracing, politics tracing, event-log delta inspection, cross-layer timeline, deterministic replay
**Principles proven**: P1 (maximal emergence — combat consequence cascades into political domain), P24 (systems interact only through state — no combat-politics coupling), P9 (combat aftermath triggers downstream emergence)

**Setup**:
- Occupied office ("War Chief") at VillageSquare with `SuccessionLaw::Force`, succession_period=5, no eligibility rules
- Agent A ("Challenger"): AI-controlled, `lethal_combat_attacker_profile()`, perception profile, 3 Coin, no known recipes. Pre-seeded hostility toward B AND pre-seeded `force_claim` on office.
- Agent B ("Incumbent"): `ControlSource::None` (passive — not AI-driven), `fragile_office_holder_profile()`, perception profile, 2 Coin, office holder. Does not plan or act autonomously.
- A and B co-located at VillageSquare. Both have DirectObservation local beliefs.

**E16c institutional belief path (validated)**: Force-law succession resolution reads authoritative world state (it is a world-state system). The challenger's AI combat candidate generation reads hostility from beliefs seeded via `seed_actor_local_beliefs()`. No political AI candidate generation participates — the installation comes from the succession system detecting the sole surviving force claimant.

**Emergent behavior proven**:
- A attacks B through the real combat system (hostility triggers attack action selection).
- B suffers wounds → bleed → death (`DeadAt` component set by combat/wound system).
- Politics system detects that the living holder is gone, activates vacancy, clears the office-holder relation.
- Force succession resolves: A is the sole live force claimant at jurisdiction → controller established → uncontested hold period → installation as office holder.
- No orchestrator connects combat to politics — the chain emerges from `DeadAt` state and force-claim relations.

**Assertion surface**:
1. Action trace: attack commits before or at the incumbent's death tick
2. Event-log delta: B dies (DeadAt), vacancy mutation occurs (office-holder relation removed), A later becomes office holder (office-holder relation added)
3. Event ordering: death → vacancy → installation, with succession delay ≥ 5 ticks
4. Negative action-path check: no `declare_support` commit occurs anywhere in the chain
5. Politics trace: VacancyActivated, ForceControllerEstablished, ForceControllerMaintained, ForceInstalled outcomes all recorded, with hold delay ≥ 4 ticks
6. Cross-layer timeline: renders action, authoritative, and political layers in one view
7. Conservation: Coin totals preserved across every tick
8. Determinism: replay companion

**Why this is distinct from Scenario 19** (force succession in `golden_offices.rs`):
- Scenario 19 starts with a vacant office and tests the AI claim→install path.
- Scenario 45 has the vacancy **emerge from simulated combat**. The combat system, wound system, and politics system interact through shared state without any coupling.

---

### Scenario 46: Social Tell Propagates Political Knowledge and Triggers Office Claim

**File**: `golden_emergent.rs` (Suite 6)
**Test functions**: `golden_tell_propagates_political_knowledge`, `golden_tell_propagates_political_knowledge_replays_deterministically`
**Systems exercised**: Social (autonomous Tell), Institutional belief store (`InstitutionalClaim` transfer via Tell), AI (ClaimOffice candidate generation from institutional belief), Political actions (DeclareSupport), Travel, Succession
**Principles proven**: P7 (information locality — political knowledge arrives via social channel at finite speed), P1 (maximal emergence — Tell → belief → travel → political action is emergent), P13 (knowledge acquisition path matters)

**Setup**:
- Vacant office ("Village Elder") at VillageSquare with `SuccessionLaw::Support`, succession_period=5, no eligibility rules
- Agent A ("Informant"): at **BanditCamp** (remote from office), `social_weighted_utility(900)`, `focused_accepting_tell_profile()`, `blind_perception_profile()`. No office knowledge at setup — office belief is seeded mid-run at tick 8 via `seed_actor_beliefs()` + `seed_office_holder_belief()` with `WitnessedEvent` source.
- Agent B ("Ambitious Listener"): at **BanditCamp**, enterprise_weight=pm(800), social_weight=pm(0), `accepting_tell_profile()`, `blind_perception_profile()`. No office knowledge initially.
- Both co-located at BanditCamp so Tell can occur. Office is at VillageSquare (requires travel after learning).

**E16c institutional belief path (validated)**: Tell transfers `InstitutionalClaim::OfficeHolder { office, holder: None }` to the listener's institutional belief store with degraded source `Report { from: informant, chain_len: 1 }`. Political candidate generation reads `believed_office_holder()` from the listener's belief store.

**Emergent behavior proven**:
- Phase 1 (no office knowledge, ticks 0–8): B generates no ClaimOffice candidates because B has no institutional belief about the office.
- Mid-run (tick 8): Informant A acquires office knowledge through seeded beliefs.
- A autonomously generates ShareBelief goal and tells B about the office. The Tell system transfers the `InstitutionalClaim` to B's institutional belief store.
- Phase 2 (after Tell): B's institutional belief store contains the office claim. B generates ClaimOffice. B travels from BanditCamp to VillageSquare (office jurisdiction). B declares support for self. Succession installs B.
- The political goal emergence is caused by social institutional belief transfer, not manual belief injection or runtime-seam shortcuts.

**Assertion surface**:
1. Decision traces: B has no ClaimOffice candidates before Tell; B generates ClaimOffice after Tell
2. Authoritative: B becomes office holder; B is at VillageSquare (traveled from BanditCamp)
3. Action traces: tell committed (with `TellTopic::InstitutionalClaim` detail) before declare_support
4. Belief provenance: listener's institutional belief has `Report { from: informant, chain_len: 1 }` source
5. Listener at BanditCamp before Tell, at VillageSquare after claiming
6. Determinism: replay companion

**Why this is distinct from Scenario 16** (information locality in `golden_offices.rs`):
- Scenario 16 manually injects beliefs via test setup. The information transfer is artificial.
- Scenario 46 has the information arrive through the **autonomous Tell system** with proper belief provenance degradation. The social system, belief store, and political AI interact through shared state without coupling.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path Validated | Scenario |
|-------------|--------|----------------|----------|
| Office vacancy after combat death | Combat system sets `DeadAt` | `DeadAt` → succession system detects → vacancy mutation → force-law controller → hold delay → installation | 45 |
| Office vacancy (remote agent via Tell) | Speaker's `WitnessedEvent` institutional belief | `seed_office_holder_belief()` → speaker's store → Tell action → `InstitutionalClaim` with degraded `Report` source → listener's store → ClaimOffice candidate → travel → declare_support → installation | 46 |
| Office vacancy (local agent) | Direct institutional belief | `seed_office_holder_belief()` with `WitnessedEvent` → agent's store → ClaimOffice candidate → ranking against care goals | 44 |
| Wound state for care priority | Wound system | `stable_wound_list()` → belief view reads wounds → care candidate generation → ranking against enterprise goals | 44 |

**Key validation**: All three scenarios use E16c institutional beliefs through proper channels. No runtime seam or omniscient shortcut participates.

### H.2 Positive-Feedback Analysis

**Loop 1: Combat death → political vacancy → political ambition → more combat**. An agent kills an office holder, gains the office, becomes a target. Bounded by combat duration, succession delay, wound accumulation, and hostility requirement. Scenario 45 exercises one iteration only.

**Loop 2: Tell → office knowledge → office claim → new Tell subjects**. An agent learns of an office via Tell, claims it, the outcome becomes a new Tell subject. Bounded by Tell duration, co-location requirement, conversation memory retention, succession delay, and enterprise motivation threshold. Scenario 46 exercises one iteration only.

**Loop 3 (negative/stabilizing): Wound pressure → care action → reduced wound → reduced care pressure**. Scenario 44 exercises this alongside enterprise pressure.

No positive-feedback loops require additional dampening beyond existing systems.

### H.3 Concrete Dampeners

- Combat action duration and wound accumulation (physical time and health cost)
- Succession period delay (physical time gate on office installation)
- Force-law uncontested hold period (physical time gate on force installation)
- Tell action duration and co-location requirement (physical proximity and time cost)
- Conversation memory retention window (prevents Tell spam)
- Enterprise weight and utility profile variation (agent-specific motivation thresholds)
- Office eligibility rules (physical precondition gate)
- Travel time for remote offices (physical distance cost)

### H.4 Stored State vs Derived

**Stored (authoritative)**: `DeadAt` component, office-holder relation, `OfficeData` component, `OfficeForceState` component, `OfficeForceProfile` component, `ContestsOffice` relation (force claims), `WoundList` component, `HomeostaticNeeds` component, `AgentBeliefStore` institutional claims, conversation memory (told/heard)

**Derived (transient)**: vacancy detection (from office-holder + DeadAt), force controller establishment (from claimant presence + timing), ClaimOffice candidate presence (from institutional beliefs), goal priority ordering (from utility weights + needs + wounds), Tell candidate selection (from belief store + conversation memory), final office-holder outcome (consequence of prior changes)

## Cross-System Interactions (Principle 24)

### Scenario 45 chain
1. Combat system reads `CombatProfile` + wounds → attack action resolves → wounds accumulate
2. Combat/wound system detects wound_load >= wound_capacity → sets `DeadAt`
3. Succession system reads office-holder relation + `DeadAt` → clears holder → activates vacancy
4. Force succession reads force claimants → sole live present claimant at jurisdiction → establishes controller
5. After uncontested hold period, force succession installs controller as office holder

### Scenario 46 chain
1. Speaker holds institutional belief about vacant office (`WitnessedEvent` source, seeded at tick 8)
2. Social candidate generation reads speaker's beliefs → ShareBelief goal
3. Tell action commits → `InstitutionalClaim` transferred to listener's store with degraded `Report` source
4. Political candidate generation reads listener's institutional beliefs → ClaimOffice candidate
5. Listener travels from BanditCamp to VillageSquare (office jurisdiction)
6. Listener declares support → succession installs listener

### Scenario 44 chain
1. Agent has institutional belief about vacant office AND wounds AND medicine
2. Candidate generation produces both TreatWounds and ClaimOffice goals
3. Ranking resolves priority based on utility weights: pain_weight vs enterprise_weight
4. Variant A: care wins → heal → then declare_support. Variant B: enterprise wins → declare_support → then heal.

## Tickets

### S13-001: Scenario 45 — Combat Death → Vacancy → Succession

**Status**: COMPLETED

**Deliverable**: `golden_combat_death_triggers_force_succession` + `golden_combat_death_triggers_force_succession_replays_deterministically` in `golden_emergent.rs` (Suite 5)

**Shipped assertion surface**:
- Action trace: attack committed before or at incumbent death tick
- Event-log delta: B dies, vacancy mutation occurs, A becomes office holder after force-law delay
- Event ordering: death → vacancy → installation with succession delay ≥ 5 ticks
- Politics trace: VacancyActivated → ForceControllerEstablished → ForceControllerMaintained → ForceInstalled
- No `declare_support` commit occurs
- Cross-layer timeline renders action + authoritative + political layers
- Conservation: Coin totals preserved every tick
- Determinism: replay companion

---

### S13-002: Scenario 46 — Social Tell → Political Emergence

**Status**: COMPLETED

**Deliverable**: `golden_tell_propagates_political_knowledge` + `golden_tell_propagates_political_knowledge_replays_deterministically` in `golden_emergent.rs` (Suite 6)

**Shipped assertion surface**:
- Decision traces: B has no ClaimOffice before Tell; B has ClaimOffice after Tell
- Authoritative: B becomes office holder at VillageSquare
- Action traces: tell (with InstitutionalClaim detail) committed before declare_support
- Belief provenance: Report { from: informant, chain_len: 1 }
- B starts at BanditCamp, ends at VillageSquare (traveled after learning)
- Determinism: replay companion

---

### S13-003: Scenario 44 — Wounded Politician Enterprise-vs-Care Priority

**Status**: COMPLETED

**Deliverable**: `golden_wounded_politician_pain_first` + `golden_wounded_politician_enterprise_first` + `golden_wounded_politician_replays_deterministically` in `golden_emergent.rs` (Suite 2)

**Shipped assertion surface**:
- Decision traces: both TreatWounds and ClaimOffice generated
- Action traces: heal and declare_support commit ticks compared per variant
- Authoritative: agent becomes office holder AND wound load decreased AND medicine consumed
- Conservation: medicine totals never increase
- Determinism: replay companion

---

### S13-004: Update golden-e2e-coverage.md

**Status**: COMPLETED

**Deliverable**: Add S13 scenarios (44/45/46) to the removed-backlog section of `docs/golden-e2e-coverage.md` with completion date and brief description.

---

### S13-005: Add structured scenario metadata to source comments

**Status**: COMPLETED

**Deliverable**: Convert the `// Suite 2`, `// Suite 5`, `// Suite 6` comment blocks in `golden_emergent.rs` to structured `// Scenario 44:`, `// Scenario 45:`, `// Scenario 46:` annotations with metadata fields (`Systems:`, `GoalKinds:`, `ActionDomains:`, `Places:`, `Principles:`). Run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate docs.

**Metadata for Scenario 44**:
```
// Scenario 44: Wounded Politician Enterprise vs Care Priority
// ---------------------------------------------------------------------------
//
// Systems: Care, Politics, AI, Succession
// GoalKinds: TreatWounds, ClaimOffice
// ActionDomains: Care, Social
// Places: VillageSquare
// Principles: 3, 20, 24
//
// Proves care and political ambition follow the shared ranking pipeline.
// Medium pain can outrank office ambition, while low pain can leave the office
// claim path ahead, all without office-specific priority exceptions.
```

**Metadata for Scenario 45**:
```
// Scenario 45: Combat Death Triggers Force Succession
// ---------------------------------------------------------------------------
//
// Systems: Combat, Politics, AI
// GoalKinds: EngageHostile
// ActionDomains: Combat, Social
// Places: VillageSquare
// Principles: 1, 9, 24
//
// Proves challenger AI can open combat against an incumbent, and the resulting
// death drives force-law succession entirely through authoritative world state
// and event history. No combat-specific political hook participates.
```

**Metadata for Scenario 46**:
```
// Scenario 46: Social Tell Propagates Political Knowledge
// ---------------------------------------------------------------------------
//
// Systems: Social, Beliefs, Travel, AI, Politics, Succession
// GoalKinds: ShareBelief, ClaimOffice
// ActionDomains: Social, Movement
// Places: BanditCamp, VillageSquare
// Principles: 1, 7, 13, 24
//
// Proves the social Tell system can lawfully move institutional office knowledge
// into the political planning layer, unlocking the ordinary office-claim path
// without belief injection shortcuts or political/social coupling.
```

## Critical Files

| File | Role |
|------|------|
| `specs/S13-political-emergence-golden-suites.md` | This spec |
| `crates/worldwake-ai/tests/golden_emergent.rs` | Contains shipped Suites 2/5/6 (~8 tests + replay companions) |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | Harness helpers used by all three suites |
| `docs/golden-e2e-coverage.md` | S13-004: add removed-backlog entry |
| `docs/generated/golden-scenario-map.md` | S13-005: regenerated from source annotations |

## Verification

Per remaining ticket:
1. S13-004: Check `docs/golden-e2e-coverage.md` contains S13 removed-backlog entry
2. S13-005: `python3 scripts/golden_inventory.py --write --check-docs` succeeds and `docs/generated/golden-scenario-map.md` contains Scenarios 44/45/46

Post-all-tickets:
3. `cargo test -p worldwake-ai golden_wounded_politician golden_combat_death golden_tell_propagates` — all 8 tests pass
4. `cargo test --workspace` — workspace suite green
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean

## Implementation Order

S13-001 ✅ → S13-002 ✅ → S13-003 ✅ → S13-004 ✅ + S13-005 ✅ (parallel)

## Outcome

**Completion date**: 2026-03-28

**What changed**:
- 7 golden tests in `golden_emergent.rs` (Suites 2/5/6): 3 main tests + 2 variant tests + 3 replay companions, covering combat-death force succession, social Tell political knowledge propagation, and wounded-politician care-vs-enterprise priority ordering
- Structured `// Scenario 44/45/46:` source annotations with metadata fields (Systems, GoalKinds, ActionDomains, Places, Principles)
- `docs/golden-e2e-coverage.md` removed-backlog entry for S13
- `docs/generated/golden-scenario-map.md` regenerated with Scenarios 44/45/46

**Deviations from original spec**:
- Scenario numbers changed from 21/22/23 to 44/45/46 (originals were already assigned to other tests)
- Scenario 45 (combat death): incumbent uses `ControlSource::None` (passive) rather than AI-driven; challenger has pre-seeded force claim + hostility (not just hostility)
- Scenario 46 (Tell): agents start at BanditCamp (remote from office), not VillageSquare; informant office knowledge seeded mid-run at tick 8 rather than at setup; uses `blind_perception_profile()` and `focused_accepting_tell_profile()`
- Scenario 44 (wounded politician): wound severities differ between variants (400 vs 200) rather than being identical

**Verification**: All 7 tests pass. `golden_inventory.py --write --check-docs` succeeds with 60 scenario blocks.
