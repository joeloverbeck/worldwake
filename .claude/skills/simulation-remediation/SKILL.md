---
name: simulation-remediation
description: "Read the simulation observer report and propose concrete remediations (golden tests, spec changes, tickets). Writes proposals to reports/simulation-remediation.md."
user-invocable: true
---

# Simulation Remediation

Read the simulation observer report and propose concrete remediations for each finding: golden test cases, spec changes, or tickets. Output is proposals only -- nothing is created or modified.

## Invocation

```
/simulation-remediation
```

No arguments. Always reads from `reports/simulation-observer-report.md`.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Read Observer Report

Read `reports/simulation-observer-report.md`.

**Hard gate**: If the file does not exist, tell the user to run `/simulation-observer` first and stop.

### Step 2: Read Context

1. Read `docs/FOUNDATIONS.md` -- needed to evaluate whether findings violate foundational principles.
2. List the `specs/` directory to know which specs exist.
3. List the `tickets/` directory to check for existing related tickets.
4. List and grep `crates/worldwake-ai/tests/golden_*.rs` for existing tests related to the findings -- needed to avoid proposing duplicate golden tests and to reference the `GoldenHarness` setup pattern.
5. Read `docs/spec-drafting-rules.md` -- only if any spec changes will be proposed. Skip if all findings map to golden tests or tickets.

### Step 3: Classify Each Finding

For each finding in the observer report (each smell with severity above NONE), determine the appropriate remediation type:

**Golden Test** -- Use when the finding describes a specific behavioral invariant that should never recur. Propose:
- Test name (following existing `golden_*.rs` naming patterns)
- Which existing test file it belongs in, or whether a new file is needed
- Setup: which agents, scenario conditions, and profiles are needed
- The specific assertion (what to check and at what tick range)
- Reference to similar existing tests in `crates/worldwake-ai/tests/`

**Spec Change** -- Use when the finding points to a design gap (e.g., perception system fires too broadly by design, or a profile parameter has no effect). Propose:
- Which spec file needs updating (path in `specs/`)
- Which section needs the change
- What the change should accomplish
- Whether this needs a new spec instead of modifying an existing one

**Symptom of another finding** -- If a finding is a downstream symptom of a higher-severity finding (e.g., sleep loops caused by dehydration), note it as "deferred to [root finding]" rather than proposing independent remediation. Include these in a "Findings Not Requiring Remediation" table at the end of the report with the reason for deferral. Revisit after the root cause is fixed.

**Ticket** -- Use when the finding points to a concrete bug or missing feature that doesn't require spec-level design work. Propose:
- A ticket title and description
- Acceptance criteria
- Which crate(s) are affected
- Priority (P0-P3)

### Step 4: Write Proposals

Write `reports/simulation-remediation.md` with this structure:

```markdown
# Simulation Remediation Proposals

Source report: `reports/simulation-observer-report.md`
Generated: [date]

## Proposed Golden Tests

### GT-1: [Test Name]
**Source finding**: [reference to observer report finding]
**Severity**: [from observer report]
**File**: `crates/worldwake-ai/tests/golden_[file].rs`
**Setup**: [agents, scenario, profiles needed]
**Assertion**: [what to check]
**Rationale**: [why this test is needed -- what invariant does it protect?]

[Repeat for each proposed test]

## Proposed Spec Changes

### SC-1: [Spec Change Title]
**Source finding**: [reference to observer report finding]
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
**Acceptance criteria**: [how to verify it's fixed]

[Repeat for each proposed ticket]

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | N | N CRITICAL, N HIGH, ... |
| Spec Changes | N | ... |
| Tickets | N | ... |
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
