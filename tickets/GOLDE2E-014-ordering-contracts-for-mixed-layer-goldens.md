# GOLDE2E-014: Ordering contracts for mixed-layer golden assertions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), [archive/tickets/E17CRITHEJUS-013.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-013.md)

## Problem

Recent golden work exposed that tickets and tests still drift into weak ordering language such as “later tick” even when the real contract is:

- same-tick action lifecycle order
- `sequence_in_tick`
- authoritative record mutation order
- durable state order rather than tick separation

That overfits tests to incidental scheduler timing and weakens causal precision. It conflicts with `FOUNDATIONS.md` Principles 1, 8, 11, and 27 because the assertion surface stops matching the actual causal boundary.

## Assumption Reassessment (2026-03-27)

1. [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already has strong ordering guidance, including explicit mention of `sequence_in_tick`, but it is still missing a tighter mixed-layer checklist for choosing the correct ordering surface before writing assertions.
2. The E17 justice golden failed temporarily because the test asserted accusation tick `<` punishment tick even though the lawful architecture allowed same-tick accusation then punishment with distinct `sequence_in_tick`.
3. Shared abstraction boundary under audit: the proof boundary between action lifecycle ordering, authoritative mutation ordering, and durable downstream state in golden tests.
4. Intended invariant: a golden should assert the earliest causal boundary that actually constitutes the contract, not a looser or later proxy.
5. This is a golden-driven docs ticket. The live surfaces it needs to name explicitly are:
   - action traces
   - request-resolution traces
   - decision traces
   - authoritative world state
   - event-log ordering
6. This is not a code-change ticket. Existing trace/runtime surfaces already support the needed assertion forms; the issue is that the authoring contract is still too easy to misapply.
7. Adjacent contradiction classification:
   - in scope: documentation and testing guidance for ordering-contract selection
   - out of scope: adding new trace mechanisms or changing scheduler semantics
8. This aligns directly with Principle 27 because the goal is not “nicer tests” but preserving inspectable, causally accurate proof surfaces.

## Architecture Check

1. The cleaner architecture is explicit ordering vocabulary and proof-surface choice in the golden guidance, rather than allowing tests to smuggle later durable effects in as a proxy for earlier causal order.
2. This is cleaner than patching individual goldens case by case because it makes the repository-wide contract explicit.
3. No compatibility language or alternative testing doctrine should be added. The live guidance should be corrected in place.

## Verification Layers

1. Ordering-contract choice is documented against the right proof surfaces -> docs diff review in `docs/golden-e2e-testing.md`
2. Mixed-layer examples name the earlier causal boundary and later downstream consequence separately -> docs examples and rationale review
3. Existing goldens remain the behavioral proof surface; this ticket is documentation-only and does not claim new runtime behavior
4. Additional trace-layer mapping is not applicable beyond documentation because the live trace surfaces already exist

## What to Change

### 1. Strengthen ordering-contract guidance in `docs/golden-e2e-testing.md`

Add a dedicated mixed-layer ordering subsection that forces authors to name:

- whether the contract is request-resolution, start/commit, same-tick sequence, authoritative mutation, or durable state
- whether later durable consequences are merely downstream evidence or the actual promise under test
- whether same-tick ordering is lawful in the current architecture

### 2. Add justice-style and planner/runtime examples

Include one or two concise examples showing:

- why “later tick” was wrong for a same-tick action sequence
- how to assert the earlier ordering boundary instead
- how to separately assert the later durable consequence without conflating them

### 3. Cross-link planner contract guidance where relevant

If needed, add a short cross-reference from `docs/golden-e2e-testing.md` to `docs/planner-contracts.md` for planner-root or mixed-layer AI scenarios so authors do not reconstruct AI order contracts from ticket lore.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify)
- `docs/planner-contracts.md` (modify, only if a cross-link or brief clarification is needed)

## Out of Scope

- Changing trace or scheduler code
- Rewriting existing goldens unless a documentation example must cite one
- Broader ticket-authoring policy changes outside golden ordering guidance

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.
2. Existing suite remains the behavioral baseline: `cargo test --workspace`

### Invariants

1. Golden guidance names causal ordering surfaces explicitly instead of collapsing them into generic tick language
2. Documentation continues to prefer the earliest lawful proof boundary, consistent with local causality and debugability principles

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test --workspace`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo clippy --workspace`
