# S59EXPOBLSUB-012: Reconcile active S59 spec overdue-detection boundary

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S59EXPOBLSUB-006

## Problem

The active spec `specs/S59-expectation-obligation-substrate.md` now contains a live contradiction. Its design-goal and FOUNDATIONS-alignment prose says overdue detection is local to the expectation owner at the expected place with no global scanner, but `S59EXPOBLSUB-006` deliberately landed a global per-tick `ExpectationCheck` maintenance system that performs only clock-driven `Active -> Overdue` mutation. The active spec needs to distinguish those two layers clearly so the roadmap does not misdescribe the implemented substrate.

## Assumption Reassessment (2026-04-06)

1. `S59EXPOBLSUB-006` is the first landed overdue-detection slice. It adds `SystemId::ExpectationCheck` plus `check_overdue_expectations` in `crates/worldwake-systems/src/expectation_check.rs`, and that system scans all `ExpectationStore` components each tick to mark overdue records by clock only.
2. The active spec still says, in multiple places, that overdue detection is local and requires the owner to be at or observe the expected place (`specs/S59-expectation-obligation-substrate.md:32`, `:51`), while also reserving the global `ExpectationCheck` `SystemFn` (`:261`, `:335`, `:349`, `:353`).
3. The live architectural split is now narrower and clearer: global clock maintenance marks records overdue; later search/report/violation behavior remains locality-sensitive and is still owned by downstream tickets.
4. Nearby active tickets `S59EXPOBLSUB-007` through `S59EXPOBLSUB-011` still correctly reserve the first live report/search/escort/candidate behavior. They do not need engine-scope expansion, but the parent spec should stop implying that `006` itself lands the fully local owner-observation rule.

## Architecture Check

1. Updating the active spec is cleaner than leaving contradictory prose that describes both “no global scanner” and the now-landed global clock scanner as if they were the same mechanism.
2. No backward-compatibility shims or code changes are involved; this is a documentation and roadmap-accuracy ticket only.

## Verification Layers

1. Active spec no longer contradicts the landed `ExpectationCheck` implementation -> direct doc-to-code comparison against `crates/worldwake-sim/src/system_manifest.rs` and `crates/worldwake-systems/src/expectation_check.rs`
2. Remaining locality-sensitive behavior is still owned by later tickets -> active-ticket cross-check against `tickets/S59EXPOBLSUB-007.md` through `tickets/S59EXPOBLSUB-011.md`
3. Documentation-only ticket -> no runtime test layer required

## What to Change

### 1. Reconcile overdue-detection language in the active spec

In `specs/S59-expectation-obligation-substrate.md`, rewrite the contradictory overdue-detection prose so it states:

- `ExpectationCheck` is a global per-tick maintenance system that performs only clock-based `Active -> Overdue` mutation
- expected-place observation, missing-person confirmation, and violation/report/search consequences remain locality-sensitive downstream behavior

### 2. Align nearby active spec sections

Update the spec's Design Goals, FOUNDATIONS/P7 language, Deliverable 7, temporal-resolution notes, and any other directly contradictory sections so they all describe the same split.

### 3. Reconfirm downstream ownership in ticket chain

Reassess nearby active S59 tickets and update them only if they still imply that overdue mutation itself is the locality-sensitive step.

## Files to Touch

- `specs/S59-expectation-obligation-substrate.md` (modify — reconcile overdue-detection boundary)

## Out of Scope

- Production-code changes to `ExpectationCheck`
- Changing candidate generation, report, search, or escort implementation boundaries
- Replacing the global clock-maintenance system with a different runtime mechanism

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.
2. Existing suite: no additional runtime test command required

### Invariants

1. The active spec must not simultaneously claim that overdue detection requires owner-local observation and that the landed `ExpectationCheck` scanner is the overdue-detection mechanism.
2. The active roadmap must continue to reserve locality-sensitive search/report behavior for later S59 tickets rather than implying it is already live.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `rg -n "Overdue detection|ExpectationCheck|check_overdue_expectations|global scanner|owner checks" specs/S59-expectation-obligation-substrate.md tickets/S59EXPOBLSUB-0*.md`
2. `cargo test -p worldwake-sim system_manifest && cargo test -p worldwake-systems expectation_check`

## Outcome

Completed on 2026-04-06.

- Rewrote the active S59 spec so it now distinguishes global clock-driven overdue maintenance from later locality-sensitive search/report/violation behavior.
- Updated the Design Goals, P7 alignment note, Deliverable 7, Section H causal-hook text, temporal-resolution wording, causal-record wording, ordering rationale, and cross-system interaction note to match the landed `ExpectationCheck` boundary.
- Rechecked `S59EXPOBLSUB-007` through `S59EXPOBLSUB-011`; no ticket text changes were needed because those downstream tickets still correctly reserve the first live search/report/escort/candidate behavior.

## Verification Result

- Passed `rg -n "Overdue detection|ExpectationCheck|check_overdue_expectations|global scanner|owner checks" specs/S59-expectation-obligation-substrate.md tickets/S59EXPOBLSUB-0*.md`
- Passed `cargo test -p worldwake-sim system_manifest && cargo test -p worldwake-systems expectation_check`
