---
name: simulation-remediation
description: "Read the simulation observer report and propose concrete remediations (golden tests, spec changes, tickets). Writes proposals to reports/simulation-remediation.md."
user-invocable: true
---

# Simulation Remediation

Read the simulation observer report and propose concrete remediations for each finding: golden test cases, spec changes, or tickets. Output is proposals only -- no tickets, specs, or code are created or modified. The only file written is the proposal report itself.

## Invocation

```
/simulation-remediation
```

No arguments. Always reads from `reports/simulation-observer-report.md`.

## Expected Input Format

The observer report (produced by `/simulation-observer`) must contain:
- **Findings** with severity levels: NONE, LOW, MEDIUM, HIGH, CRITICAL
- **Trace Quality Assessment** table with columns: ID, Limitation, Classification (Actionable / Acceptable trade-off), Rationale
- **Cross-Cutting Patterns** section identifying systemic issues across findings

If the observer report format changes, update these expectations accordingly.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Read Observer Report

Read `reports/simulation-observer-report.md`.

**Hard gate**: If the file does not exist, tell the user to run `/simulation-observer` first and stop.

### Step 2: Read Context

1. Read `docs/FOUNDATIONS.md` -- needed to evaluate whether findings violate foundational principles. If the file exceeds read limits, prioritize sections III (Knowledge, Belief, and Evidence) and IV (Agents, Institutions, and Social Order) as these are most commonly implicated by behavioral smells.
2. List the `specs/` directory to know which specs exist.
3. List the `tickets/` directory to check for existing related tickets.
4. Glob `crates/worldwake-ai/tests/golden_*.rs` to identify existing test files. Then, after reading the observer report findings, grep these files for keywords related to each finding (e.g., test function names, assertion patterns, key terms like `idle`, `travel`, `belief`, `resource`) to avoid proposing duplicate tests and to reference the `GoldenHarness` setup pattern. Batch all keyword searches into a single operation (e.g., delegate to an Explore agent with all finding-related keywords) rather than grepping per-finding sequentially. The agent prompt should list all finding-related keywords grouped by observer report finding number, request test function names and key assertions for each match, and ask for a structured report grouped by keyword category.
5. Read `docs/spec-drafting-rules.md` -- only if any spec changes will be proposed. Skip if all findings map to golden tests or tickets.
6. If `reports/simulation-remediation.md` already exists (from a prior run), read it and note which prior proposals recurred in the current observer report. Flag recurring issues by appending `RECURRING` to the severity field in the output template (e.g., `**Severity**: CRITICAL RECURRING`).
7. Note the Trace Quality Assessment section of the observer report for processing in Step 3b.

### Step 3: Classify Each Finding

For each finding in the observer report (each smell with severity above NONE), determine the appropriate remediation type. Also review the Cross-Cutting Patterns section of the observer report -- use these patterns to identify root-cause relationships between findings and to inform the "Symptom of another finding" classification below.

**Golden Test** -- Use when the finding describes a specific behavioral invariant that should never recur. Propose:
- Test name (following existing `golden_*.rs` naming patterns)
- Which existing test file it belongs in, or whether a new file is needed
- Setup: which agents, scenario conditions, and profiles are needed
- The specific assertion (what to check and at what tick range)
- Reference to similar existing tests in `crates/worldwake-ai/tests/`

**Spec Change** -- Use when the finding points to a design gap (e.g., perception system fires too broadly by design, or a profile parameter has no effect). Propose a new spec when the finding reveals a system-level design gap not covered by any existing spec. Propose modifying an existing spec when the finding points to a missing parameter, uncovered edge case, or incomplete section within an already-specified system. Include:
- Which spec file needs updating (path in `specs/`), or justification for a new spec
- Which section needs the change
- What the change should accomplish
- FOUNDATIONS alignment (which principle(s) the change serves)

**Symptom of another finding** -- If a finding is a downstream symptom of a higher-severity finding (e.g., sleep loops caused by dehydration), note it as "deferred to [root finding]" rather than proposing independent remediation. Include these in a "Findings Not Requiring Remediation" table at the end of the report with the reason for deferral. Revisit after the root cause is fixed. A deferred finding may still warrant a regression-guard golden test if the behavioral invariant it describes should be independently monitored. In this case, propose the golden test AND list the finding as deferred, noting that the test is a regression guard, not a root-cause fix.

**Ticket** -- Use when the finding points to a concrete bug or missing feature that doesn't require spec-level design work. Propose:
- A ticket title and description
- Acceptance criteria
- Which crate(s) are affected
- Priority (P0-P3)
- Dependencies (other proposals this is blocked by, or "none")
- FOUNDATIONS alignment (which principle(s) the fix serves)

### Step 3b: Classify Trace Quality Items

Read the "Trace Quality Assessment" section of the observer report. For each item classified as **Actionable** (or, if the observer report uses the older free-text format without a structured table, identify limitations and recommended additions that would materially improve future analysis):

Apply the same classification logic as Step 3:

**Ticket** -- Use when the item is a concrete engineering task (e.g., "add DeathCause component", "emit affordance snapshot events every N ticks"). Propose with the same format as behavioral tickets (title, description, priority, crate(s), acceptance criteria). Trace-quality tickets default to P2 unless the limitation forced an INCONCLUSIVE assessment on a MEDIUM+ finding or reduced confidence below HIGH on a CRITICAL finding, in which case P1. If a TQ item impacts multiple findings at different severity levels, use the highest-severity finding's confidence reduction for the escalation decision.

**Spec Change** -- Use when the item reveals a design gap requiring spec-level work (e.g., a new observability subsystem, a new profile parameter for perception granularity). Apply the same FOUNDATIONS alignment check -- FND-29 (Debuggability) is the primary principle, but also check FND-10 (Outcomes Leave Aftermath), FND-04 (Persistent Identity), and others as relevant.

**Not warranted** -- If a trace-quality item does not align with any FOUNDATIONS principle or the improvement is purely cosmetic, note it in the "Findings Deferred or Not Requiring Independent Remediation" table with reason "trace-quality item, no FOUNDATIONS alignment."

Items classified as **Acceptable trade-off** in the observer report are skipped -- note them in the deferred table with the observer's rationale.

Tickets and spec changes from trace-quality items use the same format as behavioral proposals. The **Source finding** field references the Trace Quality Assessment item ID (e.g., "TQ-3: Belief acquisition timeline") instead of a behavioral finding number.

### Step 4: Write Proposals

If invoked in plan mode, draft the report content in the plan file. After plan mode exits, read the plan file and write its content verbatim to `reports/simulation-remediation.md` -- do not re-analyze or regenerate the proposals.

Write `reports/simulation-remediation.md` with this structure:

```markdown
# Simulation Remediation Proposals

Source report: `reports/simulation-observer-report.md`
Generated: [date]

## Context

[1-2 paragraph summary: number of agents, places, ticks simulated, dominant failure mode, root cause summary. Frame the report so a reader unfamiliar with this specific run understands why certain findings are deferred and which root cause dominates.]

## Proposed Golden Tests

### GT-1: [Test Name]
**Source finding**: [reference to observer report finding]
**Severity**: [from observer report]
**File**: `crates/worldwake-ai/tests/golden_[file].rs`
**Setup**: [agents, scenario, profiles needed]
**Assertion**: [what to check]
**Rationale**: [why this test is needed -- what invariant does it protect?]
**Existing coverage**: [reference to similar existing tests and why this is not a duplicate]

[Repeat for each proposed test]

## Proposed Spec Changes

### SC-1: [Spec Change Title]
**Source finding**: [reference to observer report finding]
**Severity**: [from observer report source finding]
**Spec**: `specs/[file].md`
**Section**: [which section]
**Change**: [what to add/modify]
**FOUNDATIONS alignment**: [which principle(s) this serves]

[Repeat for each proposed spec change]

## Proposed Tickets

### TK-1: [Ticket Title]
**Source finding**: [reference to observer report finding]
**Priority**: P[0-3]
**Crate(s)**: [affected crates]
**Description**: [what needs to be done]
**Dependencies**: [other proposals this is blocked by, e.g., "blocked by TK-2", or "none"]
**FOUNDATIONS alignment**: [which principle(s) this serves]
**Acceptance criteria**: [how to verify it's fixed]

[Trace-quality tickets from Step 3b use the same format. Source finding references TQ-N IDs from the Trace Quality Assessment.]

[Repeat for each proposed ticket]

## Findings Deferred or Not Requiring Independent Remediation

| Finding | Severity | Reason for Deferral |
|---------|----------|---------------------|
| [Finding name] | [severity] | [why this finding is deferred -- e.g., "downstream symptom of Finding 8, deferred to TK-2 + TK-3"] |

[Include all findings classified as "Symptom of another finding" in Step 3]

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | N | N CRITICAL, N HIGH, ... |
| Spec Changes | N | ... |
| Tickets | N | ... |
| Deferred | N | ... |

### Root Cause Chain

[Map the dependency graph between findings and proposed remediations. Show which findings are root causes and which are downstream symptoms. Suggest implementation order — what to fix first, what can proceed in parallel.]

### FOUNDATIONS Alignment

[For each FOUNDATIONS principle relevant to the findings, state whether the simulation correctly enforces it or violates it. Reference by principle number and name. This validates that proposed remediations don't introduce new FOUNDATIONS violations.]
```

### Step 5: Guardrails

- Do NOT create ticket files in `tickets/`
- Do NOT modify any spec files in `specs/`
- Do NOT write test code in `crates/`
- Do NOT run any commands that modify the codebase
- Output is PROPOSALS ONLY -- a human decides what to act on

## Notes

- Findings with severity LOW may not warrant remediation -- note them but don't force a proposal.
- If a finding is ambiguous (could be a test OR a spec change), propose both and note the trade-off.
- Cross-reference existing golden tests to avoid proposing duplicates. Use `grep` to check if similar assertions already exist.
- For golden test proposals, reference the `GoldenHarness` pattern used in existing tests for setup consistency.
