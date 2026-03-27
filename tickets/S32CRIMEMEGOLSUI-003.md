# S32CRIMEMEGOLSUI-003: Scenario 43 — Dual Discovery Converges Without Double Accusation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — golden test only
**Deps**: E17 (crime/theft/justice), E16c (institutional beliefs), E14 (perception), E15 (social Tell), S27 (expectation-violation goals), S32CRIMEMEGOLSUI-001, S32CRIMEMEGOLSUI-002 (implementation order)

## Problem

Two independent crime discovery paths (witness firsthand observation via Path A, and owner-local expectation violation via Path B from S27) have never been exercised simultaneously in a golden test. When both paths independently produce `SuspectedTheft` evidence that reaches the same Magistrate, the duplicate check in `emit_accusation_candidates()` must prevent double accusation. This convergence is handled by institutional state (CrimeRegister entries), not inter-system coordination — proving P1 (maximal emergence) and P16 (evidence and records are world state).

## Assumption Reassessment (2026-03-27)

1. **`emit_accusation_candidates()`** exists at `crates/worldwake-ai/src/candidate_generation.rs:335`. The duplicate check (confirmed at lines 369-384) reads known institutional beliefs for matching `Accusation` or `Verdict` entries for a given `(accused, violation_id)` pair. If a match exists, no duplicate `Accuse` candidate is emitted.
2. **Path A (witness)**: Witness perceives Hidden theft -> records `SocialObservationDetail::SuspectedTheft` -> generates `ShareBelief(SocialObservation(...))` goal -> Tells Magistrate. This path is individually proven by Scenario 38.
3. **Path B (owner-local)**: Victim returns to expected location -> stale belief vs observed reality triggers `EntityMissing` violation -> `InvestigateViolation` action -> investigation upgrades to `SuspectedTheft` evidence. This path is individually proven by Scenario 37 and further by S27 golden tests.
4. **Key difference from Scenarios 37/38**: Both paths activate simultaneously. The test must prove that whichever path's evidence reaches the Magistrate first triggers the accusation, and the second path's evidence does NOT produce a duplicate.
5. **`ViolationDispositionProfile`** on the Victim is required for Path B (`EntityMissing` detection). Confirmed in `crates/worldwake-core/src/violations.rs`.
6. **Stale belief seeding**: Victim needs a pre-seeded belief that the item lot is at VillageSquare. After theft, the item moves with the Thief. When Victim returns and perceives, the stale belief triggers `EntityMissing`.
7. Golden E2E layer — full action registries and system dispatch required. This is the most complex S32 scenario: perception, social Tell, violation detection, investigation, accusation candidate generation, and institutional duplicate check all participate.
12. **Isolation**: Thief departs to a third location (e.g. CommonHouse or OrchardFarm) after stealing, so the item is gone when Victim returns. Thief is switched to `ControlSource::Human` after theft to prevent further AI interference. Victim and Witness both run AI. The test must allow sufficient ticks for both paths to converge.

## Architecture Check

1. This test proves convergence through institutional state — the CrimeRegister entry count is the authoritative assertion. The duplicate check is not a special-purpose deduplication system; it is a natural read of existing institutional records. This validates P16 and P24.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Exactly 1 Accusation in CrimeRegister for (Thief, violation_id) -> authoritative `RecordData` component read, count active entries matching accused
2. Accuse candidate generated after evidence received -> decision trace for Magistrate (`candidates.generated` contains `GoalKind::Accuse`)
3. Duplicate suppression after first accusation -> decision trace: after first accusation entry exists, subsequent ticks show no new `GoalKind::Accuse` for same (accused, violation_id) in `candidates.generated`; OR authoritative state: CrimeRegister entry count remains exactly 1
4. Both Witness and Victim develop SuspectedTheft evidence independently -> authoritative `AgentBeliefStore` reads: both have `SocialObservationDetail::SuspectedTheft` or `ViolationMemory` with matching theft facts
5. Determinism: replay companion test with identical seed produces identical `(StateHash, StateHash)`

## What to Change

### 1. Add `run_dual_discovery_converges_without_double_accusation` function to `golden_emergent.rs`

Setup:
- Thief at VillageSquare: `TheftDispositionProfile { theft_motive_weight: pm(1000), witness_risk_penalty: pm(0), steal_duration_ticks: nz(2) }`, AI-controlled initially
- Victim at GeneralStore (adjacent to VillageSquare, 1-tick travel edge): `ViolationDispositionProfile`, `PerceptionProfile`, sated needs, AI-controlled. Pre-seeded stale belief that item lot is at VillageSquare.
- Witness at VillageSquare: `PerceptionProfile` (high observation_fidelity for Hidden event detection), social Tell profile (broad_accepting), sated needs, AI-controlled
- Magistrate at RulersHall: office holder, `JusticeDispositionProfile`, `PerceptionProfile`, sated needs, AI-controlled
- CrimeRegister at RulersHall, issued by Office
- Faction entity; Thief is `member_of` Faction
- Stealable item lot (e.g. `Quantity(3) Grain`) at VillageSquare, owned by Victim
- Topology: VillageSquare <-> GeneralStore (1 tick), VillageSquare <-> RulersHall (1 tick), VillageSquare <-> OrchardFarm (1 tick)

Execution phases:
1. **Theft phase** (~12 ticks): Run until Thief commits steal. Switch Thief to `ControlSource::Human`. Teleport Thief to OrchardFarm (so item is gone from VillageSquare).
2. **Discovery phase** (~80-100 ticks): Both paths activate in parallel:
   - Witness generates `ShareBelief(SuspectedTheft)` and travels to RulersHall to Tell Magistrate
   - Victim travels to VillageSquare, detects `EntityMissing`, investigates, develops `SuspectedTheft` evidence, then travels to RulersHall to Tell or Magistrate observes
3. **Accusation phase** (~20 ticks): Magistrate generates `Accuse` goal and files accusation
4. **Convergence verification**: After first accusation, continue ticking to prove no second accusation appears

Assertions:
- CrimeRegister has exactly 1 Accusation entry for the Thief
- Both Witness and Victim independently developed SuspectedTheft evidence (different sources: DirectObservation vs investigation)
- Decision trace shows Accuse candidate was generated at least once
- After first Accusation exists, no duplicate Accuse candidates appear for same (accused, violation_id)

### 2. Add test functions

- `golden_dual_discovery_converges_without_double_accusation` — runs the scenario once
- `golden_dual_discovery_converges_without_double_accusation_replays_deterministically` — runs twice with same seed, asserts hash equality

### 3. Add scenario metadata comment block

```
// Scenario 43: Dual Discovery Converges Without Double Accusation
// Systems: Transport, Perception, AI, Social, Institutions
// GoalKinds: StealItem, ShareBelief, InvestigateViolation, Accuse
// ActionDomains: Transport, Social, Generic
// Places: VillageSquare, GeneralStore, RulersHall, OrchardFarm
// Principles: 1, 7, 13, 16, 24
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add ~250-300 lines)

## Out of Scope

- **No engine changes** — no modifications to `candidate_generation.rs`, `justice_actions.rs`, or any source crate
- **No harness changes** — no new helpers in `golden_harness/mod.rs` (unless a small convenience helper is needed for violation profile seeding, in which case keep it minimal and local to `golden_emergent.rs`)
- **No golden docs update** — that is S32CRIMEMEGOLSUI-004
- **Scenario 41 and 42** — separate tickets
- **Punishment phase** — this scenario proves accusation convergence, not punishment. Punishment is covered by Scenarios 38 (Fine) and 41 (Exile).
- **Three-way discovery** — only two paths (witness + owner-local) are tested

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges` — both main + replay companion
2. `cargo test -p worldwake-ai` — full AI crate suite (all existing goldens unchanged)
3. `cargo test --workspace` — no regressions
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean

### Invariants

1. **No duplicate accusation**: CrimeRegister contains exactly 1 Accusation entry for (accused=Thief, matching violation_id)
2. **Both discovery paths activated**: Witness has `SocialObservationDetail::SuspectedTheft` with `suspect: Some(thief)` in belief store; Victim has `ViolationMemory` with `SuspectedTheft` kind
3. **Institutional deduplication**: after first Accusation entry exists in CrimeRegister, no second `GoalKind::Accuse` for same (accused, violation_id) appears in Magistrate's decision trace candidates
4. **Determinism**: replay companion produces identical `(world_hash, event_log_hash)` for same seed
5. **Conservation**: `verify_live_lot_conservation` passes
6. **All existing golden tests remain unchanged**

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_dual_discovery_converges_without_double_accusation` — proves dual-path convergence with single accusation
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_dual_discovery_converges_without_double_accusation_replays_deterministically` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
