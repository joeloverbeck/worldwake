# S116DRIESCSUS-006: Golden coverage for drive escalation behavior

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (tests + scenario fixture)
**Deps**: archive/tickets/S116DRIESCSUS-003.md, archive/tickets/S116DRIESCSUS-004.md, S116DRIESCSUS-005

## Problem

Spec S116 D7 requires three new goldens proving escalation behavior end-to-end, plus a calibration pass over `survival-baseline.ron` and `survival-scattered.ron` to ensure default parameters do not destabilize existing survival behavior. This ticket does not tighten `golden_survival_contested::MAX_CRITICAL_RUN_TICKS` — that is ticket 007 and depends on empirical confirmation from this ticket that the full pipeline improves the contested scenario.

## Assumption Reassessment (2026-04-17)

1. Golden harness pattern: `crates/worldwake-ai/tests/golden_harness/` module reused by all existing goldens (e.g., `golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`, `golden_offices.rs`). New goldens follow the same structure.
2. Live `GoalKind` under test:
   - `GoalKind::Wash` (for dirtiness-escalation scenarios) — motive scoring path via `drive_score` in ticket 004.
   - `GoalKind::Sleep`, `GoalKind::Relieve`, consumption goals for multi-need scenarios.
3. Live affordance surface: `wash_preconditions` at `crates/worldwake-systems/src/needs_actions.rs:196` — unchanged by S116. Requires `TargetDirectlyPossessedByActor(0)` with `CommodityKind::Water`. Golden scenarios that test wash-cycle escalation must place water co-located with the wash facility to isolate the motive-priority branch under test from the affordance-level water-possession branch.
4. Scenario isolation (precision rule 8): the `dirtiness_wash_cycle_under_priority_override` golden is intended to prove the motive-score escalation branch. Lawful competing affordances the architecture still allows: hunger-driven `AcquireCommodity(food)` at a co-located or adjacent food hub. The scenario deliberately places wash's water co-located with the wash facility so an agent with the escalated Wash priority can pick up water locally and wash without competing against water-for-drink pressure. Unrelated lawful branches intentionally excluded from setup: long-distance water travel, multiple food hubs with differential pressure, combat, trading.
5. Calibration acceptance band (spec Risks #3): default parameters must leave per-agent wash/eat/drink/sleep counts on `survival-baseline.ron` and `survival-scattered.ron` within ±10% of the current golden fixtures, and must not introduce any new `MAX_CRITICAL_RUN_TICKS=400` violations on contested.
6. Intended verification layer (precision rule 3): Golden E2E coverage. Runtime `agent_tick` with full action registries required — not a local needs-only harness. Agents must travel, possess water, wash, eat, drink, sleep.
7. Shared boundary under audit: the full escalation pipeline — `needs_system` counter maintenance (ticket 003), ranking multiplier application (ticket 004), scenario-defined profile (ticket 005). Goldens validate that all three cooperate to break the wash-cycle starvation dynamic.

## Architecture Check

1. Goldens are the canonical E2E proof surface for emergent behavior changes (FND-31). Unit/focused tests in tickets 001-004 prove arithmetic; goldens prove emergence.
2. Calibration check is mechanical: existing golden fixtures are the empirical baseline; the new pipeline must keep per-agent distributions within ±10%. Drift outside the band flags tuning of `DriveEscalationProfile::default()` constants before this ticket can close.
3. Scenario isolation per precision rule 8 — every golden documents the lawful competing affordances it excludes and why.

## Verification Layers

1. Wash-cycle under priority override → golden `dirtiness_wash_cycle_under_priority_override` asserts each agent performs ≥ 4 wash cycles over 800 ticks and each agent's max consecutive dirtiness-critical run is < 250 ticks. Authoritative proof surface: action-trace `Wash` commit count + event-log `DeprivationExposure.dirtiness_critical_ticks` reset pattern.
2. Belief-only planning preserved under escalation → golden `escalation_respects_belief_only_planning` asserts the agent never plans `GoalKind::Wash` (decision-trace proof) when no belief supports a wash-capable facility, yet the escalation multiplier still grows to cap (unit-level exposure read proof — forced via test harness). FND-14 guard.
3. Escalation fades through physical relief → golden `escalation_fades_after_relief` asserts `DeprivationExposure::ticks_at_critical(HomeostaticNeedId::Dirtiness) == 0` within 1 tick of `dirtiness < critical`, and an `EventTag::Escalation` end event with canonical `action_name` is present on the transition tick.
4. Baseline survival behavior preserved → existing `golden_survival_baseline` passes with per-agent wash/eat/drink/sleep counts within ±10% of current fixtures. If drift is outside the band, retune defaults before this ticket closes.
5. Scattered survival behavior preserved → existing `golden_survival_scattered` passes under the same ±10% band.
6. Contested survival regression → existing `golden_survival_contested` still passes with its current `MAX_CRITICAL_RUN_TICKS=400` bound. Tightening to 300 is ticket 007.

## What to Change

### 1. New scenario RON

Create `scenarios/drive-escalation-wash-priority.ron` with:

- 2 agents sharing a small place graph: agent start location, wash facility (Spring Basin) with co-located water source (Spring Well or equivalent), food hub (East Orchard or Harvest Grain location) 2 hops away.
- Per-agent `utility_profile`: `dirtiness_weight: 625`, `hunger_weight: 750` (replicates the contested-scenario priority-override condition).
- Per-agent `metabolism_profile.wilderness_relief_dirtiness_penalty: 200` (replicates the wilderness-relief pressure driver).
- Default `drive_escalation_profile` (omitted — universal default applies).
- Duration: 800 ticks.

Document in a leading scenario comment: the lawful competing affordances excluded (long-distance water travel, combat, trading, multiple food hubs) and why.

### 2. New golden test file

Create `crates/worldwake-ai/tests/golden_drive_escalation.rs` with the three goldens:

- `dirtiness_wash_cycle_under_priority_override` — drives the new RON scenario, asserts ≥ 4 wash commits per agent and `max_consecutive_critical_dirtiness_ticks < 250`.
- `escalation_respects_belief_only_planning` — constructs an agent with empty wash-facility beliefs, forces dirtiness above critical via wilderness relief for 400 ticks, asserts zero `GoalKind::Wash` plans and that the exposure counter grows past `start_after_ticks`.
- `escalation_fades_after_relief` — drives a scenario with co-located wash water, forces initial dirtiness above critical, lets agent wash, asserts counter reset within 1 tick and `EventTag::Escalation` end event present on the reset tick.

Follow the harness/assertion pattern already used in `golden_survival_contested.rs`.

### 3. Calibration verification

Re-run `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` locally. Record per-agent action-count distributions (wash, eat, drink, sleep). Compare to the current fixtures' numbers (extracted from `reports/scenario-analysis-report.md` and prior golden archives where available).

If per-agent counts drift outside ±10%, tune `DriveEscalationProfile::default()` constants (`start_after_ticks`, `growth_per_tick`, `max_multiplier`) in `crates/worldwake-core/src/drive_escalation_profile.rs` (landed by ticket 002) and re-run. Here `max_multiplier` is a multiplier-scale cap in permille units, not a `Permille` pressure value. Document the chosen defaults and the empirical justification as a comment on the `Default` impl.

## Files to Touch

- `scenarios/drive-escalation-wash-priority.ron` (new)
- `crates/worldwake-ai/tests/golden_drive_escalation.rs` (new)
- `crates/worldwake-core/src/drive_escalation_profile.rs` (modify only if calibration requires re-tuning defaults — otherwise untouched)

## Out of Scope

- Tightening `golden_survival_contested::MAX_CRITICAL_RUN_TICKS` to 300 — ticket 007.
- Water-possession bottleneck follow-up (`wash_preconditions`) — separate spec.
- Multi-need escalation emergence scenarios beyond dirtiness — the three named goldens exercise the full pipeline; additional coverage is a later ticket if emergent gaps surface.

## Acceptance Criteria

### Tests That Must Pass

1. `dirtiness_wash_cycle_under_priority_override` — ≥ 4 wash commits per agent; `max_consecutive_critical_dirtiness_ticks < 250` per agent.
2. `escalation_respects_belief_only_planning` — zero `GoalKind::Wash` plans; exposure counter ≥ `start_after_ticks + 1`.
3. `escalation_fades_after_relief` — counter reset within 1 tick of sub-critical; `EventTag::Escalation` end event present at the transition tick.
4. `golden_survival_baseline` — existing assertions pass; per-agent action counts within ±10% of current fixtures.
5. `golden_survival_scattered` — same.
6. `golden_survival_contested` — existing assertions pass under current `MAX_CRITICAL_RUN_TICKS=400`.
7. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. No authoritative world-state mutation introduced by this ticket (tests + scenario fixture only, plus optional default-constant tuning).
2. Scenario-isolation documentation (per precision rule 8) present as comments in the new scenario RON and each new golden's intro.
3. Calibration delta recorded either inline in the golden's rationale comments or in the default impl doc-comment.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_drive_escalation.rs` — 3 new goldens.
2. `scenarios/drive-escalation-wash-priority.ron` — new fixture scenario for golden 1.

### Commands

1. `cargo test -p worldwake-ai --test golden_drive_escalation`
2. `cargo test -p worldwake-ai --test golden_survival_baseline`
3. `cargo test -p worldwake-ai --test golden_survival_scattered`
4. `cargo test -p worldwake-ai --test golden_survival_contested`
5. `cargo clippy --workspace --all-targets -- -D warnings`
