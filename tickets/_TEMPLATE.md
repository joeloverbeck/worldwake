# <PREFIX-NNN>: <Ticket title>

**Status**: PENDING
**Priority**: <LOW|MEDIUM|HIGH>
**Effort**: <Small|Medium|Large>
**Engine Changes**: <None|Yes — list areas>
**Deps**: <ticket/spec dependencies that currently exist>

## Problem

<What user-facing or architecture problem this solves>

## Assumption Reassessment (<YYYY-MM-DD>)

1. <Assumption checked against current code/test state, including exact existing focused/unit, runtime trace/integration, and golden/E2E coverage where relevant>
2. <Assumption checked against current specs/docs, with exact file reference>
3. <If this is an AI regression: intended layer is candidate generation, runtime `agent_tick`, or golden E2E; if `agent_tick`, state whether local needs-only harness is sufficient or full action registries are required>
4. <If removing/weakening/bypassing a heuristic or filter: name the exact heuristic, the missing substrate it is standing in for today, whether this ticket adds that substrate, and why the change does not reopen unrelated regressions>
5. <If a golden scenario isolates one intended branch from lawful competing affordances: name the isolation choice and which unrelated lawful branches were intentionally excluded from setup>
6. <Mismatch + correction (if any)>

## Architecture Check

1. <Why this approach is cleaner/more robust than alternatives>
2. <No backwards-compatibility aliasing/shims introduced>

## Verification Layers

1. <Invariant> -> <decision trace | action trace | event-log delta | authoritative world state | focused unit/runtime test>
2. <Invariant> -> <verification layer>
3. <If single-layer ticket, state why additional layer mapping is not applicable>

## What to Change

### 1. <Change area>

<Details>

### 2. <Change area>

<Details>

## Files to Touch

- `<path>` (<new|modify>)

## Out of Scope

- <explicit non-goals>

## Acceptance Criteria

### Tests That Must Pass

1. <specific behavior test>
2. <specific behavior test>
3. Existing suite: `<command>`

### Invariants

1. <must-always-hold architectural invariant>
2. <must-always-hold data contract invariant>

## Test Plan

### New/Modified Tests

1. `<path/to/test>` — <short rationale>
2. `<path/to/test>` — <short rationale>

### Commands

1. `<targeted test command>`
2. `<lint/typecheck/full test command>`
3. `scripts/verify.sh` <or explain why a narrower command is the correct verification boundary>
