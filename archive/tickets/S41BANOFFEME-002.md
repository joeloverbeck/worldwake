# S41BANOFFEME-002: Suite 1 — Pressure-Driven Raid Emergence (Scenario 47)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` `RaidTarget` ranking/satisfaction alignment plus golden coverage
**Deps**: `specs/S41-bandit-offensive-emergence-goldens.md`

## Problem

`GoalKind::RaidTarget` still has zero golden E2E coverage, and reassessment shows the live offensive raid path is architecturally inconsistent. Candidate generation already treats `RaidTarget` as a proactive bandit-only goal for co-located non-faction prey, but ranking and goal satisfaction still inherit defensive hostile/danger assumptions. Without correcting that shared contract first, a new golden would either ossify the wrong architecture or depend on scripted hostility injection that contradicts the intended emergent raid path.

## Assumption Reassessment (2026-03-30)

1. **Shared boundary under audit**: `GoalKind::RaidTarget { target }` spans three AI layers that must agree on the same contract: candidate emission in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), ranking/provenance in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), and planner satisfaction / attack binding in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
2. **Live goal surface**: `GoalKind::RaidTarget { target }` is live in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). `emit_raid_target_goals()` emits it when the actor belongs to a bandit faction, the target is a co-located living non-faction agent, and current danger pressure is below `danger.high()`. `local_raid_targets()` itself does not require hostility.
3. **Mismatch: ranking narrative was stale**. The old ticket claimed `RaidTarget` motive is `enterprise_weight`, but the live dispatch declaration still marks `RaidTarget` as `RankedGoalProvenanceFamily::Danger`, so `ranked_motive_score()` uses the defensive danger substrate whenever provenance is present. The focused test `raid_target_uses_danger_provenance_instead_of_enterprise_weight` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) confirms the current behavior.
4. **Mismatch: satisfaction/search narrative was also stale**. `GoalKind::RaidTarget` currently shares the same `is_satisfied()` branch as `EngageHostile`, so a co-located non-hostile prey target is treated as already satisfied unless it appears in `visible_hostiles_for(actor)`. Existing raid search tests in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) seed hostility just to make a colocated raid plan legal, which conflicts with candidate generation's proactive raid narrative.
5. **Authoritative combat law**: the `"attack"` action in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) requires a living co-located target but does not require a hostility relation. The hostility coupling is therefore AI architecture, not authoritative combat validation.
6. **Downstream loot path**: `GoalKind::LootCorpse { corpse }` remains the correct opportunistic follow-up. `"loot"` is still registered in the normal action registry and commits through the standard corpse/combat lifecycle.
7. **Scenario isolation**: the original Suite 1 sketch proposed scarce camp supplies plus a local harvest alternative, but the core invariant is narrower: the corrected scenario must prove proactive raid selection without hostility injection and without hiding the proof behind unrelated self-care loops.
8. **Coverage state**: `cargo test -p worldwake-ai -- --list` confirms `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` still contains only the two existing T22 goldens. `RaidTarget` has focused/unit coverage but still zero golden E2E coverage.
9. **Adjacent contradiction classification**: this is not a separate bug discovered incidentally. The ranking/satisfaction mismatch is a required consequence of the intended Suite 1 invariant, so the ticket cannot remain `Engine Changes: None`.
10. **Scope correction**: this ticket now owns the minimal architectural alignment required to make proactive raids lawful and testable. It does not own the separate Suite 3 wound-dampening gap already called out in [`specs/S41-bandit-offensive-emergence-goldens.md`](/home/joeloverbeck/projects/worldwake/specs/S41-bandit-offensive-emergence-goldens.md).

## Architecture Check

1. A clean raid architecture should treat `RaidTarget` as an offensive opportunity derived from local observed prey and expected loot, not as a disguised defensive hostile-response goal. Aligning ranking and satisfaction with candidate generation is more robust than preserving the current split-brain model.
2. The fix stays inside the existing `RaidTarget` abstraction instead of adding aliases, fallback goals, or bandit-specific attack shims. One canonical offensive path is easier to test, explain, and extend.
3. Golden coverage still belongs in [`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs), but the scenario should prove the corrected architecture rather than a ticket-local workaround.
4. No backwards-compatibility shims or duplicate raid paths will be introduced.

## Verification Layers

1. Proactive raid candidate emission for co-located non-faction prey -> decision trace `candidates.generated`
2. Raid ranking/satisfaction no longer require defensive hostility injection -> focused tests in `ranking.rs` and `goal_model.rs`
3. Goal selection prefers raid only after the traveler becomes a local loot opportunity -> decision trace `selection.selected_goal`
4. Raid resolves through ordinary combat -> action trace `ActionTraceKind::Committed` for `"attack"`
5. Post-combat looting remains a separate opportunistic goal -> action trace `ActionTraceKind::Committed` for `"loot"`
6. Commodity conservation across raid + loot -> authoritative world state via `verify_authoritative_conservation()`
7. No scripted pre-arrival raid -> decision trace and authoritative state before co-location
8. Deterministic replay -> `hash_world()` + `hash_event_log()`

## What to Change

### 1. Align the shared `RaidTarget` AI contract

- Update `RaidTarget` ranking so proactive raids are motivated by the expected local loot opportunity instead of the defensive danger provenance path.
- Update `RaidTarget` satisfaction / search assumptions so a co-located non-hostile prey target is still a live raid opportunity.
- Add or update focused tests in `ranking.rs` and `goal_model.rs` before the golden is added.

### 2. Add Suite 1 topology builder and setup

In [`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs), add a 3-place topology and `seed_s47_scenario(h: &mut GoldenHarness) -> S47Ids` that creates:

- 3 bandits at BanditCamp with `BanditFactionPolicy`, moderate `CombatProfile`, elevated hunger, non-zero `hunger_rate`, `PerceptionProfile`, and utility tuned so self-consume and raid can both compete lawfully
- Active `BanditCamp` component with a faction-owned supply container and faction-membership beliefs seeded so bandits do not raid one another
- 1 non-faction traveler at RoadJunction who travels into camp carrying Apple x4 and a weak `CombatProfile`

### 3. Add `run_s47_scenario(seed: Seed)` and two goldens

Linear tick loop:

1. Pre-arrival phase: verify no raid selection before co-location
2. Traveler arrives through ordinary travel and local perception has time to expose carried food as raidable loot
3. Post-arrival phase: accumulate flags for raid candidate emission, raid selection, attack commit, and loot commit
4. Assert all flags, assert conservation, return state hashes

Add:

- `golden_pressure_driven_raid_emergence`
- `golden_pressure_driven_raid_emergence_replays_deterministically`

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — remove defensive raid provenance coupling)
- `crates/worldwake-ai/src/ranking.rs` (modify — proactive raid motive + focused tests)
- `crates/worldwake-ai/src/goal_model.rs` (modify — align raid satisfaction/search contract + focused tests)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify — add Suite 1 tests)

## Out of Scope

- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`
- Reworking `EngageHostile`, `ReduceDanger`, or the Suite 3 wound-dampening architecture
- Suite 2 (`S41BANOFFEME-003`) and Suite 3 (`S41BANOFFEME-004`) goldens
- Golden inventory/doc regeneration from `S41BANOFFEME-005`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence -- --exact`
2. `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

### Invariants

1. `verify_authoritative_conservation()` holds after the raid-loot chain; the traveler's Apple x4 stays conserved.
2. No `RaidTarget` selection occurs before the traveler is co-located with bandits.
3. Attack and loot commit through the standard action lifecycle, not a bandit-only shortcut.
4. Deterministic replay produces identical world and event-log hashes.
5. The final implementation does not require pre-seeded hostility or current danger pressure to make a co-located raid target plan-searchable.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) — focused raid-ranking tests proving `RaidTarget` no longer depends on defensive danger provenance and instead scores from observed loot opportunity.
2. [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) — focused raid-goal tests proving a co-located non-hostile prey target is not treated as already satisfied.
3. [`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs) — `golden_pressure_driven_raid_emergence` proves proactive raid selection, combat execution, and post-combat looting in the full AI/runtime stack.
4. [`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs) — `golden_pressure_driven_raid_emergence_replays_deterministically` proves deterministic replay for Scenario 47.

### Commands

1. `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - aligned `RaidTarget` with its live offensive role by removing defensive danger provenance coupling in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs)
  - changed raid ranking in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) to score from observed loot opportunity on the target rather than danger pressure
  - changed raid satisfaction/search behavior in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) so a co-located non-hostile prey target remains a live raid opportunity
  - added focused regression tests for raid motive and raid satisfaction/search
  - added Scenario 47 goldens in [`crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs) proving proactive raid candidate emission/selection, ordinary attack, ordinary loot, and deterministic replay
- Deviations from original plan:
  - the final Scenario 47 intentionally dropped the local orchard / bread-scarcity branch because it introduced harvest/eat behavior that obscured the architectural invariant under test
  - conservation verification was narrowed to the traveler's carried Apple x4 through `verify_authoritative_conservation()` instead of the original mixed Apple-plus-Bread live-lot story
- Verification results:
  - `cargo test -p worldwake-ai raid_target_scores_from_known_loot_opportunity`
  - `cargo test -p worldwake-ai raid_goal_is_not_already_satisfied_for_colocated_non_hostile_prey`
  - `cargo test -p worldwake-ai search_raid_goal_uses_colocated_attack_affordance`
  - `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence -- --exact`
  - `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
