# S106GROITEDEC-004: Golden E2E and integration tests

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/S106GROITEDEC-001.md, archive/tickets/S106GROITEDEC-002.md, archive/tickets/S106GROITEDEC-003.md

## Problem

The item decay system (tickets 001-003) has unit test coverage for individual components. This ticket adds golden E2E tests that prove the full system works end-to-end: waste production by agents interacts with decay to reach a steady state (FND-11 dampener), conservation invariants hold across the simulation, and existing golden tests are not broken by default decay values.

## Assumption Reassessment (2026-04-17)

1. Golden tests live in `crates/worldwake-ai/tests/`, but the live suite already includes long-running 600-1440 tick scenarios (`golden_survival_baseline.rs`, `golden_survival_scattered.rs`). The earlier “typically < 200 ticks” assumption was stale, so the full `cargo test -p worldwake-ai` regression is a real decay-compatibility check, not a formality.
2. The golden harness in `crates/worldwake-ai/tests/golden_harness/mod.rs` is the right seam for this ticket. It already drives the full AI/action/system loop without needing a scenario RON file.
3. `commit_relieve_wilderness` in `crates/worldwake-systems/src/needs_actions.rs` creates one Waste lot (quantity 1) per commit and tags the event with `EventTag::WildernessRelief`. That tag is the authoritative created-waste counter for this golden.
4. `relieve_wilderness` requires outdoor place tags, not a Latrine tag. A lawful repeat-production scenario should therefore keep agents at an outdoor place such as `ForestPath` and avoid seeding remote latrine knowledge that would change the action path under test.
5. `verify_live_lot_conservation` and `verify_authoritative_conservation` remain useful integration checks, but in this scenario the strongest expected-total contract is `wilderness_relief_events - item_decay_events == live_waste_lots` at each checkpoint because Waste is created only by `relieve_wilderness` and archived only by `ItemDecay`.
6. `EventTag::ItemDecay` is the correct decay tag, and every such event should also carry `EventTag::WorldMutation`.

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

Create a dedicated golden harness scenario with:
- 2 AI agents at an outdoor place with bladder-driven needs
- No remote latrine knowledge, so repeated local `relieve_wilderness` remains the lawful action path
- `commodity_decay` set to `{Waste: 200}`
- Run for 400 ticks with periodic checkpoint assertions

Assert:
- Ground Waste entity count is bounded by the waste created within the live 200-tick decay window
- Archived Waste entity count is > 0 (decay happened)
- At least one event with `EventTag::ItemDecay` exists in the event log

### 2. Conservation integration test

Within the golden test or as a separate focused test:

At checkpoint ticks through the run, verify using `verify_live_lot_conservation` and `verify_authoritative_conservation` that the expected live Waste total equals `wilderness_relief_events - item_decay_events`.

### 3. Event log tag test

Verify that every decay event has both `EventTag::ItemDecay` and `EventTag::WorldMutation` tags. This can be asserted within the golden test after the run completes.

### 4. Regression verification

Run the full existing golden test suite to confirm default decay values (Waste: 200, Apple: 720) do not interfere with any existing test scenario. No existing test should see unexpected archival of items.

## Files to Touch

- `crates/worldwake-ai/tests/golden_item_decay.rs` (new golden E2E test for waste decay steady state)

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

1. `crates/worldwake-ai/tests/golden_item_decay.rs` — new golden E2E test for waste decay steady state and conservation

### Commands

1. `cargo test -p worldwake-ai golden_waste_decay_reaches_steady_state` — targeted golden test
2. `cargo test -p worldwake-ai` — full golden suite regression
3. `python3 scripts/golden_inventory.py --write --check-docs` — refresh golden inventory/docs
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint

## Outcome

Completed on 2026-04-17.

- Added [golden_item_decay.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_item_decay.rs) with Scenario 342, a dedicated long-run AI golden that keeps two bladder-driven agents at `ForestPath`, produces repeated local `relieve_wilderness` Waste, and runs long enough for `ItemDecay` to archive older Waste.
- The golden proves the real S106 end-to-end contract instead of only unit seams: live Waste stays bounded by the active 200-tick decay window, `ItemDecay` emits `WorldMutation` alongside its own tag, and authoritative Waste conservation holds at repeated checkpoints via `wilderness_relief_events - item_decay_events == live_waste_lots`.
- Refreshed the generated golden inventory/docs so the new file and Scenario 342 are part of the tracked golden coverage surface.

## Deviations

- The draft ticket proposed a latrine-oriented production setup. Reassessment corrected that to an outdoor-place setup because `relieve_wilderness` is gated by outdoor tags, not `PlaceTag::Latrine`, and a local-only belief surface was the cleanest way to keep the repeated action path under test stable.
- The draft also implied existing goldens were usually shorter than the Waste decay threshold. The live suite already contains longer 600-1440 tick runs, so the full `cargo test -p worldwake-ai` regression remained in-scope as a meaningful compatibility check.

## Verification Result

- Passed `cargo test -p worldwake-ai golden_waste_decay_reaches_steady_state`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
