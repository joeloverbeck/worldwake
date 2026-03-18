**Status**: PENDING

# S13: Political Emergence Golden E2E Suites

## Summary

Add 3 cross-system emergence golden tests to `golden_emergent.rs` that prove the political system (E16/E16d) participates in emergent multi-system chains. Currently all political tests live in `golden_offices.rs` and exercise politics in relative isolation. These new scenarios prove that combat, social, and care systems interact with politics through shared state (Principle 24) to produce outcomes no single system orchestrates (Principle 1).

## Phase

Phase 3: Information & Politics (post-E16d)

## Crate

`worldwake-ai` (golden tests only — no new system code)

## Dependencies

- E16d (political planning, bribe/threaten semantics, golden harness office helpers)
- E12 (combat, wounds, death)
- S07 (care golden tests — establishes the emergent test patterns)
- E14 (social/Tell system — for Scenario 22)

## Scenarios

### Scenario 21: Combat Death Triggers Office Vacancy and Autonomous Succession

**File**: `golden_emergent.rs`
**Systems exercised**: Combat (attack, wound infliction, death), Politics (vacancy detection via `DeadAt`, succession resolution), AI (ClaimOffice candidate generation, DeclareSupport planning), Conservation
**Principles proven**: P1 (maximal emergence — combat consequence cascades into political domain), P24 (systems interact only through state — no combat-politics coupling), P9 (combat aftermath triggers downstream emergence)

**Setup**:
- Vacant office ("War Chief") at VillageSquare with `SuccessionLaw::Force`, succession_period=5
- Agent A ("Challenger"): sated, armed (high attack_skill), enterprise_weight=pm(800), perception profile, believes in office. No eligibility rules on office.
- Agent B ("Incumbent"): office holder, armed (moderate guard_skill, lower attack_skill). Also has perception profile.
- A and B co-located at VillageSquare. A has hostility toward B (triggers EngageHostile).

**Emergent behavior proven**:
- A attacks B through the real combat system (EngageHostile goal → attack action).
- B suffers wounds → bleed → death (`DeadAt` component set by wound system).
- Politics system detects vacancy (holder is dead) during its tick.
- With Force law: A is the sole living eligible contender → succession system installs A.
- No orchestrator connects combat to politics — the chain emerges from `DeadAt` state.

**Assertion surface**:
1. Authoritative: B is dead (`agent_is_dead`), A is office holder
2. Action traces: attack committed before office installation
3. Conservation: commodity totals unchanged
4. Determinism: replay companion

**Why this is distinct from Scenario 19** (force succession):
- Scenario 19 pre-kills the holder via `DeadAt(Tick(0))`. The death is setup, not emergent.
- Scenario 21 has the death **emerge from simulated combat**. The combat system, wound system, and politics system interact through shared state without any coupling.

---

### Scenario 22: Social Tell Propagates Political Knowledge and Triggers Office Claim

**File**: `golden_emergent.rs`
**Systems exercised**: Social (autonomous Tell), Belief store (political entity beliefs), AI (ClaimOffice candidate generation from received belief), Political actions (DeclareSupport), Succession
**Principles proven**: P7 (information locality — political knowledge arrives via social channel at finite speed), P1 (maximal emergence — Tell → belief → political action is emergent), P13 (knowledge acquisition path matters)

**Setup**:
- Vacant office ("Village Elder") at VillageSquare with `SuccessionLaw::Support`, succession_period=5, no eligibility rules
- Agent A ("Informant"): at VillageSquare, has DirectObservation belief about the office, social_weight=pm(600), low enterprise_weight (won't claim office). Perception profile. Tell profile for sending.
- Agent B ("Ambitious Listener"): at VillageSquare, enterprise_weight=pm(800), NO belief about the office initially. Perception profile. Tell profile for reception.
- Both co-located so Tell can occur.

**Emergent behavior proven**:
- Phase 1 (no office knowledge): B generates no ClaimOffice candidates. Decision trace proves negative: no political candidate in B's traces.
- A autonomously generates ShareBelief goal and tells B about the office entity.
- Phase 2 (after Tell): B's belief store now contains office knowledge. B generates ClaimOffice. B declares support for self. Succession installs B.
- The political goal emergence is caused by social information transfer, not manual belief injection.

**Assertion surface**:
1. Decision traces: B has no political candidates before Tell; B has ClaimOffice after Tell
2. Authoritative: B becomes office holder
3. Action traces: tell committed before declare_support
4. Determinism: replay companion

**Why this is distinct from Scenario 16/20** (information locality):
- Scenario 16/20 manually injects a `PerceptionSource::Report` belief via test setup. The information transfer is artificial.
- Scenario 22 has the information arrive through the **autonomous Tell system**. The social system, belief store, and political AI interact through shared state without coupling.

**Note**: The Tell system transfers any belief entity (confirmed via `commit_tell` in `tell_actions.rs` which clones `speaker_belief` for any `subject_entity`), so office entity beliefs are transferable.

---

### Scenario 23: Wounded Politician — Enterprise vs Care Priority Resolution

**File**: `golden_emergent.rs`
**Systems exercised**: Care (self-treatment, wound state), Politics (ClaimOffice, DeclareSupport), AI (utility-weight-driven ranking across care and enterprise domains), Conservation
**Principles proven**: P3 (concrete state — wounds and utility weights, not abstract priority tiers), P20 (agent diversity — same wound + same office, different weights → different behavior), P24 (care and politics coordinate through state, not cross-system calls)

**Setup**: Two sub-variants with identical world but different utility profiles:

**Variant A (pain-first)**:
- Agent with pain_weight=pm(800), enterprise_weight=pm(400)
- Wounded (stable wound, no natural recovery), has medicine + office knowledge
- Vacant office at same location

**Variant B (enterprise-first)**:
- Agent with pain_weight=pm(300), enterprise_weight=pm(800)
- Same wounds, same medicine, same office knowledge

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

## Tickets

### S13-001: Scenario 21 — Combat Death → Vacancy → Succession

**Deliverable**: `golden_combat_death_triggers_succession` + `golden_combat_death_triggers_succession_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- B is dead, A is office holder
- Action traces: attack committed before office installation
- Conservation: commodity totals unchanged
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai golden_combat_death`, then `cargo test --workspace`, then `cargo clippy --workspace`

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
