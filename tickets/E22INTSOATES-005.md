# E22INTSOATES-005: T29 — Theft → Delayed Discovery → Wrongful Accusation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E22INTSOATES-001

## Problem

Existing crime goldens (Scenarios 41–43) test fine-to-exile fallback, witness deterrence, and dual-discovery convergence. None test accusation of a wrong suspect from imperfect perception, which exercises the belief architecture's tolerance for contradiction and error (Principle 16: ignorance and contradiction are first-class). T29 is the E22 instantiation of FOUNDATIONS Section G canonical regression scenario.

## Assumption Reassessment (2026-03-31)

1. `TheftDispositionProfile` exists with `steal_duration_ticks`, `theft_motive_weight`, `witness_risk_penalty` — confirmed in `crates/worldwake-core/src/crime.rs`.
2. `JusticeDispositionProfile` exists with `accusation_motive_weight`, `fine_severity` — confirmed.
3. `TheftFacts` struct exists with commodity/quantity/suspect fields — confirmed in `crates/worldwake-core/src/crime.rs`.
4. `ViolationKind::SuspectedTheft` exists — confirmed in `crates/worldwake-core/src/violation.rs`.
5. `ViolationKind::EntityMissing` exists — confirmed.
6. `PerceptionProfile` with `observation_fidelity` exists — confirmed in `crates/worldwake-core/src/belief.rs`.
7. `GoalKind::StealItem` exists — confirmed in goal.rs.
8. `GoalKind::ShareBelief` exists — confirmed.
9. `GoalKind::Accuse` exists — confirmed.
10. `GoalKind::PunishAccused` exists — confirmed.
11. `GoalKind::InvestigateViolation` exists — confirmed.
12. Institutional accusation records exist via `InstitutionalRecordEntry` in `crates/worldwake-core/src/institutional.rs` — confirmed.
13. T29 exercises Transport→Epistemic→Social→Generic domains. Setup includes thief, bystander, witness, owner, and authority.
14. Low `observation_fidelity` on witness (Permille(400)) may cause misattribution — this is the core test. The world must support both correct and incorrect outcomes depending on seed (Principle 16).
15. Tick budget: ≤ 2880 ticks (2 days). Theft takes 3 ticks, discovery/propagation/accusation follow.

## Architecture Check

1. T29 exercises the existing theft → perception → social propagation → institutional action chain. No new action types. The test proves that low-fidelity perception produces misattribution naturally through the general belief system.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Theft execution → action trace (StealItem action committed at Storehouse)
2. Owner discovery → authoritative component state (`ViolationKind::SuspectedTheft` or `ViolationKind::EntityMissing` in owner's `ViolationMemory`)
3. Witness perception → belief store (witness's `AgentBeliefStore` contains theft-related belief with suspect field)
4. Social propagation → event-log delta (ShareBelief action from witness to owner/authority)
5. Institutional action → authoritative state (accusation record in institutional records)
6. No omniscient correction → decision trace on authority (authority acts on received information, not world-state reads)
7. Perception-determined suspect → `TheftFacts` in accusation record (suspect determined by perception, not omniscience)
8. Cross-domain ≥ 4 → event-log scan ({Transport, Epistemic, Social, Generic})
9. Determinism → state hash comparison across 2 seeds (but outcomes may differ between seeds per Principle 16)

## What to Change

### 1. Add T29 scenario to `crates/worldwake-ai/tests/golden_integration.rs`

- Build 4-place topology: Market, Storehouse, Tavern, GuardPost
- Owner agent with owned Apple lots at Storehouse, beliefs recording their presence
- Thief agent with `TheftDispositionProfile { steal_duration_ticks: NonZeroU32(3), theft_motive_weight: Permille(800), witness_risk_penalty: Permille(400) }`
- Innocent bystander at Storehouse (present near time of theft)
- Witness agent with `PerceptionProfile { observation_fidelity: Permille(400) }` (low fidelity)
- Justice authority at GuardPost with `JusticeDispositionProfile { accusation_motive_weight: Permille(700), fine_severity: Permille(500) }`
- Run up to 2880 ticks
- Verify causal chain: theft → owner discovery → witness perception → social propagation → institutional action
- Verify `TheftFacts` suspect determined by perception, not omniscience
- Verify authority never acts on information it could not have received physically (Principle 7)
- Verify event log shows traceable information path
- `fn run_t29_wrongful_accusation(seed: Seed) -> (StateHash, StateHash)`
- Two `#[test]` functions

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to crime system, perception system, or social propagation
- Modifying existing crime golden scenarios (41–43)
- Any engine code changes
- Explicit verification of both "correct" and "incorrect" outcomes across seeds — both are acceptable; the test verifies the chain occurs, not a specific suspect

## Acceptance Criteria

### Tests That Must Pass

1. `t29_wrongful_accusation_seed_1` — theft → discovery → witness → propagation → accusation chain
2. `t29_wrongful_accusation_seed_2` — determinism verification
3. `TheftFacts` in accusation record has correct commodity and quantity but suspect determined by perception
4. Event log shows traceable information path: theft → witness perception → social transmission → institutional action
5. Authority never acts on information it could not have received through a physical channel (Principle 7)
6. Event log crosses ≥ 4 `ActionDomain` values from {Transport, Epistemic, Social, Generic}
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No omniscient correction: authority does not consult world state to determine true thief (Principle 14)
2. Information locality: all knowledge reaches agents through physical carriers (Principle 7)
3. Contradiction tolerance: world supports both correct and incorrect outcomes depending on seed (Principle 16)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t29_wrongful_accusation_seed_1` — proves imperfect-perception accusation chain
2. `crates/worldwake-ai/tests/golden_integration.rs::t29_wrongful_accusation_seed_2` — determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t29`
2. `cargo test --workspace`
