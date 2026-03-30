# S41BANOFFEME-001: Reassess S41 Spec Against Current Codebase

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — this ticket corrects the spec and documents future architecture gaps
**Deps**: S41 spec (`specs/S41-bandit-offensive-emergence-goldens.md`)

## Problem

S41 currently describes three bandit-offense golden suites as if the live code already supports supply-driven raid selection, combat-belief rerouting, and wound-driven raid dampening. Before any test implementation, the spec must match the actual AI, planner, and existing proof surfaces.

## Assumption Reassessment (2026-03-30)

1. The exact shared abstraction boundary under audit is the offensive bandit chain across three live surfaces:
   - `GoalKind::RaidTarget` emission and ranking in `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/goal_dispatch_decl.rs`, and `crates/worldwake-ai/src/ranking.rs`
   - `GoalKind::ShareBelief` / `TellTopic::EntityBelief` transport in `crates/worldwake-ai/src/candidate_generation.rs` plus `crates/worldwake-systems/src/tell_actions.rs`
   - route-danger planning cost in `crates/worldwake-ai/src/route_threat.rs` and `crates/worldwake-ai/src/planning_snapshot.rs`
2. Scenario IDs `47`, `48`, and `49` are still free. Current golden coverage tops out at Scenario `46`; `cargo test -p worldwake-ai -- --list` confirms the existing inventory.
3. The existing T22 bandit golden file and helpers are real and reusable: `build_custom_harness()`, `connect()`, `bandit_profile()`, `default_perception_profile()`, and `set_control_source()` live in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`; `seed_agent_with_recipes()` and `stable_wound_list()` live in `crates/worldwake-ai/tests/golden_harness/mod.rs`.

### Suite 1 (Scenario 47): Pressure-Driven Raid Emergence

4. `GoalKind::RaidTarget { target }` is live in `crates/worldwake-core/src/goal.rs`, and `emit_raid_target_goals()` emits it from `local_raid_targets()` for co-located living non-faction agents. Focused coverage already exists in `bandit_with_local_non_faction_agent_emits_raid_target_instead_of_engage_hostile`.
5. The old ticket assumption that Suite 1 only needed non-zero `enterprise_weight` is stale. `RaidTarget` now belongs to `RankedGoalProvenanceFamily::Danger` via `DECL_RAID_TARGET` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`, and `raid_target_uses_danger_provenance_instead_of_enterprise_weight` in `crates/worldwake-ai/src/ranking.rs` proves the live ranking path uses `danger_pressure`, not `enterprise_weight`.
6. Correction: under current code, a co-located non-faction target is not enough to make `RaidTarget` competitive. The suite needs a lawful hostility/danger surface for the raider, because the live motive/priority for `RaidTarget` are danger-provenanced. If S41 is meant to prove scarcity- or loot-driven offensive raiding, that is a future architecture gap, not a setup tweak.
7. This diverges from the archived E18 design intent in `archive/specs/E18-bandit-dynamics.md`, which describes raids as an enterprise/opportunity path with low danger increasing raid attractiveness. The current code instead treats `RaidTarget` as part of the combat-danger family. That broader architectural contradiction is not fixed in this ticket, but the S41 spec must stop claiming hunger alone currently selects raids.

### Suite 2 (Scenario 48): Raid-Belief Economic Cascade

8. The planner route-danger substrate already exists and is not speculative. `PlanningSnapshot` builds `perceived_travel_costs` from `perceived_direct_travel_cost_from_memory()`, and `route_threat_estimate_from_memory()` explicitly reads `BelievedActivity { action_domain: Combat }`, witnessed conflict observations, and wounds in `crates/worldwake-ai/src/route_threat.rs`.
9. Existing proof already reaches part of Suite 2's chain:
   - `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` proves fresh danger beliefs reroute travelers away from `BANDIT_ROAD`
   - route-threat focused tests prove combat beliefs and social conflict observations alter perceived travel cost
10. The social transport substrate also already exists. `emit_social_candidates()` relays generic `known_entity_beliefs()` as `GoalKind::ShareBelief` when the listener cannot directly observe the subject, and Scenario 40 in `crates/worldwake-ai/tests/golden_emergent.rs` proves `TellTopic::EntityBelief` propagation unlocks downstream behavior. Scenario 46 proves Tell-gated downstream planning for a different domain.
11. Correction: Suite 2 is still valuable, but it is not verifying an unknown substrate. Its real job is to prove the combat-specific end-to-end chain raid -> witness combat belief -> Tell transport -> merchant reroute in one golden scenario.

### Suite 3 (Scenario 49): Wound-Dampened Raid Spiral

12. The live spec narrative is incorrect. `derive_danger_pressure()` returns zero when the agent has no current attackers and no visible hostiles, so wounds alone do not raise `ReduceDanger` or suppress raids.
13. `derive_pain_pressure()` only feeds `TreatWounds` ranking. `RaidTarget` is never stress-suppressed in `goal_policy.rs`, and the current `emit_raid_target_goals()` guard checks only `danger_pressure >= thresholds.danger.high()`.
14. Correction: the current engine does not produce wound-only raid dampening after a victorious combat once hostiles disappear. Suite 3 is therefore blocked on an engine change.
15. The old ticket recommendation to add a bandit-only wound check directly inside `emit_raid_target_goals()` is not the cleanest long-term architecture. `RaidTarget` already shares the danger provenance family with `EngageHostile` and `ReduceDanger`; adding a one-off emitter guard would duplicate combat-deterrence logic at only one surface. The cleaner future fix is to extend the canonical combat-risk / raid-motive architecture in one place, then let candidate generation, ranking, and interrupt behavior consume that single substrate.

### Cross-Suite Scope Corrections

16. S41 currently overclaims two behaviors relative to live code:
   - Suite 1 says hunger/supply pressure selects raids, but live ranking requires danger/hostility provenance.
   - Suite 3 says wounds alone dampen raids, but live danger pressure ignores wounds without active hostiles.
17. These are required corrections for this ticket, not optional future cleanup. The spec must explicitly distinguish already-proven substrate from still-missing architecture instead of silently assuming both are live.

## Architecture Check

1. Correcting the spec to name the real live substrate is cleaner than papering over mismatches with setup folklore like "`enterprise_weight` fixes raids." That would produce brittle tests anchored to a path the current ranking code no longer uses.
2. For future implementation work, the durable architecture is:
   - either restore `RaidTarget` as a true proactive predation/enterprise goal grounded in concrete scarcity and loot opportunity
   - or formalize a single combat-deterrence substrate that all raid/combat goals use
   What is not robust is splitting the logic between danger provenance, ad-hoc bandit-only candidate guards, and ticket-local arithmetic claims.
3. No backwards-compatibility aliases or shadow paths are introduced here. This ticket only removes false assumptions from the spec.

## Verification Layers

1. `RaidTarget` emission for co-located non-faction agents -> focused candidate-generation unit test
2. Live `RaidTarget` ranking substrate -> focused ranking unit test proving danger provenance
3. Route danger affecting travel planning -> focused route-threat tests plus T22 golden route-selection proof
4. `ShareBelief` / Tell transport for entity beliefs -> existing golden Scenario 40 and Scenario 46 proof surfaces
5. Wound-only raid dampening absence -> focused pressure/ranking/policy code review against live symbols
6. Additional runtime/golden proof is not applicable for this ticket because the work product is spec correction, not behavior change

## What to Change

### 1. Correct Suite 1 in S41

- Remove the stale `enterprise_weight` assumption.
- State that the current code requires a lawful hostility/danger surface for `RaidTarget` to rank non-zero.
- Stop claiming the live engine already proves hunger-driven raid selection.

### 2. Correct Suite 2 in S41

- Replace "needs verification" language for route danger and generic `ShareBelief` transport with the actual live proof surfaces.
- Keep Suite 2 scoped as a combat-specific end-to-end golden, not as substrate discovery.

### 3. Correct Suite 3 in S41

- Replace the false wound-only dampening narrative with an explicit current engine gap.
- Remove the candidate-generation-only fix recommendation from the spec.
- Record that the future fix should preserve one canonical raid/combat deterrence substrate.

## Files to Touch

- `tickets/S41BANOFFEME-001.md` (modify)
- `specs/S41-bandit-offensive-emergence-goldens.md` (modify)

## Out of Scope

- Implementing any S41 golden test code
- Changing `RaidTarget` ranking or candidate generation
- Adding wound-aware raid suppression
- Updating archived E18 design material

## Acceptance Criteria

### Tests That Must Pass

1. Focused reassessment commands proving the cited live raid, social, and route-threat surfaces pass.
2. Existing suite: `cargo test -p worldwake-ai`
3. Existing lint: `cargo clippy --workspace`

### Invariants

1. The corrected S41 spec does not claim a live behavior without a corresponding current code path or an explicit engine-gap note.
2. The ticket does not recommend a backwards-compatibility alias or ticket-local workaround as the canonical future architecture.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai bandit_with_local_non_faction_agent_emits_raid_target_instead_of_engage_hostile`
2. `cargo test -p worldwake-ai raid_target_uses_danger_provenance_instead_of_enterprise_weight`
3. `cargo test -p worldwake-ai golden_t22_bandit_camp_destruction`
4. `cargo test -p worldwake-ai golden_supply_depletion_enables_share_belief`
5. `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-30
- What actually changed:
  - Rewrote this ticket's reassessment to match the live `RaidTarget`, `ShareBelief`, and route-threat code paths.
  - Corrected `specs/S41-bandit-offensive-emergence-goldens.md` so Suite 1 no longer claims `enterprise_weight` or hunger already drive raid selection, Suite 2 cites the existing route-danger / Tell substrate instead of treating it as unknown, and Suite 3 is documented as a real engine gap rather than a live ranking behavior.
  - Recorded the architectural recommendation that future raid-motive and wound-dampening work should use one canonical combat/raid substrate instead of a bandit-only candidate-generation patch.
- Deviations from original plan:
  - The original ticket assumed only Suite 3 had a real engine divergence. Reassessment showed Suite 1 also overclaimed the live architecture because `RaidTarget` now ranks through danger provenance rather than `enterprise_weight` or hunger.
  - The original recommendation to patch `emit_raid_target_goals()` directly was intentionally removed from the corrected spec because it would duplicate combat-deterrence logic instead of extending a canonical shared substrate.
- Verification results:
  - Focused proof commands passed for `RaidTarget` emission/ranking, T22 route rerouting, and existing entity-belief Tell goldens.
  - `cargo test -p worldwake-ai` passed.
  - `cargo clippy --workspace` passed.
