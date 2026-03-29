# E18BANDYN-006: AI candidate generation for raid and regroup goals

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (candidate_generation.rs)
**Deps**: E18BANDYN-002 (GoalKind variants), E18BANDYN-003 (Raid action def), archive/tickets/completed/E18BANDYN-004.md, E18BANDYN-010

## Problem

Bandit agents need to generate two new goal candidates through the existing AI candidate generation system: (1) `RaidTarget` when non-faction agents are present at the bandit's location, and (2) `RegroupWithFaction` when their camp is destroyed and they hold a rally-point belief. These candidates must integrate with the existing pressure-based ranking without special-case code.

## Assumption Reassessment (2026-03-29)

1. `generate_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` takes a `GoalBeliefView` trait object, `BlockedIntentMemory`, `RecipeRegistry`, and current tick. It returns `Vec<GroundedGoal>`. The function dispatches to internal helpers based on agent state queries through the belief view.
2. `GoalBeliefView` trait (in `crates/worldwake-sim/src/belief_view.rs`) provides query methods for agent beliefs. The raid candidate generator needs: (a) agent's faction membership, (b) non-faction agents at same place, (c) active `BanditCamp` existence for agent's faction. The regroup generator needs: (a) the agent's rally-point belief, (b) whether a camp currently exists for that faction. It should not depend on a place-backed `BanditCampProfile`; canonical authoritative policy comes from `BanditFactionPolicy` after `E18BANDYN-010`.
3. The spec's "enterprise signal pattern for raids" is analogous to merchant restock signals in `crates/worldwake-ai/src/enterprise.rs`. The raid opportunity assessment uses the same infrastructure.
4. `BlockedIntentMemory` with `BlockingFact::CombatTooRisky` suppresses re-engaging at locations where the agent previously failed. This naturally limits raid retries.
5. `GroundedGoal` includes the `GoalKind`, a priority/motive score, and suppression conditions. `RegroupWithFaction` has suppression: `WhenStressedAtOrAbove(Critical)` — survival comes first.
6. The belief-view implementation detail in this ticket is stale: there is no standalone `omniscient_belief_view.rs` in live code. Any new `GoalBeliefView` surface must be wired through the live belief-view implementations and helpers that exist after reassessment.
7. Adjacent contradiction exposed during reassessment: regroup policy should come from `BanditFactionPolicy` in `E18BANDYN-010`, while regroup navigation still comes from the agent's own beliefs. This ticket must consume that canonical split instead of reintroducing place-backed policy reads.

## Architecture Check

1. Extending the existing `generate_candidates()` function with two new candidate-generating branches follows the established pattern (e.g., `generate_combat_candidates()`, `generate_trade_candidates()`). Each candidate type gets its own helper function called from the main generator. This is cleaner than adding candidates through a separate system because all candidates compete through the same ranking pipeline.
2. Rally-point awareness comes from agent beliefs (FND-7, FND-12): agents who never observed the rally point do NOT generate `RegroupWithFaction` candidates. The candidate generator checks the agent's belief about rally points, not direct authoritative reads from the faction policy component.
3. No backwards-compatibility shims. New helper functions, new candidate types, additive changes to the generation pipeline.

## Verification Layers

1. Raid candidate generated when conditions met → decision trace: `RaidTarget` appears in `candidates.generated`
2. Raid candidate NOT generated when target is in same faction → decision trace: no `RaidTarget` for same-faction agents
3. Raid candidate suppressed by `CombatTooRisky` blocked intent → decision trace: `RaidTarget` absent or filtered
4. Regroup candidate generated when camp destroyed + rally belief held → decision trace: `RegroupWithFaction` appears
5. Regroup candidate NOT generated when agent lacks rally-point belief → decision trace: no `RegroupWithFaction`
6. Regroup candidate NOT generated when camp still exists → decision trace: no `RegroupWithFaction`
7. Regroup suppressed at Critical stress → decision trace: `RegroupWithFaction` suppressed
8. Dead agents generate no candidates → structural: `DeadAt` check in generation pipeline (existing)

## What to Change

### 1. Extend GoalBeliefView trait (if needed)

In `crates/worldwake-sim/src/belief_view.rs`, add methods to query:
- `agent_faction_membership(agent) -> Option<EntityId>` — which faction the agent belongs to
- `non_faction_agents_at_place(agent, faction) -> Vec<EntityId>` — potential raid targets
- `agent_believes_camp_exists(agent, faction) -> bool` — whether agent believes their faction has a camp
- `agent_rally_point_belief(agent, faction) -> Option<EntityId>` — agent's believed rally place

Implement these on the live `GoalBeliefView` surfaces that currently back candidate generation, not on a nonexistent omniscient-belief-view file.

### 2. Add raid candidate generation helper

In `crates/worldwake-ai/src/candidate_generation.rs`:

```rust
fn generate_raid_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> Vec<GroundedGoal> {
    // 1. Check agent has MemberOf to a faction with BanditCamp
    // 2. Find non-faction agents at same place
    // 3. For each potential target:
    //    a. Skip if CombatTooRisky blocked intent for this target
    //    b. Assess opportunity (cargo value heuristic) vs danger (guard presence)
    //    c. Generate RaidTarget { target } goal with priority/motive
}
```

### 3. Add regroup candidate generation helper

```rust
fn generate_regroup_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> Vec<GroundedGoal> {
    // 1. Check agent has MemberOf to a faction
    // 2. Check agent believes no BanditCamp exists for that faction
    // 3. Check agent holds a rally-point belief for that faction
    // 4. Check agent is NOT already at the rally place
    // 5. Generate RegroupWithFaction { faction } goal
    //    Priority: below ReduceDanger/ConsumeCommodity, above enterprise
    //    Suppression: WhenStressedAtOrAbove(Critical)
}
```

### 4. Wire helpers into main generate_candidates()

Call both helpers from the main candidate generation function, appending results to the candidate list.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add raid + regroup candidate helpers, wire into main function)
- `crates/worldwake-sim/src/belief_view.rs` (modify — extend trait with faction/camp query methods)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — if new belief-view methods are required on the live per-agent surface)
- `crates/worldwake-ai/src/enterprise.rs` (modify — add raid opportunity signal if following enterprise pattern)

## Out of Scope

- Planner search integration for new goal kinds (E18BANDYN-007)
- Route threat estimation affecting route selection (E18BANDYN-008)
- Raid action definition and handler (E18BANDYN-003 — must be complete before this ticket)
- EstablishCamp action (E18BANDYN-004)
- bandit_camp_system (E18BANDYN-005)
- Golden test T22 (E18BANDYN-009)
- Full belief-system redesign beyond the live `GoalBeliefView` surfaces

## Acceptance Criteria

### Tests That Must Pass

1. Bandit at location with non-faction travelers generates `RaidTarget` candidate
2. Bandit at location with only faction members generates no `RaidTarget`
3. Bandit with `CombatTooRisky` for a target does not generate `RaidTarget` for that target
4. Agent with destroyed camp + rally-point belief generates `RegroupWithFaction`
5. Agent with destroyed camp but NO rally-point belief does NOT generate `RegroupWithFaction`
6. Agent with existing camp does NOT generate `RegroupWithFaction`
7. Agent already at rally place does NOT generate `RegroupWithFaction`
8. Dead agent generates no candidates (existing invariant preserved)
9. `RegroupWithFaction` is suppressed when stress is Critical
10. Existing suite: `cargo test -p worldwake-ai`
11. Existing suite: `cargo clippy --workspace`

### Invariants

1. FND-7 (Locality): rally-point knowledge comes from agent beliefs, not authoritative state queries
2. FND-12 (Belief != State): agents who never observed rally point generate no regroup goal
3. FND-17 (Agent Symmetry): candidates use the same generation pipeline as all other goals
4. No global queries: candidate generation accesses only the agent's belief view
5. Blocked intents suppress repeated failed raids (existing mechanism)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for raid candidate generation under various conditions
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for regroup candidate generation under various conditions
3. `crates/worldwake-sim/src/per_agent_belief_view.rs` — tests for any new live belief-view method implementations

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
