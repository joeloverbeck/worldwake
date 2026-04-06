# S59EXPOBLSUB-014: ask_about_person negative-response memory

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — missing-person query memory carrier reassessment
**Deps**: S59EXPOBLSUB-008

## Problem

After `ask_about_person` lands, actors can learn positive last-seen information from another agent, but there is still no explicit stored carrier for the negative result “this witness had no sighting to share.” Without an owning ticket, the narrowed-out negative branch from `S59EXPOBLSUB-008` would remain implicit cleanup.

## Assumption Reassessment (2026-04-06)

1. `AskWitnessMemory` currently records only that a counterparty was queried about a topic; it does not distinguish positive from negative answers.
2. `LastSeenMemory` stores only positive sightings; encoding “not seen” into it would collapse distinct meanings into one carrier.
3. No other active S59 ticket currently owns a negative-response memory path for missing-person witness queries.

## Architecture Check

1. A separate ticket keeps `S59EXPOBLSUB-008` honest while preserving explicit ownership for the remaining query-memory design decision.
2. No compatibility shim is implied; the follow-up should choose one canonical negative-response carrier if the feature is still needed.

## Verification Layers

1. To be defined after reassessment against the landed `ask_about_person` runtime surface

## What to Change

### 1. Reassess whether explicit negative witness memory is needed

- Compare the landed `ask_about_person` runtime behavior against AI/planner needs after `S59EXPOBLSUB-008`, `S59EXPOBLSUB-009`, and `S59EXPOBLSUB-011`.
- If explicit negative memory is still required, choose one canonical carrier instead of overloading `LastSeenMemory`.

## Files to Touch

- To be defined after reassessment

## Out of Scope

- Positive last-seen hearsay transfer
- `search_place`
- `report_found`

## Acceptance Criteria

### Tests That Must Pass

1. To be defined after reassessment against the chosen carrier

### Invariants

1. Positive sightings and negative witness responses remain distinct data meanings
2. The repo keeps one canonical memory lane for missing-person witness answers

## Test Plan

### New/Modified Tests

1. To be defined after reassessment

### Commands

1. To be defined after reassessment
