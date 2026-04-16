# S106GROITEDEC-004: Golden E2E and integration tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/S106GROITEDEC-001.md, archive/tickets/S106GROITEDEC-002.md, archive/tickets/S106GROITEDEC-003.md

## Problem

The item decay system (tickets 001-003) has unit test coverage for individual components. This ticket adds golden E2E tests that prove the full system works end-to-end: waste production by agents interacts with decay to reach a steady state (FND-11 dampener), conservation invariants hold across the simulation, and existing golden tests are not broken by default decay values.

## Assumption Reassessment (2026-04-16)

1. Golden tests live in `crates/worldwake-ai/tests/`. The golden test infrastructure uses `SystemExecutionContext` and full action registries. Golden tests run via `cargo test -p worldwake-ai`.
2. `verify_live_lot_conservation` and `verify_authoritative_conservation` exist at `crates/worldwake-core/src/conservation.rs:20,35`. These verify commodity quantity invariants.
3. `commit_relieve_wilderness` at `crates/worldwake-systems/src/needs_actions.rs:407-451` creates one Waste lot (quantity 1) per execution — the production source for the golden test.
4. Existing golden tests in `crates/worldwake-ai/tests/` typically run < 200 ticks. Default Waste decay is 200 ticks, so existing tests should not be affected by decay. Apple decay is 720 ticks — also well beyond typical test lengths. This must be verified by running the full golden suite after tickets 001-003 are implemented.
5. `EventTag::ItemDecay` (from ticket 002) is the tag to assert on for decay event verification.

## Architecture Check

1. Golden E2E tests prove the emergent dampener effect (FND-11) — something unit tests cannot verify because they don't run the full agent decision cycle. The steady-state assertion is the key proof.
2. No backward-compatibility shims. New tests only.

## Verification Layers

1. Waste reaches steady state under decay → golden E2E (count bounded over 400 ticks)
2. Archived waste count grows → golden E2E (monotonically increasing archive count)
3. Conservation holds → authoritative world state (verify_live_lot_conservation at checkpoints)
4. Event tags present on decay events → event-log delta (filter by EventTag::ItemDecay)
5. Existing golden tests unaffected → golden E2E regression (full suite passes)

## What to Change

### 1. Golden E2E test: waste decay steady state

In `crates/worldwake-ai/tests/` (new test file or added to existing golden test file):

Create a scenario with:
- 2 agents with survival needs (hunger, thirst, bladder) and AI control
- Places with Latrine tags enabling `relieve_wilderness`
- `commodity_decay` set to `{Waste: 200}` (or use default)
- Run for 400 ticks

Assert:
- Ground Waste entity count is bounded (never exceeds `~production_rate * decay_ticks`)
- Archived Waste entity count is > 0 (decay happened)
- At least one event with `EventTag::ItemDecay` exists in the event log

### 2. Conservation integration test

Within the golden test or as a separate focused test:

At tick 400 (or at compaction checkpoints), verify using `verify_live_lot_conservation` that the total Waste quantity (live lots) plus archived quantity equals total ever created.

### 3. Event log tag test

Verify that every decay event has both `EventTag::ItemDecay` and `EventTag::WorldMutation` tags. This can be asserted within the golden test after the run completes.

### 4. Regression verification

Run the full existing golden test suite to confirm default decay values (Waste: 200, Apple: 720) do not interfere with any existing test scenario. No existing test should see unexpected archival of items.

## Files to Touch

- `crates/worldwake-ai/tests/` (new or modify — golden E2E test for waste decay steady state)

## Out of Scope

- Unit tests for individual components (covered in tickets 001, 003)
- Decay for carried or stored items (spec non-goal)
- Multi-stage decomposition chains (spec non-goal)
- Environmental modifiers on decay rate (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. Golden waste decay steady-state test — waste count bounded, archives grow, event tags present
2. Conservation verification — live + archived = total created
3. Existing golden suite: `cargo test -p worldwake-ai` — all existing tests pass (regression)

### Invariants

1. Ground waste count is bounded by approximately `production_rate * decay_ticks` (FND-11 dampener).
2. `items_created - items_archived == live_item_count` at every checkpoint (conservation).
3. Default decay values (Waste: 200, Apple: 720) do not cause archival in any existing test scenario.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/` — new golden E2E test for waste decay steady state and conservation

### Commands

1. `cargo test -p worldwake-ai golden_waste_decay` — targeted golden test (name TBD)
2. `cargo test -p worldwake-ai` — full golden suite regression
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint
