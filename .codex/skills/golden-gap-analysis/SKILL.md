---
name: golden-gap-analysis
description: "Analyze golden E2E coverage after a Worldwake spec has been implemented. Use when you want Codex to regenerate the golden coverage docs, identify meaningful missing emergent scenarios for a completed spec, reject duplicates or filler ideas, and write a new S-series spec only when real coverage gaps remain."
---

# Golden Gap Analysis

Use this skill after a spec has been implemented when you want to check whether golden E2E coverage is still missing meaningful emergent scenarios.

The goal is not exhaustive scenario accumulation. The goal is to find high-value cross-system emergence that the current golden suite does not yet demonstrate.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), and the relevant spec before writing anything.

## Workflow

### 1. Regenerate the golden coverage docs

Before any analysis, refresh the generated golden coverage docs:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

If this command fails, stop and report the error. Do not analyze stale generated docs.

Exception: if the refresh fails because live `golden_*` source has missing or duplicate `// Scenario` metadata that the inventory tool reports directly, fix that local metadata problem first, rerun the command, and only stop if the refresh still fails or the failure is not clearly a local mechanical annotation issue.

### 2. Load context

1. Resolve the completed spec from the provided spec identifier or spec path.
2. Search `specs/` first, then `archive/specs/` if needed. If the user names a specific canonical spec path, accept that live path wherever it currently resides and note separately whether the spec is still active or already archived instead of treating location alone as the completion signal.
3. If multiple specs match, stop and ask the user to disambiguate.
4. Read the resolved spec completely.
5. If the resolved spec still lives in `specs/` but the implementation is already complete, explicitly decide whether it is:
   - still the active roadmap authority for unfinished behavior, or
   - implemented but stale prose that has not yet been archived or reconciled
   In the second case, prefer the live code, generated golden coverage, and the completed ticket/archive chain over broader unlanded behavior claims in the spec text when judging gaps.
6. Read:
   - [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md)
   - [docs/generated/golden-e2e-inventory.md](../../../docs/generated/golden-e2e-inventory.md)
   - [docs/generated/golden-scenario-map.md](../../../docs/generated/golden-scenario-map.md)
   - [docs/generated/golden-coverage-matrix.md](../../../docs/generated/golden-coverage-matrix.md)
   - [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
7. Review the current codebase state for the implemented spec:
   - goal kinds
   - action domains
   - action definitions and handlers
   - components and relations
   - system functions
   - planner operations
   - cross-system interactions introduced or materially changed by the spec

### 3. Identify what the spec materially introduced

From the completed spec and live code, enumerate:
- new or materially changed `GoalKind` variants
- new or materially changed `ActionDomain` entries
- new actions, validators, or handlers
- new components, relations, or state carriers
- new system functions or planner operations
- new cross-system interaction surfaces

Name exact symbols and files. Do not infer from stale memory.

If the spec primarily adds or migrates a substrate, profile, or other state carrier without introducing new goal families, action surfaces, or planner operations, treat that explicitly as a different analysis shape. In those cases, the main question is whether the live golden suite already proves the spec's core emergent promise, not whether every moved field or read path needs its own golden.

### 4. Cross-check against the coverage matrix

Using [docs/generated/golden-coverage-matrix.md](../../../docs/generated/golden-coverage-matrix.md), identify:
- goal kinds with zero or thin scenario coverage
- action domains with zero or thin coverage
- systems materially exercised by the spec that lack strong golden demonstration
- `FOUNDATIONS` principles the spec now exercises but the golden suite still demonstrates weakly

Focus on meaningful gaps, not mere count imbalances.
If the generated matrix and scenario map disagree, or if newly added scenarios appear with thin metadata, treat the generated artifacts as incomplete for that slice and inspect the owning live `golden_*` source before judging coverage gaps.

### 5. Generate candidate emergent scenarios

Look for scenarios where the implemented spec interacts with existing systems to produce behavior that no single system alone would produce.

High-value candidates usually involve:
- multiple goal kinds or action domains chaining together
- information moving through lawful perception, rumor, or discovery paths
- downstream decisions that depend on belief rather than authoritative omniscience
- failure if one participating system were removed

Before keeping a candidate that depends on a specific `GoalKind`, verify in the live code that candidate generation can actually emit it. Check for filters or suppression paths that would prevent the goal from appearing at runtime.

Before keeping a candidate centered on a spec-defined subtype, artifact kind, or notice topic, verify that the live code already provides both:
- the implemented substrate carrying that fact
- at least one lawful downstream consumer or behavior that makes the fact matter at runtime

If the subtype exists only as stored metadata with no live consumer yet, treat it as an implementation gap or future extension, not as a golden-gap candidate.

### 6. Deduplicate aggressively

For each candidate scenario, verify it is not already covered by:
1. existing scenarios in [docs/generated/golden-scenario-map.md](../../../docs/generated/golden-scenario-map.md)
2. rejected scenarios in [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md)
3. removed backlog items in [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md)
4. relevant live `golden_*` suites when the generated docs alone are too coarse to tell whether the candidate proves a materially distinct contract

If a candidate overlaps an existing or rejected scenario in the same meaningful code path, reject it and explain why.

When a completed spec's illustrative example conflicts with a later rejected backlog item or a stronger live proof-surface decision, prefer the dashboard/live ownership boundary unless the live suite still lacks the broader emergent contract. Treat the spec example as non-binding when reassessment shows it no longer names the strongest honest golden surface.

Also reject a candidate when a recently completed ticket has already resolved the underlying contradiction at a stronger non-golden proof surface, so the remaining scenario would no longer represent a meaningful golden gap.

Do not over-apply that rule. Lower-layer proof only closes a candidate when it already owns the meaningful contract. If the completed spec still promises a broader cross-system emergence and that emergence is not yet shown in the live golden suite, the candidate may remain a valid golden gap even when focused tests are strong.

### 7. Check FOUNDATIONS alignment

For each surviving candidate, verify it aligns with [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).

Check especially:
- Principle 1: Maximal Emergence Through Local Causality
- Principle 3: Concrete State Over Abstract Scores
- Principle 7: Locality of Motion, Interaction, and Communication
- Principle 10: Belief-Only Planning
- Principle 12: System Decoupling

Also note any other principles the scenario would demonstrate particularly well.

Reject candidates that would require cheating, omniscient setup assumptions, or non-causal authoring.

### 8. Make an honest decision

If no meaningful, non-duplicative, `FOUNDATIONS`-aligned gaps remain:
- report that explicitly
- summarize the areas analyzed
- state why the remaining possibilities were rejected
- stop without writing a spec

Do not create filler specs or low-value scenarios just to produce output.

### 9. Write the gap spec when warranted

If meaningful gaps remain:

1. Scan `specs/S*.md` to find the highest existing S-series number.
2. Write the next spec to `specs/S{next}-golden-gaps-<spec-id>.md`.
3. Match the format and tone of the existing spec set.

For each proposed scenario, include:
- scenario title and identifier
- description of what happens
- goal kinds exercised
- action domains exercised
- systems exercised
- setup requirements
- what emergence it demonstrates
- `FOUNDATIONS` alignment
- why it is not a duplicate

Also include:
- an implementable task breakdown
- replay and conservation requirements
- deterministic replay companion expectation for each primary golden scenario

### 10. Update the coverage dashboard

If a new gap spec was written:
1. Add the proposed scenarios to the pending backlog in [docs/golden-e2e-coverage.md](../../../docs/golden-e2e-coverage.md).
2. If the dashboard still lists an older pending gap whose spec is now archived or whose scenarios are already live in the suite, correct that stale pending entry first by moving it to the removed/completed backlog notes before adding the new gap.
3. Cross-reference the new spec.
4. If existing golden tests relevant to this area lack `// Scenario` metadata headers, add the missing headers with non-colliding scenario ids so the inventory tooling can track them.

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

## Dashboard Updates

- <coverage dashboard edits and scenario-header edits, if any>
```

## Guardrails

- Always regenerate the golden coverage docs first.
- Never analyze stale generated docs.
- Prioritize emergent cross-system behavior over exhaustive feature checklists.
- Never propose scenarios that duplicate existing, rejected, or removed coverage.
- Never create a gap spec when the honest result is that coverage is already solid.
- Every proposed scenario must be grounded in live code, not just the completed spec text.
- Every proposed scenario must explain why it exercises multiple systems and why that emergence matters.
- Every proposed scenario must align with [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
- Include deterministic replay companions for primary golden scenarios.
