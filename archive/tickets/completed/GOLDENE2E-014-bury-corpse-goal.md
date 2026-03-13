# GOLDE2E-014: BuryCorpse Goal

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Yes — shipped as concrete corpse-action architecture with planner/action support, grave-plot facilities, and containment-based burial inaccessibility
**Deps**: Death cascade proven in scenario 8

## Problem

`GoalKind::BuryCorpse` is listed in the goal enum and AI ranking metadata, but the current codebase stops there. There is no `BuryCorpse` candidate generation, no planner op support, no registered bury action, and no burial-site mechanic in the world model. The original ticket assumed those layers might already exist; they do not.

The implementation should therefore prove a clean, concrete burial architecture rather than only adding a golden test around existing behavior.

## Report Reference

Backlog item **P16** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 1).

## Assumption Reassessment (2026-03-13)

1. `GoalKind::BuryCorpse` exists in `crates/worldwake-core/src/goal.rs`.
2. `BuryCorpse` currently participates in AI goal bookkeeping/ranking only:
   - `crates/worldwake-ai/src/goal_model.rs` tags it but assigns `NO_OPS`
   - `crates/worldwake-ai/src/search.rs` explicitly treats it as unsupported
   - `crates/worldwake-ai/src/ranking.rs` ranks it as a low-priority goal
3. There is no `bury` action definition or handler in the action registry today.
4. There is no burial-site tag/place/facility in the current world model today.
5. There is no existing “community duty” or corpse-burial social motive system to reuse for this ticket.

## Architecture Check

1. Burial should follow the same real-AI loop as other local interaction goals: local corpse detection + local burial-site detection -> goal generation -> planner step -> action execution.
2. Burial-site support should be concrete world state. The cleanest fit is a grave-plot facility marker, not hidden controller logic.
3. “Buried and inaccessible” should emerge from the existing placement/access architecture, not from a special boolean that bypasses containment rules.
4. If burial and looting both exist for the same corpse, looting remaining possessions before burial is acceptable under the current low-priority ranking model.

## Engine-First Mandate

Do NOT add a burial-only hack such as a hidden “buried” flag that bypasses the existing world access model. The implementation should make corpse burial a first-class action with concrete site requirements and concrete post-burial state.

## What to Change

### 1. Implement missing bury infrastructure

- `BuryCorpse` candidate generation in `crates/worldwake-ai/src/candidate_generation.rs`
- planner-op support for `BuryCorpse` in the AI search/goal model layers
- `bury` action definition and registration
- bury action handler in `crates/worldwake-systems`
- a concrete burial-site mechanic in world state
- concrete post-burial inaccessibility using existing placement/containment rules

### 2. New golden test in `golden_combat.rs`

**Setup**: A corpse exists at a location with a burial site. A living agent is co-located. Do not introduce a new social-duty subsystem for this ticket; rely on the concrete local corpse + burial-site conditions that the current AI can observe.

**Assertions**:
- Agent generates `BuryCorpse` goal from the local corpse + burial site.
- Agent executes the bury action through the real AI loop.
- Corpse becomes inaccessible to later loot because it is no longer directly targetable through the normal corpse-access path.
- Conservation holds.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/planner_ops.rs`
- `crates/worldwake-ai/src/search.rs`
- `crates/worldwake-systems/src/`
- `crates/worldwake-systems/src/action_registry.rs`
- Core/sim files as needed for burial-site or containment support

## Out of Scope

- Mass burial or burial ceremonies
- Burial-site capacity limits
- Corpse decay mechanics
- A new social-duty / kinship / institutional burial motivation system

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bury_corpse` — agent buries a corpse at a burial site through the real AI loop
2. Focused unit/integration coverage for the new action and corpse inaccessibility path
3. Existing suite: `cargo test -p worldwake-ai golden_`
4. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Buried corpse is inaccessible to the normal loot path without introducing hidden special cases
3. Conservation holds (burial does not destroy the corpse or its possessions)
4. GoalKind coverage increases: `BuryCorpse` → Yes

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update GoalKind coverage: `BuryCorpse` → Yes
- Remove P16 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_bury_corpse` — proves corpse burial through the real AI loop
2. `crates/worldwake-ai/src/candidate_generation.rs::local_corpse_with_grave_plot_emits_bury_goal` — proves the goal is emitted from concrete local evidence
3. `crates/worldwake-ai/src/goal_model.rs::bury_goal_uses_bury_op_family` — proves the goal now maps to a real planner op family
4. `crates/worldwake-ai/src/planner_ops.rs::build_semantics_table_classifies_all_registered_phase_two_defs` — now covers `bury` op classification
5. `crates/worldwake-sim/src/omniscient_belief_view.rs::corpse_entities_at_excludes_contained_corpses` — proves contained corpses drop out of the normal corpse-access path
6. `crates/worldwake-systems/src/combat.rs::register_bury_action_creates_public_corpse_definition` — proves the action is registered as a real corpse action
7. `crates/worldwake-systems/src/combat.rs::bury_moves_corpse_into_grave_container_and_blocks_loot_affordance` — proves burial uses containment and blocks later loot affordances

### Commands

1. `cargo test -p worldwake-ai golden_bury_corpse`
2. `cargo test -p worldwake-systems combat`
3. `cargo test -p worldwake-sim corpse_entities_at`
4. `cargo test -p worldwake-ai golden_`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

**Completion Date**: 2026-03-13

**What Actually Changed**

- Replaced the placeholder-only `BuryCorpse` path with real AI/planner support, a registered `bury` action, and a concrete `GravePlot` facility marker.
- Renamed the one-off `Loot` action domain to `Corpse` so `loot` and `bury` share a coherent architectural home.
- Implemented burial as containment: the action creates a grave container and moves the corpse into it, so inaccessibility emerges from the existing placement/access model instead of a burial-only flag.
- Tightened corpse visibility/access so contained corpses no longer appear in the normal corpse-affordance path.
- Added the golden burial scenario plus focused unit/integration coverage, and updated `reports/golden-e2e-coverage-analysis.md` to remove backlog item `P16`.

**Deviations From Original Plan**

- Did not add a new social-duty / kinship motivation layer. The shipped scope uses concrete local corpse + grave-site evidence only.
- Did not add a burial-specific place tag or prototype place. A grave-plot facility was the smaller and cleaner fit for the current architecture.
- Did not add a burial flag. Containment already provided the correct concrete access semantics.

**Verification Results**

- `cargo test -p worldwake-ai golden_bury_corpse`
- `cargo test -p worldwake-systems combat`
- `cargo test -p worldwake-sim corpse_entities_at`
- `cargo test -p worldwake-ai golden_`
- `cargo test --workspace`
- `cargo clippy --workspace`
