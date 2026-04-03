# CLI Readability & Usability Evaluation (v2)

Evaluation report for the Worldwake CLI app (`crates/worldwake-cli/`).

Each evaluation is produced by the `cli-improvement:evaluate` skill, which
interactively uses the CLI against `scenarios/cli-evaluation.ron`, runs
mandatory checklists, and scores 6 metrics.

## Metrics

| # | Metric | What It Measures |
|---|--------|-----------------|
| 1 | Decision Transparency | Can you see WHY agents chose goals? |
| 2 | Action Lifecycle Clarity | Can you see WHAT actions happen at each stage? |
| 3 | Delta Semantics | Do deltas say WHAT changed, not just WHICH component? |
| 4 | Action Reliability | Do listed actions actually work? |
| 5 | Command Self-Documentation | Are commands self-explanatory? |
| 6 | Causal Chain Readability | Can you trace consequences to causes? |

## Scoring Guide

- **1-3**: Unusable — no information, crashes, incomprehensible
- **4-5**: Poor — partial information, some crashes or opaque output
- **6-7**: Adequate — most information present but gaps remain
- **8-9**: Good — full lifecycle visible, actionable deltas, reliable commands
- **10**: Excellent — a developer unfamiliar with the project understands everything

## Checklist Gate

A metric cannot score above 5 if its mandatory checklist has any failures.

## Graduation

All 6 checklists fully pass, average score >= 8.0, and no CRITICAL or HIGH
recommendations remaining.

---

## EVALUATION #1 — 2026-04-03

### Session Notes

First evaluation under v2 metrics (informational completeness focus). Scenario updated with S44 profiles — all 4 agents now have diverse universal and role-specific profiles. Explored all 4 workflows, tried all 22 available actions. Found 3 crashing actions, opaque event deltas, and missing action type names in lifecycle events.

### Checklist Results

| Checklist | Result | Notes |
|-----------|--------|-------|
| 1. Decision Transparency | FAIL (0/3) | Tick summary has no goal names. Decision events show "AgentData: set" not goal kind. |
| 2. Action Lifecycle Clarity | FAIL (1/4) | `do N` names the action (PASS). ActionStarted/Committed events don't name the action type. Status doesn't show in-progress action name. |
| 3. Delta Semantics | FAIL (1/3) | PossessedBy delta is semantic ("added 5× Bread → Guard Theron"). AgentBeliefStore and AgentData deltas are opaque component names only. |
| 4. Action Reliability | FAIL (0/1) | 3 of 22 actions crash: declare_support (payload error), queue_for_facility_use (payload error), staff_market (missing profile). Errors show raw IDs. |
| 5. Command Self-Documentation | FAIL (3/4) | help, inspect, switch all good. trace error says `<ID>` not "event ID". |
| 6. Causal Chain Readability | FAIL (0/3) | All traces tested show only the single event — no parent chain visible. |

### Per-Command Analysis

**Explore workflow**: Clean and informative. `world`, `agents`, `places`, `look`, `inspect`, `goods`, `needs`, `inventory`, `relations` all produce well-formatted, human-readable output. Inspect shows detailed component values with ‰ formatting.

**Act workflow**: Action listing is comprehensive (22 actions). `do N` correctly names the requested action. However: 3 actions crash with opaque errors — `staff_market` (raw EntityId "e5g0"), `declare_support` (raw action def ID "adef13"), `queue_for_facility_use` (raw "adef6"). Action numbering sometimes mismatches between list and `do` command. Tick summary shows event/action counts but no agent names, goal names, or action type names.

**Control workflow**: `switch`, `observe`, status all work cleanly. In-transit status is nicely formatted.

**Debug workflow**: Event listing is reasonable. Individual event inspection shows tags, actor, targets, witnesses — but **deltas are the critical gap**. Most deltas show only component name ("AgentBeliefStore: set on X", "AgentData: set on X") without semantic content. Exception: `PossessedBy` and relation deltas are informative. ActionStarted and ActionCommitted events don't name the action type. Trace command shows only the queried event — no parent chain is visible.

### Resolved Since Previous

First evaluation.

### Scores

| # | Metric | Score | Delta | Gate | Justification |
|---|--------|-------|-------|------|---------------|
| 1 | Decision Transparency | 2 | — | FAIL | Tick output and decision events provide zero information about what goal was chosen or why. Only "N started, N completed" counts. |
| 2 | Action Lifecycle Clarity | 3 | — | FAIL | `do N` names the action (good). But ActionStarted/Committed events don't name the action type. Can't tell if Guard Theron is picking up bread or stealing it. |
| 3 | Delta Semantics | 3 | — | FAIL | PossessedBy and relation deltas are semantic. But AgentBeliefStore, AgentData, ActiveGoal deltas are opaque component names. Social tells show "AgentBeliefStore: set" — what beliefs were shared? |
| 4 | Action Reliability | 2 | — | FAIL | 3 of 22 actions crash. Errors expose raw internal IDs (e5g0, adef13, adef6) instead of human-readable names. Action numbering mismatch observed. |
| 5 | Command Self-Documentation | 5 | — | FAIL | Help is comprehensive. Inspect and switch suggest valid names. But trace says `<ID>` not "event ID" — minor gap that gates the score at 5. |
| 6 | Causal Chain Readability | 2 | — | FAIL | Trace shows only the queried event. No parent chain visible. Cannot follow causality at all. |
| | **Average** | **2.8** | **—** | | |

### Prioritized Recommendations

1. **[CRITICAL] Action crashes: declare_support, queue_for_facility_use, staff_market** — 3 actions appear in the list but crash when selected. Either filter them from the action list (if the CLI can't construct their payloads) or produce a clear error. Errors must show agent names, not raw EntityIds/ActionDefIds.
2. **[HIGH] Decision events must name the goal kind** — Decision events show "AgentData: set on X" or "ActiveGoal: set on X" but never say WHICH goal. The tick summary should name the goal for each agent that made a decision (e.g., "Merchant Vara chose ShareBelief(listener=Kael)"). Event deltas for ActiveGoal should show the goal kind.
3. **[HIGH] ActionStarted/Committed events must name the action type** — Events say "ActionStarted by Guard Theron" but not whether it's pick_up, steal, tell, or travel. The action type is critical information. Add it to the event tags or a dedicated field in the event display.
4. **[HIGH] AgentBeliefStore deltas must show what changed** — Social tells produce "AgentBeliefStore: set on X" with no content. Should say what belief was shared (e.g., "learned location of Guard Theron at Thornwall Village" or "heard observation: WitnessedConflict").
5. **[HIGH] Tick summary should name agents, actions, and goals** — Current: "2 started, 1 completed". Needed: "Merchant Vara started Tell(listener=Guard Theron). Guard Theron completed PickUp(5× Bread). Forager Lina chose ConsumeOwnedCommodity(Apple)."
6. **[MEDIUM] Trace command shows shallow chains** — All tested traces show only the queried event with no parent chain. Either the events tested have no parents (root causes), or the trace display doesn't follow parent links. Investigate whether deeper chains exist and ensure they're displayed.
7. **[MEDIUM] Trace error should say "event ID"** — `trace` without args says `<ID>`. Should say `<EVENT_ID>` or explain "provide an event ID from the events list".
8. **[LOW] Action numbering stability** — Action numbers shift between ticks as affordances change. `do 8` was `declare_support` in one list but `bribe` when selected. Consider showing a stable identifier alongside the number, or refresh the list on each `do`.
