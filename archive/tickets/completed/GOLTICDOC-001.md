# GOLTICDOC-001: Tighten Golden Ticket Reassessment and Runtime-Trace Granularity Guidance

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — live repo already reflects the intended documentation architecture
**Deps**: `tickets/README.md`; `tickets/_TEMPLATE.md`; `docs/precision-rules.md`; `docs/golden-e2e-testing.md`; `docs/golden-e2e-scenarios.md`; `docs/golden-e2e-coverage.md`; spec [`specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/specs/S19-institutional-record-consultation-golden-suites.md); archived example [`archive/tickets/completed/S19INSRECCON-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-002.md)

## Problem

Golden-ticket authoring can drift from the live planner surface, helper surface, topology math, and runtime trace granularity even when the engine architecture is already correct. The intended fix was to tighten the canonical documentation contract so tickets are corrected before code is changed.

Live reassessment shows that most of that documentation tightening has already landed. The remaining work for this ticket is to correct the ticket narrative itself, verify the existing runtime/doc surface, and archive it accurately rather than pretending the repo still lacks this guidance.

## Assumption Reassessment (2026-03-22)

1. `tickets/README.md` already requires reassessment, divergence correction, exact helper verification, and dry-run verification of AI golden test names. The specific checks now present include exact live `GoalKind` / operator-surface naming, helper verification, and ranking-sensitive arithmetic validation.
2. `tickets/_TEMPLATE.md` already includes placeholders for the live planner/operator surface, scenario isolation, ordering layer, arithmetic setup math, and mismatch correction. The intended ticket-contract tightening is already reflected there.
3. `docs/precision-rules.md` already codifies the missing precision rules this ticket proposed: exact layer/symbol naming, cumulative arithmetic, scenario isolation, helper-surface precision, divergence correction, and the distinction between focused planner surfaces and runtime plan/ordering surfaces.
4. `docs/golden-e2e-testing.md` already documents `planning.selection.selected_plan`, warns against overfitting to incidental tick boundaries, and explicitly tells authors to correct stale tickets when live planner/runtime surfaces differ.
5. The underlying S19 runtime/test surface is no longer hypothetical. `crates/worldwake-ai/tests/golden_harness/mod.rs` already contains `RULERS_HALL` and `seed_office_register`, and `crates/worldwake-ai/tests/golden_offices.rs` already contains the remote-record consultation golden.
6. The S19 spec and the live tests have naming/scope drift. The spec still names Scenario 32 as `golden_consult_record_prerequisite_political_action`, but the live repo currently proves the local information-locality/political-facts case as `golden_information_locality_for_political_facts`. This ticket should not claim the older name as missing implementation without first acknowledging that live drift.
7. `docs/golden-e2e-scenarios.md` already catalogs the local-information and remote-record office scenarios, which means the original “update scenario docs” work is at least partially done. By contrast, `docs/golden-e2e-coverage.md` still shows stale topology/cross-system summaries around `RulersHall`, so the docs layer is not perfectly synchronized even though the core reassessment contract is already present.
8. This remains a documentation/ticket-governance ticket, not an engine-change ticket. The beneficial architectural move is still “one canonical contract in the live docs, no compensating code, no aliases.” Additional production changes would not improve the architecture here.
9. Verification is primarily documentation plus existing runtime coverage. No new production behavior is required to satisfy the corrected scope.
10. Mismatch + correction: the original ticket was written as if the contract/guidance and remote-record harness surface were still missing. The live repo already includes most of that work, so this ticket’s scope must change from “implement the missing guidance” to “document that the guidance landed, verify the remaining live contract, and archive the ticket accurately.”

## Architecture Check

1. The current architecture is better than the original ticket narrative assumed. Centralizing reassessment discipline in `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/precision-rules.md`, and `docs/golden-e2e-testing.md` is cleaner and more extensible than scattering special-case reminders across new tickets or adding test-only compensating code.
2. No backward-compatibility aliasing should be added here. The right move is to keep one canonical documentation contract and correct stale tickets/specs when they drift from the live planner/test surface.
3. Further expanding this ticket into production code would be worse architecture, not better. The production planner, harness, and golden runtime already expose the intended behavior; duplicating that work would only create overlap and maintenance drag.

## Verification Layers

1. Canonical reassessment contract exists in ticket-governance docs -> `tickets/README.md` and `tickets/_TEMPLATE.md`
2. Layer/surface/arithmetic precision rules exist in the shared precision contract -> `docs/precision-rules.md`
3. Runtime selected-plan granularity guidance exists in golden conventions -> `docs/golden-e2e-testing.md`
4. The referenced remote-record runtime surface is real, not hypothetical -> focused golden runtime coverage in `crates/worldwake-ai/tests/golden_offices.rs` plus harness helpers in `crates/worldwake-ai/tests/golden_harness/mod.rs`
5. Single-change ticket after reassessment: no additional mixed-layer mapping is needed because the corrected deliverable is accurate ticket/document state plus verification of already-delivered runtime/docs

## What to Change

### 1. Correct the ticket to match the live repo

Update this ticket so it no longer claims that the reassessment contract, remote-record helper surface, or remote-record golden coverage are missing when those pieces already exist.

### 2. Verify the live contract instead of re-implementing it

Run the relevant doc-surface checks, targeted S19-related golden tests, crate suite, workspace suite, and lint. If verification passes, complete and archive the ticket with an accurate outcome describing what was already delivered versus what this ticket originally planned.

## Files to Touch

- `tickets/GOLTICDOC-001.md` (modify, then archive)

## Out of Scope

- Any new production code or trace-sink changes
- Re-implementing helpers or tests that already exist in the live repo
- Inventing alias test names to match stale spec/ticket wording
- Broad doc rewrites beyond what is required to keep this ticket accurate

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The canonical reassessment contract must remain anchored in the shared docs/template files rather than scattered into ad-hoc ticket lore.
2. The ticket must not claim missing planner/runtime/doc surface that already exists in the repo.
3. The resolution must continue to prefer correcting stale tickets/docs over introducing compensating code, aliases, or weaker assertions.

## Test Plan

### New/Modified Tests

1. `None — no code or doc behavior changes are required beyond correcting this ticket; verification relies on existing runtime coverage and current documentation surfaces.`

### Commands

1. `rg -n "reassess|GoalKind|operator|helper|scenario isolation|delta|cadence|selected_plan|selected plan|SearchSelection" tickets/README.md tickets/_TEMPLATE.md docs/precision-rules.md docs/golden-e2e-testing.md`
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
3. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically`
4. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-22
- What actually changed: the ticket itself was corrected to match the live repo. Reassessment confirmed that the core documentation contract this ticket originally proposed is already present in `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/precision-rules.md`, and `docs/golden-e2e-testing.md`, and that the referenced remote-record harness/runtime surface already exists in `crates/worldwake-ai/tests/golden_harness/mod.rs` and `crates/worldwake-ai/tests/golden_offices.rs`.
- Deviations from original plan: no new production or shared-doc changes were required. The original plan assumed missing reassessment guidance and missing S19 helper/runtime coverage; live code/docs showed those pieces were already delivered. The only necessary implementation was to correct this stale ticket narrative, verify the live contract, and archive the ticket accurately.
- Verification results: `rg` doc-surface check passed; `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action` passed; `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically` passed; `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts` passed; `cargo test -p worldwake-ai` passed; `cargo test --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed.
