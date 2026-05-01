# S129PLADIRFAC-012: Golden coverage — hygiene end-to-end

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — new golden test file exercising the full S129 stack
**Deps**: archive/tickets/S129PLADIRFAC-005.md, archive/tickets/S129PLADIRFAC-006.md, archive/tickets/S129PLADIRFAC-007.md, archive/tickets/S129PLADIRFAC-008.md, archive/tickets/S129PLADIRFAC-010.md, archive/tickets/S129PLADIRFAC-011.md

## Problem

Without golden coverage, S129's emergent chains ("dirty place → bad sleep → travel decision", "full latrine → wilderness fallback", "empty basin → partial wash → stay dirty longer") have no end-to-end proof — the per-feature focused tests in tickets 005–011 each validate their slice, but they do not prove the slices compose. This ticket lands the six target-pattern goldens spec D12 declares plus the adversarial-sweep scenarios so the architecture can be falsified per FND-31. It also covers Authoritative-to-AI Impact Rule checklist point 7 (golden tests) for the wash refactor.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The full S129 production stack landed in tickets 001–011. Every component, event tag, precondition arm, handler extension, maintenance step, candidate emission split, ranking integration, and scenario `*Def` wrapper is in place. Coverage gap classification (precision-rules §3): this ticket added the missing `golden_place_dirtiness.rs` coverage; before implementation the only related golden was `forensic_wash_vs_water_competition.rs`.
2. The shared abstraction boundary under audit is the full S129 emergent contract — wilderness relief writes `PlaceDirtiness`, place dirtiness biases sleep ranking, wash/latrine actions mutate facility/place hygiene state, and maintenance transfers water into basins. The landed goldens prove the strongest live cross-layer seams for these target patterns rather than forcing every case through a long autonomous scenario.
3. Each golden scenario must declare its target pattern (FND-31 + spec D12 § Target Patterns): the intended invariant under test, the lawful competing affordances the architecture would otherwise allow (per precision-rules §8 scenario isolation), and what the test must never produce.
4. The adversarial sweeps (zero `decay_per_tick` saturation, zero `refill_per_tick` plateau, zero `critical_threshold` immediate overflow) are listed in spec D12 as "must support" but not necessarily "must exercise this ticket". This ticket exercises the saturation and plateau cases; the immediate-overflow case is exercised in ticket 006's focused tests already.
5. Ticket 007's basin-state refactor left `forensic_wash_vs_water_competition.rs` passing without an in-scope rewrite. Reconfirm during this ticket's reassessment phase, but do not assume a rewrite is required solely because the wash action target shape changed.
6. Scenario isolation choices (precision-rules §8): each golden uses minimal scenarios that isolate one specific S129 behavior. Competing affordances (e.g., the agent could also harvest food, drink water) are intentionally limited so the place-dirtiness or basin-state behavior dominates ranking. Document the isolation explicitly per the rule.
7. Per repo golden guidance, the AI-driven goldens seed local/world beliefs explicitly at the same surfaces the live planner consumes. Scripted human requests are used where the target invariant is action/maintenance aftermath rather than autonomous goal choice.

## Architecture Check

1. A single golden file `golden_place_dirtiness.rs` cohabits with sibling hygiene-focused goldens (`forensic_wash_vs_water_competition.rs` and any future hygiene work). Splitting per-target-pattern into separate files would scatter the S129 contract; the single-file approach mirrors how S128's `golden_sleep_episode.rs` (or equivalent) consolidates one spec's E2E coverage.
2. The landed goldens use harness-authored in-process fixtures rather than separate RON scenario files. This keeps the proof at the smallest truthful end-to-end seam for each S129 target pattern while still exercising the full action registry, scheduler, event log, AI decision trace, maintenance pass, and authoritative state mutations.
3. No backward-compat shim. The new goldens are net-new; `forensic_wash_vs_water_competition.rs` was rerun unchanged as adjacent hygiene coverage.

## Verification Layers

Each target pattern below maps to a specific verification surface (precision-rules §5):

1. **Place dirtiness accumulation** (target pattern 1) → authoritative world state assertion (`PlaceDirtiness.value` after N ticks) plus event-log delta (`WasteCreated` count).
2. **Sleep ranking under dirtiness** (target pattern 2) → decision-trace assertion that the cleaner place is generated and selected as the sleep opportunity.
3. **Wash partial success** (target pattern 3) → `WashFacilityUsed` event-payload assertion (`partial: true`) plus authoritative `WashBasinState` and agent `HomeostaticNeeds.dirtiness` post-commit.
4. **Latrine overcapacity** (target pattern 4) → event-log delta (`WasteCreated` with `OvercapacityLatrine` source on the threshold-crossing tick) plus authoritative `PlaceDirtiness.value` increment.
5. **Basin natural refill from co-located source** (target pattern 5) → authoritative state delta over multiple ticks (`clean_water_units` increment, `available_quantity` decrement).
6. **Auth-to-AI replan on basin emptiness** (target pattern 6) → action-trace assertion that a stale empty-basin `BestEffort` request records `StartFailed`, plus decision-trace assertion that the AI selects the usable second basin in the same planning epoch.

## What to Change

### 1. New `crates/worldwake-ai/tests/golden_place_dirtiness.rs`

Six target-pattern goldens, each as a `#[test]` function with a harness-authored fixture. For each:

- Construct the minimal fixture via the existing golden test harness and direct authoritative setup helpers.
- Run the simulation for the target tick count.
- Assert the verification-layer surfaces named above.
- Document the scenario isolation choice in a comment block at the top of each test (per precision-rules §8).

#### Test 1: `place_dirtiness_accumulates_from_repeated_wilderness_relief`

Three human-controlled agents at the outdoor farm each run `relieve_wilderness` twice. Assert six `WasteCreated` payloads with `WildernessRelief` source, each backed by a concrete Waste `ItemLot`, and assert `PlaceDirtiness.value` does not decrease during the relief phase. Must never produce: `value` decreasing during the relief phase, `WasteCreated` events without corresponding Waste `ItemLot` entities (conservation regression).

#### Test 2: `sleep_ranking_prefers_clean_place_over_dirty_place`

Two known candidate places — a dirty current camp and a clean reachable farm — have identical `SleepQualityProfile`. Decision-trace ranking asserts the cleaner place is the selected sleep opportunity. Must never produce: agent picks the dirtier place when other recoveries are equal.

#### Test 3: `wash_partial_success_proportional_dirtiness_reduction`

Single human-controlled agent at a place with one basin configured as `WashBasinState { clean_water_units: 1, units_per_full_wash: 2, max_clean_water: 10, dirtiness_per_use: pm(50) }`. Agent dirtiness is `pm(1000)`. Agent runs wash. Event payload asserts `WashFacilityUsed { partial: true, water_consumed: 1 }`. Authoritative state asserts agent dirtiness `pm(500)`, basin `clean_water_units: 0`, basin `dirtiness_level: pm(25)` (proportional half-increment). Must never produce: full success when water insufficient, basin going negative.

#### Test 4: `latrine_overflow_creates_waste_at_place_and_increments_place_dirtiness`

Latrine-tagged place is set up with `LatrineFullness { fill: pm(800), fill_per_use: pm(80), critical_threshold: pm(800) }` (already at threshold). Agent runs `toilet`. Event-log asserts `WasteCreated` with `OvercapacityLatrine` source. Authoritative `PlaceDirtiness.value` asserts increment by the place's `dirtiness_per_use`. Must never produce: `LatrineFullness.fill` decreasing without a maintenance action, overcapacity not creating Waste.

#### Test 5: `basin_natural_refill_from_colocated_water_source`

Basin at a place is set up with `WashBasinState { clean_water_units: 0, max_clean_water: 5, refill_per_tick: 1, .. }`, co-located with `ResourceSource { commodity: Water, available_quantity: q(100), .. }`. Run 6 ticks with no agent activity. Authoritative state asserts `clean_water_units == 5`, source `available_quantity == 95`. Must never produce: basin refilling without consuming source, basin overshooting `max_clean_water`.

#### Test 6: `wash_ai_selects_non_empty_basin_when_other_basin_is_empty`

Two basins at one place — "Basin A" empty and "Basin B" still usable. A stale `BestEffort` request against Basin A records `StartFailed`, and the same AI planning tick excludes Basin A from generated Wash opportunities while selecting Basin B by `OpportunityAnchor::Entity`. Must never produce: agent attempts or selects a known-empty basin without `PreconditionFailed` and a usable-basin replan.

### 2. Adversarial sweep tests (smaller scope, in same file or separate `golden_place_dirtiness_sweeps.rs`)

- `place_dirtiness_saturates_with_zero_decay` — `decay_per_tick = pm(0)` + continuous wilderness relief; assert `value` reaches `pm(1000)` and stays there.
- `wash_basin_plateaus_at_zero_with_zero_refill` — `refill_per_tick = 0` + continuous wash demand; assert `clean_water_units` plateaus at zero.

(The "zero `critical_threshold`" sweep is already covered by ticket 006's focused tests.)

### 3. If `forensic_wash_vs_water_competition.rs` needs rewriting

If ticket 007's reassessment deferred the rewrite, perform it here: update the scenario to use the new basin-state-buffered wash semantics. If ticket 007 already handled it, no work needed in this ticket.

### 4. Harness-authored scenario fragments

The landed goldens use the existing `golden_harness` in-process fixture style rather than separate RON files. This keeps the assertions at the earliest causal boundary for each S129 target pattern while still exercising the full action registry, scheduler, event log, AI decision trace, and maintenance pass.

## Files to Touch

- `crates/worldwake-ai/tests/golden_place_dirtiness.rs` (new — six target-pattern goldens + adversarial sweeps)
- `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` (no change — rerun as adjacent hygiene coverage)

## Out of Scope

- Per-component focused/unit tests — landed in tickets 001–011.
- Adversarial sweep for zero `critical_threshold` — covered by ticket 006's `toilet_already_over_threshold_emits_waste_created_each_tick`.
- Cross-day or cross-season hygiene dynamics — outside spec scope.
- Disease propagation goldens — explicitly out of scope per spec Non-Goals.
- Latrine maintenance / `clean_latrine` action goldens — deferred per spec Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. `place_dirtiness_accumulates_from_repeated_wilderness_relief` — target pattern 1.
2. `sleep_ranking_prefers_clean_place_over_dirty_place` — target pattern 2.
3. `wash_partial_success_proportional_dirtiness_reduction` — target pattern 3.
4. `latrine_overflow_creates_waste_at_place_and_increments_place_dirtiness` — target pattern 4.
5. `basin_natural_refill_from_colocated_water_source` — target pattern 5.
6. `wash_ai_selects_non_empty_basin_when_other_basin_is_empty` — target pattern 6.
7. `place_dirtiness_saturates_with_zero_decay` — adversarial sweep.
8. `wash_basin_plateaus_at_zero_with_zero_refill` — adversarial sweep.
9. `forensic_wash_vs_water_competition.rs` continues to pass (whether unchanged or rewritten in scope).
10. Existing suite: `cargo test --workspace`, plus the live `./scripts/verify.sh` gates run directly.

### Invariants

1. Every Waste lot created during the goldens has a corresponding `WasteCreated` event tag in the event log — counts match exactly (conservation chain proof).
2. Sleep candidate ranking is FND-7 / FND-14A-compliant at the tested seam: explicitly seeded local/world beliefs expose the relevant `PlaceDirtiness`, and decision trace shows the cleaner place selected.
3. Wash partial-success arithmetic is exactly proportional: `agent_dirtiness_delta / prev_dirtiness == water_consumed / units_per_full_wash` (within Permille rounding).
4. `LatrineFullness.fill` is monotonically non-decreasing during the goldens (no maintenance action exists yet).
5. Authoritative-to-AI replan on basin emptiness completes within the same simulation epoch — the agent does not get stuck holding an invalid plan across many ticks.
6. Reproducibility: every golden uses an explicit seeded `GoldenHarness`, so the executed assertions are deterministic under the same seeded setup.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_place_dirtiness.rs` (new) — six target-pattern tests + two adversarial sweep tests.
2. `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` — no change; rerun to confirm adjacent hygiene coverage still passes.

### Commands

1. `cargo test -p worldwake-ai golden_place_dirtiness`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo fmt --all -- --check`
6. `bash scripts/check_active_goal_removed.sh`
7. `cargo clippy --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo run -p worldwake-cli --bin scenario-coverage -- --check`

## Outcome

Completed on 2026-05-01.

- Added `crates/worldwake-ai/tests/golden_place_dirtiness.rs` with eight S129 golden scenarios: six target-pattern tests plus the zero-decay and zero-refill adversarial sweeps.
- Refreshed generated golden inventory artifacts, including the new `docs/generated/golden-scenario-details/place-dirtiness.md` page and global inventory/index/matrix updates.
- Confirmed `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` still passes unchanged.

## Deviations

- The goldens use harness-authored in-process fixtures rather than separate RON scenario files. This is the current strongest live golden seam for these small target patterns and avoids long autonomous scenario churn.
- The basin-empty replan golden proves the paired live boundary: a stale empty-basin `wash` request records `StartFailed`, and the same AI planning tick selects the usable basin by entity anchor.
- `./scripts/verify.sh` was not run as a wrapper after the final edit; its live gates were inspected and run directly.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_place_dirtiness -- --list`
- Passed `cargo test -p worldwake-ai --test golden_place_dirtiness`
- Passed `cargo test -p worldwake-ai --test forensic_wash_vs_water_competition`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo fmt --all -- --check`
- Passed `bash scripts/check_active_goal_removed.sh`
- Passed `cargo clippy --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
