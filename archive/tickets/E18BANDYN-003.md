# E18BANDYN-003: Reassess proposed Raid action definition against live combat architecture

**Status**: NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E18BANDYN-001 (BanditCamp component), E18BANDYN-002 (GoalKind variants), E12 (combat system — completed)

## Problem

This ticket originally proposed adding a distinct authoritative `raid` action, payload variant, action definition, and handler. Reassessment against the live codebase shows that proposal would duplicate an already-canonical combat action path and would make the combat architecture less clean, less robust, and less extensible.

The live architecture already separates:

1. **AI motive / intent identity**: `GoalKind::RaidTarget { target }`
2. **Planner operator surface**: `PlannerOpKind::Attack`
3. **Authoritative combat execution**: the existing `attack` action using `ActionPayload::Combat(CombatActionPayload)`

That split is the cleaner long-term design. Bandit-specific semantics belong at the goal/candidate/ranking layer unless the authoritative combat rules truly diverge. Today they do not.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit:
`worldwake_core::GoalKind` -> `worldwake_ai` planner dispatch / candidate generation -> `worldwake_systems::combat` authoritative `attack` action.

Corrected findings:

1. The ticket's core premise is stale. AI does **not** need a distinct authoritative action in order to distinguish raids.
   Evidence:
   - [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) already defines `GoalKind::RaidTarget { target }`.
   - [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) already gives `RaidTarget` its own dispatch declaration and trace label.
   - [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) already assigns `RaidTarget` a distinct discriminant/order from `EngageHostile`.

2. The live planner intentionally maps both `EngageHostile` and `RaidTarget` onto the same combat operator.
   Evidence:
   - [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) sets both goal families' relevant ops to `PlannerOpKind::Attack`.
   - [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) builds the same `ActionPayload::Combat(CombatActionPayload { .. })` override for both goal kinds and tests that both bind to `PlannerOpKind::Attack`.

3. `ActionPayload` does **not** follow a one-payload-struct-per-semantic-goal pattern for this case. The canonical authoritative combat payload is already `CombatActionPayload`.
   Evidence:
   - [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) exposes `ActionPayload::Combat(CombatActionPayload)` and no `RaidActionPayload`.

4. The authoritative combat execution path is already centralized and reusable.
   Evidence:
   - [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) defines the single `attack` action def, payload enumeration, validation, start/tick/commit/abort handling, and wound/death resolution.

5. The proposed distinct `raid` action would create a second lawful transport path for the same authoritative fact: "agent initiated standard combat against a colocated target."
   Canonical path after reassessment:
   - Keep the single authoritative path as `attack` + `ActionPayload::Combat`.
   Duplicate path handling:
   - Do **not** add a duplicate `raid` action in scope.

6. The actual live gap is elsewhere: bandit-specific candidate generation still appears to emit only `EngageHostile` in the current candidate-generation surface.
   Evidence:
   - [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) currently emits `GoalKind::EngageHostile { target }` in combat candidate generation, with no `RaidTarget` emission path found during reassessment.
   Scope consequence:
   - That is an AI-candidate-generation concern and belongs with E18BANDYN-006, not here.

7. Several original assumptions in this ticket were therefore wrong or unnecessary:
   - No new `RaidActionPayload` is warranted.
   - No new `raid` action def or handler is warranted.
   - No new action-registry wiring is warranted.
   - No authoritative precondition surface for `raid` should be added while combat rules remain identical to `attack`.

## Architecture Check

Keeping one authoritative combat action is cleaner than introducing `raid` as a second action because:

1. It preserves a single source of truth for combat start-gate validation, payload validation, lifecycle handling, wound resolution, death handling, visibility, and action traces.
2. It keeps bandit-specific semantics where they belong: the AI's goal identity, ranking, and candidate-generation layer.
3. It avoids duplicated combat preconditions and duplicated handler registration that would inevitably drift.
4. It keeps traces and debugging simpler. When combat behavior changes, there remains one authoritative action surface to verify end-to-end.
5. It matches the existing architecture already present in `GoalKind`, dispatch, ranking, and goal-model binding.

Alternatives considered:

1. Add a distinct `raid` action anyway.
   Result:
   - Rejected. It duplicates authoritative combat without any distinct mechanical rule.

2. Add a flag or sub-kind inside `CombatActionPayload`.
   Result:
   - Rejected for now. There is no authoritative rule branch that needs such a flag.

3. Keep one `attack` action and fix the AI layer to emit `RaidTarget` where appropriate.
   Result:
   - Recommended. This preserves a clean separation between motive semantics and authoritative execution.

## Verification Layers

1. Distinct raid intent already exists at the goal layer -> focused unit coverage in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) for `RaidTarget` binding to `PlannerOpKind::Attack`
2. Single authoritative combat execution path exists -> focused unit coverage in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) for `attack` lifecycle and registration
3. Planner/catalog surface remains canonical -> focused registry coverage in [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)
4. Remaining missing bandit behavior is candidate-generation, not authoritative combat -> follow-up implementation should prove `RaidTarget` presence/absence through focused candidate-generation coverage and decision traces in E18BANDYN-006

## Corrected Scope

This ticket is corrected from "implement a distinct `raid` action" to:

1. Document that the proposed authoritative `raid` action is architecturally inferior to the current single-action combat design.
2. Confirm that `RaidTarget` already exists as the correct AI-level semantic distinction.
3. Defer the real missing behavior to candidate generation / bandit AI follow-up work rather than adding a duplicate combat action.

## What Not To Change

Do not:

1. Add `RaidActionPayload`
2. Add `ActionPayload::Raid`
3. Add a `raid` `ActionDef`
4. Add a duplicate combat handler or duplicate registry entry
5. Add any backwards-compatibility alias between `attack` and `raid`

## Files Reviewed

- [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs)
- [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs)
- [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
- [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs)
- [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs)
- [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)
- [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md)

## Out of Scope

1. Emitting `RaidTarget` from candidate generation
2. Regroup candidate generation
3. Planner changes for new goal families
4. Any authoritative combat rule changes
5. Any duplicate combat action introduction

## Acceptance Criteria

1. Ticket accurately reflects the live architecture and removes the stale requirement to add a second combat action.
2. Ticket clearly names the canonical shared boundary: `RaidTarget` goal semantics over the existing `attack` authoritative action.
3. Ticket records the real remaining gap as follow-up AI work rather than forcing an inferior combat duplication.

## Tests

### New/Modified Tests

None. Reassessment only; no production code change is warranted in this ticket.

### Verification Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-systems -- --list`
3. `cargo test -p worldwake-ai goal_model::tests::raid_target_match`
4. `cargo test -p worldwake-systems combat::tests::register_attack_action_creates_public_combat_definition`

## Outcome

- Date: 2026-03-29
- What actually changed:
  - Reassessed the ticket against live code and corrected the scope from "implement raid as a distinct action" to "do not implement a duplicate combat action."
- Deviations from original plan:
  - The originally proposed implementation was rejected as architecturally redundant. The live codebase already separates bandit semantics at the goal layer while sharing the canonical combat action.
- Verification results:
  - Test inventories for `worldwake-ai` and `worldwake-systems` were enumerated successfully.
  - Focused follow-up verification is expected on the existing `RaidTarget` goal binding and `attack` action surfaces.
