# S76GOLGAPSIOBS-003: Golden S76-D — utility profile diversity drives different self-care choices

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The simulation observer found all 3 AI agents collapsed into identical sleep+relieve patterns despite having different `UtilityProfile` weights (Finding 2). The repo already has `golden_utility_weight_diversity_in_need_selection()`, but that scenario proves cross-domain divergence (`ConsumeOwnedCommodity` vs `RestockCommodity`) under different agent conditions rather than same-state self-care divergence under identical needs. There is still no focused golden proving that agents with the same local self-care substrate but different `UtilityProfile` weights split between different owned-consume self-care branches. Without this regression guard, changes to ranking or candidate generation could silently collapse local self-care diversity back into a single branch.

## Assumption Reassessment (2026-04-09)

1. `UtilityProfile` exists at `crates/worldwake-core/src/utility_profile.rs:8-24` with 15 `Permille` weight fields including `hunger_weight`, `thirst_weight`, `fatigue_weight`. These are the parameters varied across agents.
2. `golden_reasoning_diversity.rs` (228 lines) contains only `search_depth_divergence()` today; it does not already own a utility-profile scenario. However, `golden_ai_decisions.rs` already contains `golden_utility_weight_diversity_in_need_selection()` (Scenario `S02b`), so S76-D must stay distinct from that existing proof.
3. Shared boundary: golden harness + ranking system. No production code changes.
4. Existing lower-layer ranking coverage already proves the expected weighted order for simultaneous critical hunger/thirst/fatigue in `ranking.rs::simultaneous_critical_self_care_needs_rank_by_weighted_order()`. The golden should therefore prove the full AI/action path for the strongest live same-state branch split, not re-litigate the ranking math in isolation.
5. GoalKinds under test: `ConsumeOwnedCommodity` (`Bread`/`Water`) only. Focused reassessment falsified the earlier three-way `Sleep`, `Wash`, and `Relieve` variants under the live planner surface; the strongest honest same-state divergence is the two-way `eat` vs `drink` split.
6. Ticket/spec setup drift: the proposed shared scarce pool (`3 apple lots, 2 water lots, 1 bed`) would introduce contention and commodity-shape noise that can mask the utility-weight signal. The honest live isolation is per-agent owned bread and water with no contention and no third competing branch claim.
12. Scenario isolation: 2 agents start at the same place with identical critical hunger/thirst and the same owned self-care substrate. Only `UtilityProfile` weights differ. Contention, enterprise signals, remote travel, fatigue/sleep competition, and hygiene/relief branches are intentionally excluded so the golden proves the strongest same-state utility divergence the live system actually exposes.

## Architecture Check

1. Adding to `golden_reasoning_diversity.rs` (228 lines) is appropriate — same domain (reasoning diversity), small file. Both search-depth and utility-profile diversity tests belong together.
2. No backwards-compatibility shims. Tests only.

## Verification Layers

1. Same-state agents select different self-care goals at tick 0 -> decision trace (`selected_goal()` differs by agent despite identical needs and local substrate)
2. Divergent selected goals become divergent first self-care actions -> action trace (`eat` vs `drink`)
3. Deterministic replay -> authoritative world state equality across two runs with same seed
6. Single-layer ticket (golden E2E tests only). No production code changes.

## What to Change

### 1. Implement S76-D scenario runner

Add to `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`:

Create `run_utility_profile_diversity(seed: Seed)` returning an observation struct:

- Use `VillageSquare` with no scarcity contention.
- Spawn 2 AI agents with identical critical hunger/thirst and different `UtilityProfile` weights:
  - Agent A: hunger-prioritizing
  - Agent B: thirst-prioritizing
- Give each agent the same owned local self-care substrate: bread + water. Do not add enterprise signals, remote resources, or contested beds.
- Give all agents the same `PerceptionProfile` and `CognitiveProfile`.
- Step tick 0 with decision tracing enabled, then continue only long enough to observe each first started self-care action.
- Collect: per-agent selected tick-0 goal and first started action.

### 2. Implement S76-D test and replay companion

```rust
// Scenario S76-D: Different Utility Profiles Produce Different Goal Orderings
#[test]
fn golden_utility_profile_diversity() { ... }

#[test]
fn golden_utility_profile_diversity_replays_deterministically() { ... }
```

Use `Seed([179; 32])`. Assert the hunger-weighted agent selects/starts `eat` and the thirst-weighted agent selects/starts `drink`. This is the same-state, same-place utility-divergence contract the existing `S02b` scenario does not cover.

## Files to Touch

- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (modify)

## Out of Scope

- Fixing the planner or ranking system — this test guards existing behavior
- Planner fallback testing (S76GOLGAPSIOBS-001)
- Perception belief coverage (S76GOLGAPSIOBS-002)
- Search-depth diversity (already covered by existing `search_depth_divergence()`)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_utility_profile_diversity` — 2 same-state agents with different `UtilityProfile`s select different tick-0 self-care goals and start different first self-care actions
2. `golden_utility_profile_diversity_replays_deterministically` — identical observations across two runs
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code changes — engine behavior is unchanged
2. Deterministic replay: same seed produces identical observation structs
3. Behavioral diversity: different `UtilityProfile` weights produce different selected self-care goals under identical needs and local substrate (P22)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs::golden_utility_profile_diversity` — regression guard for same-state utility-profile-driven `eat` vs `drink` divergence
2. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs::golden_utility_profile_diversity_replays_deterministically` — determinism guard

### Commands

1. `cargo test -p worldwake-ai golden_utility_profile_diversity`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completion date: 2026-04-09

Implemented S76-D in `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` as Scenario 129 plus a deterministic replay companion. The original three-way same-state contract from the stale ticket/spec narrative did not hold under live focused proof: fatigue-, dirtiness-, and bladder-driven third-branch variants all collapsed back into `drink` at tick 0. I corrected the ticket to the strongest honest live contract and landed a two-agent same-state owned-consume scenario instead.

The completed golden now proves that two agents with identical critical hunger/thirst, identical local owned bread+water substrate, and different `UtilityProfile` weights diverge at both the decision-trace boundary and the action-trace boundary: the hunger-weighted agent selects/starts `eat`, while the thirst-weighted agent selects/starts `drink`. This remains distinct from the existing `golden_utility_weight_diversity_in_need_selection()` coverage because it is same-state, same-place, same-substrate self-care divergence rather than cross-domain divergence under different agent conditions.

Generated golden docs were refreshed as expected. The owning scenario detail page, inventory, scenario index, and coverage matrix all changed from the new Scenario 129 registration; no unexpected generated spillover appeared beyond that normal inventory/index fallout.

## Verification Result

- Passed: `cargo test -p worldwake-ai golden_utility_profile_diversity -- --nocapture`
- Passed: `cargo test -p worldwake-ai`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Passed: `python3 scripts/golden_inventory.py --write --check-docs`
