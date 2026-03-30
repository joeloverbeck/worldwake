# E20COMBEH-006B: Document outdoor-place affordance trap in golden testing guide

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — documentation only
**Deps**: E20COMBEH-006

## Problem

During E20COMBEH-006 implementation, two test iterations failed because agents at outdoor places (EastFieldTrail: Trail + Field tags) used `relieve_wilderness` locally instead of traveling to a distant latrine. The planner correctly found the zero-travel-cost local action — this is emergent behavior per Principle 1 — but the test author expected travel.

This is a recurring trap for golden test authors designing bladder/relief scenarios: outdoor places offer `relieve_wilderness`, which short-circuits any travel the test intends to force. The same pattern applies to any scenario where a local affordance satisfies the goal the test relies on for motivating travel.

`docs/golden-e2e-testing.md` should document this pattern so future authors avoid the same mistake.

## Assumption Reassessment (2026-03-30)

1. **`docs/golden-e2e-testing.md`**: Exists, contains Scenario Isolation section (lines 133-154) that discusses removing lawful competing affordances. Does not specifically mention the outdoor-place / wilderness-relief trap.
2. **`relieve_wilderness` constraint**: `Constraint::ActorAtPlaceWithAnyTag(OUTDOOR_RELIEF_TAGS)` where `OUTDOOR_RELIEF_TAGS = [Forest, Trail, Field, Farm, Road]`. Confirmed in `crates/worldwake-systems/src/needs_actions.rs:104`.
3. **Prototype topology**: EastFieldTrail (Trail + Field), OrchardFarm (Farm + Field), ForestPath (Forest + Trail), NorthCrossroads (Crossroads + Road), SouthGate (Gate + Road), BanditCamp (Camp + Forest) — all outdoor. Only VillageSquare (Village), GeneralStore (Store + Village), CommonHouse (Inn + Village), RulersHall (Hall + Village), GuardPost (Barracks + Village), PublicLatrine (Latrine + Village) are indoor.
4. **Single-layer ticket**: Documentation only. No code or test changes.

## Architecture Check

1. Documentation captures a lesson learned from implementation experience. Aligns with Principle 29 (debuggability) — making the test environment more legible to future authors.
2. No shims or code changes.

## Verification Layers

1. Documentation accuracy → manual review against `OUTDOOR_RELIEF_TAGS` and prototype topology
2. No code invariants — documentation-only ticket

## What to Change

### 1. Add "Outdoor Place Affordance Trap" section to `docs/golden-e2e-testing.md`

Add after the "Scenario Isolation" section, a new subsection:

**Outdoor Place Affordance Trap**

When designing golden scenarios that require an agent to travel for relief (bladder, dirtiness), be aware that `relieve_wilderness` is available at any outdoor place. The planner will prefer it over traveling to a distant latrine because it has zero travel cost.

Outdoor places in the prototype world: EastFieldTrail, OrchardFarm, ForestPath, NorthCrossroads, SouthGate, BanditCamp. Indoor places: VillageSquare, GeneralStore, CommonHouse, RulersHall, GuardPost, PublicLatrine.

To force travel for relief:
- Start the agent at an indoor place (no wilderness relief available)
- Or use a different need driver (hunger, thirst) where the resource is distant

This generalizes: any scenario that relies on travel must ensure no local affordance satisfies the motivating goal at the starting place.

### 2. Add "Multi-Hop Travel Observation" note

Multi-hop travel (e.g., VillageSquare → SouthGate → EastFieldTrail → OrchardFarm) creates one travel action per leg. Between legs, the agent replans (~1 tick gap). Tests counting total travel ticks must tolerate inter-leg gaps rather than breaking out of the observation loop after the first leg ends.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify: add two new subsections)

## Out of Scope

- Changes to production code
- Changes to existing golden tests
- Changes to prototype topology

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Invariants

1. Documentation accurately reflects `OUTDOOR_RELIEF_TAGS` and prototype topology
2. Documentation does not contradict existing Scenario Isolation guidance

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. Manual review of `docs/golden-e2e-testing.md` changes against `crates/worldwake-core/src/topology.rs` (OUTDOOR_RELIEF_TAGS, prototype place specs)

## Outcome

- **Completion date**: 2026-03-30
- **What changed**: Added two subsections to `docs/golden-e2e-testing.md` — "Outdoor Place Affordance Trap" (documenting that `relieve_wilderness` is available at any outdoor place and listing indoor vs outdoor prototype places) and "Multi-Hop Travel Observation" (documenting inter-leg replan gaps in multi-hop travel).
- **Deviations**: None.
- **Verification**: Manual review confirmed documentation matches `OUTDOOR_RELIEF_TAGS` constant and prototype topology. No code changes.
