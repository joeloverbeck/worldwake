# S107PRODIV-006: Proactive exploration candidate emission — emit, select, familiarity/novelty

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate emitter in worldwake-ai, LastProactiveExplorationTick update
**Deps**: archive/tickets/S107PRODIV-002.md, archive/tickets/S107PRODIV-003.md, archive/tickets/S107PRODIV-004.md

## Problem

The core proactive exploration logic: when an agent has a `DiversificationProfile` and comfortable needs, emit `ExploreLocation` goals targeting the least-familiar believed place. This is the central behavioral change of S107 — everything else is infrastructure.

## Assumption Reassessment (2026-04-17)

1. `emit_exploration_candidates` at `candidate_generation.rs:2342` follows `(candidates, diagnostics, ctx, needs, ...)` pattern. Called from `generate_candidates_with_travel_horizon` at line 270. New emitter follows same pattern.
2. `exploration_candidate_places` at `candidate_generation.rs:4307` — signature: `(view: &dyn GoalBeliefView, agent: EntityId, frontier_depth: u16) -> BTreeMap<EntityId, Option<Tick>>`. Reusable for proactive target selection with `max_exploration_hops` as depth.
3. `emit_candidate_with_trace` at `candidate_generation.rs:3920` — standard emission helper. Takes `(candidates, diagnostics, kind, anchor, evidence, evidence_trace)`.
4. `HomeostaticNeeds::max_value() -> u16` at `needs.rs:55` — returns highest need value across all five needs. Used for need-slack veto.
5. `GenerationContext` at `candidate_generation.rs:147` — holds `view`, `agent`, `place`, `travel_horizon`, `blocked`, `recipes`, `current_tick`, etc.
6. `agent_belief_store` accessor on GoalBeliefView at `belief_view.rs:91` — returns `Option<&AgentBeliefStore>`, giving access to `place_visits`.
7. `update_exploration_counter_for_adopted_goal` in `crates/worldwake-ai/src/agent_tick/mod.rs` is the live commitment hook. It already updates `ExplorationProfile.consecutive_exploration_count` for all `ExploreLocation` goals and is the honest place to stamp `LastProactiveExplorationTick(Some(tick))` for `ExplorationMotivation::Proactive`.
8. `ranking.rs` still intentionally leaves proactive exploration inert: `exploration_motive(...)` returns `0` for `ExplorationMotivation::Proactive`, and the focused test is `explore_location_proactive_motive_stays_zero_until_proactive_ranking_lands`. This ticket owns flipping that behavior live.

- `Already landed`: `ExplorationMotivation`, `DiversificationProfile` / `LastProactiveExplorationTick` belief accessors, `AgentBeliefStore.place_visits`, and CLI spawn wiring from tickets 002-005.
- `Still live`: proactive `ExploreLocation` emission/selection in `candidate_generation.rs`, proactive ranking in `ranking.rs`, and proactive commitment timestamp updates in `agent_tick/mod.rs`.
- `New fallout`: focused unit coverage in the existing `candidate_generation.rs`, `ranking.rs`, and `agent_tick/tests.rs` modules.
- `No-change cited files`: none.

## Architecture Check

1. Proactive emission follows established emitter pattern (GenerationContext, emit_candidate_with_trace). No special-casing in the ranking or planning pipeline — proactive ExploreLocation uses the same GoalKind as reactive, just with different ExplorationMotivation.
2. Familiarity/novelty are derived on query (FND-3/FND-27), never stored. Visit history in PlaceVisitRecord is the concrete source of truth.
3. Need-slack veto ensures proactive exploration never competes with survival needs (FND-11 dampener).
4. No backward-compatibility shims.

## Verification Layers

1. Need-slack veto suppresses emission when max_need > comfort_threshold → focused unit test (candidate generation)
2. Cooldown suppresses emission within exploration_cooldown_ticks → focused unit test
3. Curiosity pressure accumulates linearly with ticks since last exploration → focused unit test
4. Target selection picks highest-novelty place → focused unit test
5. Unvisited places have maximum novelty (1000) → focused unit test
6. Proactive ExploreLocation uses ExplorationMotivation::Proactive → focused unit test
7. Agent without DiversificationProfile never emits proactive candidates → focused unit test

## What to Change

### 1. Add familiarity/novelty computation functions

In `crates/worldwake-ai/src/candidate_generation.rs`, add:
- `compute_familiarity(record, current_tick, profile) -> Permille`
- `compute_novelty(record, current_tick, profile) -> Permille`

These are private functions used only by the proactive emission logic.

### 2. Add select_proactive_target

In `crates/worldwake-ai/src/candidate_generation.rs`, add target selection function that reuses `exploration_candidate_places` BFS with `max_exploration_hops` as depth, then scores by novelty.

### 3. Add emit_proactive_exploration_candidates

In `crates/worldwake-ai/src/candidate_generation.rs`, implement the emitter with four gates:
1. Need-slack veto: `max_value() > comfort_threshold.value()` → return
2. Cooldown: ticks since last proactive < cooldown_ticks → return
3. Curiosity pressure: linear accumulation clamped at 1000
4. Utility gate: multiplicative product of base_curiosity × curiosity_pressure × need_slack × novelty = 0 → return

### 4. Wire into generate_candidates

Call `emit_proactive_exploration_candidates` from `generate_candidates_with_travel_horizon` alongside the existing `emit_exploration_candidates` call (~line 270).

### 5. Update LastProactiveExplorationTick on goal commitment

Find where `consecutive_exploration_count` is incremented (in agent_tick or goal commitment logic) and add a parallel update: when a committed ExploreLocation has `ExplorationMotivation::Proactive`, set `LastProactiveExplorationTick(Some(current_tick))`.

### 6. Handle ExplorationMotivation::Proactive in ranking

In `crates/worldwake-ai/src/ranking.rs`, make `motive_score` for `ExploreLocation` with `Proactive` motivation live using the already-exposed diversification carrier and cooldown/buildup context, replacing the current explicit zero-motive placeholder test.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — familiarity/novelty functions, target selection, emitter, wire into generate_candidates
- `crates/worldwake-ai/src/ranking.rs` (modify) — handle Proactive motivation in motive_score
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify) — update LastProactiveExplorationTick on goal commitment
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — focused commitment timestamp proof

## Out of Scope

- Golden E2E tests (ticket 007)
- CLI scenario creation (ticket 005)
- PlaceVisitRecord update mechanism (ticket 004 — assumed complete)
- ExplorationMotivation type migration (ticket 002 — assumed complete)

## Acceptance Criteria

### Tests That Must Pass

1. Agent with DiversificationProfile, comfortable needs, and known unvisited places → emits proactive ExploreLocation
2. Agent with DiversificationProfile but max_need > comfort_threshold → no proactive emission
3. Agent with DiversificationProfile within cooldown window → no proactive emission
4. Agent without DiversificationProfile → no proactive emission regardless of need state
5. Target selection prefers unvisited places (novelty=1000) over visited places
6. Familiarity increases with visit count, decreases with time away, bounded by floor
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Proactive exploration never fires when any survival need exceeds comfort_threshold (need-slack veto)
2. Proactive candidates use ExplorationMotivation::Proactive, not NeedDriven
3. All familiarity/novelty values are derived on query, never stored
4. Proactive ExploreLocation counts toward consecutive_exploration_count (S80 safety limit)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for: need-slack veto, cooldown enforcement, curiosity accumulation, target selection by novelty, emission with correct ExplorationMotivation
2. `crates/worldwake-ai/src/candidate_generation.rs` — familiarity/novelty unit tests: visit count scaling, time recovery, floor clamping

### Commands

1. `cargo test -p worldwake-ai -- proactive`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added proactive exploration helpers in `candidate_generation.rs`: curiosity buildup, familiarity/novelty derivation, proactive target selection, and proactive `ExploreLocation` emission gated by comfort threshold, cooldown, and nonzero utility.
- Made proactive exploration rankable in `ranking.rs` using `DiversificationProfile.base_curiosity`, curiosity buildup since the last proactive commitment, and current need slack instead of the previous zero-motive placeholder.
- Extended `update_exploration_counter_for_adopted_goal` to stamp `LastProactiveExplorationTick(Some(tick))` when a proactive exploration goal is adopted, while preserving the existing consecutive exploration counter path for all `ExploreLocation` goals.
- Added focused proofs in the existing `candidate_generation.rs`, `ranking.rs`, and `agent_tick/tests.rs` modules for proactive emission, familiarity recovery, curiosity buildup, proactive motive scoring, and commitment timestamp updates.

## Deviations

- The active spec's drafted familiarity and curiosity snippets divided per-visit / per-tick accumulation by `1000`, which would quantize to zero under the landed default profile values. The implementation and active spec were corrected to the direct per-visit / per-tick interpretation that matches the ticket's stated linear accumulation contract and the live default magnitudes.
- Proactive target selection landed directly on `DiversificationProfile.max_exploration_hops` and the existing belief/travel-horizon boundary. The drafted `exploration_profile` input in the active spec snippet was removed because it was not part of the live proactive contract.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_emits_proactive_exploration_for_comfortable_agent -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::proactive_familiarity_scales_with_visits_recovers_over_time_and_respects_floor -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::proactive_exploration_commit_updates_last_proactive_tick -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
