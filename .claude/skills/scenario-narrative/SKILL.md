---
name: scenario-narrative
description: "Run a scenario headlessly via the observer binary and produce a plain-English design-facing narrative report (gameplay mechanics exercised + per-agent narrative). Output is intended for external deep-research synthesis (e.g., ChatGPT Pro game-theory / game-design enrichment), distinct from the diagnostic scenario-analysis skill."
user-invocable: true
---

# Scenario Narrative

Run a single scenario headlessly, read the observer dump, and write a timestamped plain-English narrative report that describes what happened during the run for an external research audience. The report has two principal payloads:

1. The gameplay mechanics that were exercised in this run, with the specific authored substrate and concrete numeric outcomes.
2. A per-agent narrative covering decisions, decision failures and their causes, belief evolution, and inflection moments — sized to demonstrate realism, resourcefulness, and resilience under the scenario's authored circumstances.

This skill is **not** a diagnostic. It does not flag smells, score severity, or propose remediations. The user invokes [`scenario-analysis`](../scenario-analysis/SKILL.md) for that purpose. Both skills can be invoked on the same scenario; they are independent.

## Audience Contract

The deliverable is read by an external LLM (e.g., ChatGPT Pro) doing deep research on game theory, game design, and richer agent AI. The report must therefore be:

- Standalone — comprehensible without access to the codebase or specs.
- Plain English — no code, no Rust types, no `snake_case` symbol names except when naming an action family or feature row that has no readable equivalent.
- Numerically grounded — include thresholds, tick counts, permille values, capacities, and observed counts whenever they materially affect the narrative.
- Free of redundancy — repeat a fact only when reframing it serves the narrative arc.

## Invocation

```
/scenario-narrative scenarios/survival-patrol.ron
/scenario-narrative scenarios/survival-trade.ron --ticks 1440
/scenario-narrative scenarios/final-integration.ron --days 1
```

- **First argument**: path to a `.ron` scenario file (required). If absent, glob `scenarios/*.ron` and present the list.
- **`--ticks N`** overrides the 1440-tick (one simulated day) default.
- **`--days N`** is sugar for `N*1440`.

## Output

Timestamped report at `reports/scenario-narrative-<scenario-stem>-<YYYYMMDD-HHMMSS>.md`. Each invocation writes a new file; prior runs are never overwritten.

The intermediate observer dump at `reports/scenario-narrative-dump.md` is deleted after the report is written. If a Hybrid traceability fix was applied or tickets were created, those are noted in a closing "Run Notes" appendix in the report itself.

## Process

Follow these steps in order. Do not skip any step.

1. **Pre-flight scan** of the `.ron` — load `references/observer-run.md`. Extract authored intent comment, agents (names + profiles), places + topology, resources/facilities, survival-health contract if any, authored seed.

2. **Build & run observer** — same reference. Hard gates on build failure and on missing/empty dump.

3. **Read the dump** for narrative extraction (not anomaly detection) — same reference. Build the EntityId → name map from Section 1 of the dump.

4. **Map exercised gameplay features** — load `references/gameplay-feature-mapping.md`. Cross-reference dump signals against the live feature catalog from `docs/scenario-roadmap.md` to identify which feature rows fired in this run, which authored substrate enabled each, and which numeric outcomes the run produced.

5. **Per-agent narrative pass** — load `references/agent-narrative-structure.md`. For each agent, walk the dump in the prescribed order (starting state → goals → committed actions → failed plans → belief evolution → inflection ticks → final state). Use the fixed decision-failure vocabulary defined there.

6. **Hybrid traceability decision** — load `references/traceability-fix-protocol.md`. For any narrative section that lacks data to write honestly, follow the cheap-fix vs. structural-fix decision tree. Hard cap: at most one inline observer fix per invocation.

7. **Cross-agent / emergent phenomena pass** — only if the run produced multi-agent interactions worth narrating (witness chains, contention episodes, trade exchanges, social transfers, hostile encounters, escort handoffs).

8. **Compose the report** — load `references/report-template.md`. Sections A, B, C, and E always present; Section D and the Run Notes appendix conditional. Write to the timestamped output path.

9. **Clean up** — delete `reports/scenario-narrative-dump.md`. The narrative report is the deliverable. Tickets, if any, remain in `tickets/`.

10. **Final summary to user** — report path, any tickets created, any inline observer fix applied (with the affected file and a one-line description).

## Hard Gates

- If the observer build fails, stop and report. Do not proceed.
- If the dump is missing or empty after the observer run, stop and report. Do not proceed.
- If the scenario `.ron` cannot be parsed, stop and report.
- If a second cheap traceability fix is needed in the same run, the second one is reclassified as structural and routed to a ticket. Inline patches are capped at one per invocation.

## Guardrails

- This skill never adapts or rationalizes simulation behavior. It reports what occurred.
- Numeric values quoted in the report must trace to dump rows, scenario fields, or authoritative event records — never invented or estimated.
- Plain English does not mean vague. "The merchant staged apples for sale at tick 42" beats "the merchant did some trading early on."
- The report may name a feature row as "authored but inactive" but it must not claim activation that the dump cannot substantiate.
- The report's "Realism / Resourcefulness / Resilience" section must ground each lens in 2–4 specific moments captured earlier in the same report, not in generic claims.

## When This Skill Is The Wrong Tool

- The user wants smell detection, severity ratings, or root-cause classification → use `scenario-analysis`.
- The user wants a multi-scenario design brief covering several `.ron` files at once → write a hand-authored editorial doc instead; this skill produces single-scenario reports.
- The user wants to verify a scenario lands a specific roadmap row → use the corresponding golden test, not this skill.
