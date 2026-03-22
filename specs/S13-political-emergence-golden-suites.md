**Status**: PENDING

# S13: Political Emergence Golden E2E Suites

## Summary

Add 3 cross-system emergence golden tests to `golden_emergent.rs` that prove the political system (E16/E16d) participates in emergent multi-system chains. Currently all political tests live in `golden_offices.rs` and exercise politics in relative isolation. These new scenarios prove that combat, social, and care systems interact with politics through shared state (Principle 24) to produce outcomes no single system orchestrates (Principle 1).

## Phase

Phase 3: Information & Politics (post-E16c)

## Crate

`worldwake-ai` (golden tests only — no new system code)

## Dependencies

- E16c (institutional beliefs — provides `InstitutionalClaim` types, institutional belief storage in `AgentBeliefStore`, belief-based `office_holder()` and `office_data()` queries; replaces the pre-E16c runtime seam where `PerAgentBeliefView::office_holder()` reads live world state)
- E16d (political planning, bribe/threaten semantics, golden harness office helpers)
- E12 (combat, wounds, death)
- S07 (care golden tests — establishes the emergent test patterns)
- E14 (perception/belief system — belief boundary, social observation, Tell)

## Scenarios

### Scenario 21: Combat Death Triggers Office Vacancy and Autonomous Succession

**File**: `golden_emergent.rs`
**Systems exercised**: Combat (attack, wound infliction, death), Politics (vacancy detection via `DeadAt`, succession resolution), action tracing, event-log delta inspection, deterministic replay
**Principles proven**: P1 (maximal emergence — combat consequence cascades into political domain), P24 (systems interact only through state — no combat-politics coupling), P9 (combat aftermath triggers downstream emergence)

**Setup**:
- Occupied office ("War Chief") at VillageSquare with `SuccessionLaw::Force`, succession_period=5
- Agent A ("Challenger"): sated, armed (high attack_skill), perception profile. No eligibility rules on office.
- Agent B ("Incumbent"): office holder, armed (moderate guard_skill, lower attack_skill). Also has perception profile.
- A and B co-located at VillageSquare. A has hostility toward B (triggers EngageHostile).

**Emergent behavior proven**:
- A attacks B through the real combat system (EngageHostile goal → attack action).
- B suffers wounds → bleed → death (`DeadAt` component set by wound system).
- Politics system later detects that the living holder is gone, clears the office-holder relation, and after the succession delay installs A as the sole living eligible contender.
- No `ClaimOffice` / `DeclareSupport` path participates in the installation.
- No orchestrator connects combat to politics — the chain emerges from `DeadAt` state.

**Assertion surface**:
1. Action trace: `attack` commits before any political installation result
2. Event-log delta / authoritative state: B is dead (`DeadAt`), vacancy mutation occurs, and A later becomes office holder
3. Negative action-path check: no `declare_support` commit occurs anywhere in the chain
4. Determinism: replay companion

**Why this is distinct from Scenario 19** (force succession):
- Scenario 19 pre-kills the holder via `DeadAt(Tick(0))`. The death is setup, not emergent.
- Scenario 21 has the death **emerge from simulated combat**. The combat system, wound system, and politics system interact through shared state without any coupling.

**E16c requirement**: The succession system itself reads authoritative world state (it is a world-state system, not belief-dependent). However, if A's autonomous ClaimOffice candidate generation participates (e.g., under support-law variant), that path must read institutional beliefs from E16c's `AgentBeliefStore` institutional claim storage, not from the pre-E16c runtime seam in `PerAgentBeliefView::office_holder()`. Even under force-law where succession resolution is world-state-driven, the golden test should verify that any political AI candidate generation that fires uses the E16c belief path.

---

### Scenario 22: Social Tell Propagates Political Knowledge and Triggers Office Claim

**File**: `golden_emergent.rs`
**Systems exercised**: Social (autonomous Tell), Institutional belief store (E16c `InstitutionalClaim` transfer via Tell), AI (ClaimOffice candidate generation from institutional belief), Political actions (DeclareSupport), Succession
**Principles proven**: P7 (information locality — political knowledge arrives via social channel at finite speed), P1 (maximal emergence — Tell → belief → political action is emergent), P13 (knowledge acquisition path matters)

**Setup**:
- Vacant office ("Village Elder") at VillageSquare with `SuccessionLaw::Support`, succession_period=5, no eligibility rules
- Agent A ("Informant"): at VillageSquare, has DirectObservation institutional belief about the office vacancy (E16c `InstitutionalClaim` in institutional belief state), social_weight=pm(600), low enterprise_weight (won't claim office). Perception profile. Tell profile for sending.
- Agent B ("Ambitious Listener"): at VillageSquare, enterprise_weight=pm(800), NO institutional belief about the office initially (no `InstitutionalClaim` for this office in belief state). Perception profile. Tell profile for reception.
- Both co-located so Tell can occur.

**Emergent behavior proven**:
- Phase 1 (no office knowledge): B generates no ClaimOffice candidates because B has no institutional belief about the office. Decision trace proves negative: no political candidate in B's traces. The absence is caused by missing institutional belief state (E16c), not by missing entity belief.
- A autonomously generates ShareBelief goal and tells B about the office entity. The Tell system transfers institutional belief state (E16c `InstitutionalClaim`) alongside the entity belief, giving B knowledge of the office vacancy.
- Phase 2 (after Tell): B's institutional belief store now contains the office claim. B generates ClaimOffice. B declares support for self. Succession installs B.
- The political goal emergence is caused by social institutional belief transfer, not manual belief injection or runtime-seam shortcuts.

**Assertion surface**:
1. Decision traces: B has no political candidates before Tell; B has ClaimOffice after Tell
2. Authoritative: B becomes office holder
3. Action traces: tell committed before declare_support
4. Determinism: replay companion

**Why this is distinct from Scenario 16/20** (information locality):
- Scenario 16/20 manually injects a `PerceptionSource::Report` belief via test setup. The information transfer is artificial.
- Scenario 22 has the information arrive through the **autonomous Tell system**. The social system, belief store, and political AI interact through shared state without coupling.

**E16c requirement**: The current Tell system transfers `BelievedEntityState` which contains no institutional fields (no office-holder, no faction, no support data). E16c must extend the belief transfer mechanism so that institutional claims about an office entity can be transferred via Tell. This scenario validates that the E16c institutional belief transfer works end-to-end through the autonomous social system.

---

### Scenario 23: Wounded Politician — Enterprise vs Care Priority Resolution

**File**: `golden_emergent.rs`
**Systems exercised**: Care (self-treatment, wound state), Politics (ClaimOffice, DeclareSupport), AI (utility-weight-driven ranking across care and enterprise domains), Conservation
**Principles proven**: P3 (concrete state — wounds and utility weights, not abstract priority tiers), P20 (agent diversity — same wound + same office, different weights → different behavior), P24 (care and politics coordinate through state, not cross-system calls)

**Setup**: Two sub-variants with identical world but different utility profiles:

**Variant A (pain-first)**:
- Agent with pain_weight=pm(800), enterprise_weight=pm(400)
- Wounded (stable wound, no natural recovery), has medicine + institutional belief about the office vacancy (E16c `InstitutionalClaim` in institutional belief state)
- Vacant office at same location

**Variant B (enterprise-first)**:
- Agent with pain_weight=pm(300), enterprise_weight=pm(800)
- Same wounds, same medicine, same institutional belief about the office vacancy

**Emergent behavior proven**:
- Variant A: Agent self-heals first (wound load decreases), then claims office (DeclareSupport)
- Variant B: Agent claims office first (DeclareSupport), then self-heals
- Both agents eventually both heal and become office holder — but the ordering differs based on concrete utility weights
- This is the political extension of the wound-vs-hunger pattern (Suites S07a/S07b)

**Assertion surface**:
1. State deltas: which outcome occurs first (wound decrease vs office installation)
2. Authoritative: both variants end with agent as office holder AND wound load decreased
3. Conservation: medicine and commodity totals
4. Determinism: replay companion

**Why this is distinct from Scenario 17** (survival suppresses politics):
- Scenario 17 tests hunger suppression — a binary gate (suppressed or not).
- Scenario 23 tests **priority ordering** between two non-survival domains (care vs enterprise) resolved by concrete utility weights, proving Principle 20 diversity across the political domain.

**E16c requirement**: Pre-E16c, the AI reads office data through the runtime seam in `PerAgentBeliefView::office_holder()` which queries live world state. Post-E16c, agents must read institutional beliefs from their own belief store. This scenario validates that priority ordering between care and enterprise goals works correctly when political knowledge arrives through proper institutional beliefs rather than the runtime seam.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path Validated | Scenario |
|-------------|--------|----------------|----------|
| Office vacancy after combat death | Combat system sets `DeadAt` | `DeadAt` → succession system detects → vacancy mutation → institutional belief observation (DirectObservation or institutional event witness) → AI candidate generation | 21 |
| Office vacancy (remote agent via Tell) | Office world state | Speaker DirectObservation → E16c institutional belief in speaker's store → Tell action transfers `InstitutionalClaim` → listener institutional belief store → ClaimOffice candidate generation | 22 |
| Office vacancy (local agent) | Office world state | DirectObservation → E16c institutional belief in agent's store → enterprise candidate generation → ranking against care goals | 23 |
| Wound state for care priority | Combat/needs systems | `WoundList` component → belief view reads wounds → care candidate generation → ranking | 23 |

**Key validation**: All three scenarios prove political knowledge travels through E16c institutional beliefs, not through the pre-E16c runtime seam.

### H.2 Positive-Feedback Analysis

**Loop 1: Combat death → political vacancy → political ambition → more combat**. An agent kills an office holder, gains the office, becomes a target. Bounded by combat duration, succession delay, wound accumulation, and hostility requirement. Scenario 21 exercises one iteration only.

**Loop 2: Tell → office knowledge → office claim → new Tell subjects**. An agent learns of an office via Tell, claims it, the outcome becomes a new Tell subject. Bounded by Tell duration, co-location requirement, conversation memory retention, succession delay, and enterprise motivation threshold. Scenario 22 exercises one iteration only.

**Loop 3 (negative/stabilizing): Wound pressure → care action → reduced wound → reduced care pressure**. Scenario 23 exercises this alongside enterprise pressure.

No positive-feedback loops require additional dampening beyond existing systems.

### H.3 Concrete Dampeners

- Combat action duration and wound accumulation (physical time and health cost)
- Succession period delay (physical time gate on office installation)
- Tell action duration and co-location requirement (physical proximity and time cost)
- Conversation memory retention window (prevents Tell spam)
- Enterprise weight and utility profile variation (agent-specific motivation thresholds)
- Office eligibility rules (physical precondition gate)
- Travel time for remote offices (physical distance cost)

### H.4 Stored State vs Derived

**Stored (authoritative)**: `DeadAt` component, office-holder relation, `OfficeData` component, `WoundList` component, `HomeostaticNeeds` component, `AgentBeliefStore` institutional claims (E16c), conversation memory (told/heard)

**Derived (transient)**: vacancy detection (from office-holder + DeadAt), ClaimOffice candidate presence (from institutional beliefs), goal priority ordering (from utility weights + needs + wounds), Tell candidate selection (from belief store + conversation memory), final office-holder outcome (consequence of prior changes)

## Cross-System Interactions (Principle 24)

### Scenario 21 chain
1. Combat system reads `CombatProfile` + wounds → attack action resolves → wounds accumulate
2. Needs/wound system detects wound_load >= wound_capacity → sets `DeadAt`
3. Succession system reads office-holder relation + `DeadAt` → clears holder → starts succession period
4. After succession delay, succession installs sole eligible contender under force-law

### Scenario 22 chain
1. Speaker holds E16c institutional belief about vacant office (DirectObservation)
2. Social candidate generation reads speaker's beliefs → ShareBelief goal
3. Tell action commits → `InstitutionalClaim` transferred to listener's belief store
4. Political candidate generation reads listener's institutional beliefs → ClaimOffice candidate
5. Listener declares support → succession installs listener

### Scenario 23 chain
1. Agent has E16c institutional belief about vacant office AND wounds AND medicine
2. Candidate generation produces both TreatWounds and ClaimOffice goals
3. Ranking resolves priority based on utility weights: pain_weight vs enterprise_weight
4. Variant A: care wins → treat → then claim. Variant B: enterprise wins → claim → then treat.

## Tickets

### S13-001: Scenario 21 — Combat Death → Vacancy → Succession

**Deliverable**: `golden_combat_death_triggers_force_succession` + `golden_combat_death_triggers_force_succession_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- Action trace: attack committed before office installation
- Event-log delta / authoritative state: B dies, vacancy mutation occurs, and A becomes office holder after the force-law delay
- No `declare_support` commit occurs
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession`, then `cargo test --workspace`, then `cargo clippy --workspace`

---

### S13-002: Scenario 22 — Social Tell → Political Emergence

**Deliverable**: `golden_tell_propagates_political_knowledge` + `golden_tell_propagates_political_knowledge_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- Decision traces: B has no political candidates before Tell
- Authoritative: B becomes office holder
- Action traces: tell committed before declare_support
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai golden_tell_propagates`, then `cargo test --workspace`, then `cargo clippy --workspace`

---

### S13-003: Scenario 23 — Wounded Politician Enterprise-vs-Care Priority

**Deliverable**: `golden_wounded_politician_pain_first` + `golden_wounded_politician_enterprise_first` + `golden_wounded_politician_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- State deltas: which outcome occurs first (wound decrease vs office installation)
- Authoritative: both variants end with agent as office holder AND wound load decreased
- Conservation: medicine totals
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai golden_wounded_politician`, then `cargo test --workspace`, then `cargo clippy --workspace`

---

### S13-004: Update golden-e2e-coverage.md

**Deliverable**: Add S13 scenarios to coverage matrix and cross-system chains in `docs/golden-e2e-coverage.md`.

---

### S13-005: Update golden-e2e-scenarios.md

**Deliverable**: Add detailed scenario descriptions for S21-S23 in `docs/golden-e2e-scenarios.md`.

## Critical Files

| File | Role |
|------|------|
| `specs/S13-political-emergence-golden-suites.md` | This spec |
| `crates/worldwake-ai/tests/golden_emergent.rs` | Add 3 new suites (~8 tests + replay companions) |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | May need new helpers |
| `docs/golden-e2e-coverage.md` | Update coverage matrix |
| `docs/golden-e2e-scenarios.md` | Add scenario descriptions |

## Verification

Per ticket:
1. `cargo test -p worldwake-ai <test_name>` — targeted test
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo test --workspace` — workspace suite
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint

After all tickets:
5. Verify `docs/golden-e2e-coverage.md` reflects new scenarios in all matrices
6. Verify `docs/golden-e2e-scenarios.md` has detailed descriptions for S21-S23

## Implementation Order

S13-001 → S13-002 → S13-003 → S13-004 + S13-005 (parallel)
