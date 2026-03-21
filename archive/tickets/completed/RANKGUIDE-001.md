# RANKGUIDE-001: Tighten Ticket And Golden Guidance For Live Ranking Arithmetic And Branch Symmetry

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md), [docs/precision-rules.md](/home/joeloverbeck/projects/worldwake/docs/precision-rules.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md), [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md)

## Problem

The current authoring guidance is strong on mixed-layer precision, but it still left room for one specific ranking-authoring mistake that just happened in S17: a ticket/spec treated "equal utility weights" as if that implied equal motive scores between competing branches. Under the live architecture, equal weights can still produce asymmetric motive scores because the pressures differ.

That mistake propagated into:

- a ticket claim that the branches were symmetric except for priority promotion,
- a spec line saying equal weights ensured priority class, not motive score, determined order,
- a weaker-than-ideal initial proof shape.

This is a documentation and ticket-authoring gap. The architecture is fine; the contract for writing about ranking arithmetic needs to be sharper.

## Assumption Reassessment (2026-03-21)

1. [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already enforces reassessment, precision rules, and real command verification, but it does not yet explicitly say that equal weights are insufficient evidence for equal motive scores or branch symmetry in ranking-sensitive tickets.
2. [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already warns against "same-state, weight-only divergence" claims when the branches use different substrates. The missing precision is stronger: authors should explicitly validate the live arithmetic before claiming a tie, neutrality, or "priority-class only" divergence.
3. [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md) is already corrected. Scenario 30 now explicitly states that equal weights leave motive scores asymmetric because dirtiness pressure exceeds hunger pressure, and that the stronger contract is priority-class promotion overriding the higher wash motive.
4. The S17 scenarios already exist in runtime coverage: `golden_deprivation_wound_worsening_consolidates_not_duplicates` is present in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs), and `golden_recovery_aware_boost_eats_before_wash` is present in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs). Both targeted commands pass under the current code.
5. The generated inventory pipeline is already aligned: [docs/generated/golden-e2e-inventory.md](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md) lists both S17 tests, and `python3 scripts/golden_inventory.py --write --check-docs` currently passes. The remaining drift is the hand-written authoring/dashboard/catalog guidance.
6. [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md) is stale in a more specific way than this ticket originally said: the docs it referenced do exist, the generated inventory is already correct, and the real gap is that the hand-written coverage dashboard and scenario catalog lag the shipped S17 tests.
7. This is not an AI-runtime behavior ticket. The verification surface is documentation review plus real targeted test/doc validation commands.
8. No ordering-sensitive runtime behavior, heuristics, stale-request boundaries, political closure boundaries, or `ControlSource` semantics are under change.
9. Mismatch corrected: the missing substrate is not engine code or spec repair. It is tighter authoring guidance about live ranking arithmetic plus hand-written docs alignment with the current S17 implementation.

## Architecture Check

1. The clean solution is to tighten the written contract where the mistake occurred: ticket authoring guidance, golden testing guidance, and the stale hand-written golden docs. That is better than adding compensating code, adding special-case runtime assertions, or relying on reviewers to re-derive ranking arithmetic ad hoc.
2. The docs should point authors toward concrete live arithmetic, not abstract narrative claims. That aligns with Principle 3 (concrete state over abstract scores) and Principle 27 (debuggability).
3. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Ticket authoring contract explicitly requires validating full live ranking arithmetic before claiming branch symmetry/ties -> manual doc review in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md)
2. Golden testing guidance explicitly distinguishes equal weights from equal motive scores and points to live arithmetic validation -> manual doc review in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md)
3. Golden coverage dashboard matches the current S17 inventory and counts -> manual doc review in [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md)
4. Golden scenario catalog documents the shipped S17 scenarios 29 and 30 -> manual doc review in [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md)
5. Active S17 docs catch-up ticket no longer references stale assumptions about missing docs or missing test implementation -> manual ticket review in [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md)
6. Single-layer note: this is a documentation/ticket-authoring ticket; no additional runtime verification layers apply

## What to Change

### 1. Tighten the ticket authoring contract

Update [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) so ranking-sensitive tickets must explicitly validate the live arithmetic when they claim:

- equal motive scores,
- branch symmetry,
- "priority-class only" divergence,
- or neutrality from equal utility weights.

The contract should state that authors must check the actual formula inputs: pressure, weight, caps/promotions, and any other live ranking substrate that can break symmetry.

### 2. Tighten golden testing guidance

Update [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) with a short explicit rule:

- equal weights do not imply equal motive scores,
- validate live arithmetic before writing a symmetry claim,
- if the compared branches are asymmetric in motive even with equal weights, say so directly and explain what stronger substrate actually drives the outcome.

### 3. Align the hand-written golden docs with the shipped S17 tests

Update [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) and [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) so they reflect the already-implemented Scenario 29 and Scenario 30 tests and the current 133-test inventory.

### 4. Correct the active S17 docs catch-up ticket

Update [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md) so its assumptions and file references match the current repository.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `tickets/S17WOULIFGOLSUI-003.md` (modify)

## Out of Scope

- Any production code changes
- Any new trace payloads or ranking behavior changes
- Editing the already-corrected S17 spec text
- Rewriting historical archived tickets beyond what is needed for cross-reference accuracy
- Reformatting unrelated docs sections

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
2. `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

### Invariants

1. The authoring contract explicitly prevents future "equal weights => equal motive scores" mistakes
2. Golden guidance explicitly requires validating live ranking arithmetic before claiming symmetry or neutrality
3. The hand-written golden coverage docs match the shipped Scenario 29/30 coverage
4. The active S17 docs catch-up ticket matches the current repository instead of stale assumptions
5. No production or test code changes are introduced

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
2. `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-21
- What changed: tightened [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) so ranking-sensitive tickets/goldens must validate live arithmetic instead of inferring symmetry from equal weights; aligned [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) and [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) with the already-shipped S17 Scenario 29/30 tests and current 133-test inventory; corrected stale assumptions in [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md).
- Deviations from original plan: no S17 spec edit was needed because [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md) was already correct; the real remaining drift was in hand-written golden docs and the active catch-up ticket.
- Verification: `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`, `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`, `python3 scripts/golden_inventory.py --write --check-docs`, `cargo test --workspace`, `cargo clippy --workspace`.
