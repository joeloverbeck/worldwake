# Worldwake Foundational Principles

These principles define what Worldwake is and how every system, feature, and line of code should be evaluated. They are non-negotiable unless explicitly revised by the project owner. All contributors — human and AI — must internalize these before making design decisions.

---

## I. Causal Foundations

### 1. Maximal Emergence Through Causality

The simulation's purpose is to produce emergent behavior through chains of consequences, never through scripts or authored sequences. Every event in the world should be the natural result of a prior cause — an agent's action, a world process, or a consequence of another event.

**Example**: A dangerous beast consumes all food in a territory, so it moves to another and starts hunting merchant caravans. Survivors flee to the nearby town and report to the authorities, who post bounties. Local adventurers take the contract and go hunt the beast. None of this is scripted. The engine must be clean, extensible, and robust enough that this sequence arises from independent systems reacting to shared world state.

**Test**: If you have to ask "but what triggers this sequence?", the answer must always be a concrete prior event or state change — never "the designer wanted it to happen."

### 2. No Magic Numbers

Outcomes in the simulation must derive from concrete world state, agent decisions, or natural processes — never from abstract tuning variables like `chanceOfEncounter`, `spawnRate`, or `eventProbability`.

If something can happen, it happens because the conditions for it exist in the world. A merchant gets ambushed because bandits are physically present on the route the merchant is traveling, not because a dice roll against an encounter table succeeded. Food spoils because time has passed and the food's material properties dictate decay, not because a `decayChance` variable fired.

**Test**: For any outcome, you must be able to trace a concrete causal chain back to world state. If the chain bottoms out at a probability constant with no physical grounding, the design violates this principle.

If a system computes an aggregate (average, total, percentage) and applies threshold-based multipliers to produce an outcome, verify that both the aggregate and the thresholds derive from concrete world properties. A pricing formula that says "if stock < 50%, multiply price by 1.5" is a magic number unless the 50% and 1.5 arise from traceable world state (e.g., the count of unsatisfied buyers who arrived and found nothing). Shortcutting with lookup tables what should emerge from agent interactions violates this principle.

### 3. Concrete State Over Abstract Scores

Prefer modeling the thing itself over a score that represents it. Instead of assigning `danger_score: 0.7` to a road, model the actual bandits present on that route. The "danger" is an emergent property of who is there and what they do, not a number someone assigned.

Abstract scores hide causality. When a score changes, you cannot ask "why?" in a way that traces back to world events. When concrete state changes, the cause is always traceable.

**Test**: If a system uses a numeric score to represent a world condition, ask whether that condition could instead be derived from concrete entities and their states. If yes, use the concrete representation.

---

## II. World Dynamics

### 4. Simulate Carriers of Consequence, Not Generic Realism

Only model things that can propagate downstream effects: goods, facilities, wounds, waste, offices, loyalties, debts, contracts, witness knowledge, rumors, route danger. Do not model molecules, full weather systems, or broad crafting trees for the sake of realism.

The simulation's fidelity comes from causal depth, not from breadth of modeled phenomena. A wound matters because it affects an agent's capacity, which affects their decisions, which affects the world. Atmospheric humidity does not matter unless it concretely changes something an agent cares about.

**Test**: For any proposed system or component, ask: "What downstream consequence does this enable that wouldn't otherwise exist?" If the answer is "it's more realistic," that is not sufficient.

### 5. World Runs Without Observers

The simulation must continue meaningfully when no human is watching. An unobserved village must still have its merchant run out of goods, its guards get tired, its bandit camp grow. There are no "Schrodinger's NPCs" who freeze when offscreen.

This is not merely a technical requirement — it is the foundation of emergence. If the world only advances where the human is looking, causal chains cannot propagate through unobserved regions, and the domino effects that define Worldwake cannot occur.

**Test**: Advance the simulation 1000 ticks with no human-controlled agent. The world state should have changed in internally consistent, causally traceable ways.

### 6. Every Action Has Physical Cost

Actions consume time, materials, energy, or attention. Nothing is free. Travel takes ticks, crafting consumes inputs, fighting causes fatigue and risk of injury, even conversation occupies the agent's time.

This principle is the engine of emergent resource pressure. When everything costs something, agents must prioritize, trade off, and make imperfect decisions — which is where interesting emergence lives.

**Test**: For any action in the system, you must be able to name at least one concrete resource it consumes. If an action is "free," it will be spammed and will short-circuit the resource pressure that drives emergence.

### 7. Locality of Interaction and Information

All agent-to-agent interaction requires co-location or an explicit communication channel that consumes time and traverses the place graph. No system may query global world state on behalf of an agent. Information propagates at finite speed: an event at place A cannot influence an agent at place C without passing through intermediate places or agents, and each hop costs time.

**Example**: A merchant in Town cannot know that the Bandit Camp was destroyed until a traveler who witnessed it arrives at Town and tells them, or a chain of rumors propagates through the graph. The merchant does not "check" whether bandits exist — they act on beliefs formed from local perception and received reports.

**Test**: For any information that reaches an agent, trace the propagation path. Every hop must correspond to either (a) co-location perception, (b) a report transmitted between co-located agents, or (c) a physical carrier that traveled the graph. If information arrives without a traceable multi-hop path proportional to graph distance, the design violates locality.

### 8. Every Amplifying Loop Must Have a Physical Dampener

For every system interaction that creates positive feedback (A increases B, B increases A), there must be a concrete mechanism in the world model that naturally limits the amplification. The dampener must be a physical world process — resource exhaustion, natural recovery, competing pressures — not a numerical clamp or cap. Death spirals are design bugs, not emergence.

**Example**: Crime -> fear -> guard desertion -> more crime is a positive feedback loop. The dampener might be: bandits who succeed too easily attract rivals or run out of targets, guards who flee face hunger and return when the alternative is starvation, or a neighboring authority sends reinforcements when trade revenue drops. The dampener is NOT `fear = min(fear, 1.0)`.

**Test**: For any positive feedback loop in the system, identify the concrete world mechanism that breaks or dampens it. If the only dampener is a numerical cap, or if no dampener exists, the design violates this principle. Each system spec must include a feedback analysis section.

---

## III. Agent Architecture

### 9. Agent Symmetry

The engine makes zero distinction between human-controlled and AI-controlled agents. All agents use the same action set, the same precondition checks, the same effect pipeline. The human player can swap to any agent at any time. `ControlSource` determines only the input source (keyboard vs. planner), never the available actions or world rules.

This is not a nice-to-have — it is a structural guarantee that the simulation is fair and that emergence is genuine. If the human agent had special powers or shortcuts, every emergent chain involving that agent would be suspect.

**Test**: Swap `ControlSource` on any agent from `Ai` to `Human` (or vice versa). The simulation must continue without errors, and the agent's available actions must not change.

### 10. Intelligent Agency Over Behavioral Scripts

AI-controlled agents must pursue goals through the same reasoning an intelligent actor would: assessing their beliefs, weighing options, and choosing actions that serve their needs and ambitions. The AI layer (currently GOAP + utility scoring) exists to produce decisions that are plausible for the character, not to create entertaining gameplay or predictable patterns.

A merchant should reroute because they believe a road is dangerous, not because a behavior tree says "flee when danger > 0.5." A guard should investigate a noise because they heard it and it conflicts with their expectation of safety, not because a scripted patrol routine told them to.

The AI architecture may evolve, but the standard is always: would a reasoning person in that situation, with those beliefs, make that choice?

**Test**: For any AI decision, you should be able to explain it as "Agent X chose Y because they believed Z." If the explanation is "the behavior tree reached this node" or "the random roll said so," the design violates this principle.

### 11. Agent Diversity Through Concrete Variation

Agents in the same role must differ in their need rates, utility weights, risk tolerance, and initial beliefs. These differences must come from concrete per-agent parameters seeded at creation, not from role labels or random per-decision noise. Two guards given identical beliefs at the same tick should plausibly choose different goals because their internal parameters differ.

Homogeneous populations produce herd behavior — all agents converge on the same strategy, and the simulation degenerates to a single-path outcome. Diversity is a necessary condition for emergence (Holland 1998).

**Example**: Guard A has high loyalty and low fear-sensitivity; they stay at their post when bandits approach. Guard B has moderate loyalty and high self-preservation; they flee. The succession crisis plays out differently because the guards split, not because a dice roll decided.

**Test**: Create two agents of the same role with different seeded parameters. Present both with the same belief state at the same tick. They must sometimes select different goals. If all agents of a role always select the same goal given identical beliefs, diversity is insufficient.

---

## IV. System Architecture

### 12. Systems Interact Through State, Not Through Each Other

Systems must not invoke each other's logic directly. All inter-system effects must propagate through mutations to shared world state (component changes, events emitted to the log). A system reads the current world state, applies its rules, and writes new state. It never imports or calls functions from another system's module.

This is the architectural condition for combinatorial emergence. When N systems interact only through shared state, any system can trigger any other system through state changes — including combinations no one foresaw. Direct inter-system coupling limits emergence to only the interactions the programmer explicitly coded.

**Example**: The combat system inflicts a wound (writes a Wound component). The needs system independently reads Wound and increases pain. The decision architecture independently reads elevated pain and reprioritizes goals. No system "calls" another. A fire system could later read the same wound component to determine movement speed reduction — without the combat system knowing fire exists.

**Test**: For any two system modules X and Y, verify that X does not import or call functions from Y (and vice versa). All influence from X to Y must pass through world state or events. In Rust, this is enforceable through module visibility: system modules in `worldwake-systems` depend only on `worldwake-core` and `worldwake-sim`, never on each other.

### 13. No Backward Compatibility

We do not maintain backward compatibility, alias paths, deprecated shims, or compatibility layers. When a design changes, everything that depended on the old design is updated or removed. If something breaks, it gets fixed — not papered over.

This keeps the codebase honest and prevents the accumulation of dead paths, confused abstractions, and "legacy" code that silently rots.

**Test**: If you find yourself writing a wrapper, redirect, or `// deprecated` comment to keep old code working alongside new code, stop. Update or remove the old code.
