# Worldwake Foundational Principles

These principles define what Worldwake is and how every system, feature, and line of code should be evaluated. They are non-negotiable unless explicitly revised by the project owner. All contributors — human and AI — must internalize these before making design decisions.

## 1. Maximal Emergence Through Causality

The simulation's purpose is to produce emergent behavior through chains of consequences, never through scripts or authored sequences. Every event in the world should be the natural result of a prior cause — an agent's action, a world process, or a consequence of another event.

**Example**: A dangerous beast consumes all food in a territory, so it moves to another and starts hunting merchant caravans. Survivors flee to the nearby town and report to the authorities, who post bounties. Local adventurers take the contract and go hunt the beast. None of this is scripted. The engine must be clean, extensible, and robust enough that this sequence arises from independent systems reacting to shared world state.

**Test**: If you have to ask "but what triggers this sequence?", the answer must always be a concrete prior event or state change — never "the designer wanted it to happen."

## 2. No Magic Numbers

Outcomes in the simulation must derive from concrete world state, agent decisions, or natural processes — never from abstract tuning variables like `chanceOfEncounter`, `spawnRate`, or `eventProbability`.

If something can happen, it happens because the conditions for it exist in the world. A merchant gets ambushed because bandits are physically present on the route the merchant is traveling, not because a dice roll against an encounter table succeeded. Food spoils because time has passed and the food's material properties dictate decay, not because a `decayChance` variable fired.

**Test**: For any outcome, you must be able to trace a concrete causal chain back to world state. If the chain bottoms out at a probability constant with no physical grounding, the design violates this principle.

## 3. No Backward Compatibility

We do not maintain backward compatibility, alias paths, deprecated shims, or compatibility layers. When a design changes, everything that depended on the old design is updated or removed. If something breaks, it gets fixed — not papered over.

This keeps the codebase honest and prevents the accumulation of dead paths, confused abstractions, and "legacy" code that silently rots.

**Test**: If you find yourself writing a wrapper, redirect, or `// deprecated` comment to keep old code working alongside new code, stop. Update or remove the old code.

## 4. Simulate Carriers of Consequence, Not Generic Realism

Only model things that can propagate downstream effects: goods, facilities, wounds, waste, offices, loyalties, debts, contracts, witness knowledge, rumors, route danger. Do not model molecules, full weather systems, or broad crafting trees for the sake of realism.

The simulation's fidelity comes from causal depth, not from breadth of modeled phenomena. A wound matters because it affects an agent's capacity, which affects their decisions, which affects the world. Atmospheric humidity does not matter unless it concretely changes something an agent cares about.

**Test**: For any proposed system or component, ask: "What downstream consequence does this enable that wouldn't otherwise exist?" If the answer is "it's more realistic," that is not sufficient.

## 5. World Runs Without Observers

The simulation must continue meaningfully when no human is watching. An unobserved village must still have its merchant run out of goods, its guards get tired, its bandit camp grow. There are no "Schrodinger's NPCs" who freeze when offscreen.

This is not merely a technical requirement — it is the foundation of emergence. If the world only advances where the human is looking, causal chains cannot propagate through unobserved regions, and the domino effects that define Worldwake cannot occur.

**Test**: Advance the simulation 1000 ticks with no human-controlled agent. The world state should have changed in internally consistent, causally traceable ways.

## 6. Concrete State Over Abstract Scores

Prefer modeling the thing itself over a score that represents it. Instead of assigning `danger_score: 0.7` to a road, model the actual bandits present on that route. The "danger" is an emergent property of who is there and what they do, not a number someone assigned.

Abstract scores hide causality. When a score changes, you cannot ask "why?" in a way that traces back to world events. When concrete state changes, the cause is always traceable.

**Test**: If a system uses a numeric score to represent a world condition, ask whether that condition could instead be derived from concrete entities and their states. If yes, use the concrete representation.

## 7. Every Action Has Physical Cost

Actions consume time, materials, energy, or attention. Nothing is free. Travel takes ticks, crafting consumes inputs, fighting causes fatigue and risk of injury, even conversation occupies the agent's time.

This principle is the engine of emergent resource pressure. When everything costs something, agents must prioritize, trade off, and make imperfect decisions — which is where interesting emergence lives.

**Test**: For any action in the system, you must be able to name at least one concrete resource it consumes. If an action is "free," it will be spammed and will short-circuit the resource pressure that drives emergence.

## 8. Agent Symmetry

The engine makes zero distinction between human-controlled and AI-controlled agents. All agents use the same action set, the same precondition checks, the same effect pipeline. The human player can swap to any agent at any time. `ControlSource` determines only the input source (keyboard vs. planner), never the available actions or world rules.

This is not a nice-to-have — it is a structural guarantee that the simulation is fair and that emergence is genuine. If the human agent had special powers or shortcuts, every emergent chain involving that agent would be suspect.

**Test**: Swap `ControlSource` on any agent from `Ai` to `Human` (or vice versa). The simulation must continue without errors, and the agent's available actions must not change.

## 9. Intelligent Agency Over Behavioral Scripts

AI-controlled agents must pursue goals through the same reasoning an intelligent actor would: assessing their beliefs, weighing options, and choosing actions that serve their needs and ambitions. The AI layer (currently GOAP + utility scoring) exists to produce decisions that are plausible for the character, not to create entertaining gameplay or predictable patterns.

A merchant should reroute because they believe a road is dangerous, not because a behavior tree says "flee when danger > 0.5." A guard should investigate a noise because they heard it and it conflicts with their expectation of safety, not because a scripted patrol routine told them to.

The AI architecture may evolve, but the standard is always: would a reasoning person in that situation, with those beliefs, make that choice?

**Test**: For any AI decision, you should be able to explain it as "Agent X chose Y because they believed Z." If the explanation is "the behavior tree reached this node" or "the random roll said so," the design violates this principle.
