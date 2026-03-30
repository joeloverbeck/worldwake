# S41BANOFFEME-004: Suite 3 — Wound-Dampened Raid Spiral (Scenario 49)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/pressure.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, small `GoalBeliefView`/runtime extension for bandit flee threshold, Scenario 49 golden coverage
**Deps**: S41BANOFFEME-001 (spec reassessment — confirms engine gap), S41BANOFFEME-002 (Suite 1 must pass to confirm basic raid mechanics)

## Problem

No golden test currently validates FND-10 physical dampening for the bandit offensive loop. Scenario 49 is meant to prove that repeated raid combat produces concrete wounds that eventually deter further raids without introducing cooldowns, abstract fatigue counters, or scripted pacing.

## Assumption Reassessment (2026-03-30)

1. The live `RaidTarget` path exists and is already golden-covered for proactive offensive emergence in Scenario 47 (`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`). This ticket is not about creating raids; it is about proving and implementing a concrete dampener on top of that existing path.
2. The exact shared boundary under audit is the bandit raid-deterrence contract: how concrete wound load on a bandit agent affects `GoalKind::RaidTarget` candidate emission and raid ranking. That boundary currently has no live implementation.
3. Current candidate generation only suppresses raids through immediate threat pressure. `emit_raid_target_goals()` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) returns early only when `derive_danger_pressure(view, agent) >= thresholds.danger.high()`. Wounds alone do not trigger that branch.
4. Current pressure/ranking split confirms the gap. `derive_pain_pressure()` in [`crates/worldwake-ai/src/pressure.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/pressure.rs) sums wound severity, but `priority_class()` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) feeds that pain only into `GoalKind::TreatWounds`. `GoalKind::RaidTarget` remains flat `Medium` priority and uses loot-based motive only.
5. `goal_policy.rs` intentionally keeps `GoalKind::RaidTarget` unsuppressed (`SuppressionRule::Never`). That means the missing deterrence is not a policy toggle already waiting to be configured; the raid-specific substrate itself is absent.
6. `BanditFactionPolicy.flee_wound_threshold` exists in [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) and is already seeded in T22/S47/S48 test fixtures, but `rg -n "flee_wound_threshold" crates` shows no live AI consumer beyond storage/default wiring. The field is dead data today.
7. Candidate generation can already read courage through `GoalBeliefView::courage()`, but the current belief-view/runtime surface exposes no equivalent accessor for `flee_wound_threshold`. A small view extension is therefore part of the real implementation scope if the deterrence helper is to remain explicit rather than hard-coded.
8. The earlier ticket draft overfit the fix to candidate generation. That was incomplete. A candidate-only omission would hide the symptom at one AI surface while leaving ranking semantics unchanged. The corrected scope is a shared raid-deterrence helper consumed by both candidate generation and raid ranking so the contract stays explicit and diagnosable.
9. Reusing generic `danger_pressure` for wound-only raid deterrence would be the wrong abstraction. `ReduceDanger` and `derive_danger_pressure()` model immediate external hostility (`current_attackers`, `visible_hostiles`) plus wound escalation under active threat. Folding “too injured to initiate another raid” into that same signal would conflate proactive raid deterrence with reactive self-defense.
10. The live `GoalKind` under test is `RaidTarget`, and its current lawful operator surface remains the ordinary combat/loot chain proven in Scenario 47. Scenario 49 must not rewrite that claim into “wounds suppress all combat goals” or “wounds mean danger.”
11. Current focused coverage is missing for this exact contract. `cargo test -p worldwake-ai -- --list` shows existing candidate-generation bandit tests and Scenario 47/48 goldens, but no Scenario 49 test and no focused test for wound-driven raid suppression or raid-ranking deterrence.
12. Scenario isolation remains necessary. The intended invariant is “concrete injury eventually deters further raids despite continued prey availability.” Competing lawful branches include `TreatWounds`, `ReduceDanger`, harvesting, and generic self-consumption. The scenario should remove medicine/healing and immediate hostiles, keep limited food pressure, and avoid introducing unrelated local production that could dominate the branch under test.
13. The arithmetic in the earlier draft was directionally useful but not yet verified against the live combat outcomes. This ticket should keep the threshold math explicit while treating the exact wound cadence as something the focused/unit coverage and the final golden setup must validate against current combat survivability.
14. Adjacent contradiction exposed during reassessment: the live architecture has a faction policy field for flee-by-wounds but no consumer, and the AI view layer currently cannot read that field. Both are required consequences of this ticket, not separate cleanup.
15. Correction: this ticket should no longer claim a “~10-line guard clause” or a candidate-generation-only engine change. The correct scope is a small shared bandit raid-deterrence helper, the minimal belief-view/runtime accessor it needs, and focused/golden coverage around that helper.

## Architecture Check

1. The cleaner architecture is a single raid-deterrence helper that reads concrete wound load plus faction policy and agent courage, then is consumed wherever `RaidTarget` semantics are decided. That keeps the deterrence rule explicit, reusable, and testable without misusing generic danger or adding one-off numeric state.
2. A candidate-generation-only patch is weaker because it buries deterrence in one emitter while leaving raid ranking semantics unchanged. A generic danger rewrite is broader but wrong because it collapses “under attack” and “too injured to start another raid” into the same concept.
3. No backward-compatibility aliasing or shims are needed. `flee_wound_threshold` already exists; this ticket gives that field its first real consumer and keeps the old dead path from lingering as dead data.

## Verification Layers

1. Bandit wound load crosses the deterrence threshold -> focused unit/runtime coverage for the shared raid-deterrence helper or candidate-generation/ranking entry points.
2. `RaidTarget` candidate disappears once deterrence is active -> decision trace plus focused candidate-generation test.
3. Pre-threshold bandit still lawfully selects raids -> decision trace in Scenario 49 plus focused candidate-generation/ranking tests.
4. Raid commitment and wound accumulation happen through ordinary combat -> action trace plus authoritative `WoundList` state in Scenario 49.
5. The behavior is driven by concrete wound state rather than blocked-intent memory or cooldowns -> authoritative `WoundList` inspection plus absence of `BlockedIntentMemory` reliance in focused coverage.
6. Replay remains deterministic -> `hash_world()` and `hash_event_log()` equality in the replay golden.

## What to Change

### 1. Add a shared bandit raid-deterrence helper

Introduce a small helper in the AI layer that answers whether a bandit agent's current wound load is high enough to deter `RaidTarget`, using:

- concrete wound severity sum from `derive_pain_pressure()`
- faction-scoped `BanditFactionPolicy.flee_wound_threshold`
- agent-scoped courage from `UtilityProfile`

The helper should stay raid-specific. Do not rewrite generic danger or `ReduceDanger` around it.

### 2. Consume the helper at the raid AI surfaces

Use the shared helper in:

- `emit_raid_target_goals()` so deterrence is visible at candidate-generation time
- raid ranking so the deterrence rule is not hidden in generation-only behavior

If diagnostics can truthfully expose the omission reason without distorting existing trace enums, add that trace detail. If not, keep the proof at the strongest existing focused surface instead of adding noisy or misleading trace vocabulary.

### 3. Add focused tests first

Add focused tests that prove:

- high wound load + bandit faction policy + low courage suppresses `RaidTarget`
- sub-threshold wound load does not suppress `RaidTarget`
- raid ranking is neutral/non-zero below threshold and drops out at/above threshold

### 4. Add Scenario 49 golden coverage

Extend [`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs) with Scenario 49:

- a minimal topology for repeated prey arrival
- pre-threshold raid opportunities that still resolve through live `RaidTarget` -> `attack`
- authoritative wound-state transitions between phases so the golden proves the threshold boundary deterministically while focused tests carry the exact deterrence arithmetic
- assertions that raids happen before threshold crossing and stop after it
- deterministic replay coverage

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/pressure.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify)

## Out of Scope

- Reinterpreting `ReduceDanger` as a generic wound-deterrence goal
- Cross-crate redesign of combat, wounds, or bandit-camp components
- Adding wound recovery or healing mechanics
- Golden inventory refresh ticket work outside the touched scenario file

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral -- --exact`
2. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral_replays_deterministically -- --exact`
3. Focused candidate-generation/ranking tests covering the raid-deterrence helper
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

### Invariants

1. Wound-based raid deterrence is derived from concrete authoritative wound state plus existing faction/agent parameters, not cooldowns or abstract fatigue counters.
2. The deterrence rule remains raid-specific and does not redefine generic external danger.
3. Bandits below threshold still generate/select `RaidTarget` normally.
4. Deterministic replay produces identical world and event-log hashes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — prove wound-threshold deterrence suppresses raid emission only when the concrete threshold is crossed.
2. `crates/worldwake-ai/src/ranking.rs` — prove raid ranking remains live below threshold and drops out once deterrence is active.
3. `crates/worldwake-ai/src/pressure.rs` — prove the courage-scaled threshold arithmetic and threshold crossing boundary directly.
4. `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` — add Scenario 49 golden and replay coverage for pre-threshold raid execution plus post-threshold suppression under explicit authoritative wound state.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::bandit_raid_target_is_suppressed_by_wound_deterrence`
2. `cargo test -p worldwake-ai ranking::tests::raid_target_is_zero_motive_when_wound_deterrence_is_active`
3. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral -- --exact`
4. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral_replays_deterministically -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-30
- What changed:
  - Added a shared courage-scaled bandit wound-deterrence helper in `pressure.rs`.
  - Extended the AI belief/runtime surface so the helper can read `BanditFactionPolicy.flee_wound_threshold`.
  - Wired the deterrence rule into both `emit_raid_target_goals()` and raid ranking so the rule is not generation-only.
  - Added focused tests for threshold arithmetic, candidate suppression, and raid ranking dropout.
  - Added Scenario 49 golden + replay coverage in `golden_t22_bandit_camp_destruction.rs`.
- Deviations from original plan:
  - The final golden uses explicit authoritative wound-state transitions between phases rather than requiring every wound increment to arise from a stable retaliatory combat cadence in the golden itself.
  - The exact wound-threshold arithmetic and raid-surface contract are covered directly by focused `pressure.rs`, `candidate_generation.rs`, and `ranking.rs` tests instead of overloading the golden with brittle combat setup.
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation::tests::bandit_raid_target_is_suppressed_by_wound_deterrence -- --exact`
  - `cargo test -p worldwake-ai ranking::tests::raid_target_is_zero_motive_when_wound_deterrence_is_active -- --exact`
  - `cargo test -p worldwake-ai pressure::tests::bandit_raid_deterrence_triggers_only_when_pain_meets_threshold -- --exact`
  - `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral -- --exact`
  - `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
