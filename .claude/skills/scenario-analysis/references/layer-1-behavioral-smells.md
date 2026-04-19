# Layer 1: Behavioral Smell Analysis (Step 4)

Analyze the dump for all 10 smell categories. For each, state whether the smell was detected, its severity (CRITICAL / HIGH / MEDIUM / LOW / NONE / INCONCLUSIVE), and the reasoning. Use INCONCLUSIVE when trace data is insufficient and explain the data limitation. If an agent died, focus on ticks leading to death, not post-death idle.

## Mechanically Flagged Smells

Already in Section 3 — add narrative context and root-cause hypotheses.

1. **Redundant Perception** — Agent observes the same unchanged entity repeatedly. Suggests overly broad perception or belief never updating.

2. **Action Loops** — Agent repeats the same action sequence (not patrol — patrol agents are excluded from this detector in the binary; verify patrol behavior in raw traces if suspicious) without progress. Cross-reference Section 7's decision timeline to see what the planner was selecting. Sub-patterns:
   - **Behavioral collapse**: Agents settling into a minimal-action pattern (e.g., only sleep+relieve) for extended periods. Section 2 pre-computes behavioral transition markers — use as starting points, then verify against Section 7 decision timeline bins. Transitions in the last 100 ticks with all needs below 300 permille are typically end-of-sim artifacts (sleep is the correct low-urgency default). Note but don't escalate unless needs were rising at the transition.
   - **Degenerate plan loops**: Same goal selected repeatedly with plans found but 0 actions executed. Grep `GoalSatisfied[steps=0` — hundreds of occurrences across multiple 100-tick bins confirm a degenerate loop.
   - **Affordance-reporting gaps**: If an action type appears frequently in the action timeline but is absent from all affordance snapshots, note the discrepancy in Cross-Cutting Patterns.

3. **Stuck Agents** — No actions for many consecutive ticks. Distinguish:
   - *Explainable idle*: human-controlled agent with no input (always appears stuck — expected), needs satisfied, no affordances available.
   - *Pathological*: needs rising while agent does nothing. Check Section 7 for planner outcomes during the idle period. If candidate count dropped to 0, the agent is idle because no goal candidates were generated.
   - *Post-death*: if the agent has dead ticks, idle status post-death is expected — focus on ticks leading to death.

   **Detector caveat**: the mechanical stuck-agent detector counts consecutive ticks with no action *started or in-progress*. Multi-tick actions like sleep, wash, and travel legs occupy the agent and are not counted as idle. Therefore "max consecutive idle ticks" in Section 2 may exceed the detector's threshold without triggering an anomaly.

4. **Failed Action Spirals** — Agent keeps attempting actions that fail validation. Which precondition is failing? Is the agent's belief stale?

5. **Sustained Critical Needs** — A need stays above 750 permille for 100+ consecutive ticks. Cross-reference the agent's actions during that range and Section 7's failed plan attempts. Distinguish `frontier-exhausted` (plan definitively not found) from `budget-exhausted` (search space too large — plan may exist but can't be found within budget). Note candidate counts and max depth.

6. **Unaddressed Needs** — Need average exceeds 750 permille but no corresponding relief action (eat/drink/sleep/toilet/wash) was ever attempted. Cross-reference Section 7's blocked desires and affordances. If the relief action doesn't appear in the latest affordance snapshot, it's a missing affordance.

## LLM-Only Smells

Cross-reference dump sections to detect.

7. **Impossible Knowledge** — Did an agent act on information about an entity they never observed and never heard about through Tell/AskWitness? Cross-reference action targets vs. entities in perception trace.

8. **Belief Staleness** — Cross-reference Section 5 beliefs with action traces, perception traces, and Section 6 end-state. Does the agent believe resources exist at locations they haven't visited recently? Do believed entity locations match current placement?

9. **Social Isolation** — Agents co-located for 20+ ticks with no Tell, AskWitness, or Trade actions. Also flag: no Trade despite complementary needs/inventory, heavily unidirectional social actions, role-specific social actions unused, tell actions producing SocialArtifacts with no behavior change.

10. **Economic Stagnation** — Agents with unmet needs (hunger/thirst > 500 permille) in locations with resource sources (Section 6), but no harvest/craft/trade actions attempted. Cross-reference Section 5 beliefs with Section 6 place contents. Section 7's failed plan attempts reveal whether agents tried economic actions and failed.

## Known Pathology Signatures

Recurring patterns for faster diagnosis:

- **FreeCarryCapacity degenerate loop**: Inventory fills with Waste, `GoalSatisfied[steps=0]` repeats 50+ times per bin, zero actions executed. Cross-reference Section 6 inventory and smell 10.
- **AcquireCommodity budget exhaustion spiral**: Multi-location plan generates 1000–6000+ candidates at depth 5–9, exceeding budget every time. Manifests as sustained critical needs (smell 5) despite commodity existing at reachable location.
- **Obligation spam loop**: Fast-completing obligation action (post_notice, investigate) fires 50+ times per bin while survival needs are critical. Obligation goal's drive score overwhelms hunger/thirst/fatigue. Distinct from other signatures: plans succeed, actions execute, but the wrong goal is chosen.
- **Sleep+relieve behavioral collapse**: Action repertoire narrows to only sleep and relieve_wilderness for 500+ ticks. All non-trivial goals fail planning or lack local affordances. Often caused by geographic food desert.
- **Geographic Convergence**: 2+ agents anchor on the same place for >=60% of a 200-tick window. Mechanical label: `GEOGRAPHIC_CONVERGENCE`. Investigate whether this is a lawful shared hub or a collapsed routing pattern.
- **Maintenance Starvation**: A need's accumulation outpaces relief over a 200-tick window while average need stays in the agent's high band. Mechanical label: `MAINTENANCE_STARVATION`. Investigate maintenance cadence, relief distance, and whether the anomaly is a real stress smell inside an otherwise surviving baseline.
- **Recipe Monoculture**: >=95% of an agent's need-category recipe actions concentrate on one recipe despite multiple known and belief-gated alternatives. Mechanical label: `RECIPE_MONOCULTURE`. Investigate whether unused alternatives are truly reachable or are just knowledge without workable substrate.
- **Acute Need Spike**: A need stays at or above the agent's critical threshold for 30-99 consecutive ticks. Mechanical label: `ACUTE_NEED_SPIKE`. Investigate whether this is a bounded crisis inside an otherwise healthy run or part of a longer sustained-critical episode.

After analyzing all 10 smells, record which data gaps (if any) prevented confident assessment — this feeds Layer 3.
