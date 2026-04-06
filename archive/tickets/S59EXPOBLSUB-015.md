# S59EXPOBLSUB-015: reconcile active S59 spec with landed ask_about_person boundary

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — active spec and roadmap alignment only
**Deps**: S59EXPOBLSUB-008

## Problem

`S59EXPOBLSUB-008` landed a narrower lawful runtime boundary for `ask_about_person`: direct missing-subject payloads from overdue expectations, positive `LastSeenMemory` hearsay transfer, and no Tell-based last-seen carrier or stored `SearchTarget`. The active S59 spec still describes the older `SearchTarget` + Tell model, so the live roadmap no longer matches the implemented branch.

## Assumption Reassessment (2026-04-06)

1. The landed action in `crates/worldwake-systems/src/ask_about_person_actions.rs` validates overdue expectations directly and carries `{ target, subject }` in `AskAboutPersonActionPayload`.
2. `worldwake-sim/src/action_payload.rs` and `action_trace.rs` now expose a dedicated `AskAboutPersonActionPayload` / `ActionTraceDetail::AskAboutPerson` path rather than reusing `TellTopic`.
3. The active spec at `specs/S59-expectation-obligation-substrate.md:212-221` still says `ask_about_person` requires a `SearchTarget` and shares last-seen information via the existing Tell mechanism.
4. `tickets/S59EXPOBLSUB-009.md` has already been corrected to stop depending on a nonexistent stored `SearchTarget`, and `S59EXPOBLSUB-014` is archived with the negative-response memory question closed narrowly, so the remaining drift is the active spec text rather than the active ticket chain.
5. The active spec still declares `SearchTarget` in its type sketch even though no such shared type landed; this is part of the same doc drift and should be removed in-scope.

## Architecture Check

1. This is a bounded doc-alignment ticket: it keeps the active S59 spec truthful without reopening production scope.
2. No compatibility shim is implied; the update should describe the one live boundary that `008` actually landed.

## Verification Layers

1. Active spec text matches landed `ask_about_person` runtime boundary -> doc/code comparison against `ask_about_person_actions.rs`, `action_payload.rs`, and `action_trace.rs`
2. Nearby active ticket chain remains consistent after the spec edit -> ticket/doc review of `S59EXPOBLSUB-009` through `S59EXPOBLSUB-011` and `S59EXPOBLSUB-014`

## What to Change

### 1. Reconcile the active S59 spec

- Update the `ask_about_person` action description in `specs/S59-expectation-obligation-substrate.md` to remove the stale `SearchTarget` and Tell-based last-seen assumptions.
- If nearby prose still treats `SearchTarget` as a live carrier for the search/report chain, narrow that text to the current honest substrate or explicitly defer it.

## Files to Touch

- `specs/S59-expectation-obligation-substrate.md` (modify — factual reconciliation)

## Out of Scope

- Production code or test changes
- Negative-response witness memory design
- `search_place` implementation itself

## Acceptance Criteria

### Tests That Must Pass

1. `! rg -n 'pub enum SearchTarget|Actor has a \`SearchTarget\`|existing Tell mechanism' specs/S59-expectation-obligation-substrate.md`
2. `rg -n "ask_about_person|SearchTarget" tickets/S59EXPOBLSUB-009.md archive/tickets/S59EXPOBLSUB-014.md`

### Invariants

1. The active S59 spec does not claim a live `SearchTarget` carrier for `ask_about_person`
2. The active S59 spec does not describe Tell as the canonical carrier for last-seen transfer that `008` already implemented directly
3. The active S59 spec does not declare a standalone `SearchTarget` type that the current branch never landed

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `! rg -n 'pub enum SearchTarget|Actor has a \`SearchTarget\`|existing Tell mechanism' specs/S59-expectation-obligation-substrate.md`
2. `rg -n "ask_about_person|SearchTarget" tickets/S59EXPOBLSUB-009.md archive/tickets/S59EXPOBLSUB-014.md`

## Outcome

Completed on 2026-04-06.

- Reconciled the active S59 spec in `specs/S59-expectation-obligation-substrate.md` with the landed `ask_about_person` runtime boundary from `S59EXPOBLSUB-008`.
- Removed the stale `SearchTarget` type sketch, rewrote `ask_about_person` and `search_place` preconditions to match direct subject binding, and replaced the old Tell-based last-seen wording with the dedicated typed action/hearsay-transfer boundary.
- Narrowed the Section H information-path and cross-system Tell prose so the active spec no longer implies a live `SearchTarget` carrier or Tell-based `ask_about_person` transport.

## Verification Result

- Passed `! rg -n 'pub enum SearchTarget|Actor has a \`SearchTarget\`|existing Tell mechanism' specs/S59-expectation-obligation-substrate.md`
- Passed `rg -n "ask_about_person|SearchTarget" tickets/S59EXPOBLSUB-009.md archive/tickets/S59EXPOBLSUB-014.md`
