# S32CRIMEMEGOLSUI-002: Scenario 42 — Witness Deterrence Suppresses Theft Candidate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: E17 (crime/theft/justice), E14 (perception), S32CRIMEMEGOLSUI-001 (implementation order)

## Problem

The witness deterrence path in `emit_theft_candidates()` has zero golden E2E coverage. When `witness_risk_penalty * witness_count >= theft_motive_weight`, the function early-returns with zero StealItem candidates. This is the P10 physical dampener that prevents crime-escalation loops — the deterrent is the concrete presence of other agents, not a numeric cap. No existing golden test exercises the **absence** of crime as an emergent outcome.

## Assumption Reassessment (2026-03-27)

1. **`emit_theft_candidates()`** exists at `crates/worldwake-ai/src/candidate_generation.rs:1923`. The witness penalty calculation uses `witness_risk_penalty * witness_count` compared against `theft_motive_weight`. Confirmed: the function counts co-located living agents with `PerceptionProfile` as potential witnesses via locally-observed entities.
2. **`TheftDispositionProfile`** has fields `theft_motive_weight: Permille`, `witness_risk_penalty: Permille`, `steal_duration_ticks: NonZeroU32`. Defined in `crates/worldwake-core/src/crime.rs`.
3. **Scenarios 37-39** all have theft succeed (witness_risk_penalty=pm(0) or insufficient witnesses). No existing golden test exercises the suppression path.
4. **Competing goal requirement**: The spec requires the Thief to pursue an alternative goal (hunger-driven Eat or AcquireCommodity) to prove the agent is not idle. This requires moderate hunger on the Thief and a reachable food source.
5. **Live GoalKind surface**: When theft candidates are suppressed, the agent falls through to needs-driven goals (`ConsumeCommodity`, `AcquireCommodity`). These are well-established in E09/E13 golden coverage.
6. Golden E2E layer — full action registries and system dispatch required (perception system must run to maintain witness count observation, needs system for hunger progression).
12. **Isolation**: The 3 witness agents are set to `ControlSource::Human` to prevent their AI from moving away or generating competing goals that might change co-location. The Thief alone runs AI. The stealable item is placed on the ground (no container, owned by a separate human-controlled agent) to ensure it would be a valid theft target if witnesses were absent.

## Architecture Check

1. This test proves a negative (theft does NOT happen) through positive evidence (decision trace shows zero StealItem candidates when witnesses >= 3, and an alternative goal executes). This is the correct verification strategy — checking authoritative state alone would be ambiguous (item still at location could mean theft wasn't tried OR theft failed).
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. No StealItem in candidates.generated when 3+ witnesses co-located -> decision trace (`DecisionOutcome::Planning`, check `candidates.generated` at every tick with co-location)
2. Non-theft goal selected and planned -> decision trace (check selected goal is ConsumeCommodity or AcquireCommodity)
3. Item lot remains at VillageSquare with original owner -> authoritative relation query (`World::effective_place`, `World::owner_of`)
4. Thief's hunger decreases -> authoritative `HomeostaticNeeds` component read (hunger value decreased from initial)
5. Determinism: replay companion test with identical seed produces identical `(StateHash, StateHash)`

## What to Change

### 1. Add `run_witness_deterrence_suppresses_theft_candidate` function to `golden_emergent.rs`

Setup:
- Thief at VillageSquare: `TheftDispositionProfile { theft_motive_weight: pm(400), witness_risk_penalty: pm(150), steal_duration_ticks: nz(2) }`, `HomeostaticNeeds` with moderate hunger (e.g. `hunger: pm(600)`), `PerceptionProfile`, carry capacity sufficient for target
- 3 witness agents at VillageSquare: all with `PerceptionProfile`, `ControlSource::Human`, sated needs
- Stealable target: `Quantity(3) Grain` item lot at VillageSquare, owned by a human-controlled agent, not possessed, not in container
- Food source: place a reachable food item (e.g. Apple lot at VillageSquare or adjacent) so Thief can pursue hunger goal
- Topology: standard prototype world (VillageSquare hub)

Tick loop (e.g. 15-20 ticks):
- At each tick, assert via decision trace that no `StealItem` candidate appears for Thief
- After loop, assert Thief selected a non-theft goal and hunger decreased
- Assert stolen item lot unchanged (same place, same owner)

### 2. Add test functions

- `golden_witness_deterrence_suppresses_theft_candidate` — runs the scenario once
- `golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` — runs twice with same seed, asserts hash equality

### 3. Add scenario metadata comment block

```
// Scenario 42: Witness Deterrence Suppresses Theft Candidate
// Systems: AI, Perception, Transport, Needs
// GoalKinds: ConsumeCommodity, AcquireCommodity (NOT StealItem)
// ActionDomains: Needs, Production
// Places: VillageSquare
// Principles: 1, 10, 20, 24
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add ~150 lines)

## Out of Scope

- **No engine changes** — no modifications to `candidate_generation.rs` or any source crate
- **No harness changes** — no new helpers in `golden_harness/mod.rs`
- **No golden docs update** — that is S32CRIMEMEGOLSUI-004
- **Scenario 41 and 43** — separate tickets
- **Testing the threshold boundary** (2 witnesses flipping the outcome) — the spec notes this as a corollary but the golden test proves the 3-witness suppression case only
- **Witness agent AI behavior** — witnesses are human-controlled; their decision pipeline is not under test

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft` — both main + replay companion
2. `cargo test -p worldwake-ai` — full AI crate suite (all existing goldens unchanged)
3. `cargo test --workspace` — no regressions
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean

### Invariants

1. **No theft candidates**: at every tick where Thief is co-located with 3+ witnesses, `candidates.generated` contains zero `GoalKind::StealItem` entries
2. **Alternative goal executes**: Thief selects and plans a non-theft goal (needs-driven)
3. **Item untouched**: stealable item lot remains at original place with original owner, quantity unchanged
4. **Hunger satisfied**: Thief's `HomeostaticNeeds.hunger` value is lower at end than at start
5. **Determinism**: replay companion produces identical `(world_hash, event_log_hash)` for same seed
6. **Conservation**: `verify_live_lot_conservation` passes
7. **All existing golden tests remain unchanged**

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witness_deterrence_suppresses_theft_candidate` — proves witness presence suppresses theft candidates
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
