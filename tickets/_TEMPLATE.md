# <PREFIX-NNN>: <Ticket title>

**Status**: PENDING
**Priority**: <LOW|MEDIUM|HIGH>
**Effort**: <Small|Medium|Large>
**Engine Changes**: <None|Yes — list areas>
**Deps**: <ticket/spec dependencies that currently exist>

## Problem

<What user-facing or architecture problem this solves>

## Assumption Reassessment (<YYYY-MM-DD>)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. <Assumption checked against current code/test state, including exact existing focused/unit, runtime trace/integration, and golden/E2E coverage where relevant>
2. <Assumption checked against current specs/docs, with exact file reference>
3. <If this is an AI regression: intended layer is candidate generation, runtime `agent_tick`, or golden E2E; if `agent_tick`, state whether local needs-only harness is sufficient or full action registries are required>
4. <If the ticket depends on ordering: name the ordering layer, whether the compared branches are symmetric in the current architecture, and whether the divergence depends on priority class, motive score, suppression/filtering, delayed system resolution, or a mixed-layer combination>
5. <If removing/weakening/bypassing a heuristic or filter: name the exact heuristic, the missing substrate it is standing in for today, whether this ticket adds that substrate, and why the change does not reopen unrelated regressions>
6. <If this is a stale-request, contested-affordance, or start-failure ticket: name the first failure boundary and the exact shared runtime symbols checked during reassessment>
7. <If this is a political office-claim ticket: name the exact closure boundary being asserted (support declaration / visible-vacancy loss / succession resolution / office-holder mutation) and the exact AI-layer + authoritative-layer symbols checked>
8. <If the ticket manipulates ControlSource, queued inputs, driver resets, or other runtime conditions: state whether retained runtime intent can still lawfully continue and which exact runtime/trace symbols prove that>
9. <If a golden scenario isolates one intended branch from lawful competing affordances: name the isolation choice and which unrelated lawful branches were intentionally excluded from setup>
10. <Mismatch + correction (if any)>
11. <If the scenario depends on authoritative arithmetic or cumulative state: state the concrete delta/cadence/threshold/capacity math that makes it reachable under current code, plus the survivability or failure envelope when repeated accumulation is material>

## Architecture Check

1. <Why this approach is cleaner/more robust than alternatives>
2. <No backwards-compatibility aliasing/shims introduced>

## Verification Layers

1. <Invariant> -> <decision trace | action trace | event-log delta | authoritative world state | focused unit/runtime test>
2. <Invariant> -> <verification layer>
3. <If this is a stale-request, contested-affordance, or start-failure ticket: map request resolution, authoritative start/abort, and AI recovery to distinct proof surfaces where applicable>
4. <If delayed authoritative effects exist, state why they are not being used as a proxy for earlier action/planning ordering, or justify why that later layer is itself the contract>
5. <If single-layer ticket, state why additional layer mapping is not applicable>

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
3. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.` <use this instead when no tests change>

### Commands

1. `<targeted test command>`
2. `<lint/typecheck/full test command>`
3. `scripts/verify.sh` <or explain why a narrower command is the correct verification boundary>
