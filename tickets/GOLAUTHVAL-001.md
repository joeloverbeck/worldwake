# GOLAUTHVAL-001: Require Concrete Numeric Validation In Golden Ticket And Spec Authoring

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `docs/golden-e2e-testing.md`, `docs/FOUNDATIONS.md`, `archive/tickets/completed/S17WOULIFGOLSUI-001.md`

## Problem

Recent golden work exposed a process-level architecture failure: a ticket described a lawful-seeming deprivation scenario that the current authoritative math made impossible. The issue was not missing code. The issue was that the ticket/spec authoring contract did not require concrete validation of survivability, threshold cadence, or cumulative authoritative deltas before implementation. That gap lets tickets drift away from `FOUNDATIONS.md` Principle 3 ("Concrete State Over Abstract Scores") by reasoning in narrative terms ("second fire should happen") instead of concrete state transitions ("second fire adds current hunger to wound severity and therefore kills the agent under these numbers").

## Assumption Reassessment (2026-03-21)

1. The immediate motivating mismatch is documented in [S17WOULIFGOLSUI-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-001.md): the original ticket setup used `HomeostaticNeeds::new(pm(920), ...)` plus default hunger thresholds, but `crates/worldwake-systems/src/needs.rs::worsen_or_create_deprivation_wound()` increases wound severity by current `needs.hunger`, so a second starvation fire necessarily reaches the `CombatProfile.wound_capacity` cap. The archived outcome records that correction explicitly.
2. Existing docs already require precision, but not this specific kind of concrete numeric validation. `tickets/README.md` requires assumption reassessment and exact layer naming; `docs/golden-e2e-testing.md` requires scenario-isolation disclosure and strongest-surface assertions. Neither currently requires authors to compute whether a proposed physiology/combat/economy setup is actually reachable and survivable under live formulas.
3. This is not an AI regression ticket. The target is documentation/process quality for ticket/spec authoring, not candidate generation, runtime `agent_tick`, or golden AI behavior.
4. No ordering contract drives this ticket.
5. No heuristic/filter removal is involved.
6. Not a stale-request or start-failure ticket.
7. Not a political office-claim ticket.
8. No `ControlSource`, queued-input, or runtime-intent manipulation is involved.
9. This ticket is about ticket/spec authoring for golden scenarios generally, not a single isolated scenario branch.
10. Mismatch to correct: the current authoring docs are strong on assertion-surface precision but weak on concrete numeric feasibility checks for scenarios whose behavior depends on authoritative arithmetic, tolerance windows, or cumulative load.

## Architecture Check

1. Tightening the authoring contract is cleaner than fixing future mismatches ad hoc in individual tickets. It pushes scenario design toward concrete-state reasoning at the point of specification, which aligns directly with `docs/FOUNDATIONS.md` Principles 3, 4, 8, 9, and 25.
2. This approach avoids backwards-compatibility shims, test-only exceptions, or production loosening. It improves how work is specified, not how the live engine behaves.

## Verification Layers

1. The docs explicitly require concrete numeric validation for threshold/load/capacity-driven scenarios -> documentation text in `tickets/README.md`
2. Golden authoring guidance explicitly requires authors to state the authoritative arithmetic or survivability envelope when scenario feasibility depends on it -> documentation text in `docs/golden-e2e-testing.md`
3. The motivating example remains accurately represented after the docs update -> archived ticket reference in `archive/tickets/completed/S17WOULIFGOLSUI-001.md`
4. This is a docs/process ticket, so no additional runtime trace or action-layer mapping is required.
5. Single-layer ticket: the contract being changed is documentation/process guidance, not runtime behavior.

## What to Change

### 1. Strengthen `tickets/README.md`

Add an explicit rule for tickets/specs whose contract depends on authoritative arithmetic or cumulative state:

- authors must validate that the proposed setup is actually reachable under current formulas
- authors must validate survivability/non-survivability if repeated damage, recovery, depletion, or accumulation is part of the contract
- authors must state the concrete per-fire / per-tick / per-step delta when that delta is material to the scenario
- authors must correct the ticket before implementation if the proposed numbers contradict current authoritative behavior

The wording should reference concrete-state reasoning, not "sanity checks" or "rough estimates."

### 2. Strengthen `docs/golden-e2e-testing.md`

Add golden-specific guidance for scenario design:

- when a scenario depends on repeated threshold firing, wound accumulation, resource depletion, recovery gating, or similar cumulative mechanics, the ticket/spec must state the authoritative delta and the survival/failure envelope
- if the scenario only works by changing the setup numbers, that is acceptable and preferred over weakening production rules, so long as the setup remains lawful and explicit
- if the intended branch is impossible under current formulas, the ticket must be corrected instead of papering over the mismatch in code or assertions

### 3. Cross-reference the motivating example

Point to the archived S17 ticket as a concrete example of this rule: the right fix was to adjust the scenario thresholds to fit the current concrete-state model, not to weaken deprivation worsening semantics.

## Files to Touch

- `tickets/README.md` (modify)
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
2. The updated `docs/golden-e2e-testing.md` explicitly requires authors to document authoritative deltas and survivability/failure envelopes when cumulative mechanics drive the scenario
3. Existing suite: `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. Future tickets must be forced toward concrete-state reasoning instead of narrative scenario assumptions
2. The docs must steer authors toward correcting impossible setups rather than weakening production semantics for test convenience

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
3. `scripts/verify.sh`
