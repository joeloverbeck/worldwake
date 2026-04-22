# S114PLASTGUA-012: Add lawful `ExpectationStore` ID allocation and restore counter integrity

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — add core-owned `ExpectationStore` record-allocation helper and migrate production callers away from ad hoc ID synthesis.
**Deps**: `archive/tickets/S114PLASTGUA-004.md`, `archive/tickets/S114PLASTGUA-008.md`

## Problem

`ExpectationStore` serializes a private `next_expectation_id` counter, but the live branch still had one production writer and several focused test helpers that did not maintain it consistently. `crates/worldwake-ai/src/plan_step_expectations.rs` synthesized new `ExpectationId`s directly from the live record set instead of advancing the serialized counter, and expectation-seeding helpers in `crates/worldwake-systems/src/search_actions.rs`, `crates/worldwake-systems/src/report_actions.rs`, and `crates/worldwake-systems/src/ask_about_person_actions.rs` duplicated similar ad hoc allocation. That left the stored counter stale relative to the actual records and weakened the expectation subsystem's long-term ID-allocation contract.

## Assumption Reassessment (2026-04-22)

1. `ExpectationStore` in `crates/worldwake-core/src/expectation.rs:80-90` still owns both `records: BTreeMap<ExpectationId, ExpectationRecord>` and a private serialized `next_expectation_id: ExpectationId`.
2. No public core-owned allocator helper exists today. The field is private to `worldwake-core`, so sibling crates cannot lawfully advance it directly.
3. Live writer sweep on the current branch shows one production bypass and several focused test-helper bypasses:
   - Production: `crates/worldwake-ai/src/plan_step_expectations.rs` scans `store.records.keys().max()` and inserts without updating `next_expectation_id`
   - Focused test helpers: `crates/worldwake-systems/src/search_actions.rs`, `crates/worldwake-systems/src/report_actions.rs`, and `crates/worldwake-systems/src/ask_about_person_actions.rs` still synthesize IDs directly while seeding expectation stores for tests
4. The serialized counter is part of the component contract, not dead test-only state. `crates/worldwake-core/src/delta.rs:520-535` includes `next_expectation_id` in the structural component example payload, so stale values persist across saved component state.
5. Shared boundary under audit: core-owned expectation-record allocation. The lawful end state is a single `worldwake-core` helper that allocates a fresh `ExpectationId`, inserts the record, repairs stale counters against live record IDs when needed, and advances `next_expectation_id`, with sibling crates constructing record payloads but not minting IDs themselves.
6. This is a cleanup / integrity ticket, not a behavior-change ticket. Existing expectation semantics (`Active`, `Overdue`, `Resolved`, `Expired`) remain unchanged.

## Architecture Check

1. Core-owned allocation is cleaner than per-crate ID synthesis because it restores one canonical source of truth for `ExpectationStore` identity management.
2. This removes a duplicated information path for "what ID comes next," aligning with FOUNDATIONS state-contract discipline and avoiding future cross-crate drift.

## Verification Layers

1. Allocator integrity (`inserted record gets fresh ID and counter advances`) -> focused `worldwake-core` unit test on `ExpectationStore`.
2. Caller migration (`plan_step_expectations` production writes and expectation-seeding helpers use the lawful helper`) -> targeted compile/test coverage in the owning crates.
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
- `crates/worldwake-ai/src/plan_step_expectations.rs`
- expectation-seeding test helpers that currently bypass the counter in:
  - `crates/worldwake-systems/src/search_actions.rs`
  - `crates/worldwake-systems/src/report_actions.rs`
  - `crates/worldwake-systems/src/ask_about_person_actions.rs`

The callers should build the record payload, delegate ID allocation/insertion to the new core helper, and stop deriving IDs from `records.len()` or `max()`.

### 3. Add focused coverage

Add/extend tests to prove:
- fresh insert increments `next_expectation_id`
- inserted records remain retrievable by the allocated ID
- migrated callers still create the expected records through their existing focused proof seams

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (modify)
- `crates/worldwake-ai/src/plan_step_expectations.rs` (modify)
- `crates/worldwake-systems/src/search_actions.rs` (modify)
- `crates/worldwake-systems/src/report_actions.rs` (modify)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify)

## Out of Scope

- Any new expectation lifecycle semantics
- Changes to `ExpectationBasis` or mismatch classification
- Broad refactors of last-seen or other memory stores

## Acceptance Criteria

### Tests That Must Pass

1. A focused `ExpectationStore` test proves inserting a new record returns a fresh ID and advances `next_expectation_id`.
2. Existing focused coverage for expectation-producing system actions stays green after helper migration.
3. Existing focused AI plan-step expectation coverage stays green after production caller migration.
4. `cargo test -p worldwake-core expectation`
5. `cargo test -p worldwake-systems expectation`
6. `cargo test -p worldwake-ai plan_step_expectations`

### Invariants

1. Production writers no longer synthesize `ExpectationId` from `records.keys().max()`, and focused expectation-seeding helpers no longer derive IDs from `records.len()`.
2. `ExpectationStore.next_expectation_id` always points past the highest allocated live record ID after helper-driven insertion, even when the persisted counter started stale.
3. Cross-crate callers construct record payloads but do not own ID-allocation policy.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/expectation.rs` tests module — new allocator-integrity coverage.
2. Existing focused expectation-producing action tests in `worldwake-systems` — no new scenario needed if helper migration stays within current proof seams.
3. `crates/worldwake-ai/src/plan_step_expectations.rs` tests module — confirm helper-driven inserts still write the expected records and advance the persisted counter.

### Commands

1. `cargo test -p worldwake-core expectation`
2. `cargo test -p worldwake-systems expectation`
3. `cargo test -p worldwake-ai plan_step_expectations`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-22.

- Added a core-owned `ExpectationStore::allocate_record(...)` helper plus a read-only `next_expectation_id()` accessor in `crates/worldwake-core/src/expectation.rs`.
- The allocator now normalizes stale persisted counters against the highest live record ID before insertion, then advances the serialized counter exactly once.
- Migrated the production plan-step expectation writer in `crates/worldwake-ai/src/plan_step_expectations.rs` to the new helper.
- Migrated focused expectation-seeding test helpers in `crates/worldwake-systems/src/search_actions.rs`, `crates/worldwake-systems/src/report_actions.rs`, and `crates/worldwake-systems/src/ask_about_person_actions.rs` to the same helper so expectation-store setup no longer bypasses the allocation contract.
- Added focused coverage for fresh allocation, stale-counter repair, round-trip persistence of the advanced counter, and AI-side counter advancement after plan-step expectation writes.

## Deviations

- Reassessment corrected the drafted ownership claim: `search_actions` and `report_actions` were not live production writers on this branch. The only production bypass was `plan_step_expectations`; the system-crate files were focused test helpers and were migrated as consistency fallout.

## Verification Result

- Passed `cargo test -p worldwake-core expectation`
- Passed `cargo test -p worldwake-systems expectation`
- Passed `cargo test -p worldwake-ai plan_step_expectations`
- Passed `./scripts/verify.sh`
