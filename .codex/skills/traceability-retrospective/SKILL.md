---
name: traceability-retrospective
description: "Reflect on a just-finished Worldwake implementation session to identify traceability gaps, debugging blind spots, setup surprises, or documentation shortfalls. Use when you want a same-session retrospective that checks `docs/FOUNDATIONS.md`, triages whether any lesson warrants action, and proposes or creates follow-up ticket/doc work only when the evidence is strong."
---

# Traceability Retrospective

Use this skill after finishing an implementation when you want to capture lessons about debugging surfaces, traceability gaps, misleading setup assumptions, or documentation shortfalls.

The default is to do nothing. Not every implementation difficulty warrants a ticket, a doc note, or a file edit. The burden of proof is on creating follow-up work.

This skill is tuned for Worldwake's debugging surfaces:
- decision traces
- action traces
- event log
- belief state
- perception pipeline
- conservation checks

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [tickets/README.md](../../../tickets/README.md), and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) before acting on findings.

## Preconditions

- Use this only in the same Codex session as the implementation work. It depends on actual session evidence: failed attempts, confusing behavior, trace gaps, and setup surprises encountered while implementing.
- The referenced ticket must exist in `tickets/` or `archive/tickets/`.

## Workflow

### 1. Load context

1. Resolve the implemented ticket from the provided ticket id or ticket path.
2. If multiple tickets match, stop and ask the user to disambiguate.
3. Read the implemented ticket from `tickets/` or `archive/tickets/`.
4. Read [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md) if it was not already read earlier in this session.
5. Read [tickets/README.md](../../../tickets/README.md) and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md).
6. List active tickets in `tickets/` so duplicates can be checked later.
7. If the implemented ticket is golden-test heavy, also read [docs/golden-e2e-testing.md](../../../docs/golden-e2e-testing.md).

### 2. Reflect on the implementation session

Review the current session and extract lessons only from what actually happened.

Use these lenses:
- difficulties that took multiple attempts or required backing out
- confusions where assumptions did not match the live code
- failures such as compile breaks, failing tests, or wrong runtime behavior
- traceability gaps where expected debug surfaces were absent, weak, or misleading
- edge cases or cross-system interactions that surfaced unexpectedly
- documentation gaps or stale guidance
- setup calibration problems in golden work: scenario parameters, topology, tick budgets, agent placement, belief seeding or local-knowledge calibration, and similar setup assumptions

Optionally note what worked on the first attempt, but keep that brief and only as context for the harder lessons.

For each lesson, record:
- **What happened**
- **Why it was non-obvious**
- **Impact**

Keep lessons atomic. One lesson per item.

If no lessons surface, report that the implementation matched the ticket assumptions and the verification flow worked cleanly, then continue only with the drift check in Step 3.

### 3. Check ticket drift

Compare the ticket's `What to Change` and `Assumption Reassessment` against what was actually implemented.

Focus on drift that changes the architectural meaning of the ticket:
- different proof surfaces
- different scenario structure
- omitted or added actors, systems, or invariants
- changed setup that materially alters what the test or implementation proves

Minor calibration changes that do not change the architectural claim can be noted briefly without treating them as meaningful drift.

If significant drift exists, capture it as:
- **Ticket says**
- **Implementation does**
- **Section that should be updated**

If significant drift existed at the start of implementation but was corrected before coding and the final ticket now matches the implementation, report that explicitly as pre-implementation drift that was already corrected rather than treating the drift check as simply empty.

If no significant drift exists, say so and move on.

### 4. Triage each lesson

For each lesson from Step 2, apply these checks in order.

1. **Recurrence**
   - If this was a one-off mistake that is unlikely to recur, route it to `DO NOTHING`.
2. **Already addressed**
   - If the implementation already fixed the underlying issue rather than just the local symptom, route it to `DO NOTHING`.
3. **Duplicate coverage**
   - Check active tickets in `tickets/` for existing coverage. If the concern is already covered, route it to `COVERED`.
4. **FOUNDATIONS alignment**
   - Check whether the lesson reveals a direct principle violation, a missing infrastructure hook, a missing information path, or a missing debuggability surface.
   - Cite exact principle numbers when applicable.
5. **Change type**
   - Classify surviving items as `DOC NOTE`, `TICKET WARRANTED`, or `MULTIPLE`.

Use skepticism. Lessons that do not connect to recurrence, architectural risk, or `FOUNDATIONS` alignment are unlikely to justify follow-up work.

### 5. Decide the follow-up surface

Use these buckets:
- `DO NOTHING`: no follow-up warranted
- `DOC NOTE`: a small change to an existing guide or ticket would prevent future confusion
- `COVERED`: already handled by an existing active ticket
- `TICKET WARRANTED`: a new follow-up ticket is justified

For each lesson, output:
- bucket
- reason
- `FOUNDATIONS` references when applicable

If all lessons route to `DO NOTHING` or `COVERED`, and there is no meaningful drift, report that explicitly and stop.

If only `DOC NOTE` items survive and no ticket is warranted, present the proposed doc notes and any drift corrections, then wait for approval before editing files.

If any `TICKET WARRANTED` items survive, proceed to Step 6.

### 6. Prepare proposals

Present all proposed changes in one message and wait for explicit approval before editing files.

Order:
1. drift corrections to the implemented ticket, if any
2. doc notes, if any
3. ticket proposals

For each warranted ticket, include:
- title
- problem
- change type
- `FOUNDATIONS` alignment
- likely affected files
- estimated effort

Prefer one lesson per ticket. If multiple lessons are genuinely coupled, explain the coupling instead of silently bundling them.

### 7. Apply approved changes

After approval:

1. Apply approved ticket drift corrections to the implemented ticket.
2. Apply approved doc notes to their target files.
3. Create approved tickets in `tickets/` using [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) and [tickets/README.md](../../../tickets/README.md).
4. Follow [docs/precision-rules.md](../../../docs/precision-rules.md) for technical claims in new or updated tickets.
5. Re-check new tickets against current code and docs before finalizing them.

For created tickets:
- keep scope narrow and evidence-backed
- name exact symbols, invariants, and proof surfaces
- cite the relevant `FOUNDATIONS` principles
- avoid duplicating existing active tickets

If a doc note affects `docs/golden-e2e-testing.md` in a way that impacts scenario metadata or inventory parsing, run:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

## Report Format

Use this structure in the conversation:

```markdown
# Traceability Retrospective: <ticket-id>

**Ticket**: <path>
**Session date**: YYYY-MM-DD

## Lessons

1. **<title>**
   - **What happened**: <evidence>
   - **Why it was non-obvious**: <reason>
   - **Impact**: <implementation impact>
   - **Bucket**: DO NOTHING | DOC NOTE | COVERED | TICKET WARRANTED
   - **FOUNDATIONS**: <principle refs or "none">

## Drift Check

- <no significant drift> or
- <ticket says / implementation does / section to update>

## Proposed Changes

### Drift Corrections

- <only when applicable>

### Doc Notes

- <target file and proposed text>

### Ticket Proposals

- <title, problem, change type, effort, likely files>

## Summary

**Result**: <no action / doc notes proposed / tickets proposed / changes applied>
```

## Guardrails

- Same-session evidence is mandatory. Do not reconstruct lessons from git history alone.
- `DO NOTHING` is a valid and often correct outcome.
- Do not create filler tickets.
- Do not duplicate active ticket coverage.
- Every ticket proposal must cite at least one relevant `FOUNDATIONS` principle or explain why the follow-up is necessary for architectural traceability.
- Wait for explicit approval before editing files or creating tickets.
- Prefer small, focused tickets over umbrella cleanup.
- Keep retrospective findings honest. A clean implementation with no meaningful gaps is valuable signal.
