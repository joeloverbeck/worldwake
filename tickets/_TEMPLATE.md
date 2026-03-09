# <PREFIX-NNN>: <Ticket title>

**Status**: PENDING
**Priority**: <LOW|MEDIUM|HIGH>
**Effort**: <Small|Medium|Large>
**Engine Changes**: <None|Yes — list areas>
**Deps**: <ticket/spec dependencies that currently exist>

## Problem

<What user-facing or architecture problem this solves>

## Assumption Reassessment (<YYYY-MM-DD>)

1. <Assumption checked against current code/test state>
2. <Assumption checked against current specs>
3. <Mismatch + correction (if any)>

## Architecture Check

1. <Why this approach is cleaner/more robust than alternatives>
2. <No backwards-compatibility aliasing/shims introduced>

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

### Commands

1. `<targeted test command>`
2. `<lint/typecheck/full test command>`
