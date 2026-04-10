---
name: golden-gap-analysis
description: "Analyze golden E2E coverage after a Worldwake spec has been implemented. Use when you want Codex to regenerate the golden coverage docs, identify meaningful missing emergent scenarios for a completed spec, reject duplicates or filler ideas, and write a new S-series spec only when real coverage gaps remain."
---

# Golden Gap Analysis Skill

Analyzes golden E2E test coverage after a spec has been implemented, identifies meaningful gaps, and creates an S-series spec for recommended new golden scenarios that demonstrate emergent cross-system behavior.

The goal is not exhaustive scenario accumulation. The goal is to find high-value cross-system emergence that the current golden suite does not yet demonstrate.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), and the relevant spec before writing anything.

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

Exception: if the refresh fails because live `golden_*` source has missing or duplicate `// Scenario` metadata that the inventory tool reports directly, fix that local metadata problem first, rerun the command, and only stop if the refresh still fails or the failure is not clearly a local mechanical annotation issue.

Exception: if the refresh fails because the golden test target or its dependencies do not compile, diagnose and fix the build error before retrying. Use systematic debugging if the cause is not immediately obvious. Only stop if the build failure persists after investigation or is unrelated to the golden test target.

## Phase 1 — Context Loading

Read ALL of the following files completely:

1. **Completed spec**: Find `specs/*{arg}*.md` or `archive/specs/*{arg}*.md`. Search `specs/` first, then `archive/specs/` if needed. If the user provides a specific canonical spec path, accept that path wherever the spec currently resides and note separately whether it is still active or already archived — do not treat location alone as the completion signal. If multiple matches, list them and ask the user to disambiguate.
2. **Stale spec assessment**: If the resolved spec still lives in `specs/` but the implementation is already complete, explicitly decide whether it is (a) still the active roadmap authority for unfinished behavior, or (b) implemented but stale prose that has not yet been archived or reconciled. Check `archive/tickets/` for the spec's decomposed tickets — if all tickets are completed and archived, the spec's implemented scope is defined by those tickets, not by the broader spec prose. In the second case, prefer live code, generated golden coverage, and the completed ticket/archive chain over broader unlanded behavior claims in the spec text when judging gaps.
3. **Coverage dashboard**: [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md) — pay special attention to:
   - "Evaluated and Rejected Scenarios" section
   - "Removed Backlog Items" section
   - "Pending Backlog Summary" section
4. **Test inventory**: [docs/generated/golden-e2e-inventory.md](../../../docs/generated/golden-e2e-inventory.md)
5. **Scenario index**: [docs/generated/golden-scenario-index.md](../../../docs/generated/golden-scenario-index.md) (gameplay overview) and per-file details in [docs/generated/golden-scenario-details/](../../../docs/generated/golden-scenario-details/)
6. **Coverage matrix**: [docs/generated/golden-coverage-matrix.md](../../../docs/generated/golden-coverage-matrix.md)
7. **Foundations**: [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
8. **Codebase review**: Review the current codebase state for the implemented spec — goal kinds, action domains, action definitions and handlers, components and relations, system functions, planner operations, and cross-system interactions introduced or materially changed by the spec.

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

Name exact symbols and files. Do not infer from stale memory.

For each new GoalKind variant, verify that `candidate_generation.rs` has an emission path (e.g., `emit_*_candidates`). GoalKind variants without candidate generation cannot be tested in golden E2E scenarios — note these as implementation gaps, not golden gaps.

If the spec primarily adds or migrates a substrate, profile, or other state carrier without introducing new goal families, action surfaces, or planner operations, treat that explicitly as a different analysis shape. In those cases, the main question is whether the live golden suite already proves the spec's core emergent promise, not whether every moved field or read path needs its own golden.

### Step 2: Cross-Reference Against Coverage Matrix

Using [docs/generated/golden-coverage-matrix.md](../../../docs/generated/golden-coverage-matrix.md), identify:
- New GoalKinds that appear in zero scenarios
- New ActionDomains with no coverage
- Systems exercised by the spec that have thin coverage
- Foundation principles the spec exercises that lack golden demonstration

Focus on meaningful gaps, not mere count imbalances. If the generated matrix and scenario map disagree, or if newly added scenarios appear with thin metadata, treat the generated artifacts as incomplete for that slice and inspect the owning live `golden_*` source before judging coverage gaps.

### Step 3: Identify Cross-System Emergent Scenarios

The highest-value golden tests are those that demonstrate **emergent behavior across multiple systems**. Look for scenarios where:
- The spec's features interact with existing systems (needs, production, trade, combat, perception, social, offices) to produce behavior that no single system would produce alone
- Agent decisions chain through multiple goal kinds and action domains
- Information flows through perception, rumor, or discovery to trigger downstream decisions
- The scenario would fail if any participating system were removed (true emergence, not coincidence)

**GoalKind existence verification**: Before proposing a scenario that depends on a specific goal, first verify the `GoalKind` variant exists in `goal.rs`. If the action and `PlannerOpKind` exist but the `GoalKind` does not, the scenario cannot be autonomously planned — treat it as an implementation gap, not a golden gap candidate.

**Candidate emission verification**: Before proposing a scenario that depends on a specific `GoalKind` being generated, grep `candidate_generation.rs` for that goal kind and verify there are no suppression filters (e.g., `sale_kinds` exclusions, `already_satisfied` gates, `evaluate_suppression` checks) that would prevent emission. A scenario that assumes a goal kind is available but the candidate generation silently filters it will fail at runtime with 0 candidates and no diagnostic output.

**Subtype and artifact kind verification**: Before keeping a candidate centered on a spec-defined subtype, artifact kind, or notice topic, verify that the live code already provides both: (a) the implemented substrate carrying that fact, and (b) at least one lawful downstream consumer or behavior that makes the fact matter at runtime. If the subtype exists only as stored metadata with no live consumer yet, treat it as an implementation gap or future extension, not a golden-gap candidate.

### Step 4: Mandatory Deduplication

Before proposing any scenario, verify it is NOT already covered by:
1. **Existing scenarios** in [docs/generated/golden-scenario-index.md](../../../docs/generated/golden-scenario-index.md) and per-file details in [docs/generated/golden-scenario-details/](../../../docs/generated/golden-scenario-details/)
2. **Rejected scenarios** in the "Evaluated and Rejected Scenarios" section of [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md)
3. **Removed backlog items** in the "Removed Backlog Items" section of [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md)
4. **Live golden suites** — when the generated docs alone are too coarse to tell whether the candidate proves a materially distinct contract, inspect the relevant live `golden_*` source directly

If a proposed scenario exercises the same code paths as an existing or rejected scenario, do not propose it. Explain why it was filtered.

When a completed spec's illustrative example conflicts with a later rejected backlog item or a stronger live proof-surface decision, prefer the dashboard/live ownership boundary unless the live suite still lacks the broader emergent contract. Treat the spec example as non-binding when reassessment shows it no longer names the strongest honest golden surface.

Also reject a candidate when a recently completed ticket has already resolved the underlying contradiction at a stronger non-golden proof surface, so the remaining scenario would no longer represent a meaningful golden gap.

Do not over-apply that rule. Lower-layer proof only closes a candidate when it already owns the meaningful contract. If the completed spec still promises a broader cross-system emergence and that emergence is not yet shown in the live golden suite, the candidate may remain a valid golden gap even when focused tests are strong.

### Step 5: Foundation Alignment Check

For each surviving candidate scenario, verify alignment with [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md):
- Does it demonstrate Principle 1 (Maximal Emergence)?
- Does it demonstrate Principle 3 (Concrete State Over Abstract Scores)?
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
- **Foundation principle alignment**: Which principles from [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md) it exercises
- **Why it is not a duplicate**: Brief explanation of what distinguishes it from existing coverage

The spec should also include:
- A ticket breakdown with implementable task items
- Replay and conservation test requirements (each primary golden test should have a deterministic replay companion)

## Phase 4 — Summary

1. **Report findings**: List proposed scenarios with one-line summaries
2. **Report spec location**: Where the spec was written
3. **Update coverage dashboard**: If scenarios were proposed, add them to the "Pending Backlog Summary" section of [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md) with a brief description and cross-reference to the new spec. Before adding new pending entries, check if the dashboard still lists older pending gaps whose specs are now archived or whose scenarios are already live in the suite — correct those stale pending entries first by moving them to the removed/completed backlog notes.
4. **Add missing scenario headers**: If existing golden tests for the spec lack `// Scenario` metadata headers, add them so the inventory script can track coverage. Assign sequential scenario IDs that don't collide with existing IDs.
5. **If no gaps found**: Report the analysis was thorough and coverage is complete for this spec

## Report Format

Use this structure in the conversation:

```markdown
# Golden Gap Analysis: <spec-id>

**Spec**: <path>
**Analysis date**: YYYY-MM-DD

## Coverage Refresh

- <command status>

## What The Spec Introduced

- <exact symbols, files, and interaction surfaces>

## Candidate Gaps Reviewed

1. **<candidate title>**
   - **Why it looked promising**: <emergent interaction>
   - **Dedup result**: <new / duplicate / rejected>
   - **FOUNDATIONS**: <principles exercised>
   - **Decision**: <keep / reject>

## Outcome

- <no meaningful gaps found> or
- <new spec written at path>

## Proposed Scenarios

- <only when a spec was written>

## Deferred Candidates

- <candidates that are valid but blocked by implementation gaps or deferred for complexity — optional section>

## Dashboard Updates

- <coverage dashboard edits and scenario-header edits, if any>
```

## Guardrails

- **Never propose scenarios that duplicate existing coverage** — check all four dedup sources
- **Never create filler specs** — if coverage is solid, say so
- **Always regenerate docs first** — stale docs lead to false gap detection
- **Always check FOUNDATIONS.md alignment** — golden tests should demonstrate the simulation's foundational principles
- **Prioritize emergence over exhaustiveness** — one cross-system emergent scenario is worth more than five single-system unit-test-like goldens
- **Every proposed scenario must be grounded in live code** — not just the completed spec text
- **Every proposed scenario must explain why it exercises multiple systems** and why that emergence matters
- **Include replay companions** — every primary golden test should have a `*_replays_deterministically` variant
- **Reject non-causal scenarios** — never propose scenarios that require cheating, omniscient setup assumptions, or non-causal authoring
- **Follow spec conventions** — match the format and style of existing specs in `specs/`
