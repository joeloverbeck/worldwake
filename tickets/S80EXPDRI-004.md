# S80EXPDRI-004: Candidate generation and ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate emitter, ranking integration, counter management in agent tick
**Deps**: S80EXPDRI-001, S80EXPDRI-002, S80EXPDRI-003

## Problem

Agents with unmet needs and no known satisfaction path currently enter indefinite sleep+relieve loops because no live exploration candidate emitter exists. `S80EXPDRI-001` landed only inert ranking substrate for `ExploreLocation` so the symbol compiles safely; this ticket still needs to add the need+ignorance emitter, promote ranking from inert `Background`/`0` handling to the real exploration motive formula, and manage consecutive-exploration counters.

## Assumption Reassessment (2026-04-10)

1. `generate_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:187` takes `view: &dyn GoalBeliefView, agent: EntityId, blocked: &BlockedIntentMemory, recipes: &RecipeRegistry, current_tick: Tick`. The view provides `homeostatic_needs()`, `agent_belief_store()`, `resource_sources_at()`, `adjacent_places_with_travel_ticks()`, and (after ticket 002) `exploration_profile()`.
2. `GoalBeliefView::homeostatic_needs()` at `belief_view.rs:191` returns `Option<HomeostaticNeeds>`. `HomeostaticNeeds` has fields `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness` (all `Permille`).
3. Shared abstraction boundary: candidate generation reads through `GoalBeliefView` (P26), never directly from ECS. Exploration emitter follows the same pattern as existing emitters (drive, enterprise, social, combat).
5. The spec's planner-driven scope: `GoalKind::ExploreLocation` with `GoalPriorityClass::Low` and motive score `need_level.as_raw() * curiosity_weight.as_raw() / 1000`. The ranking system uses `RankedGoal` with `priority_class` and `motive_score: u32` fields.
6. `GoalBeliefView::resource_sources_at()` at `belief_view.rs:183` returns `Vec<EntityId>` — used to check whether the agent knows of any resource source for a need's commodity at reachable places.
7. Counter management: `consecutive_exploration_count` on `ExplorationProfile` is incremented when ExploreLocation is selected, reset to 0 when any other goal is selected. This happens during goal selection in the agent tick, not in candidate generation itself.
8. `S80EXPDRI-001` already added compile-safe inert ranking coverage for `ExploreLocation` in `crates/worldwake-ai/src/ranking.rs` (`GoalPriorityClass::Background`, motive `0`, discriminant branch). This ticket owns replacing that inert handling with the live `Low` priority / motive formula rather than introducing the symbol for the first time.

## Architecture Check

1. The exploration emitter follows the established emitter pattern: read agent state through GoalBeliefView → check trigger conditions → enumerate candidates → emit goals. Target selection uses existing belief store and topology queries — no new global state access. The ranking formula (`need * curiosity / 1000`) is a simple local computation consistent with how other goals compute motive scores.
2. No backward-compatibility shims. New emitter is additive — existing candidate generation unaffected. Counter management is a small addition to goal selection flow.

## Verification Layers

1. Exploration candidate emitted when need above threshold + no known resource source → decision trace (candidate generation focused test)
2. Exploration candidate NOT emitted when resource source is known → decision trace (candidate generation focused test)
3. Exploration candidate NOT emitted when consecutive count >= max → decision trace (candidate generation focused test)
4. ExploreLocation ranked at GoalPriorityClass::Low → ranking focused test
5. Motive score follows formula → ranking focused test
6. Counter incremented on exploration, reset on other goal → agent tick integration test

## What to Change

### 1. Add exploration emitter function

In `crates/worldwake-ai/src/candidate_generation.rs` (or new `exploration_candidates.rs` module):

```rust
fn emit_exploration_candidates(candidates: &mut CandidateCollector, ctx: &GenerationContext)
```

**Trigger conditions** (all must hold):
1. Agent has `ExplorationProfile` (via `view.exploration_profile(agent)`)
2. At least one need above `need_activation_threshold`
3. No believed resource source for that need's commodity at any reachable place
4. `consecutive_exploration_count < max_consecutive_explorations` (or max == 0 for no limit)

**Target selection**:
1. Enumerate known places from `AgentBeliefStore.known_entities` where kind == Place
2. Add adjacent-to-known via `view.adjacent_places_with_travel_ticks(known_place)`
3. Filter: exclude current place, exclude places visited within `visit_lookback_ticks` (from belief timestamps)
4. Rank: proximity → novelty → RNG tiebreak
5. Emit `GoalKind::ExploreLocation { target_place, motivating_need }` for top candidate

### 2. Wire emitter into generate_candidates

Call `emit_exploration_candidates` as a new emitter group in `generate_candidates_with_travel_horizon`, after existing emitters.

### 3. Replace inert ranking handling with live exploration ranking

In the ranking system (`crates/worldwake-ai/src/ranking.rs` or `goal_model.rs`):
- Replace the inert `ExploreLocation` ranking branches shipped in `S80EXPDRI-001`
- ExploreLocation gets `GoalPriorityClass::Low`
- Motive score: `need_level.as_raw() as u32 * curiosity_weight.as_raw() as u32 / 1000`

### 4. Counter management in agent tick

In the agent tick goal selection flow:
- When ExploreLocation is selected: increment `consecutive_exploration_count` on the agent's `ExplorationProfile`
- When any other goal is selected: reset `consecutive_exploration_count` to 0
- Write updated profile back to ECS store

### 5. Need-to-commodity mapping

Add a helper that maps `HomeostaticNeedId` to the commodity that satisfies it (e.g., `Hunger → Food/Apple`, `Thirst → Water`). This enables checking whether the agent knows of resource sources for a given need. If such a mapping already exists, reuse it.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add exploration emitter call)
- `crates/worldwake-ai/src/exploration_candidates.rs` (new — emitter function, target selection)
- `crates/worldwake-ai/src/ranking.rs` (modify — ExploreLocation priority class and motive score)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — counter management on goal selection)

## Out of Scope

- Golden E2E tests (ticket 005)
- Modifications to travel actions or perception system (existing flow, no changes)
- Systematic cartography or map-building mechanics
- Random wandering — targets are selected from known/adjacent-to-known places only
- Exploration as permanent background activity — activates only when needs unmet + no known path

## Acceptance Criteria

### Tests That Must Pass

1. Agent with hunger above threshold, no known food source → ExploreLocation candidate emitted
2. Agent with hunger above threshold, known food source at reachable place → NO ExploreLocation candidate
3. Agent with all needs below threshold → NO ExploreLocation candidate
4. Agent with `consecutive_exploration_count >= max_consecutive_explorations` → NO ExploreLocation candidate
5. Agent with `curiosity_weight: Permille(0)` → NO ExploreLocation candidate (motive score 0)
6. Target selection excludes current place and recently visited places
7. ExploreLocation ranking is promoted from inert `Background`/`0` handling to `GoalPriorityClass::Low` with the correct motive score
8. Counter increments on exploration selection, resets on other goal selection
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Exploration candidates are belief-gated: agent queries only `AgentBeliefStore` and `GoalBeliefView`, never authoritative world state directly (P14)
2. No global state access: topology neighbors accessed through `GoalBeliefView::adjacent_places_with_travel_ticks()` (P7)
3. Exploration pressure is derived, never stored — computed from need levels + belief gaps each tick (P3)
4. Replacing the inert ranking branch preserves the intended ordering: GoalPriorityClass::Low ensures exploration never outprioritizes direct need satisfaction

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exploration_candidates.rs` (test module) — emitter trigger conditions, target selection, filtering
2. `crates/worldwake-ai/src/ranking.rs` (test module) — ExploreLocation priority class and motive score
3. `crates/worldwake-ai/src/candidate_generation.rs` (test module) — integration: exploration candidates appear in generate_candidates output

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`
