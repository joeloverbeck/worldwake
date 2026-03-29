# E18BANDYN-011: Add lawful rally-point belief substrate for `RegroupWithFaction`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief/perception substrate plus AI candidate consumer
**Deps**: [archive/tickets/completed/E18BANDYN-010.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E18BANDYN-010.md), `E18BANDYN-005`, `E18BANDYN-006`

## Problem

`RegroupWithFaction` exists as a goal kind, but the live code still has no lawful way for an agent to hold, retain, and consult the specific belief "my faction's rally point is place X." The spec requires that regrouping depend on belief acquisition at an active camp, not on omniscient reads from authoritative faction policy.

Without that missing information path, any regroup implementation would either:

- read `BanditFactionPolicy.rally_place` directly during candidate generation, or
- invent a shadow alias path that bypasses normal belief transport.

Both would violate the repo's architectural rules.

## Assumption Reassessment (2026-03-29)

1. The exact boundary under audit is rally-point information transport: authoritative faction policy in [`BanditFactionPolicy`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) -> lawful local observation/perception -> stored agent belief -> `GoalBeliefView` query consumed by regroup candidate generation.
2. The live `AgentBeliefStore` / `BelievedEntityState` model in [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) has no dedicated field or carrier for faction rally-point knowledge today.
3. The live `GoalBeliefView` in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) likewise has no rally-point belief query today.
4. `factions_of(entity)` and institutional belief storage already exist, so the likely clean implementation is to reuse institutional-belief transport rather than add a one-off omniscient AI helper. The specific carrier still needs reassessment before code.
5. The implementation must align with `docs/FOUNDATIONS.md`:
   - locality of information
   - world state != belief state
   - records/beliefs as world state
   - no alias path for one fact
6. This ticket must explicitly avoid direct candidate-generation reads from authoritative faction policy as a substitute for belief. That shortcut would make regroup "work," but it would make the architecture worse.

## Architecture Check

1. The cleaner architecture is to add one lawful rally-point belief path and consume it everywhere, rather than letting AI read faction policy directly.
2. The ideal end state is:
   - `BanditFactionPolicy.rally_place` remains the authoritative source of doctrine on the faction
   - co-located bandit members can lawfully acquire a belief about that doctrine through perception/observation
   - `RegroupWithFaction` candidate generation depends only on the agent-held belief
3. No backwards-compatibility aliasing. Once the lawful belief path exists, regroup should consume that path directly rather than keep any temporary direct-policy read.

## Verification Layers

1. rally-point belief can be lawfully acquired while co-located with an active camp -> focused perception/belief test
2. agents who never lawfully acquire the belief do not gain it -> focused belief-store/perception test
3. `GoalBeliefView` can read the stored rally-point belief without direct authoritative faction-policy access -> focused belief-view test
4. regroup candidate generation consumes the belief path, not the faction-policy component -> focused AI candidate-generation/runtime test in the follow-up implementation ticket
5. golden regroup assertions remain deferred until the full bandit chain ticket

## What to Change

### 1. Choose and implement the canonical rally-point belief carrier

Use an existing lawful belief transport if possible; otherwise add the smallest belief-side extension that preserves provenance and locality.

### 2. Project rally-point knowledge through the live belief pipeline

Bandit members should acquire rally-point belief only while lawfully positioned to observe faction camp doctrine.

### 3. Expose a belief-view query for regroup consumers

Add the minimal `GoalBeliefView` / live view support needed for `RegroupWithFaction` candidate generation to read the agent-held belief later.

## Files to Touch

- To be finalized after reassessment against the live belief/perception architecture; likely under `crates/worldwake-core/src/belief.rs`, `crates/worldwake-systems/src/perception.rs`, and `crates/worldwake-sim/src/belief_view.rs`

## Out of Scope

- `RaidTarget` candidate generation
- route threat estimation
- golden T22 scenario
- any omniscient direct-policy regroup shortcut

## Acceptance Criteria

### Tests That Must Pass

1. A bandit can lawfully acquire rally-point belief through the chosen belief/perception path
2. An agent without that lawful observation path does not hold rally-point belief
3. The live belief view can query the stored rally-point belief
4. No direct authoritative read is required at AI candidate-generation time once the substrate lands

### Invariants

1. Rally-point knowledge is belief-borne, not omniscient
2. One fact has one canonical transport path from faction doctrine to agent reasoning
3. The chosen carrier is aligned with `docs/FOUNDATIONS.md`

## Test Plan

### New/Modified Tests

1. `None yet — this ticket is being created during reassessment so the exact test files will be finalized after the belief carrier is chosen against live code.`

### Commands

1. `cargo test -p worldwake-sim -- --list`
2. `cargo test -p worldwake-systems -- --list`
3. `cargo test -p worldwake-ai -- --list`
