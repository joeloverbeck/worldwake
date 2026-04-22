# S114PLASTGUA-012: Add lawful `ExpectationStore` ID allocation and restore counter integrity

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — add core-owned `ExpectationStore` record-allocation helper and migrate production callers away from ad hoc ID synthesis.
**Deps**: `archive/tickets/S114PLASTGUA-004.md`, `archive/tickets/S114PLASTGUA-008.md`

## Problem

`ExpectationStore` serializes a private `next_expectation_id` counter, but current production writers do not use or maintain it consistently. `crates/worldwake-systems/src/search_actions.rs`, `crates/worldwake-systems/src/report_actions.rs`, and `crates/worldwake-ai/src/plan_step_expectations.rs` all synthesize new `ExpectationId`s directly from the live record set instead of advancing the serialized counter. That leaves the stored counter stale relative to the actual records and weakens the expectation subsystem's long-term ID-allocation contract.

## Assumption Reassessment (2026-04-22)

1. `ExpectationStore` in `crates/worldwake-core/src/expectation.rs:80-90` still owns both `records: BTreeMap<ExpectationId, ExpectationRecord>` and a private serialized `next_expectation_id: ExpectationId`.
2. No public core-owned allocator helper exists today. The field is private to `worldwake-core`, so sibling crates cannot lawfully advance it directly.
3. Production writers currently bypass the counter:
   - `crates/worldwake-systems/src/search_actions.rs:497-498` uses `ExpectationId(store.records.len() as u64 + 1)`
   - `crates/worldwake-systems/src/report_actions.rs:1045-1046` uses the same pattern
   - `crates/worldwake-ai/src/plan_step_expectations.rs:18-24,46-60` scans `store.records.keys().max()` and inserts without updating `next_expectation_id`
4. The serialized counter is part of the component contract, not dead test-only state. `crates/worldwake-core/src/delta.rs:520-535` includes `next_expectation_id` in the structural component example payload, so stale values persist across saved component state.
5. Shared boundary under audit: core-owned expectation-record allocation. The lawful end state is a single `worldwake-core` helper that allocates a fresh `ExpectationId`, inserts the record, and advances `next_expectation_id`, with sibling crates constructing record payloads but not minting IDs themselves.
6. This is a cleanup / integrity ticket, not a behavior-change ticket. Existing expectation semantics (`Active`, `Overdue`, `Resolved`, `Expired`) remain unchanged.

## Architecture Check

1. Core-owned allocation is cleaner than per-crate ID synthesis because it restores one canonical source of truth for `ExpectationStore` identity management.
2. This removes a duplicated information path for "what ID comes next," aligning with FOUNDATIONS state-contract discipline and avoiding future cross-crate drift.

## Verification Layers

1. Allocator integrity (`inserted record gets fresh ID and counter advances`) -> focused `worldwake-core` unit test on `ExpectationStore`.
2. Caller migration (`search_actions`, `report_actions`, and AI plan-step writes use the lawful helper`) -> targeted compile/test coverage in the owning crates.
3. Serialized counter preservation (`ExpectationStore` round-trips with updated next ID after insertion) -> focused `worldwake-core` round-trip/unit test.
4. Single-layer ticket beyond that: no additional runtime/golden proof is required because the behavior change is internal allocation discipline, not planner semantics.

## What to Change

### 1. Add lawful core allocation helper

In `crates/worldwake-core/src/expectation.rs`, add a small `ExpectationStore` helper that:
- returns the next fresh `ExpectationId`
- inserts the provided `ExpectationRecord`
- advances `next_expectation_id` exactly once

Keep the API narrow and core-owned; sibling crates should not mutate `next_expectation_id` directly.

### 2. Migrate production writers

Replace ad hoc ID synthesis in:
- `crates/worldwake-systems/src/search_actions.rs`
- `crates/worldwake-systems/src/report_actions.rs`
- `crates/worldwake-ai/src/plan_step_expectations.rs`

The callers should build the record payload, delegate ID allocation/insertion to the new core helper, and stop deriving IDs from `records.len()` or `max()`.

### 3. Add focused coverage

Add/extend tests to prove:
- fresh insert increments `next_expectation_id`
- inserted records remain retrievable by the allocated ID
- migrated callers still create the expected records through their existing focused proof seams

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (modify)
- `crates/worldwake-systems/src/search_actions.rs` (modify)
- `crates/worldwake-systems/src/report_actions.rs` (modify)
- `crates/worldwake-ai/src/plan_step_expectations.rs` (modify)

## Out of Scope

- Any new expectation lifecycle semantics
- Changes to `ExpectationBasis` or mismatch classification
- Broad refactors of last-seen or other memory stores

## Acceptance Criteria

### Tests That Must Pass

1. A focused `ExpectationStore` test proves inserting a new record returns a fresh ID and advances `next_expectation_id`.
2. Existing focused coverage for expectation-producing system actions stays green after caller migration.
3. Existing focused AI plan-step expectation coverage stays green after caller migration.
4. `cargo test -p worldwake-core expectation`
5. `cargo test -p worldwake-systems expectation`
6. `cargo test -p worldwake-ai plan_step_expectations`

### Invariants

1. Production callers no longer synthesize `ExpectationId` from `records.len()` or `records.keys().max()`.
2. `ExpectationStore.next_expectation_id` always points past the highest allocated live record ID after helper-driven insertion.
3. Cross-crate callers construct record payloads but do not own ID-allocation policy.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/expectation.rs` tests module — new allocator-integrity coverage.
2. Existing focused expectation-producing action tests in `worldwake-systems` — no new scenario needed if caller migration stays within current proof seams.
3. `crates/worldwake-ai/src/plan_step_expectations.rs` tests module — confirm helper-driven inserts still write the expected records.

### Commands

1. `cargo test -p worldwake-core expectation`
2. `cargo test -p worldwake-systems expectation`
3. `cargo test -p worldwake-ai plan_step_expectations`
4. `./scripts/verify.sh`
