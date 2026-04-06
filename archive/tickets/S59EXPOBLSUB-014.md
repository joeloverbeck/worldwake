# S59EXPOBLSUB-014: ask_about_person negative-response memory

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — reassessment confirmed the current branch does not need a new negative-response carrier
**Deps**: S59EXPOBLSUB-008

## Problem

After `ask_about_person` lands, actors can learn positive last-seen information from another agent, but there is still no explicit stored carrier for the negative result “this witness had no sighting to share.” Without an owning ticket, the narrowed-out negative branch from `S59EXPOBLSUB-008` would remain implicit cleanup.

## Assumption Reassessment (2026-04-06)

1. `AskWitnessMemory` currently records only that a counterparty was queried about a topic; it does not distinguish positive from negative answers.
2. `LastSeenMemory` stores only positive sightings; encoding “not seen” into it would collapse distinct meanings into one carrier.
3. No other active S59 ticket currently owns a negative-response memory path for missing-person witness queries.
4. Live `ask_about_person` behavior in `crates/worldwake-systems/src/ask_about_person_actions.rs` already records the only currently-used negative branch: the actor stores `AskWitnessMemory` even when the witness has no `LastSeenRecord`, and the actor's `LastSeenMemory` remains unchanged.
5. Reassessment across `S59EXPOBLSUB-009`, `S59EXPOBLSUB-011`, `crates/worldwake-ai/src/goal_model.rs`, and `crates/worldwake-ai/src/search/candidates.rs` found no current consumer that requires a richer “witness had no information” result than the existing ask-memory suppression lane.

## Architecture Check

1. `S59EXPOBLSUB-008` remained honest by leaving the negative branch narrow: query suppression is stored in the existing `AskWitnessMemory` lane, while positive sightings continue to use `LastSeenMemory`.
2. No compatibility shim is implied. If a later ticket needs durable semantic distinction between “asked recently” and “asked and got a negative answer,” that ticket should introduce one canonical carrier at the point of first real consumption.

## Verification Layers

1. Existing `ask_about_person` negative branch remains lawful: focused runtime proof that no-record witness responses leave `LastSeenMemory` unchanged while recording `AskWitnessMemory`
2. No current AI/planner consumer requires a distinct negative-response carrier: code/ticket audit against `S59EXPOBLSUB-009`, `S59EXPOBLSUB-011`, `goal_model.rs`, and `search/candidates.rs`

## What to Change

### 1. Reassess whether explicit negative witness memory is needed

- Compare the landed `ask_about_person` runtime behavior against AI/planner needs after `S59EXPOBLSUB-008`, `S59EXPOBLSUB-009`, and `S59EXPOBLSUB-011`.
- Record the honest outcome for the current branch: no new carrier lands now because the only live need is already satisfied by `AskWitnessMemory` duplicate-query suppression.

## Files to Touch

- `tickets/S59EXPOBLSUB-014.md` (reassessment + close-out)

## Out of Scope

- Positive last-seen hearsay transfer
- `search_place`
- `report_found`
- Inventing a richer negative-response carrier before a live consumer exists

## Acceptance Criteria

### Tests That Must Pass

1. `ask_about_person` with no witness record still records `AskWitnessMemory` and leaves `LastSeenMemory` unchanged
2. Reassessment confirms no current active S59 ticket or live AI search surface requires a distinct negative-response memory carrier

### Invariants

1. Positive sightings and negative witness responses remain distinct data meanings
2. The repo keeps one canonical current memory lane for missing-person witness-query suppression: `AskWitnessMemory`

## Test Plan

### New/Modified Tests

1. No new tests; reuse the focused `ask_about_person` no-record proof from `S59EXPOBLSUB-008`

### Commands

1. `cargo test -p worldwake-systems ask_about_person_without_record_leaves_last_seen_unchanged_but_records_query`
2. `rg -n "AskWitnessMemory|ask_about_person|SearchForMissing|ReportMissing" tickets/S59EXPOBLSUB-009.md tickets/S59EXPOBLSUB-011.md crates/worldwake-ai/src/goal_model.rs crates/worldwake-ai/src/search/candidates.rs`

## Outcome

Completed on 2026-04-06 after reassessment. No production change was needed: the current branch already stores the only live negative witness-query outcome through `AskWitnessMemory` cooldown/suppression, and no active S59 search/candidate surface currently consumes a richer negative-answer carrier.

## Verification Result

- Passed: `cargo test -p worldwake-systems ask_about_person_without_record_leaves_last_seen_unchanged_but_records_query`
- Passed: `rg -n "AskWitnessMemory|ask_about_person|SearchForMissing|ReportMissing" tickets/S59EXPOBLSUB-009.md tickets/S59EXPOBLSUB-011.md crates/worldwake-ai/src/goal_model.rs crates/worldwake-ai/src/search/candidates.rs`
