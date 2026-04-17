# Layer 3: Detection Meta-Analysis (Step 6)

**This layer always runs**, regardless of whether the scenario is healthy or has failures. Detection quality matters in both cases — healthy scenarios may be masking problems that better detection would catch.

Layer 3 evaluates the anomaly detection system itself by cross-referencing raw trace data against what was flagged (and what wasn't).

## Step 6.1: False Positive Assessment

Review every anomaly flagged in Section 3 (mechanical) and every smell identified in Layer 1 (LLM-only). For each, assess whether it is a **true positive** or a **false positive**.

A flagged anomaly is a **false positive** if:

- The behavior is expected given the agent's role, scenario design, or control source (e.g., human-controlled agent flagged as stuck).
- The behavior is a correct adaptation to scenario constraints (e.g., agent loops sleep+relieve because that's genuinely the best available action at a resource-poor location — the problem is the scenario, not the agent).
- The threshold is too sensitive for this scenario type (e.g., 20-tick idle flagged as stuck in a scenario where agents need 30 ticks between resource runs).
- The detector pattern-matches on surface behavior without considering intent (e.g., redundant perception flagged for an entity that *should* be re-observed because it changes state between observations — the refinement missed it).

For each false positive, document:
- **Smell**: Which smell category
- **Agent(s)**: Affected agents
- **Why it's false**: Concrete reasoning
- **Detector improvement**: What change would prevent this false positive (threshold adjustment, additional filtering, context-awareness)

## Step 6.2: Detection Gap Analysis

Scan the trace data for problematic behaviors that are NOT caught by any of the 6 mechanical anomaly kinds or 4 LLM-only smells.

**Systematic scan**: For each agent, cross-reference:
- Section 7 decision timeline vs. Section 2 needs trajectories — periods where needs rise but action pattern doesn't change?
- Section 7 goal selection vs. Section 7 affordances — affordances available that are never selected?
- Section 7 plan outcomes vs. Section 2 action counts — plans found but actions never committed?
- Section 5 beliefs vs. Section 6 reality — belief-reality mismatches beyond what smell 8 covers?
- Section 2 location history vs. Section 7 goals — is travel purposeful or aimless?
- Section 2 perception counts vs. Section 5 beliefs — observations made but beliefs not forming?

**Common gap patterns**:

*Goal / planning pathologies*:
- **Aimless travel**: Agent repeatedly travels between locations without executing any goal-relevant action at destinations.
- **Goal oscillation**: Planner alternates between two goals every few ticks; neither completes because the other keeps interrupting.
- **Silent plan degradation**: Plan quality drops over time (more budget exhaustions, fewer plans found) without scenario changes — suggests accumulating state pollution.
- **Action timing pathology**: Correct actions at wrong times (e.g., eating when hunger is low, sleeping when fatigue is low) — priority inversion.
- **Belief-action disconnect**: Agent has correct beliefs about resource locations but never plans actions toward them.

*Belief / perception pathologies*:
- **Perception without belief formation**: Agent observes entities but beliefs don't update (observations pass but belief store shows no corresponding entries).
- **Dead-end exploration**: ExploreLocation succeeds (new places visited) but never leads to resource discovery because explored places are also resource-poor.

*Multi-agent pathologies*:
- **Resource hoarding**: Agent acquires resources far beyond consumption rate while other agents starve. No sharing or trade despite co-location.
- **Asymmetric agent outcomes**: Agents with identical profiles and similar starting conditions have vastly different outcomes — suggests hidden sensitivity to initial placement or stochastic choices.

*Scenario-structural patterns*:
- **Geographic convergence**: All or most agents settle at the same subset of locations. Compare Section 2 location ticks across agents — if 2+ agents spend >60% of ticks at the same location(s) while other places get <5%, the scenario's spatial design may be collapsing to a dominant corridor. Scenario design signal, not necessarily an agent bug.
- **Single-source resource dependency**: All consumption of a commodity type (food, water) comes from one resource source when alternatives exist. Compare action counts (harvest types) against scenario resource_sources. If an entire commodity class is sourced from one facility while agents have recipes for unused alternatives, agents lack resilience to disruption.

For each detected gap, document:
- **Pattern name**: Short descriptive label
- **Evidence**: Specific data from the dump
- **Agent(s)**: Affected agents
- **Why current detectors miss it**: Which existing smell is closest and why it doesn't cover this case
- **Impact**: CRITICAL / HIGH / MEDIUM / LOW

## Step 6.3: Threshold Assessment

Evaluate whether current mechanical anomaly thresholds are appropriate for this scenario:

| Threshold | Current Value | Assessment | Recommendation |
|-----------|--------------|------------|----------------|
| Stuck agent idle ticks | 20 consecutive ticks | [Too low / Appropriate / Too high] | [Suggested value if change needed, with reasoning] |
| Redundant perception count | 10 observations | [Too low / Appropriate / Too high] | [Suggested value] |
| Critical need threshold | 750 permille | [Too low / Appropriate / Too high] | [Suggested value] |
| Sustained critical duration | 100 consecutive ticks | [Too low / Appropriate / Too high] | [Suggested value] |
| Failed action spiral rate | >75% failure with 5+ attempts | [Too low / Appropriate / Too high] | [Suggested value] |
| Unaddressed need average | 750 permille | [Too low / Appropriate / Too high] | [Suggested value] |

Base the assessment on what this specific scenario reveals. A threshold that works for a survival-baseline scenario may be wrong for a trade-heavy or combat scenario.

## Step 6.4: Proposed New Smell Categories

For each detection gap identified in Step 6.2 with MEDIUM or higher impact, propose a concrete new smell category:

```markdown
#### Proposed Smell [N]: [Name]

**Detection logic**: [How to detect this mechanically in the observer binary or via LLM analysis]
**Threshold**: [Specific values — e.g., "3+ consecutive travels with no non-travel action between them"]
**Mechanical vs. LLM**: [Can this be detected mechanically in the observer binary, or does it require LLM cross-referencing?]
**Implementation scope**: [Observer binary change / New dump section / LLM-only analysis instruction]
**Example from this run**: [Concrete instance from the current scenario showing the pattern]
**False positive risk**: [What benign behavior could trigger this detector, and how to filter it]
```
