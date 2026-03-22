# GOLAUTHVAL-001: Require Concrete Numeric Validation In Golden Ticket And Spec Authoring

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/golden-e2e-testing.md`, `docs/FOUNDATIONS.md`, `archive/tickets/completed/S17WOULIFGOLSUI-001.md`

## Problem

Recent golden work exposed a process-level authoring failure: a ticket described a lawful-seeming deprivation scenario that the current authoritative math made impossible. The runtime bug is already closed; the remaining gap is in the authoring contract. Current ticket/spec guidance does not explicitly require authors to validate survivability, threshold cadence, or cumulative authoritative deltas before implementation. That gap lets tickets drift away from `docs/FOUNDATIONS.md` Principle 3 ("Concrete State Over Abstract Scores") by reasoning in narrative terms ("second fire should happen") instead of concrete state transitions ("second fire adds current hunger to wound severity and therefore kills the agent under these numbers").

## Assumption Reassessment (2026-03-21)

1. The immediate motivating mismatch is documented in [S17WOULIFGOLSUI-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-001.md): the original ticket setup used `HomeostaticNeeds::new(pm(920), ...)` plus default hunger thresholds, but `crates/worldwake-systems/src/needs.rs::worsen_or_create_deprivation_wound()` increases wound severity by current `needs.hunger`, so a second starvation fire necessarily reaches the `CombatProfile.wound_capacity` cap. The archived outcome records that correction explicitly.
2. The motivating runtime coverage already exists and passes today. `crates/worldwake-ai/tests/golden_emergent.rs` contains `golden_deprivation_wound_worsening_consolidates_not_duplicates` and `golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically`; `docs/generated/golden-e2e-inventory.md` lists both tests; `python3 scripts/golden_inventory.py --write --check-docs` succeeds; and `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening` passes. This ticket is therefore no longer about adding or correcting runtime coverage.
3. Existing docs already require precision, but not this specific kind of concrete numeric validation. `tickets/README.md` requires assumption reassessment and exact layer naming; `tickets/_TEMPLATE.md` is the drafting surface authors copy first; `docs/golden-e2e-testing.md` requires scenario-isolation disclosure and strongest-surface assertions. None of those artifacts currently requires authors to compute whether a proposed physiology/combat/economy setup is actually reachable and survivable under live formulas.
4. The clean scope is documentation/process quality for ticket/spec authoring, not candidate generation, runtime `agent_tick`, or golden AI behavior. The current architecture already handled the motivating deprivation case correctly once the scenario was written against live arithmetic.
5. No ordering contract drives this ticket.
6. No heuristic/filter removal is involved.
7. Not a stale-request, start-failure, or political office-claim ticket.
8. No `ControlSource`, queued-input, or runtime-intent manipulation is involved.
9. Mismatch to correct: the ticket's current scope omits `tickets/_TEMPLATE.md`, even though leaving the default scaffold weaker than the contract would preserve the same authoring failure at the main drafting surface.
10. Mismatch to correct: the current verification plan implies broader runtime work is still pending. The live S17 scenario already exists; the remaining acceptance boundary is doc/template guidance plus normal verification that the cited runtime example still passes.

## Architecture Check

1. Tightening the ticket contract, the template, and the golden authoring guide together is cleaner than fixing future mismatches ad hoc in individual tickets. It pushes scenario design toward concrete-state reasoning at the point of specification, which aligns directly with `docs/FOUNDATIONS.md` Principles 3, 4, 8, 9, and 25.
2. Updating `tickets/_TEMPLATE.md` is materially better than a README-only fix. Otherwise the main drafting surface would stay silent on the new requirement and authors could still miss it while technically violating the contract.
3. This approach avoids backwards-compatibility shims, test-only exceptions, or production loosening. It improves how work is specified, not how the live engine behaves.
4. Adding brittle prose-string tests for docs wording is not the ideal architecture here. The durable change is stronger authoring guidance plus existing runtime coverage that proves the motivating case; machine-locking exact prose would increase maintenance noise without strengthening causal guarantees.

## Verification Layers

1. The docs explicitly require concrete numeric validation for threshold/load/capacity-driven scenarios -> documentation text in `tickets/README.md`
2. The default ticket scaffold prompts authors to record the same validation requirement at draft time -> documentation text in `tickets/_TEMPLATE.md`
3. Golden authoring guidance explicitly requires authors to state the authoritative arithmetic or survivability envelope when scenario feasibility depends on it -> documentation text in `docs/golden-e2e-testing.md`
4. The motivating example remains accurately represented after the docs update -> archived ticket reference in `archive/tickets/completed/S17WOULIFGOLSUI-001.md` plus current passing golden test
5. This is a docs/process ticket, so no additional runtime trace or action-layer mapping is required.
6. Single-layer ticket: the contract being changed is documentation/process guidance, not runtime behavior.

## What to Change

### 1. Strengthen `tickets/README.md`

Add an explicit rule for tickets/specs whose contract depends on authoritative arithmetic or cumulative state:

- authors must validate that the proposed setup is actually reachable under current formulas
- authors must validate survivability/non-survivability if repeated damage, recovery, depletion, or accumulation is part of the contract
- authors must state the concrete per-fire / per-tick / per-step delta when that delta is material to the scenario
- authors must correct the ticket before implementation if the proposed numbers contradict current authoritative behavior

The wording should reference concrete-state reasoning, not "sanity checks" or "rough estimates."

### 2. Strengthen `tickets/_TEMPLATE.md`

Add prompts so new tickets/specs record:

- when the proposed scenario depends on authoritative arithmetic or cumulative state
- the concrete delta/cadence/capacity math that makes the setup reachable
- the survivability or failure envelope when repeated damage, depletion, or recovery is material
- that mismatched numbers must be corrected in the ticket before implementation

This keeps the requirement at the default authoring surface instead of only in the reference contract.

### 3. Strengthen `docs/golden-e2e-testing.md`

Add golden-specific guidance for scenario design:

- when a scenario depends on repeated threshold firing, wound accumulation, resource depletion, recovery gating, or similar cumulative mechanics, the ticket/spec must state the authoritative delta and the survival/failure envelope
- if the scenario only works by changing the setup numbers, that is acceptable and preferred over weakening production rules, so long as the setup remains lawful and explicit
- if the intended branch is impossible under current formulas, the ticket must be corrected instead of papering over the mismatch in code or assertions

### 4. Cross-reference the motivating example

Point to the archived S17 ticket as a concrete example of this rule: the right fix was to adjust the scenario thresholds to fit the current concrete-state model, not to weaken deprivation worsening semantics.

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `archive/tickets/completed/S17WOULIFGOLSUI-001.md` (reference-only; no change expected unless wording needs a stable cross-reference)

## Out of Scope

- Any production code change in `crates/`
- Any new trace sink or debugging feature
- Rewriting existing tickets/specs en masse
- Reclassifying assertion surfaces already documented correctly

## Acceptance Criteria

### Tests That Must Pass

1. The updated `tickets/README.md` explicitly requires concrete numeric feasibility validation for threshold/load/capacity-driven scenarios
2. The updated `tickets/_TEMPLATE.md` prompts authors to record concrete numeric validation where cumulative mechanics drive the scenario
3. The updated `docs/golden-e2e-testing.md` explicitly requires authors to document authoritative deltas and survivability/failure envelopes when cumulative mechanics drive the scenario
4. Existing suite: `python3 scripts/golden_inventory.py --write --check-docs`
5. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
6. Existing suite: `cargo test --workspace`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Future tickets must be forced toward concrete-state reasoning instead of narrative scenario assumptions
2. The docs must steer authors toward correcting impossible setups rather than weakening production semantics for test convenience

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What changed:
  - updated `tickets/README.md` to require concrete numeric feasibility and survivability validation for threshold/load/capacity-driven scenarios
  - updated `tickets/_TEMPLATE.md` so new tickets prompt authors for the authoritative arithmetic and survival/failure envelope when cumulative mechanics matter
  - updated `docs/golden-e2e-testing.md` to require concrete setup math for cumulative golden scenarios and to cross-reference the S17 deprivation example
- Deviations from original plan:
  - scope widened from README + golden docs to also include `tickets/_TEMPLATE.md`, because leaving the drafting scaffold weaker than the contract would preserve the same authoring failure
  - no runtime code or new runtime tests were added; the motivating S17 golden already exists and the architectural gap was documentation/process, not executable behavior
- Verification results:
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
