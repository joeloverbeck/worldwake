**Status**: COMPLETED

# S32: Crime Emergence Golden E2E Suites

## Summary

Add 3 cross-system emergence golden tests to `golden_emergent.rs` that prove the crime/justice system (E17) participates in emergent multi-system chains. Currently the 3 existing crime golden tests (Scenarios 37-39) cover owner-local discovery, the witnessed-theft accusation chain, and stale-Fine traceability. These new scenarios prove that three implemented-but-untested mechanics produce emergent outcomes through system interaction (Principle 24) without scripted orchestration (Principle 1): the Exile punishment fallback, witness-based theft deterrence, and dual-discovery convergence.

## Phase

Phase 3: Information & Politics (post-E17)

## Crate

`worldwake-ai` (golden tests only -- no new system code)

## Dependencies

- E17 (crime/theft/justice -- provides Steal, Accuse, Fine, Exile actions, TheftDispositionProfile, JusticeDispositionProfile, CrimeRegister records, emit_theft_candidates(), emit_justice_candidates())
- E16c (institutional beliefs -- CrimeRegister as RecordKind, InstitutionalClaim::Accusation/Verdict, institutional belief storage)
- E16d (political planning, office helpers in golden harness)
- E14 (perception/belief system -- Hidden visibility evaluation, PerceptionProfile)
- E15 (social transmission -- Tell with SocialObservationKind::SuspectedTheft)
- S27 (expectation-violation goals -- EntityMissing, InvestigateViolation)

## Scenarios

### Scenario 41: Exile Punishment When Fine Is Not Locally Collectible

**File**: `golden_emergent.rs`
**Systems exercised**: Transport (Steal action, Hidden visibility), Perception (witness perceives Hidden event), Social (Tell with SuspectedTheft evidence, Accuse, Exile), AI (candidate generation: emit_theft_candidates, emit_justice_candidates with Fine-to-Exile fallback), Institutions (CrimeRegister, Accusation entry, Verdict entry with PunishmentKind::Exile)
**Principles proven**: P1 (maximal emergence -- justice adapts punishment to material reality without scripted branching), P7 (locality -- evidence travels via witness Tell at finite speed), P21 (institutional authority -- only office holder can punish), P22 (ownership/custody/membership distinctions -- exile revokes faction membership, a relation distinct from ownership or possession), P23 (social artifacts first-class -- Verdict with PunishmentKind::Exile is a durable institutional record), P24 (systems interact through state -- no crime-justice coupling)

**Setup**:
- Thief at VillageSquare: has TheftDispositionProfile (steal_duration_ticks=2, theft_motive_weight=pm(700), witness_risk_penalty=pm(100)), PerceptionProfile, member_of Faction
- Victim: owns an item lot (e.g. Quantity(4) Grain) at VillageSquare, placed elsewhere initially (e.g. GeneralStore) so theft is unwitnessed by victim
- Witness at VillageSquare: PerceptionProfile with sufficient perception to detect Hidden theft, social Tell profile (social_weight=pm(600)), sated needs (so social goals are not suppressed)
- Magistrate at RulersHall: office holder of Office with JusticeDispositionProfile (accusation_motive_weight=pm(700), fine_severity=pm(500)), PerceptionProfile, sated needs
- Office has EligibilityRule::FactionMember(Faction) -- required for Exile to be viable
- CrimeRegister record entity at RulersHall, issued by Office
- Faction entity; Thief is member_of Faction
- Topology: VillageSquare <-> RulersHall (travel edge, 1 tick), VillageSquare <-> GeneralStore (travel edge, 1 tick)
- Isolation step: after the accusation is filed and before punishment selection, the stolen commodity is moved out of local collectible reach while the accused is brought to `RulersHall`, so the authority can still punish the accused but can no longer lawfully generate `Fine` from local observation alone

**Emergent behavior proven**:
The justice system adapts punishment to the accused's locally observable material circumstances without any designer-authored branching. `candidate_punishment_for_case()` attempts Fine first: it checks whether the authority and accused are co-located and whether `locally_observed_commodity_quantity()` for the accused is at least the fine amount. When the stolen commodity is no longer locally collectible from the accused at punishment-selection time, Fine is not emitted. The function falls through to `office_governed_faction_for_accused()`, finds the Thief is member_of a governed Faction, and emits `PunishAccused(Exile)`. The Exile action then removes faction membership and adds hostility from the faction toward the accused. The entire chain -- from theft through witness perception, social Tell, evidence accumulation, institutional accusation, local Fine infeasibility, and Exile fallback -- emerges from system interaction through state without orchestration.

**Why this is distinct from Scenario 38** (witnessed theft accusation chain):
- Scenario 38 tests the Fine path: the accused has commodities, Fine succeeds.
- Scenario 41 tests the **Exile fallback**: the authority can no longer locally collect the fine from the accused, so the system naturally adapts to a different punishment. The branching is driven by material world state and locality of observation, not by a designer "if poor then exile" rule. Additionally, Scenario 41 proves P22 (membership as a distinct revocable relation) and P23 (Verdict with Exile as durable social artifact), which Scenario 38 does not exercise.

**Assertion surface**:
1. Authoritative state: Thief is NOT member_of Faction (faction membership removed)
2. Authoritative state: `hostile_towards(Thief)` includes Faction
3. Authoritative state: CrimeRegister has exactly 1 active Verdict entry with PunishmentKind::Exile { from_faction: Faction }
4. Action trace: Exile action committed by Magistrate (not Fine)
5. Decision trace: PunishAccused goal with PunishmentKind::Exile is generated after the local-collectibility change, and PunishAccused(Fine) is not
6. Conservation: total commodity quantities unchanged (no Fine transfer occurred)
7. Determinism: replay companion

---

### Scenario 42: Witness Deterrence Suppresses Theft Candidate

**File**: `golden_emergent.rs`
**Systems exercised**: AI candidate generation (emit_theft_candidates witness_risk_penalty gate), Perception (co-location observation), Transport (Steal action NOT taken), Needs (competing goal drives alternative behavior)
**Principles proven**: P1 (maximal emergence -- deterrence arises from witness presence, not a deterrence subsystem), P10 (physical dampener on positive feedback -- more witnesses at a location suppress theft, preventing crime-escalation spirals), P20 (agent diversity through concrete variation -- different TheftDispositionProfile values produce different deterrence thresholds), P24 (systems interact through state -- perception feeds into candidate generation without coupling)

**Setup**:
- Would-be Thief at VillageSquare: TheftDispositionProfile with theft_motive_weight=pm(400), witness_risk_penalty=pm(150), steal_duration_ticks=2. Also has HomeostaticNeeds with moderate hunger (e.g. Permille 600), PerceptionProfile, carry capacity sufficient for target item
- 3 other living agents co-located at VillageSquare: all have PerceptionProfile (so they count as observed witnesses in the Thief's locally_observed_entities). Sated needs, no TheftDispositionProfile themselves.
- Stealable target: an item lot (e.g. Quantity(3) Grain) at VillageSquare, owned by another agent, not possessed by anyone, not in a container, within Thief's carry capacity
- A food source or food lot reachable by Thief (so the competing hunger goal has a viable plan)

**Emergent behavior proven**:
Crime rate self-regulates through witness presence. In `emit_theft_candidates()`, the witness penalty calculation is: witness_risk_penalty(pm(150)) * witness_count(3) = 450 > theft_motive_weight(pm(400)). The function early-returns with zero StealItem candidates. The Thief, with no theft option available, pursues the next-highest-priority goal (hunger-driven Eat or AcquireCommodity). This is a **physical dampener** (P10): the deterrent is not a numeric cap or probability roll, but the concrete presence of other agents who could witness the crime. Reducing the witness count below 3 would flip the outcome (400 > 150*2=300), demonstrating that the threshold is emergent from per-agent profile values (P20).

**Why this is distinct from all existing scenarios**:
No existing golden test exercises the witness_risk_penalty suppression path. Scenarios 37-39 all have theft succeed. This scenario proves the **absence** of crime as an emergent outcome of social context, which is architecturally the P10 dampener that prevents crime-escalation loops.

**Assertion surface**:
1. Decision trace: at every tick where Thief is co-located with 3+ witnesses, no StealItem candidate appears in candidates.generated
2. Decision trace: Thief selects a non-theft goal (e.g. ConsumeCommodity, AcquireCommodity) and plans successfully
3. Authoritative state: item lot remains at VillageSquare, owned by original owner, possession unchanged
4. Authoritative state: Thief's hunger decreases (proving the alternative goal executed)
5. Determinism: replay companion

---

### Scenario 43: Dual Discovery Converges Without Double Accusation

**File**: `golden_emergent.rs`
**Systems exercised**: Transport (Steal), Perception (Hidden event detection, EntityMissing violation), AI (InvestigateViolation candidate from S27, ShareBelief candidate, Accuse candidate from emit_accusation_candidates), Social (Tell with SuspectedTheft evidence), Institutions (CrimeRegister accusation duplicate check)
**Principles proven**: P1 (maximal emergence -- two independent discovery paths converge without orchestration), P7 (information locality -- each path has distinct physical travel requirements), P13 (knowledge acquired locally, travels physically -- witness carries firsthand observation, victim derives evidence from expectation violation), P16 (evidence and records are world state -- CrimeRegister accusation is a durable institutional record, not ephemeral), P24 (systems interact through state -- duplicate check is institutional state, not cross-system logic)

**Setup**:
- Thief at VillageSquare: TheftDispositionProfile, steals item lot owned by Victim
- Victim initially at GeneralStore (adjacent to VillageSquare, travel edge 1 tick): has ViolationDispositionProfile, PerceptionProfile. Victim has a stale belief that item lot is at VillageSquare (will trigger EntityMissing on return when item is gone). Sated needs.
- Witness at VillageSquare: PerceptionProfile (perceives Hidden theft), social Tell profile, sated needs
- Magistrate at RulersHall: office holder, JusticeDispositionProfile, PerceptionProfile, sated needs
- CrimeRegister at RulersHall, issued by Office
- Topology: VillageSquare <-> GeneralStore (1 tick), VillageSquare <-> RulersHall (1 tick)
- Thief departs to CommonHouse after stealing (so item is gone when Victim returns)

**Emergent behavior proven**:
Two independent information channels discover the same crime through different mechanisms:
- **Path A (witness)**: Witness perceives Hidden theft at VillageSquare → generates ShareBelief(SocialObservation(SuspectedTheft)) goal → travels to RulersHall → tells Magistrate → Magistrate receives SuspectedTheft evidence with suspect=Some(Thief)
- **Path B (owner-local)**: Victim travels to VillageSquare → stale belief produces EntityMissing violation → InvestigateViolation action → investigation upgrades to SuspectedTheft evidence → Victim tells Magistrate (or Magistrate observes)

Both paths produce SuspectedTheft evidence that reaches the Magistrate. The duplicate check in `emit_accusation_candidates()` (lines 369-384 of candidate_generation.rs) ensures that once an Accusation or Verdict exists for a given (accused, violation_id) pair, no duplicate accusation is emitted. The institutional system handles convergence gracefully through state, not through inter-system coordination.

**Why this is distinct from Scenarios 37 and 38**:
- Scenario 37 tests only the owner-local path (Path B) in isolation.
- Scenario 38 tests only the witness path (Path A) in isolation.
- Scenario 43 has **both paths active simultaneously**, proving that convergence is handled by institutional state (the duplicate check reads existing CrimeRegister entries) rather than by disabling one path.

**Assertion surface**:
1. Authoritative state: CrimeRegister contains exactly 1 Accusation entry for (accused=Thief, violation_id matching the theft)
2. Decision trace: Magistrate generates Accuse candidate in at least one tick after receiving evidence from Path A
3. Decision trace or authoritative state: after the first accusation is filed, subsequent Accuse candidates for the same (accused, violation_id) are suppressed by the duplicate check
4. Authoritative state: both Witness and Victim develop SuspectedTheft evidence (proving both paths activated independently)
5. Determinism: replay companion

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path Validated | Scenario |
|-------------|--------|----------------|----------|
| Theft evidence (witness firsthand) | Hidden theft event at co-location | Witness PerceptionProfile evaluates Hidden event -> SuspectedTheft social observation -> Tell to Magistrate -> Magistrate ViolationMemory | 41, 43 |
| Theft evidence (owner-local discovery) | Stale belief vs observed reality | Victim returns to expected location -> EntityMissing violation -> InvestigateViolation -> SuspectedTheft with suspect:None | 43 |
| Accused material state (for punishment) | Magistrate co-location observation | Magistrate locally_observed_commodity_quantity() reads accused's locally collectible inventory at the punishment place -> Fine feasibility check | 41 |
| Faction membership (for Exile) | Office eligibility rules + membership relation | office_governed_faction_for_accused() reads EligibilityRule::FactionMember -> believed_membership() | 41 |
| Witness count (for deterrence) | Co-location observation | emit_theft_candidates() counts locally_observed living agents at same place | 42 |
| Existing accusation (for duplicate check) | CrimeRegister institutional beliefs | emit_accusation_candidates() reads known institutional beliefs for matching Accusation/Verdict | 43 |

**Key validation**: All three scenarios prove crime information travels through explicit perception, social Tell, and institutional record consultation -- never through omniscient shortcuts.

### H.2 Positive-Feedback Analysis

**Loop 1: Theft -> punishment -> deterrence -> less theft (stabilizing)**. Theft triggers justice response (accusation, punishment). Punishment consequences (exile, fines) and increased witness alertness reduce future theft incentive. Scenario 42 exercises the witness-deterrence segment of this loop. Scenario 41 exercises the punishment segment. The loop is self-dampening.

**Loop 2: Crime -> accusation -> more accusations for same crime (would-be amplifying, but checked)**. Multiple discovery paths could produce multiple accusations for the same crime. The duplicate check in emit_accusation_candidates() prevents amplification. Scenario 43 exercises this check.

No positive-feedback loops require additional dampening beyond existing systems.

### H.3 Concrete Dampeners

- Witness_risk_penalty * witness_count >= theft_motive_weight: physical presence of observers deters theft (Scenario 42)
- Fine precondition: authority must locally observe sufficient collectible commodity on the accused at punishment-selection time; otherwise Fine is naturally blocked (Scenario 41)
- Exile precondition: accused must be faction member; one-time action per faction membership (Scenario 41)
- Steal action duration and Hidden visibility: theft is not instant, giving witnesses time to perceive (Scenarios 41, 43)
- Tell action duration and co-location: evidence transfer requires physical travel and time (Scenarios 41, 43)
- Duplicate accusation check: institutional state prevents same crime from generating redundant proceedings (Scenario 43)
- Investigation action duration: owner-local discovery path requires physical return and investigation time (Scenario 43)

### H.4 Stored State vs Derived

**Stored (authoritative)**: TheftDispositionProfile (per-agent), JusticeDispositionProfile (per-agent), member_of relation, hostile_to relation, CrimeRegister record entity with InstitutionalClaim::Accusation and InstitutionalClaim::Verdict entries, ViolationMemory records, SocialObservation entries in AgentBeliefStore, item lot possession/ownership relations, HomeostaticNeeds

**Derived (transient, recomputable)**: StealItem candidates from emit_theft_candidates() (derived each tick from co-located stealable items, witness count, and profile), Accuse candidates from emit_accusation_candidates() (derived from ViolationMemory + known institutional beliefs), PunishAccused candidates from emit_punishment_candidates() (derived from known accusations + accused material state + faction membership), witness_count * witness_risk_penalty computation (derived from co-located agent observation), Fine feasibility (derived from locally_observed_commodity_quantity)

## Tickets

### GOLDE2E-017: Scenario 41 -- Exile Punishment When Fine Is Not Locally Collectible

**Deliverable**: `golden_exile_punishment_when_fine_is_not_locally_collectible` + `golden_exile_punishment_when_fine_is_not_locally_collectible_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- Authoritative: Thief not member_of Faction; `hostile_towards(Thief)` includes Faction
- Authoritative: CrimeRegister Verdict with PunishmentKind::Exile
- Action trace: Exile committed by Magistrate (no Fine action)
- Decision trace: PunishAccused(Exile) generated after the local-collectibility change, with PunishAccused(Fine) absent
- Conservation: commodity totals unchanged
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible`, then `cargo test --workspace`, then `cargo clippy --workspace`

---

### GOLDE2E-018: Scenario 42 -- Witness Deterrence Suppresses Theft Candidate

**Deliverable**: `golden_witness_deterrence_suppresses_theft_candidate` + `golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- Decision trace: no StealItem in candidates.generated when 3+ witnesses co-located
- Decision trace: non-theft goal selected and planned
- Authoritative: item lot remains at original location with original owner
- Authoritative: Thief's hunger decreases (alternative goal executed)
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft`, then `cargo test --workspace`, then `cargo clippy --workspace`

---

### GOLDE2E-019: Scenario 43 -- Dual Discovery Converges Without Double Accusation

**Deliverable**: `golden_dual_discovery_converges_without_double_accusation` + `golden_dual_discovery_converges_without_double_accusation_replays_deterministically` in `golden_emergent.rs`

**Assertion surface**:
- Authoritative: exactly 1 Accusation in CrimeRegister for (Thief, violation_id)
- Decision trace: Accuse candidate generated after evidence received
- Decision trace or authoritative: duplicate suppression prevents second accusation
- Authoritative: both Witness and Victim develop SuspectedTheft evidence
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges`, then `cargo test --workspace`, then `cargo clippy --workspace`

---

### GOLDE2E-020: Update golden-e2e-coverage.md and regenerate artifacts

**Deliverable**: Add S32 scenarios to pending backlog in `docs/golden-e2e-coverage.md`. After implementation of GOLDE2E-017/018/019, add structured scenario metadata comments to source and run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate `docs/generated/golden-scenario-map.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-coverage-matrix.md`.

## Critical Files

| File | Role |
|------|------|
| `archive/specs/S32-crime-emergence-golden-suites.md` | This spec |
| `crates/worldwake-ai/tests/golden_emergent.rs` | Add 3 new suites (~6 tests + replay companions) |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | May need new crime/justice setup helpers |
| `docs/golden-e2e-coverage.md` | Update backlog |
| `docs/generated/golden-scenario-map.md` | Regenerated from source annotations |

## Verification

Per ticket:
1. `cargo test -p worldwake-ai <test_name>` -- targeted test
2. `cargo test -p worldwake-ai` -- full AI crate suite
3. `cargo test --workspace` -- workspace suite
4. `cargo clippy --workspace --all-targets -- -D warnings` -- lint

After all tickets:
5. Verify `docs/golden-e2e-coverage.md` reflects new scenarios
6. Verify `docs/generated/golden-scenario-map.md` has metadata for S41-S43 (run `python3 scripts/golden_inventory.py --write --check-docs`)

## Implementation Order

GOLDE2E-017 -> GOLDE2E-018 -> GOLDE2E-019 -> GOLDE2E-020

## Outcome

- **Completion date**: 2026-03-27
- **What actually changed**: Shipped all three planned crime-emergence scenarios in `crates/worldwake-ai/tests/golden_emergent.rs` with replay companions:
  - Scenario 41: `golden_exile_punishment_when_fine_is_not_locally_collectible`
  - Scenario 42: `golden_witness_deterrence_suppresses_theft_candidate`
  - Scenario 43: `golden_dual_discovery_converges_without_double_accusation`
  The source annotations now generate scenario-map, inventory, and coverage-matrix entries for the S32 scenarios.
- **Deviations from original plan**: The final closeout used the repository's canonical source-annotation -> generated-doc pipeline rather than keeping S32 in a hand-maintained pending backlog. `GOLDE2E-020`'s pre-implementation wording about "adding S32 to the pending backlog" became stale once the scenarios shipped; the completed end-state is removal from pending backlog, regenerated artifacts, and archival.
- **Verification results**:
  - `python3 scripts/golden_inventory.py --write --check-docs` ✅
  - `cargo test -p worldwake-ai golden_ -- --nocapture` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
