# S129PLADIRFAC-012: Golden coverage — hygiene end-to-end

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — new golden test file exercising the full S129 stack
**Deps**: S129PLADIRFAC-005, S129PLADIRFAC-006, S129PLADIRFAC-007, S129PLADIRFAC-008, S129PLADIRFAC-010, S129PLADIRFAC-011

## Problem

Without golden coverage, S129's emergent chains ("dirty place → bad sleep → travel decision", "full latrine → wilderness fallback", "empty basin → partial wash → stay dirty longer") have no end-to-end proof — the per-feature focused tests in tickets 005–011 each validate their slice, but they do not prove the slices compose. This ticket lands the six target-pattern goldens spec D12 declares plus the adversarial-sweep scenarios so the architecture can be falsified per FND-31. It also covers Authoritative-to-AI Impact Rule checklist point 7 (golden tests) for the wash refactor.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The full S129 production stack lands in tickets 001–011. By the time this ticket runs, every component, event tag, precondition arm, handler extension, maintenance step, candidate emission split, ranking integration, and scenario `*Def` wrapper is in place. Coverage gap classification (precision-rules §3): six new golden E2E scenarios plus three adversarial sweep scenarios; current `crates/worldwake-ai/tests/` has no `golden_place_dirtiness*` file (verified during reassessment — the only related golden is `forensic_wash_vs_water_competition.rs`).
2. The shared abstraction boundary under audit is the full S129 emergent contract — wilderness relief writes `PlaceDirtiness`, place dirtiness biases sleep ranking, sleep ranking drives travel candidacy, travel changes the agent's location, location changes which place's dirtiness accumulates next. The goldens prove the chain end-to-end, not any single slice.
3. Each golden scenario must declare its target pattern (FND-31 + spec D12 § Target Patterns): the intended invariant under test, the lawful competing affordances the architecture would otherwise allow (per precision-rules §8 scenario isolation), and what the test must never produce.
4. The adversarial sweeps (zero `decay_per_tick` saturation, zero `refill_per_tick` plateau, zero `critical_threshold` immediate overflow) are listed in spec D12 as "must support" but not necessarily "must exercise this ticket". This ticket exercises the saturation and plateau cases; the immediate-overflow case is exercised in ticket 006's focused tests already.
5. The `forensic_wash_vs_water_competition.rs` existing golden may need rewriting if ticket 007's basin-state refactor invalidates its assumptions (the original golden assumed two-target wash with direct well consumption). If reassessment during 007's implementation deferred the rewrite to 012, that rewrite belongs here. Confirm during this ticket's reassessment phase whether 007's implementation already updated it or whether the golden currently fails — if the latter, include the rewrite in scope.
6. Scenario isolation choices (precision-rules §8): each golden uses minimal scenarios that isolate one specific S129 behavior. Competing affordances (e.g., the agent could also harvest food, drink water) are intentionally limited so the place-dirtiness or basin-state behavior dominates ranking. Document the isolation explicitly per the rule.
7. Per CLAUDE.md "Golden production tests require `PerceptionProfile` on agents that need to observe post-production output" — agents in goldens 1, 2, 4, 6 must carry `PerceptionProfile` so they can perceive the new `PlaceDirtiness` / `LatrineFullness` / `WashBasinState` state at their place/facility.

## Architecture Check

1. A single golden file `golden_place_dirtiness.rs` cohabits with sibling hygiene-focused goldens (`forensic_wash_vs_water_competition.rs` and any future hygiene work). Splitting per-target-pattern into separate files would scatter the S129 contract; the single-file approach mirrors how S128's `golden_sleep_episode.rs` (or equivalent) consolidates one spec's E2E coverage.
2. Each golden uses RON-authored scenarios (per the existing golden test convention) with explicit `place_dirtiness:`, `latrine_fullness:`, `wash_basin_state:` fields exercised through ticket 011's authoring surface. This validates the full authoring → spawn → simulation → ranking → outcome stack.
3. No backward-compat shim. The new goldens are net-new; if `forensic_wash_vs_water_competition.rs` needs rewriting, the rewrite is a clean replacement, not an alias.

## Verification Layers

Each target pattern below maps to a specific verification surface (precision-rules §5):

1. **Place dirtiness accumulation** (target pattern 1) → authoritative world state assertion (`PlaceDirtiness.value` after N ticks) plus event-log delta (`WasteCreated` count).
2. **Sleep ranking under dirtiness** (target pattern 2) → decision-trace assertion (the agent's chosen sleep target) plus authoritative location at the post-sleep tick.
3. **Wash partial success** (target pattern 3) → action-trace assertion (the wash action's commit outcome carries `partial: true`) plus authoritative `WashBasinState` and agent `HomeostaticNeeds.dirtiness` post-commit.
4. **Latrine overcapacity** (target pattern 4) → event-log delta (`WasteCreated` with `OvercapacityLatrine` source on the threshold-crossing tick) plus authoritative `PlaceDirtiness.value` increment.
5. **Basin natural refill from co-located source** (target pattern 5) → authoritative state delta over multiple ticks (`clean_water_units` increment, `available_quantity` decrement).
6. **Auth-to-AI replan on basin emptiness** (target pattern 6) → decision-trace assertion (replan invocation after `BestEffort` precondition fail) plus action-trace (the eventually-chosen second basin's commit).

## What to Change

### 1. New `crates/worldwake-ai/tests/golden_place_dirtiness.rs`

Six target-pattern goldens, each as a `#[test]` function with an authored scenario fragment. For each:

- Construct the scenario via the existing golden test harness (likely `golden_harness::Setup` or similar — confirm during implementation).
- Run the simulation for the target tick count.
- Assert the verification-layer surfaces named above.
- Document the scenario isolation choice in a comment block at the top of each test (per precision-rules §8).

#### Test 1: `place_dirtiness_accumulates_from_repeated_wilderness_relief`

Three agents staying at "Fertile Fields" (no latrine, no shelter), each running `relieve_wilderness` twice over 4 ticks (6 reliefs total, default `dirtiness_per_use = pm(80)`, default `decay_per_tick = pm(2)`). After 4 ticks: assert `PlaceDirtiness.value` ≈ `pm(480 - 8)` = `pm(472)` (saturating math for the small decay during the run). Assert 6 `WasteCreated` events with `WildernessRelief` source. Must never produce: `value` decreasing during the relief phase, `WasteCreated` events without corresponding Waste `ItemLot` entities (conservation regression).

#### Test 2: `sleep_ranking_prefers_clean_place_over_dirty_place`

Two candidate places — "Dirty Camp" with `PlaceDirtiness { value: pm(800), .. }`, "Clean Shelter" with `value: pm(100)` — identical `SleepQualityProfile`. Single agent at a third place reachable to both. After ranking, agent travels to and sleeps at "Clean Shelter". Decision-trace asserts the chosen sleep candidate is "Clean Shelter". Must never produce: agent picks "Dirty Camp" when other recoveries are equal.

#### Test 3: `wash_partial_success_proportional_dirtiness_reduction`

Single agent at "Riverside Camp" with one basin authored as `WashBasinState { clean_water_units: 1, units_per_full_wash: 2, max_clean_water: 10, dirtiness_per_use: pm(50) }`. Agent dirtiness `pm(1000)`. Agent runs wash. Action-trace asserts `WashFacilityUsed { partial: true, water_consumed: 1 }`. Authoritative state asserts agent dirtiness `pm(500)`, basin `clean_water_units: 0`, basin `dirtiness_level: pm(25)` (proportional half-increment). Must never produce: full success when water insufficient, basin going negative.

#### Test 4: `latrine_overflow_creates_waste_at_place_and_increments_place_dirtiness`

Latrine-tagged place authored with `LatrineFullness { fill: pm(800), fill_per_use: pm(80), critical_threshold: pm(800) }` (already at threshold). Agent runs `toilet`. Event-log asserts `WasteCreated` with `OvercapacityLatrine` source. Authoritative `PlaceDirtiness.value` asserts increment by the place's `dirtiness_per_use`. Must never produce: `LatrineFullness.fill` decreasing without a maintenance action, overcapacity not creating Waste.

#### Test 5: `basin_natural_refill_from_colocated_water_source`

Basin at "Riverside Camp" authored with `WashBasinState { clean_water_units: 0, max_clean_water: 5, refill_per_tick: 1, .. }`, co-located with `ResourceSource { commodity: Water, available_quantity: q(100), .. }`. Run 6 ticks with no agent activity. Authoritative state asserts `clean_water_units == 5`, source `available_quantity == 95`. Must never produce: basin refilling without consuming source, basin overshooting `max_clean_water`.

#### Test 6: `wash_auth_to_ai_replan_when_basin_drained_between_affordance_and_start`

Two basins at one place — "Basin A" and "Basin B", both with `clean_water_units: 1, units_per_full_wash: 2`. Two agents, both want to wash. Tie-break has agent A's wash first. After agent A washes "Basin A" (which is now empty), agent B's plan still references "Basin A" (planned in the same tick before the drain). Agent B's `BestEffort` start fails per ticket 003's precondition. Decision-trace asserts agent B replans onto "Basin B". Action-trace asserts agent B's eventual wash commit at "Basin B". Must never produce: agent B attempts wash at empty "Basin A" without `PreconditionFailed` and replan.

### 2. Adversarial sweep tests (smaller scope, in same file or separate `golden_place_dirtiness_sweeps.rs`)

- `place_dirtiness_saturates_with_zero_decay` — `decay_per_tick = pm(0)` + continuous wilderness relief; assert `value` reaches `pm(1000)` and stays there.
- `wash_basin_plateaus_at_zero_with_zero_refill` — `refill_per_tick = 0` + continuous wash demand; assert `clean_water_units` plateaus at zero.

(The "zero `critical_threshold`" sweep is already covered by ticket 006's focused tests.)

### 3. If `forensic_wash_vs_water_competition.rs` needs rewriting

If ticket 007's reassessment deferred the rewrite, perform it here: update the scenario to use the new basin-state-buffered wash semantics. If ticket 007 already handled it, no work needed in this ticket.

### 4. RON scenario fragments

Each golden's scenario is authored either inline (via `PlaceDef { ... }` and `FacilityDef { ... }` literals — using ticket 011's new field additions) or in a small RON fragment loaded by the test. The existing golden test harness (`golden_harness::Setup`) handles both styles.

## Files to Touch

- `crates/worldwake-ai/tests/golden_place_dirtiness.rs` (new — six target-pattern goldens + adversarial sweeps)
- Likely: `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` (modify — if not already rewritten in ticket 007)

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
6. `wash_auth_to_ai_replan_when_basin_drained_between_affordance_and_start` — target pattern 6.
7. `place_dirtiness_saturates_with_zero_decay` — adversarial sweep.
8. `wash_basin_plateaus_at_zero_with_zero_refill` — adversarial sweep.
9. `forensic_wash_vs_water_competition.rs` continues to pass (whether unchanged or rewritten in scope).
10. Existing suite: `cargo test --workspace`, `./scripts/verify.sh`.

### Invariants

1. Every Waste lot created during the goldens has a corresponding `WasteCreated` event tag in the event log — counts match exactly (conservation chain proof).
2. Sleep candidate ranking is FND-7 / FND-14A-compliant: agent's perceived `PlaceDirtiness` drives the choice, observable through `ProfileBeliefView` accessors only.
3. Wash partial-success arithmetic is exactly proportional: `agent_dirtiness_delta / prev_dirtiness == water_consumed / units_per_full_wash` (within Permille rounding).
4. `LatrineFullness.fill` is monotonically non-decreasing during the goldens (no maintenance action exists yet).
5. Authoritative-to-AI replan on basin emptiness completes within the same simulation epoch — the agent does not get stuck holding an invalid plan across many ticks.
6. Reproducibility: every golden uses the standard `ChaCha8Rng` seed; rerunning under the same seed produces byte-identical state and event-log surfaces (per the determinism invariant in CLAUDE.md).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_place_dirtiness.rs` (new) — six target-pattern tests + two adversarial sweep tests.
2. Possibly: `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` — rewrite if ticket 007 deferred it.

### Commands

1. `cargo test -p worldwake-ai golden_place_dirtiness`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
