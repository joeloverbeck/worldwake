# DOCTRAC-001: Layer Verification Contracts and Ground Coverage Claims

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, `specs/S13-political-emergence-golden-suites.md`

## Problem

Recent cross-system ticket work exposed two documentation failures:

1. verification language was specified at the scenario level instead of the architectural-layer level, which blurred action-lifecycle assertions together with authoritative system-mutation assertions
2. golden-suite coverage claims drifted away from the real test inventory and understated existing focused/system coverage

That makes tickets easier to misread and weakens the repo's stated contract that assumptions must be grounded in current code and tests before implementation.

## Assumption Reassessment (2026-03-18)

1. `tickets/README.md` already requires assumption reassessment, exact test naming, and distinguishing AI/planning logic from authoritative/system logic, but it does not yet require per-invariant verification-layer mapping for mixed cross-system scenarios.
2. `tickets/_TEMPLATE.md` has no explicit slot for "this invariant is checked via decision trace vs action trace vs event-log/world-state". Contributors can satisfy the template while still writing vague assertion surfaces.
3. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` are manually maintained and can drift from the real golden inventory. Re-checking against `cargo test -p worldwake-ai -- --list` plus the current `golden_*.rs` declarations shows the current `golden_*` counts are already correct at 103 total; the real gap is that the inventory source and scope were not stated explicitly enough to prevent confusion with non-`golden_*` harness tests.
4. `docs/golden-e2e-scenarios.md` already describes Scenario 21 correctly as force-law succession proven via action trace plus authoritative/event-log checks. The remaining stale wording is in `specs/S13-political-emergence-golden-suites.md`, which still attributes Scenario 21 to `ClaimOffice` / `DeclareSupport` behavior and old test names.
5. Existing focused/runtime coverage already locks down the force-law political boundary in addition to the golden scenario: `candidate_generation::tests::political_candidates_skip_force_law_offices` and `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning` in `worldwake-ai`.
6. The gap here is documentation/authoring precision and inventory grounding, not missing engine behavior.

## Architecture Check

1. Tightening the ticket/spec contract is cleaner than compensating with ad hoc reviewer memory. The repo should encode how to specify mixed-layer invariants instead of relying on implementers to infer it every time.
2. Grounding coverage claims in `cargo test -p worldwake-ai -- --list` and exact focused/system test names is more robust than maintaining hand-wavy counts or scenario summaries.
3. Long-term, the cleanest architecture would derive inventory counts from the real test list instead of maintaining them manually in docs. That automation is out of scope here, but the ticket should acknowledge the current docs are still a drift-prone cache.
4. No backward-compatibility shims or aliasing are involved; this is a precision upgrade to ticket/spec authoring rules and S13 wording.

## Verification Layers

1. Mixed-layer ticket authoring contract -> `tickets/README.md` rules plus explicit template slot in `tickets/_TEMPLATE.md`
2. Scenario 21 action ordering -> action trace wording in `specs/S13-political-emergence-golden-suites.md`
3. Scenario 21 authoritative death/vacancy/installation -> event-log delta and authoritative-world-state wording in `specs/S13-political-emergence-golden-suites.md`
4. Golden inventory interpretation -> explicit `golden_*` inventory-source wording in `docs/golden-e2e-coverage.md`

## What to Change

### 1. Tighten ticket authoring rules

Update `tickets/README.md` and `tickets/_TEMPLATE.md` so mixed-system tickets must explicitly map each important invariant to its verification layer, for example:

- AI reasoning or candidate absence: decision trace / focused runtime test
- action lifecycle ordering: action trace
- authoritative mutation ordering: event-log deltas and/or authoritative world state

The contract should explicitly forbid writing a single vague assertion surface for a scenario that spans multiple layers.

### 2. Ground golden-suite claims in real inventory

Update the authoring guidance and coverage dashboard wording to require `cargo test -p worldwake-ai -- --list` before claiming golden-suite counts or gaps, to interpret those claims against the docs' stated `golden_*` scope, and to require exact existing focused/system tests to be named when a ticket claims a missing coverage area.

### 3. Correct S13 political-emergence wording

Update `specs/S13-political-emergence-golden-suites.md` so the force-law succession scenario names the actual architecture:

- combat -> `DeadAt`
- politics -> `succession_system()`
- no `ClaimOffice` / `DeclareSupport` path for force-law installation

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `specs/S13-political-emergence-golden-suites.md` (modify)
- `docs/golden-e2e-coverage.md` (modify — clarify inventory source and `golden_*` scope wording)

## Out of Scope

- Adding new runtime traces or harness helpers
- Changing political, combat, or AI behavior
- Auto-generating the golden docs (still the cleaner long-term architecture, but not this ticket)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- --list`
2. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

### Invariants

1. Ticket/spec language for cross-system scenarios must distinguish verification layers instead of collapsing them into one generic "trace" assertion surface.
2. Coverage-gap claims must be grounded in the current repository state, including existing focused/system coverage where applicable.

## Test Plan

### New/Modified Tests

1. No new Rust tests expected — this ticket strengthens ticket/spec/docs precision.
2. Re-read the updated docs/tickets against at least one mixed-layer scenario (`S13` force-law succession) to confirm the wording now maps invariant -> verification layer explicitly.
3. Re-run the real golden inventory list and confirm the coverage dashboard's stated `golden_*` scope and inventory source remain accurate.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-18

**What changed**:
- Tightened `tickets/README.md` so mixed-layer and cross-system tickets must include an explicit invariant-to-verification-layer mapping instead of collapsing everything into one vague trace/assertion surface.
- Added a dedicated `Verification Layers` section to `tickets/_TEMPLATE.md` so contributors have an explicit place to document decision-trace vs action-trace vs event-log/world-state proofs.
- Corrected `specs/S13-political-emergence-golden-suites.md` Scenario 21 to describe the real architecture: combat death leads to force-law succession through authoritative vacancy and delayed installation, with no `ClaimOffice` / `DeclareSupport` path.
- Updated the S13-001 deliverable and verification text to the real test names: `golden_combat_death_triggers_force_succession` and its replay companion.
- Clarified `docs/golden-e2e-coverage.md` so its `golden_*` inventory source and scope are explicit, reducing future confusion between raw `cargo -- --list` output and the narrower documented `golden_*` inventory.

**Deviations from original plan**:
- `docs/golden-e2e-scenarios.md` did not need changes. Reassessment showed its Scenario 21 wording was already corrected before this ticket ran.
- The real inventory issue was narrower than the original ticket claimed: the documented `golden_*` counts were already correct at 103 total. The necessary change was clarifying the inventory source/scope, not correcting numeric totals.
- No Rust tests were added or changed because the underlying runtime behavior and focused coverage already matched the intended architecture; the gap was documentation precision only.
- The cleaner long-term architecture would be to generate or derive inventory counts from the real test list instead of manually caching them in docs. That remains out of scope for this ticket.

**Verification**:
- `cargo test -p worldwake-ai -- --list`
- `cargo test -p worldwake-ai --test golden_emergent`
- `cargo test --workspace`
- `cargo clippy --workspace`
