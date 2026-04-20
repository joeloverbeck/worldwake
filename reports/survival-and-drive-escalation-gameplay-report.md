# Survival And Drive-Escalation Gameplay Report

## Purpose

This report explains the gameplay features exercised by these authored scenarios:

- `scenarios/survival-baseline.ron`
- `scenarios/survival-scattered.ron`
- `scenarios/survival-contested.ron`
- `scenarios/drive-escalation-wash-priority.ron`

It is written as a design-facing plain-English brief for further research and deepening work. It describes the live behavior in the current codebase, not just the scenario intent text.

## What These Scenarios Are Really About

Taken together, these scenarios are not just “survival tests.” They are a focused examination of a broader gameplay cluster:

- bodily maintenance as a constant pressure system
- local resource discovery instead of omniscient access
- travel as a real cost, not a free connector
- repeated self-care loops under scarcity
- contention over shared survival infrastructure
- belief correction and replanning when the expected affordance is gone
- motive escalation when a need stays critical for too long

In plain terms, the game is currently testing whether agents can keep themselves alive and minimally functional in a world where food, water, hygiene, and relief all have different locations, different costs, and different planning burdens.

## The Scenario Ladder

### Survival Baseline

This is the easiest survival proving ground.

Its role is to prove that the core loop works at all:

- agents can notice important nearby affordances
- agents can discover food and water through ordinary exploration and perception
- agents can repeatedly eat, drink, sleep, relieve themselves, and wash
- no agent should get stuck idle while under real need pressure

Baseline is the “the basic body-maintenance game exists” scenario.

### Survival Scattered

This is the first scenario where survival stops being local convenience and becomes logistics.

Its role is to prove:

- no single place gives everything an agent needs
- travel itself worsens fatigue, thirst, and bladder pressure
- relieving outdoors makes later hygiene worse
- an isolated agent can still discover the world and reach food

Scattered is the “survival becomes route planning” scenario.

### Survival Contested

This adds population pressure and real competition over the same survival substrate.

Its role is to prove:

- multiple agents can survive while sharing tight-capacity wells and food sources
- depleted water sources force replanning toward alternatives
- both sides of the map can cross the chokepoint world and reach needed food
- the system can survive overlapping need windows without collapsing into idle failure

Contested is the “survival becomes social pressure without requiring explicit social systems” scenario.

### Drive Escalation Wash Priority

This is a pathology fix scenario, not a general sandbox.

Its role is to prove that agents no longer settle into a bad equilibrium where they:

- stay near food
- keep relieving outdoors
- keep getting dirtier
- keep postponing washing because hunger still wins every local comparison

This scenario exists to make sure prolonged critical dirtiness eventually becomes urgent enough to break that loop.

It is the “maintenance debt must eventually overrule short-term convenience” scenario.

## The Main Gameplay Features Involved

### Homeostatic Survival

The survival model is built around five concrete maintenance pressures:

- hunger
- thirst
- fatigue
- bladder pressure
- dirtiness

These are not abstract mood tags. They are persistent body-state values on each agent. Each agent also has authored thresholds that define when a need becomes low, medium, high, or critical.

This creates two important gameplay effects:

- different agents can become urgent for different reasons even in the same world
- “survival” is not one meter, but a bundle of competing maintenance obligations

That makes the gameplay more interesting than a single health-or-hunger loop because the agent is often choosing which problem to neglect for a while.

### Bodily Maintenance Actions

The scenarios rely on five self-care action families:

- eat
- drink
- sleep
- relieve
- wash

In the current implementation:

- eating and drinking require actually possessing a consumable item lot
- sleeping gradually reduces fatigue over time
- toilet use requires a latrine-tagged place
- wilderness relief is allowed in outdoor places and leaves waste plus a disturbance marker
- washing requires a wash basin and an available local water source at the same place

The important design quality here is that maintenance is grounded in physical affordances. There is no abstract “clean up now” button and no global bathroom availability.

### Travel As A Costly Activity

Travel is a duration-bearing action over an explicit place graph.

That matters because these scenarios depend on travel having real consequences:

- it takes time
- it occupies the actor
- it moves the actor through a specific route structure
- it can worsen fatigue, thirst, and bladder pressure if the metabolism profile says so

In baseline, travel cost is mostly removed so the core loop can be proven.

In scattered and contested, travel becomes part of the problem itself. Going to solve one need can make another need worse before arrival. This is one of the most important sources of depth in the current system.

### Harvest-Based Survival Economy

Food and water are currently produced from simple canonical harvest recipes:

- apples from orchard rows
- grain from field plots
- water from wells

These recipes:

- require that the agent knows the recipe
- require the right workstation
- draw from a concrete local resource source
- produce concrete item lots
- consume time and body effort while harvesting

This means survival is not just about finding a place. It is about finding a place that still has available stock, doing the work there, and then using the result.

The current survival scenarios are therefore lightly economic. They are not yet rich markets, but they already rely on extraction, depletion, regeneration, and local production access.

### Depletion And Regeneration

Resource sources have real available stock, real maximum stock, and optional regeneration over time.

That gives the scenarios their pressure:

- if a well or orchard is used, it really empties
- if it is regenerative, it slowly comes back
- agents can arrive after someone else and find less than expected

This is the backbone of contested survival. The pressure is not authored as “now things are scarce.” It comes from multiple agents drawing down the same stock.

### Hygiene As A Full Gameplay Loop

Dirtiness is the most distinctive feature in this scenario set because it is not just passive flavor.

The current loop is:

- ordinary life slowly increases dirtiness
- wilderness relief adds a large extra dirtiness penalty
- washing consumes time and local water and fully clears dirtiness

This makes hygiene a route-and-infrastructure problem:

- where can I wash
- when is it worth going back
- how much does relief behavior create later cleaning debt
- how much does hygiene compete with hunger or thirst

The drive-escalation scenario exists because hygiene can otherwise lose too often in raw motive comparisons.

### Relief And Aftermath

Relief behavior is more grounded than it first appears.

Using a latrine and relieving in the wild are different gameplay events.

Wild relief currently:

- is only legal in certain outdoor place types
- creates waste
- creates a disturbance marker as evidence
- resets bladder pressure
- makes the actor dirtier

This is already more interesting than a hidden bladder reset because it creates both bodily consequences and world aftermath. It is a strong foundation for later sanitation, tracking, disgust, disease, social order, or territory-sign gameplay if those are ever desired.

### Belief-Limited Planning

These scenarios do not let agents plan from hidden world truth.

The agent can directly use what it is physically co-located with, but anything remote depends on belief, observation history, and retained knowledge.

That matters in several ways:

- agents must discover remote food or water
- agents can act on stale expectations
- agents can reach a place and find it changed
- motive escalation does not magically create knowledge of unseen washing infrastructure

This is one of the most important constraints in the whole feature cluster. It means “survival intelligence” is partly a knowledge problem, not just an optimization problem.

### Exploration As Survival Recovery

Exploration is not currently a separate fantasy pillar. In these scenarios it is a fallback recovery behavior for blocked self-care.

Exploration appears when:

- the need pressure is high enough
- curiosity is enabled
- the agent is not already succeeding locally
- the agent does not know a usable acquisition path
- recent acquisition attempts have failed enough times

So exploration is currently a “go find a better answer to this need” behavior, not just random wandering.

That is why the survival scenarios care so much about isolated starts and scattered resource layouts. They are testing whether survival pressure can lawfully produce outward search.

### Goal Competition And Switching

The AI treats self-care goals as reactive and never suppresses them under stress. That is important because it prevents political, social, or opportunistic goals from crowding out survival.

Within survival itself, though, there is still competition:

- hunger can beat dirtiness
- thirst can beat fatigue
- a current plan can persist unless a challenger clears the switching margin

This creates stability, but it also creates the possibility of bad local equilibria. The wash-priority scenario exists because a stable but unhealthy loop had emerged.

### Drive Escalation

Drive escalation is the current answer to “what happens if a need stays critical for too long?”

Each need can have escalation settings that say:

- how long the need can stay critical before escalation starts
- how quickly the motive multiplier rises after that
- how high it is allowed to rise

In the current defaults, escalation is linear after a grace period and capped at a multiple of the original motive.

In gameplay terms, this means long-neglected critical needs become more politically important inside the mind of the agent. The system is trying to model mounting desperation or mounting maintenance debt.

For the wash-priority scenario, this is what finally lets washing beat the orchard loop.

### Contention And Competition

There are two kinds of contention visible in these features.

The first is soft contention through depletion:

- two agents want the same water
- one gets there first
- the other finds less stock than expected and must replan

That is the main kind used by the survival scenarios.

The second is hard contention through reservation or grants on exclusive facilities. The broader codebase supports that for some facility-driven actions, especially harvesting and other managed exclusive uses.

The important present detail is that washing is not yet using a rich contention queue in the same way. Washing currently competes mostly through shared location, shared travel burden, and shared water depletion rather than explicit wash-basin queue drama.

### Perception And Local Observation

The perception system continuously updates what agents know from:

- local observation
- witnessed events
- retained beliefs
- claim confidence and staleness rules

In these survival scenarios, perception matters most for:

- learning where food and water exist
- noticing local resource changes
- preserving place visits
- updating beliefs after moving through the world

This is why the scenarios can work without seeded omniscience. The agents are meant to build a practical map of useful places by living in the world.

## What The Current Feature Set Already Does Well

- Survival is physical, local, and route-based rather than menu-based.
- The body model creates real multi-need tradeoffs instead of one dominant meter.
- Travel makes survival spatial.
- Resource sources can be exhausted and later recover.
- Exploration has a lawful role in survival rather than being pure flavor.
- Washing and relief already create a meaningful maintenance sub-loop.
- Contested survival emerges from shared substrate, not scripted events.
- Motive escalation gives the system a way to escape unhealthy loops without hardcoding a special case for one scenario.

## Where The Current System Still Feels Thin Or Constrained

### Washing Is Still Narrow

Washing is currently a simple reset action:

- be at a place with a basin
- have local water access there
- spend the action time
- consume one unit of source water
- clear dirtiness completely

That works, but it is still mechanically blunt. There is no partial washing, no gradation of wash quality, no social consequences of staying dirty, and no richer chain of hygiene logistics.

### Wash Discovery Is Still Fragile

The survival test suite explicitly acknowledges a current planner limitation: remote washing opportunities can still exhaust planning budget before the agent discovers a wash basin in some harder scenarios.

That means washing is mechanically present, but the remote-search side of the loop is not yet as robust as food and water acquisition.

### Contention Is More Depletion Than Procedure

The contested scenario creates good pressure, but most of that pressure comes from resource stock and travel topology, not from richer social procedure around shared infrastructure.

Agents mostly compete by arriving first or replanning after loss, not by visibly negotiating, queueing, contesting turn rights, or maintaining longer-lived access claims around wash and water use.

### Relief Has Aftermath But Not Yet Society

Wild relief already produces waste and a disturbance marker, which is a strong causal hook. But right now the survival scenarios mostly use it as a hygiene cost. The broader downstream meaning of that aftermath is still thin.

### Production Variety Is Intentionally Minimal

The survival economy is currently intentionally simple. Apples, grain, and water are enough to test the core survival architecture, but not enough to make subsistence life feel culturally or strategically rich.

### The Survival Contract Is Still Mostly About Failure Bounds

The authored survival contracts are excellent for catching collapse, excessive critical runs, or idle stalls. They are less about the quality of life that emerges in a successful run.

At the moment, “survives and keeps acting” is more strongly specified than “develops convincing maintenance routines, territory preferences, habits, or lifestyle patterns.”

## Design-Relevant Takeaways For Deeper Research

If the goal is to make these features fuller, rounder, and more in-depth while staying aligned with the project’s principles, the strongest current leverage points appear to be:

- deepening hygiene from a reset action into a broader maintenance ecology
- deepening contested infrastructure use from depletion races into more visible social procedure
- deepening exploration from fallback search into lived route knowledge and habitual territory use
- deepening bodily maintenance so different agents develop distinct survival styles rather than only different urgency curves
- deepening aftermath so waste, dirty spaces, repeated use patterns, and maintenance neglect leave more downstream traces
- deepening belief friction so stale resource expectations and partial local knowledge create richer miscoordination without cheating

## Short Feature Summary

The survival scenarios are currently testing a world where staying alive is a recurring local logistics problem, not an abstract status bar problem. The drive-escalation scenario adds a crucial correction: long-ignored maintenance needs must eventually become urgent enough to break convenience loops.

The strongest existing foundations are already in place: physical needs, physical places, physical travel, depletable resources, belief-limited planning, and explicit maintenance actions. The biggest opportunity for future depth is not inventing a new survival framework, but thickening the consequences, procedures, and social meaning around the one that already exists.
