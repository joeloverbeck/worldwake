---
name: golden-gap-analysis
description: Post-implementation golden E2E coverage gap analysis — identifies missing cross-system emergent scenarios after a spec is implemented, creates an S-series spec for recommended additions
---

# Golden Gap Analysis Skill

Analyzes golden E2E test coverage after a spec has been implemented, identifies meaningful gaps, and creates an S-series spec for recommended new golden scenarios that demonstrate emergent cross-system behavior.

## Invocation

```
/golden-gap-analysis <spec-identifier>
```

Example: `/golden-gap-analysis E19`

The argument is the spec identifier (e.g., `E19`, `E16b`, `S07`). The skill resolves the full spec filename by matching against `specs/`.

## Phase 0 — Regenerate Coverage Docs

Before any analysis, ensure the generated coverage docs reflect the current codebase:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

If this script fails, **stop and report the error**. Do not analyze stale generated docs.

## Phase 1 — Context Loading

Read ALL of the following files completely:

1. **Completed spec**: Find `specs/*{arg}*.md` (e.g., for `E19`, find `specs/E19-guard-patrol.md`). If multiple matches, list them and ask the user to disambiguate.
2. **Coverage dashboard**: `docs/golden-e2e-coverage.md` — pay special attention to:
   - "Evaluated and Rejected Scenarios" section
   - "Removed Backlog Items" section
   - "Pending Backlog Summary" section
3. **Test inventory**: `docs/generated/golden-e2e-inventory.md`
4. **Scenario map**: `docs/generated/golden-scenario-map.md`
5. **Coverage matrix**: `docs/generated/golden-coverage-matrix.md`
6. **Foundations**: `docs/FOUNDATIONS.md`

## Phase 2 — Gap Analysis

### Step 1: Identify What the Spec Introduced

From the completed spec and recent commits, enumerate:
- New `GoalKind` variants
- New `ActionDomain` entries
- New action definitions and handlers
- New components or relations
- New system functions
- New planner operations (`PlannerOpKind`)
- Cross-system interactions the spec enables

### Step 2: Cross-Reference Against Coverage Matrix

Using `docs/generated/golden-coverage-matrix.md`, identify:
- New GoalKinds that appear in zero scenarios
- New ActionDomains with no coverage
- Systems exercised by the spec that have thin coverage
- Foundation principles the spec exercises that lack golden demonstration

### Step 3: Identify Cross-System Emergent Scenarios

The highest-value golden tests are those that demonstrate **emergent behavior across multiple systems**. Look for scenarios where:
- The spec's features interact with existing systems (needs, production, trade, combat, perception, social, offices) to produce behavior that no single system would produce alone
- Agent decisions chain through multiple goal kinds and action domains
- Information flows through perception, rumor, or discovery to trigger downstream decisions
- The scenario would fail if any participating system were removed (true emergence, not coincidence)

### Step 4: Mandatory Deduplication

Before proposing any scenario, verify it is NOT already covered by:
1. **Existing scenarios** in `docs/generated/golden-scenario-map.md`
2. **Rejected scenarios** in the "Evaluated and Rejected Scenarios" section of `docs/golden-e2e-coverage.md`
3. **Removed backlog items** in the "Removed Backlog Items" section of `docs/golden-e2e-coverage.md`

If a proposed scenario exercises the same code paths as an existing or rejected scenario, do not propose it. Explain why it was filtered.

### Step 5: Foundation Alignment Check

For each surviving candidate scenario, verify alignment with `docs/FOUNDATIONS.md`:
- Does it demonstrate Principle 1 (Maximal Emergence)?
- Does it respect Principle 7 (Information Locality) — information reaches agents through traceable paths?
- Does it respect Principle 10 (Belief-Only Planning)?
- Does it respect Principle 12 (System Decoupling)?
- Does it exercise any principle that currently has thin golden coverage?

### Step 6: Honest Assessment

If after deduplication and filtering, no meaningful gaps remain:
- **Report that explicitly**. State which areas were analyzed and why they are already covered.
- **Do NOT create a spec with low-value filler scenarios** just to produce output.
- This is a valid and valuable outcome — it means the coverage is solid.

## Phase 3 — Spec Output

If meaningful gaps were found:

1. **Auto-detect next S-series number**: Scan `specs/S*.md`, find the highest number, increment by 1.
2. **Write the spec** to `specs/S{next}-golden-gaps-{arg}.md` (e.g., `specs/S46-golden-gaps-E19.md`).

The spec MUST include for each proposed scenario:
- **Scenario title and identifier**
- **Description**: What happens, step by step
- **GoalKinds exercised**: Which goal kinds the scenario triggers
- **ActionDomains exercised**: Which action domains are involved
- **Systems exercised**: Which systems interact
- **Setup requirements**: What entities, topology, and state are needed
- **What emergence it demonstrates**: Why this scenario requires multiple systems working together
- **Foundation principle alignment**: Which principles from `docs/FOUNDATIONS.md` it exercises
- **Why it is not a duplicate**: Brief explanation of what distinguishes it from existing coverage

The spec should also include:
- A ticket breakdown with implementable task items
- Replay and conservation test requirements (each primary golden test should have a deterministic replay companion)

## Phase 4 — Summary

1. **Report findings**: List proposed scenarios with one-line summaries
2. **Report spec location**: Where the spec was written
3. **Update coverage dashboard**: If scenarios were proposed, add them to the "Pending Backlog Summary" section of `docs/golden-e2e-coverage.md` with a brief description and cross-reference to the new spec
4. **If no gaps found**: Report the analysis was thorough and coverage is complete for this spec

## Important Rules

- **Never propose scenarios that duplicate existing coverage** — check all three dedup sources
- **Never create filler specs** — if coverage is solid, say so
- **Always regenerate docs first** — stale docs lead to false gap detection
- **Always check FOUNDATIONS.md alignment** — golden tests should demonstrate the simulation's foundational principles
- **Prioritize emergence over exhaustiveness** — one cross-system emergent scenario is worth more than five single-system unit-test-like goldens
- **Include replay companions** — every primary golden test should have a `*_replays_deterministically` variant
- **Follow spec conventions** — match the format and style of existing specs in `specs/`
