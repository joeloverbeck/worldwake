# S40REMPUR-007: Golden tests — bandit pursuit scenarios

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Deps**: S40REMPUR-001 through S40REMPUR-006 (all infrastructure must be in place)

## Problem

The spec defines three golden test scenarios that prove remote pursuit works end-to-end as emergent behavior: (1) bandit witnesses traveler leave, pursues, attacks on arrival; (2) bandit pursues stale information, target gone, falls back to replanning; (3) combat-flee-re-pursue cycle bounded by travel budget and confidence. Without these golden tests, there is no E2E proof that the pursuit infrastructure produces the intended emergent chains.

## Assumption Reassessment (2026-03-30)

1. Golden tests live in `crates/worldwake-ai/tests/` (e.g., `golden_social.rs`, `golden_combat.rs`, etc.).
2. Golden test harness uses `TestHarness` with `step_once()`, `enable_tracing()`, `enable_action_tracing()`.
3. Per CLAUDE.md, golden production tests require `PerceptionProfile` on agents that need to observe post-production output.
4. The test topology needs at least 2 places with a travel edge so pursuit involves actual Travel.
5. Agents need: `CombatProfile`, `PursuitProfile`, `PerceptionProfile`, `BlockedIntentMemory`, `UtilityProfile`, `HomeostaticNeeds`, `AgentBeliefStore`.
6. The bandit must have a lawful raid reason (e.g., `BanditFactionPolicy`).
7. Target must be setup as a lawful raid target.
8. Belief about target's remote place must come from lawful perception (direct observation of departure, not injected).
9. Deterministic replay companions are required for all golden tests per spec.
10. The GoalKind under test is `RaidTarget { target }` and `EngageHostile { target }`.
11. The operator surface is `PlannerOpKind::Attack` (terminal) with `PlannerOpKind::Travel` (prerequisite).
12. No adjacent contradictions exposed.

## Architecture Check

1. Golden tests prove emergent behavior through the full pipeline (perception → belief → candidate → search → execution → outcome). This is the correct proof surface for end-to-end pursuit.
2. Tests should be minimal: smallest possible world that exercises the scenario. Over-complex golden tests obscure what they prove.
3. No backwards-compatibility shims.

## Verification Layers

1. Pursuit-then-attack → authoritative world state: verify target receives wounds after pursuer travels + attacks
2. Stale pursuit failure → decision trace: verify candidate emitted, plan executed, arrival failure recorded, replan triggered
3. Combat-flee-re-pursue → action trace + decision trace: verify flee → new belief → new pursuit → bounded by budget/confidence
4. Deterministic replay → replay verification: same seed produces same outcome
5. Cross-layer: perception (worldwake-systems) → belief (core) → candidate (ai) → search (ai) → action execution (sim) → outcome (core). Full-stack verification.

## What to Change

### 1. Golden test: Bandit witnesses traveler leave, pursues, attacks

File: `crates/worldwake-ai/tests/golden_pursuit.rs` (new)

Setup:
- Two places (A, B) connected by a travel edge
- Bandit at place A with `PursuitProfile`, `CombatProfile`, `PerceptionProfile`, `BanditFactionPolicy`
- Traveler at place A (co-located initially)
- Bandit observes traveler (perception tick)
- Traveler moves to place B (Travel action)
- Bandit observes departure (gains belief: traveler at B)

Verification:
- Bandit emits `RaidTarget { target: traveler }` candidate
- Bandit plans `Travel(A→B) + Attack(traveler)`
- After travel completes, bandit arrives at B
- If traveler still at B: attack occurs, traveler receives wounds
- Deterministic replay matches

### 2. Golden test: Bandit pursues stale target, arrival failure

Setup:
- Three places (A, B, C). A↔B, B↔C
- Bandit at A, traveler at A
- Traveler departs to B, bandit observes
- Traveler continues to C before bandit arrives at B

Verification:
- Bandit arrives at B, target absent
- `BlockingFact::TargetGone` recorded
- Bandit replans normally (no omniscient continuation to C)
- Decision trace shows arrival failure
- Deterministic replay matches

### 3. Golden test: Combat-flee-re-pursue

Setup:
- Two places (A, B) connected by short travel edge
- Bandit and target both at A
- Combat begins, target flees to B
- Bandit observes departure direction

Verification:
- Bandit gains fresh belief about target at B
- Bandit initiates new pursuit to B (bounded by travel budget and confidence)
- If target at B: second engagement occurs
- Deterministic replay matches

## Files to Touch

- `crates/worldwake-ai/tests/golden_pursuit.rs` (new) — all three golden test scenarios

## Out of Scope

- Guard/justice pursuit golden tests (future ticket, different goal kinds)
- Performance benchmarks
- Multi-agent pursuit scenarios (single pursuer per test is sufficient)
- Pursuit across more than 3 places (minimal topology suffices)
- Changes to any infrastructure code (all infrastructure is from S40REMPUR-001..006)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bandit_witnesses_and_pursues`: bandit observes traveler departure, pursues, attacks at destination.
2. `golden_stale_pursuit_arrival_failure`: bandit pursues stale info, target gone, honest failure and replanning.
3. `golden_combat_flee_re_pursue`: combat → flee → observation → fresh pursuit bounded by budget/confidence.
4. All three have deterministic replay companions that verify same-seed reproducibility.
5. All existing golden tests still pass: `cargo test -p worldwake-ai`

### Invariants

1. Attack occurs only at real co-location — no synthesized combat.
2. Pursuit uses belief-backed information only — no omniscient world queries.
3. Arrival at stale location produces failure, not omniscient continuation.
4. All pursuits are bounded by `PursuitProfile` parameters.
5. Existing ranking and interrupt hierarchies unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_pursuit.rs` — `golden_bandit_witnesses_and_pursues`
2. `crates/worldwake-ai/tests/golden_pursuit.rs` — `golden_stale_pursuit_arrival_failure`
3. `crates/worldwake-ai/tests/golden_pursuit.rs` — `golden_combat_flee_re_pursue`

### Commands

1. `cargo test -p worldwake-ai golden_pursuit`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace && cargo test --workspace`
