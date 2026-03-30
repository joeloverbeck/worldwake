# E20COMBEH-006: Golden tests — travel physiology (escalation, interrupt, diversity)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E20COMBEH-001, E20COMBEH-002, E20COMBEH-003 (travel body cost wiring must be complete)

## Problem

The travel physiology feature (per-agent body cost multipliers on travel) needs golden E2E coverage to verify that: (a) travel increases needs faster than basal rate, (b) critical bladder during travel causes interrupt and replan, and (c) agents with different multipliers diverge in escalation rate. These are spec-required golden tests T-TravelEscalation, T-TravelInterrupt, and T-AgentDiversity.

## Assumption Reassessment (2026-03-30)

1. **Golden test harness**: Golden tests in `worldwake-ai` use the full simulation harness with `SimulationState`, `AgentTickDriver`, action registration, and `step_once()`. They exercise the full pipeline: needs system → candidate generation → ranking → planning → action execution.
2. **GoalKind::Relieve**: Live goal kind. Candidate generation emits it when bladder crosses high threshold. Ranking assigns priority based on threshold band. Confirmed in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs`.
3. **Interrupt evaluation**: `evaluate_interrupt` in `crates/worldwake-ai/src/interrupts.rs` checks whether a running action should be interrupted for replan. Critical needs trigger `InterruptForReplan`.
4. **Travel action**: Multi-tick action (`DurationExpr::TravelToTarget`). Interruptibility: `InterruptibleWithPenalty` (confirmed in travel_actions.rs:52 — actually need to verify; spec says travel is `InterruptibleWithPenalty`).
5. **Decision trace**: Available via `h.driver.enable_tracing()` for debugging test failures. Action trace via `h.enable_action_tracing()`.
6. **Scenario isolation**: Travel physiology tests must set non-zero MetabolismProfile multipliers on test agents. Other agents in the prototype world use default (zero) multipliers and are unaffected.
7. **PerceptionProfile**: Required on agents that need to observe events. For T-TravelEscalation and T-TravelInterrupt, perception is not strictly needed — these test needs escalation and AI decision, not observation. T-AgentDiversity also doesn't need perception.

## Architecture Check

1. Three focused golden tests, each verifying one specific behavior. No overlap. Each test sets up a minimal world with the minimum configuration needed to exercise the feature.
2. No backward-compatibility shims. These are new tests.

## Verification Layers

1. Travel need escalation rate → authoritative world state (HomeostaticNeeds.bladder/fatigue/thirst values after N ticks of travel)
2. Travel interrupt → decision trace (InterruptForReplan trigger) + action trace (travel Aborted)
3. Agent diversity → authoritative world state (compare needs values between two agents after same travel)
4. These are golden E2E tests — they verify the full pipeline from needs system through AI decision.

## What to Change

### 1. T-TravelEscalation golden test

**Setup**: One agent with non-zero `travel_bladder_multiplier` (e.g., `Permille(500)`) at a place with a long travel edge to another place. Start travel action.

**Assert**: After N ticks of travel, `HomeostaticNeeds.bladder` is higher than `basal_bladder_rate * N` alone. The additional amount equals `basal_rate * multiplier / 1000 * N` (within Permille integer arithmetic). Similarly for fatigue and thirst if those multipliers are non-zero.

### 2. T-TravelInterrupt golden test

**Setup**: One agent with high `travel_bladder_multiplier` and moderate `bladder_rate`, starting near a place with `PlaceTag::Latrine` (or outdoor tag). Set initial bladder to a value close to critical threshold so that a few ticks of travel push it over.

**Assert**: Travel is interrupted (action trace shows travel Aborted). Agent replans for `GoalKind::Relieve`. Agent starts a relief action (toilet or relieve_wilderness depending on available places).

### 3. T-AgentDiversity golden test

**Setup**: Two agents with different `travel_bladder_multiplier` values (e.g., `Permille(200)` and `Permille(800)`) starting travel on the same route.

**Assert**: After the same number of ticks, the agent with higher multiplier has higher bladder value. The difference is proportional to the multiplier difference.

## Files to Touch

- `crates/worldwake-ai/src/golden_tests/` (new test file or addition to existing golden test module)

## Out of Scope

- Relief fallback golden tests (E20COMBEH-007)
- Witness/social golden tests (E20COMBEH-008)
- Unit tests for individual components (covered in E20COMBEH-001 through E20COMBEH-005)
- Changes to production code (this ticket is tests only)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_travel_escalation` — travel increases needs by basal + travel multiplier amount
2. `golden_travel_interrupt` — critical bladder during travel triggers interrupt and replan to Relieve
3. `golden_agent_diversity` — different multipliers produce different escalation rates
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Need values increase monotonically during travel (no silent resets)
2. Travel interrupt occurs at critical threshold, not before
3. Agent with higher multiplier always has higher need value after identical travel duration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/golden_tests/` — `golden_travel_escalation` — verifies escalation arithmetic
2. `crates/worldwake-ai/src/golden_tests/` — `golden_travel_interrupt` — verifies interrupt pipeline
3. `crates/worldwake-ai/src/golden_tests/` — `golden_agent_diversity` — verifies per-agent variation

### Commands

1. `cargo test -p worldwake-ai golden_travel`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
