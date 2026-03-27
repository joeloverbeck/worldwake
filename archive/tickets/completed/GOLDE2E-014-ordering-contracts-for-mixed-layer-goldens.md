# GOLDE2E-014: Ordering contracts for mixed-layer golden assertions

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), [archive/tickets/completed/S14TRACEORD-003-docs-and-ticket-contract-cleanup-for-ordering-semantics.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S14TRACEORD-003-docs-and-ticket-contract-cleanup-for-ordering-semantics.md), [archive/tickets/DOCTESTPOL-001-ordering-contracts-for-ai-and-goldens.md](/home/joeloverbeck/projects/worldwake/archive/tickets/DOCTESTPOL-001-ordering-contracts-for-ai-and-goldens.md), [archive/tickets/E17CRITHEJUS-013.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-013.md)

## Problem

This ticket originally assumed the repository still lacked explicit guidance for mixed-layer golden ordering contracts. Reassessment against the live docs, archived ticket history, and current E17 justice coverage shows that the requested architecture is already in place. The live problem is stale duplicate ticket scope, not missing ordering semantics.

## Assumption Reassessment (2026-03-27)

1. [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already contains the core mixed-layer ordering contract this ticket proposed to add. It explicitly distinguishes request-resolution, action-trace, decision-trace, authoritative-state, and event-log surfaces; it warns against incidental tick-boundary assertions; and it already says same-tick cross-agent ordering should use the explicit `(tick, sequence_in_tick)` key.
2. Archived [`S14TRACEORD-003-docs-and-ticket-contract-cleanup-for-ordering-semantics.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S14TRACEORD-003-docs-and-ticket-contract-cleanup-for-ordering-semantics.md) already delivered the repository-wide same-tick ordering cleanup this ticket was implicitly re-requesting. Repeating that work would add duplicate doctrine, not stronger architecture.
3. Archived [`DOCTESTPOL-001-ordering-contracts-for-ai-and-goldens.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/DOCTESTPOL-001-ordering-contracts-for-ai-and-goldens.md) already tightened the broader mixed-layer documentation contract: authors must name the exact layer under test, avoid delayed-authoritative proxies for earlier ordering, and state substrate asymmetry instead of flattening everything into vague "ordering."
4. Shared abstraction boundary under audit: proof-surface selection across request resolution, action lifecycle, AI reasoning, authoritative mutation, and durable downstream state in golden tests. That boundary is already named in the live docs; no new runtime or docs abstraction is missing.
5. The motivating E17 scenario is no longer hypothetical. [`golden_witnessed_theft_accusation_chain`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs#L6078) and its helper in [`golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs#L5233) already assert the stronger contract: `tell` precedes punishment and `accuse` precedes punishment via action-trace order, while accusation/verdict durability is proved separately through `CrimeRegister` state.
6. The original ticket narrative overstates the remaining docs gap. The live docs may not use the exact subsection title or checklist phrasing proposed here, but the architectural content is already present and enforced in the canonical shared locations. Adding another narrowly overlapping subsection would make the guidance noisier, not clearer.
7. Mismatch + correction: this is not an implementation ticket anymore. No production code, tests, or shared docs need changes. The correct scope is to mark the ticket complete as a duplicate/no-op after verifying the live architecture and test surfaces.

## Architecture Check

1. The cleaner architecture is one canonical documentation contract, not a second ticket re-adding the same mixed-layer ordering rules under slightly different wording.
2. The current architecture is already preferable to the ticket's proposed follow-up changes because it keeps ordering guidance centralized in `docs/golden-e2e-testing.md` and prior ticket-contract docs, with the E17 golden proving the live pattern directly.
3. No backwards-compatibility layer, alias path, or parallel testing doctrine should be introduced. Consolidation is the robust option here.

## Verification Layers

1. Same-tick and mixed-layer ordering guidance already exists -> doc review in [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md)
2. Prior repository-wide ordering cleanup already landed -> archived ticket review in [`archive/tickets/completed/S14TRACEORD-003-docs-and-ticket-contract-cleanup-for-ordering-semantics.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S14TRACEORD-003-docs-and-ticket-contract-cleanup-for-ordering-semantics.md)
3. Broader AI/golden ordering-contract tightening already landed -> archived ticket review in [`archive/tickets/DOCTESTPOL-001-ordering-contracts-for-ai-and-goldens.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/DOCTESTPOL-001-ordering-contracts-for-ai-and-goldens.md)
4. The motivating E17 justice chain already proves the intended mixed-layer assertion shape -> targeted golden coverage in [`golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs#L6078)
5. Single-ticket archival correction only; no new runtime-layer mapping is applicable beyond verifying the existing surfaces above

## What to Change

### 1. Correct the ticket scope to match the live repository

Update this ticket so it records that:

- the proposed mixed-layer ordering guidance already exists
- the E17 motivating scenario already uses the stronger ordering proof surface
- no additional shared-doc or runtime changes are justified

### 2. Archive the ticket as completed duplicate scope

Move the ticket into `archive/tickets/completed/` with an `Outcome` section describing:

- what was reassessed
- why duplicate doc changes were rejected
- which verification commands passed

## Files to Touch

- `tickets/GOLDE2E-014-ordering-contracts-for-mixed-layer-goldens.md` (modify, then archive)

## Out of Scope

- New runtime trace features
- New mixed-layer golden assertions or scheduler changes
- Additional edits to `docs/golden-e2e-testing.md`, `tickets/README.md`, or `docs/precision-rules.md` for guidance that already exists

## Acceptance Criteria

### Tests That Must Pass

1. The ticket records that mixed-layer ordering guidance already exists in the live docs
2. The ticket records that the motivating E17 scenario is already covered by live golden tests
3. Existing targeted coverage: `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --nocapture`
4. Existing targeted coverage: `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
5. Existing targeted coverage: `cargo test -p worldwake-ai golden_traceability_explains_stale_fine_branch_without_source_diving -- --nocapture`
6. Workspace verification: `cargo test --workspace`
7. Workspace lint: `cargo clippy --workspace`

### Invariants

1. Mixed-layer goldens must continue to assert the earliest lawful proof boundary rather than a weaker tick-proxy narrative
2. Duplicate ticket scope must be resolved by consolidation and archival, not by re-adding overlapping documentation

## Test Plan

### New/Modified Tests

1. `None — duplicate-scope ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --nocapture`
2. `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
3. `cargo test -p worldwake-ai golden_traceability_explains_stale_fine_branch_without_source_diving -- --nocapture`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-27

What actually changed:
- Reassessed the ticket against the live repository docs, archived ordering tickets, and the current E17 justice goldens.
- Confirmed that the requested mixed-layer ordering contract had already been implemented in shared docs and was already exercised by live golden coverage.
- Corrected this ticket from an implementation request to a duplicate-scope archival record.
- Applied a minimal clippy-only cleanup in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) so the required workspace lint gate passes after verification.

Deviations from original plan:
- No shared docs or runtime code were changed because the requested guidance and example pattern were already present.
- The clean architectural decision was to avoid another overlapping documentation pass and keep one canonical contract.
- Verification surfaced pre-existing lint failures outside the ticket's original scope; they were fixed in place because the requested completion criteria require passing workspace lint.

Verification results:
- Passed `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --nocapture`
- Passed `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
- Passed `cargo test -p worldwake-ai golden_traceability_explains_stale_fine_branch_without_source_diving -- --nocapture`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace`
