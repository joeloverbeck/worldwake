# S116DRIESCSUS-006: Golden coverage for drive escalation behavior

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (tests + scenario fixture)
**Deps**: archive/tickets/S116DRIESCSUS-003.md, archive/tickets/S116DRIESCSUS-004.md, archive/tickets/S116DRIESCSUS-005.md, archive/tickets/S116DRIESCSUS-008.md, archive/tickets/S116DRIESCSUS-009.md, archive/tickets/S116DRIESCSUS-010.md

## Problem

Spec S116 D7 requires three new goldens proving escalation behavior end-to-end, plus a calibration pass over `survival-baseline.ron` and `survival-scattered.ron` to ensure default parameters do not destabilize existing survival behavior. This ticket does not tighten `golden_survival_contested::MAX_CRITICAL_RUN_TICKS` — that is ticket 007 and depends on empirical confirmation from this ticket that the full pipeline improves the contested scenario.

## Assumption Reassessment (2026-04-17)

1. Golden harness pattern: `crates/worldwake-ai/tests/golden_harness/` module reused by all existing goldens (e.g., `golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`, `golden_offices.rs`). New goldens follow the same structure.
2. Live `GoalKind` under test:
   - `GoalKind::Wash` (for dirtiness-escalation scenarios) — motive scoring path via `drive_score` in ticket 004.
   - `GoalKind::Sleep`, `GoalKind::Relieve`, consumption goals for multi-need scenarios.
3. Live affordance surface after active ticket `S116DRIESCSUS-010`: `wash_preconditions` at `crates/worldwake-systems/src/needs_actions.rs:196` no longer accepts a directly possessed water lot. Wash now requires two co-located facility targets at the actor's place: a `WorkstationTag::WashBasin` facility and a `ResourceSource { commodity: Water, available_quantity >= 1 }` facility. Golden scenarios that test wash-cycle escalation must therefore author concrete local basin-plus-source access, not portable inventory water.
4. Scenario isolation (precision rule 8): the `dirtiness_wash_cycle_under_priority_override` golden is intended to prove the motive-score escalation branch. Lawful competing affordances the architecture still allows: hunger-driven `AcquireCommodity(food)` at a co-located or adjacent food hub. The scenario must deliberately keep wash access concrete and local via basin-plus-source infrastructure so an agent with the escalated Wash priority can return to that place and wash without relying on a second portable-water contract. Unrelated lawful branches intentionally excluded from setup: long-distance water travel, multiple food hubs with differential pressure, combat, trading.
5. Calibration acceptance band (spec Risks #3): default parameters must leave per-agent wash/eat/drink/sleep counts on `survival-baseline.ron` and `survival-scattered.ron` within ±10% of the current golden fixtures, and must not introduce any new `MAX_CRITICAL_RUN_TICKS=400` violations on contested.
6. Intended verification layer (precision rule 3): Golden E2E coverage. Runtime `agent_tick` with full action registries required — not a local needs-only harness. Agents must travel, possess water, wash, eat, drink, sleep.
7. Shared boundary under audit: the full escalation pipeline — `needs_system` counter maintenance (ticket 003), ranking multiplier application (ticket 004), scenario-defined profile (ticket 005). Goldens validate that all three cooperate to break the wash-cycle starvation dynamic.
8. Spec drift: the parent spec's D8 (`golden_survival_contested` bound tightening) is now split into active ticket `S116DRIESCSUS-007`, and D9 unit motive-math coverage already landed in archived ticket `S116DRIESCSUS-002.md`. This ticket remains a golden/scenario ticket; it does not re-own ranking unit tests or contested-bound tightening.
9. Live `GoalKind::Wash` admission is no longer inventory-driven after active ticket `S116DRIESCSUS-010`. Candidate generation now requires local wash access, and planner-relevant Wash destinations come from believed places that have both a wash basin and a water source. For the belief-only golden, the honest invariant remains "no found/committed Wash plan without believed wash access," not "Wash never appears anywhere in trace data."
10. Calibration proof surface is existing authored survival goldens plus command-backed observation, not a second duplicate assertion layer. The honest current-ticket obligation is to rerun `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested`, confirm no retune is required, and record that result in closeout unless focused reruns expose a real default-calibration regression.
11. Separate planner-boundary concern: `PlanningSnapshot::collect_entities()` in `crates/worldwake-ai/src/planning_snapshot.rs` currently admits authoritative remote entities at included places, which may expose planner-visible facilities without belief carriage. That concern is now tracked explicitly in active ticket `S116DRIESCSUS-008`; this ticket does not own the planner snapshot repair itself.
12. Live harness mismatch: the existing `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` tests are all `#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]` on this branch. Truthful calibration verification therefore requires running those binaries with `-- --ignored`; plain `cargo test --test ...` would only compile the binary and skip the owned assertions.

## Architecture Check

1. Goldens are the canonical E2E proof surface for emergent behavior changes (FND-31). Unit/focused tests in tickets 001-004 prove arithmetic; goldens prove emergence.
2. Calibration check is mechanical: existing golden fixtures are the empirical baseline; the new pipeline must keep per-agent distributions within ±10%. Drift outside the band flags tuning of `DriveEscalationProfile::default()` constants before this ticket can close.
3. Scenario isolation per precision rule 8 — every golden documents the lawful competing affordances it excludes and why.

## Verification Layers

1. Wash-cycle under priority override → golden `dirtiness_wash_cycle_under_priority_override` asserts each agent performs ≥ 4 wash cycles over 800 ticks and each agent's max consecutive dirtiness-critical run is < 250 ticks. Authoritative proof surface: action-trace `Wash` commit count + event-log `DeprivationExposure.dirtiness_critical_ticks` reset pattern.
2. Belief-only planning preserved under escalation → golden `escalation_respects_belief_only_planning` asserts no found `GoalKind::Wash` plan or committed `wash` action appears when no believed wash-basin place exists, while authoritative `DeprivationExposure.dirtiness_critical_ticks` still grows past `start_after_ticks`. FND-14 guard.
3. Escalation fades through physical relief → golden `escalation_fades_after_relief` asserts `DeprivationExposure::ticks_at_critical(HomeostaticNeedId::Dirtiness) == 0` within 1 tick of `dirtiness < critical`, and an `EventTag::Escalation` end event with canonical `action_name` is present on the transition tick.
4. Baseline/scattered/contested calibration verification → existing `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` rerun cleanly after the new golden/scenario lands. If those reruns expose a real default-calibration regression, retune defaults in the owning core component and record the deviation; otherwise leave production defaults untouched and capture the no-retune result in closeout.

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
- `escalation_respects_belief_only_planning` — constructs an agent with no believed wash-basin place, forces dirtiness above critical via wilderness relief for 400 ticks, asserts no successful `GoalKind::Wash` plan or committed `wash` action appears and that the exposure counter grows past `start_after_ticks`.
- `escalation_fades_after_relief` — drives a scenario with co-located wash water, forces initial dirtiness above critical, lets agent wash, asserts counter reset within 1 tick and `EventTag::Escalation` end event present on the reset tick.

Follow the harness/assertion pattern already used in `golden_survival_contested.rs`.

### 3. Calibration verification

Re-run `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` locally. Record per-agent action-count distributions (wash, eat, drink, sleep). Compare to the current fixtures' numbers (extracted from `reports/scenario-analysis-report.md` and prior golden archives where available).

If those reruns expose a real default-calibration regression, tune `DriveEscalationProfile::default()` constants (`start_after_ticks`, `growth_per_tick`, `max_multiplier`) in `crates/worldwake-core/src/drive_escalation_profile.rs` (landed by ticket 002) and re-run. Here `max_multiplier` is a multiplier-scale cap in permille units, not a `Permille` pressure value. If no regression appears, leave production defaults untouched and record the no-retune result in closeout.

## Files to Touch

- `scenarios/drive-escalation-wash-priority.ron` (new)
- `crates/worldwake-ai/tests/golden_drive_escalation.rs` (new)
- `crates/worldwake-core/src/drive_escalation_profile.rs` (modify only if calibration reruns expose a real default regression — otherwise untouched)

## Out of Scope

- Tightening `golden_survival_contested::MAX_CRITICAL_RUN_TICKS` to 300 — ticket 007.
- Ranking/unit motive-math coverage from spec D9 — already delivered in archived ticket `S116DRIESCSUS-002`.
- Further authoritative wash-water contract changes beyond active `tickets/S116DRIESCSUS-010.md`.
- Multi-need escalation emergence scenarios beyond dirtiness — the three named goldens exercise the full pipeline; additional coverage is a later ticket if emergent gaps surface.

## Acceptance Criteria

### Tests That Must Pass

1. `dirtiness_wash_cycle_under_priority_override` — ≥ 4 wash commits per agent; `max_consecutive_critical_dirtiness_ticks < 250` per agent.
2. `escalation_respects_belief_only_planning` — no found `GoalKind::Wash` plan or committed `wash` action without a believed wash-basin place; exposure counter ≥ `start_after_ticks + 1`.
3. `escalation_fades_after_relief` — counter reset within 1 tick of sub-critical; `EventTag::Escalation` end event present at the transition tick.
4. `golden_survival_baseline` — existing assertions pass after the new golden/scenario lands.
5. `golden_survival_scattered` — existing assertions pass after the new golden/scenario lands.
6. `golden_survival_contested` — existing assertions pass under current `MAX_CRITICAL_RUN_TICKS=400`.
7. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. No authoritative world-state mutation introduced by this ticket (tests + scenario fixture only, plus optional default-constant tuning).
2. Scenario-isolation documentation (per precision rule 8) present as comments in the new scenario RON and each new golden's intro.
3. Calibration delta recorded either inline in the golden's rationale comments or in the default impl doc-comment.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_drive_escalation.rs` — 3 new goldens.
2. `scenarios/drive-escalation-wash-priority.ron` — new fixture scenario for golden 1 and the authored co-located wash-water relief path reused by golden 3.

### Commands

1. `cargo test -p worldwake-ai --test golden_drive_escalation`
2. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
3. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
4. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
5. `cargo clippy --workspace --all-targets -- -D warnings`
