# S128SLEEPIPLA-007: Golden coverage for sleep episodes

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: No — adds new golden E2E test file `crates/worldwake-ai/tests/golden_sleep_episode.rs` with 6 tests covering episode lifecycle, projected-need wake, place-quality recovery differentiation, partial recovery aftermath, site preference via candidate ranking, and decision-trace integration.
**Deps**: S128SLEEPIPLA-004, S128SLEEPIPLA-005, S128SLEEPIPLA-006

## Problem

After S128SLEEPIPLA-004 lands the episode-based handler, S128SLEEPIPLA-005 lands per-place candidate emission and ranking, and S128SLEEPIPLA-006 lands scenario-side authoring, the spec's six behavioral guarantees (D13) need golden E2E coverage to prove they hold end-to-end. Current AI tests cover sleep at the unit-emission level (`fatigue_and_bladder_emit_sleep_and_relieve`) and the feasibility level (`test_sleep_always_likely`) but no scenario-driven golden exercises a full sleep episode through the action lifecycle, event log, and decision trace.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing focused/unit coverage for sleep: `crates/worldwake-systems/src/needs_actions.rs::sleep_episode_reduces_fatigue_at_default_place` (reframed in S128SLEEPIPLA-004 from `sleep_reduces_fatigue_without_a_bed`); `crates/worldwake-ai/src/feasibility.rs::test_sleep_always_likely` (line 689); `crates/worldwake-ai/src/candidate_generation.rs::fatigue_and_bladder_emit_sleep_and_relieve` (reframed in S128SLEEPIPLA-005). No existing golden E2E for sleep episodes — this ticket adds the file.
2. Per the project canonical inventory (`docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`), golden tests live at `crates/worldwake-ai/tests/golden_*.rs`. The golden harness conventions (test fixture seeding, deterministic RNG, assertion patterns) are documented in `docs/golden-e2e-testing.md` and exemplified in adjacent files like `golden_perception_exposure.rs`, `golden_planner_pathology.rs`, `golden_simulation_gaps.rs`.
3. Shared boundary under audit: the spec's D13 acceptance contract. Each test maps to a specific spec invariant:
   - **Test 1 (episode lifecycle)** — D2 (`SleepEpisode` runtime), D4 (event tags), D7 (handler refactor)
   - **Test 2 (projected need breach wake)** — D1 (`WakeCondition::ProjectedNeedBreach`), D7 wake evaluation, S126 `FrameAssumption::NeedSafeUntilTick` consumption
   - **Test 3 (place-quality recovery differentiation)** — D3 (`SleepQualityProfile`), D7 modulated recovery
   - **Test 4 (interrupted-sleep partial recovery)** — D7 partial-recovery aftermath, D4 `SleepEpisodeEnded.accumulated_recovery`
   - **Test 5 (site preference via candidate ranking)** — D8 per-place emission, D8 ranking
   - **Test 6 (decision-trace integration)** — D11 surfacing
4. Authoring fidelity (rule 3.3B Scenario Content Validation): each test fixture references concrete `PlaceTag`, `WorkstationTag`, agent profiles, and commodity names. Reuse fixtures from `crates/worldwake-ai/tests/golden_harness/soak_world.rs` where possible; otherwise author minimal in-test scenarios. Verify `SleepQualityProfile` authored values match spec D3 examples (Hillside Shelter `1300`, Riverside Camp `1100`, Forest Clearing `1000`, Fertile Fields `900`).
5. S126 dependency: Test 2 requires `FrameAssumption::NeedSafeUntilTick` to be populated for the test agent. S126 is `✅ COMPLETED` per `archive/specs/S126-need-projection-time-budget.md:9`; the assumption-population path is exercised in `crates/worldwake-ai/tests/golden_need_projection.rs`. Reuse the population pattern from there.
6. Coverage gap classification (Rule 3): missing golden/E2E coverage. Focused unit coverage exists from -004 and -005 for action handler and candidate emission respectively. Golden coverage proves the integration: planner adopts a Sleep candidate at a specific place → action handler runs the episode through wake → event log and decision trace record the full causal chain. This is the strongest end-to-end proof surface for the spec's behavioral guarantees.
7. Test 5 prerequisite: `survival-baseline.ron` must have authored `SleepQualityProfile` for at least two named places, OR the test must construct a minimal scenario with two authored places. Per S128SLEEPIPLA-006's optional Section 4, the four named places may already carry authored profiles. If not, the test fixture seeds them inline.
8. Determinism (CLAUDE.md Critical Invariants): all tests use deterministic seeds (`ChaCha8Rng`-seeded), `BTreeMap`/`BTreeSet` only, no floats, no wall-clock. Existing golden harness conventions enforce this.

## Architecture Check

1. Per `references/worldwake-validation-patterns.md` "New Scenario Design", any new scenario fixture must verify `WorkstationTag`, `PlaceTag`, recipe names (none here, sleep doesn't use recipes), `AgentDef` fields, commodity names. The tests do not require new scenario `.ron` files — they can construct fixtures inline, mirroring the patterns in `golden_harness/soak_world.rs`.
2. Six tests partition the spec's D13 deliverable cleanly. Each test asserts one invariant; multi-layer assertions are split per layer (action trace for lifecycle ordering, event log for payload contents, decision trace for ranking/anchor selection, world state for final `HomeostaticNeeds.fatigue`). This honors `docs/precision-rules.md` Rule 5 (verification surface mapping) and Rule 6 (decision-trace preference).
3. Test names follow the existing convention: `<fixture>_<observed_behavior>` (e.g., `sleep_episode_at_default_place_runs_to_intended_max`, `projected_hunger_breach_wakes_sleep_early`).

## Verification Layers

1. Episode lifecycle (one start, one end, no intermediate re-commits) → action trace + event log delta.
2. Wake-condition firing (which condition won) → event-log `SleepEpisodeEnded.end_reason` payload.
3. Per-place candidate ranking (which place was adopted) → decision trace.
4. Authoritative recovery (final fatigue) → world state read after commit.
5. Each test maps each asserted invariant to one of the four surfaces above; no test collapses multiple surfaces into a single assertion.
6. The golden harness is full-action-registries (not local needs-only) because the test exercises planning, action lifecycle, and event log together.

## What to Change

### 1. New file `crates/worldwake-ai/tests/golden_sleep_episode.rs`

Create with the following six tests:

**Test 1 — `sleep_episode_at_default_place_runs_to_intended_max`** (covers D2 + D4 + D7 episode lifecycle):

- Seed: one agent with high fatigue (`pm(900)`) at a default-quality place (`SleepQualityProfile::default()`). Default `MetabolismProfile`.
- Run: tick the simulation until the agent commits a sleep episode and either runs to `intended_max_ticks` or recovers fully (whichever fires first under default `WakeCondition::IntendedDurationReached` priority).
- Assert (action trace): exactly one `start_sleep_episode` action lifecycle entry; exactly one `commit_sleep_episode`; no intermediate sleep re-commits.
- Assert (event log): exactly one `SleepEpisodeStarted` event with payload's `place == agent's start place`; exactly one `SleepEpisodeEnded` with `end_reason in {IntendedDuration, TargetRecovery}` (whichever path the formula resolves to first).
- Assert (world state): `HomeostaticNeeds.fatigue` after commit equals `pm(900) - accumulated_recovery`, where `accumulated_recovery == SleepEpisodeEnded.accumulated_recovery` from the event payload.

**Test 2 — `projected_hunger_breach_wakes_sleep_early`** (covers D1 `ProjectedNeedBreach` + D10 synthesis + S126 integration):

- Seed: one agent with high fatigue (`pm(900)`) and rising hunger such that `FrameAssumption::NeedSafeUntilTick { need: Hunger, until_tick: T_breach }` is populated with `T_breach < current_tick + intended_max_ticks`. Reuse the S126 population pattern from `golden_need_projection.rs`.
- Run: tick until sleep adopts and either wakes or completes.
- Assert (event log): `SleepEpisodeEnded.end_reason == WakeReason::ProjectedNeedBreach { need: Hunger, projected_breach_tick: T_breach }`.
- Assert (timing): wake tick `< start_tick + intended_max_ticks` (the breach fired before the duration cap).

**Test 3 — `place_quality_modulates_per_tick_recovery`** (covers D3 + D7 modulated recovery):

- Seed: two agents with identical starting fatigue (`pm(800)`) and identical `MetabolismProfile`. Agent A spawned at a place with `SleepQualityProfile { ..., recovery_modifier: Permille::new_unchecked(1100) }`; agent B at a place with `recovery_modifier: Permille::new_unchecked(900)`.
- Run: tick until both agents wake (whichever wake reason fires).
- Assert (world state): both agents wake; agent A's wake tick `< agent B's wake tick` (A recovers faster, hits target sooner). The exact tick difference is computed from the spec's recovery formula and asserted with concrete values per `docs/precision-rules.md` Rule 7 (cumulative arithmetic).

**Test 4 — `interrupted_sleep_records_partial_recovery`** (covers D7 partial-recovery aftermath + D4 `SleepEpisodeEnded.accumulated_recovery`):

- Seed: one agent with fatigue `pm(900)`. Either pre-load a wake condition that will fire before `intended_max_ticks` (e.g., a scheduled commitment), or use the projected-breach path from Test 2 with a tighter breach tick.
- Run: tick until wake.
- Assert (event log): `SleepEpisodeEnded.accumulated_recovery > Permille::new_unchecked(0)` and `< Permille::new_unchecked(1000)` (partial, not full).
- Assert (world state): `HomeostaticNeeds.fatigue` after commit equals `pm(900) - accumulated_recovery` exactly (no rounding loss; saturating subtraction).

**Test 5 — `site_preference_adopts_higher_quality_sleep_place`** (covers D8 per-place emission + ranking):

- Seed: one agent with high fatigue and belief of two reachable places. Place 1: `recovery_modifier: 1300` (Hillside-Shelter analog). Place 2: `recovery_modifier: 1100` (Riverside-Camp analog). Equal travel distance from agent's current location.
- Run: one tick of agent decision (or until adoption).
- Assert (decision trace): the adopted Sleep candidate's `OpportunityAnchor` references Place 1 (the higher-quality place).
- Assert (decision trace): two Sleep candidates were emitted (one per believed place); ranking ordered Place 1 above Place 2.

**Test 6 — `sleep_episode_events_render_in_decision_trace`** (covers D11 decision-trace surfacing):

- Seed: any minimal sleep-eligible setup (reuse Test 1's fixture).
- Run: tick until episode completes.
- Assert: event log contains `SleepEpisodeStarted` and `SleepEpisodeEnded`; both are renderable through the existing decision-trace path (assert via the existing observer/trace test helper that iterates events and produces formatted output — confirm during reassessment which helper is the canonical call).

### 2. Update golden inventory

Per `tickets/README.md`: regenerate `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/` via `python3 scripts/golden_inventory.py --write --check-docs`. This step is automatic from the script — no manual doc authoring.

## Files to Touch

- `crates/worldwake-ai/tests/golden_sleep_episode.rs` (new — 6 tests)
- `docs/generated/golden-e2e-inventory.md` (regenerated by script)
- `docs/generated/golden-scenario-index.md` (regenerated by script)
- `docs/generated/golden-scenario-details/` (regenerated by script — new entry per test)

## Out of Scope

- Modifying existing golden tests beyond regeneration of inventory docs
- Adding scenario `.ron` files for sleep — fixtures are inline per existing golden patterns
- Authoring `sleep_quality` in `survival-baseline.ron` — handled by S128SLEEPIPLA-006 (or deferred to follow-up scenario-tuning ticket per that ticket's Section 4)
- Performance/regression-guard tests for sleep — not a P12 concern; sleep is a behavioral refactor, not a perf optimization

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_sleep_episode` — all 6 tests pass.
2. `cargo test -p worldwake-ai` — full AI suite (regression check on existing goldens; sleep changes should not perturb unrelated tests).
3. `python3 scripts/golden_inventory.py --check-docs` — generated docs match the new test set.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. Six tests in `golden_sleep_episode.rs`, one per spec D13 listed test, with names following the existing `golden_*` convention.
2. Each test asserts at least one invariant per Verification Layer above; no test collapses action-trace lifecycle and event-log payload into a single assertion.
3. Tests use deterministic seeds (`ChaCha8Rng`); no floats, no wall-clock, `BTreeMap`/`BTreeSet` only in fixtures (CLAUDE.md Critical Invariants).
4. Generated golden docs (`docs/generated/golden-e2e-*`) include entries for all 6 new tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_sleep_episode.rs` (new) — 6 tests per Section 1 above.
2. `docs/generated/golden-e2e-inventory.md` and siblings — regenerated; not authored manually.

### Commands

1. `cargo test -p worldwake-ai --test golden_sleep_episode`
2. `cargo test -p worldwake-ai`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
