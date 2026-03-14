# E14PERBEL-013: Replace Omniscient Golden Harness Belief Refresh With Lawful Test Belief Setup

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` golden harness belief setup, selected golden scenario setup, and regression coverage around belief-local test execution
**Deps**: `archive/tickets/completed/E14PERBEL-011-add-passive-local-observation-to-perception-pipeline.md`, `archive/tickets/E14PERBEL-006.md`, `specs/E14-perception-beliefs.md`, `specs/IMPLEMENTATION-ORDER.md`

## Problem

The AI golden harness currently bypasses the intended E14 belief architecture:

- `crates/worldwake-ai/tests/golden_harness/mod.rs::refresh_test_beliefs()` iterates every agent over every entity and writes `DirectObservation` snapshots directly into each `AgentBeliefStore`
- the helper runs after setup mutations and after every simulated tick
- this grants broad omniscient world knowledge to tests that are supposed to exercise belief-mediated planning

That weakens the long-term architecture in three ways:

1. it can hide real bugs in perception and belief-gated candidate generation
2. it makes golden behavior depend on a test-only omniscient sync that production does not have
3. it blurs the E14 boundary between lawful local knowledge acquisition and test convenience

The current production code is cleaner than this harness behavior. The test architecture should catch up.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` currently defines `refresh_test_beliefs()` and calls it from setup helpers such as `seed_agent()`, `give_commodity()`, `place_workstation_with_source()`, and `GoldenHarness::step_once()`.
2. The helper performs broad full-world belief sync rather than localized lawful perception:
   - it seeds remote entities
   - it seeds entities that have never been perceived
   - it seeds beliefs after every tick regardless of whether a legal information path exists
3. `archive/tickets/completed/E14PERBEL-011-add-passive-local-observation-to-perception-pipeline.md` intentionally did not remove this helper because passive same-place observation alone does not justify broad harness de-omniscience by itself.
4. `specs/E14-perception-beliefs.md` and the Phase 3 gate in `specs/IMPLEMENTATION-ORDER.md` require belief-only planning and explicit information channels. A permanent omniscient golden harness is therefore architecturally mismatched with the spec.
5. The cleanup should not be a naive delete:
   - some golden tests genuinely need explicit belief setup for scenarios that begin with prior knowledge
   - those cases should be modeled as narrow, scenario-authored belief fixtures or lawful perception-driving setup, not as a global omniscient refresh
6. No active ticket in `tickets/` currently owns this harness architecture cleanup.

## Architecture Check

1. The clean design is to make golden tests choose their knowledge source explicitly:
   - lawful local perception by running the real simulation/perception path
   - narrow scenario-authored initial beliefs when prior knowledge is the point of the test
2. This is better than the current global refresh because it keeps tests aligned with the same belief constraints production code is expected to honor.
3. The replacement should be explicit and local, not another hidden abstraction that silently reintroduces omniscience under a different name.
4. A small library of targeted belief-seeding helpers is acceptable if each helper makes the information path explicit and scoped. A universal “sync everything” helper is not.
5. No backwards-compatibility shim is permitted. `refresh_test_beliefs()` should be removed rather than retained beside a new lawful path.

## What to Change

### 1. Remove global omniscient belief refresh from the golden harness

Delete `refresh_test_beliefs()` and stop calling it automatically from shared harness helpers and `GoldenHarness::step_once()`.

Replace the shared default with one of these explicit patterns, depending on the scenario:

- no belief seeding at all, when the scenario should begin from ignorance
- localized explicit belief setup for a specific actor and entity set
- lawful setup that lets the real perception system create the beliefs

The default harness behavior should no longer grant worldwide `DirectObservation` snapshots.

### 2. Introduce narrow, explicit belief-fixture helpers where needed

If some goldens require initial knowledge to remain readable, add narrowly scoped helpers such as:

- seed one actor's belief about one entity
- seed one actor's belief about colocated local state
- seed one actor's belief about a specific commodity source

These helpers must:

- operate on a single actor or tightly bounded actor set
- require explicit targets
- make the belief source and observed tick explicit
- avoid scanning `world.entities()`

Do not add a generalized alias for “synchronize beliefs from world.”

### 3. Rework affected goldens to use lawful or explicit belief setup

Update the golden suites that currently rely on the omniscient refresh. Prefer the smallest local setup at each call site over growing the shared harness unnecessarily.

Likely affected files include:

- `crates/worldwake-ai/tests/golden_ai_decisions.rs`
- `crates/worldwake-ai/tests/golden_production.rs`
- `crates/worldwake-ai/tests/golden_trade.rs`
- `crates/worldwake-ai/tests/golden_combat.rs`
- `crates/worldwake-ai/tests/golden_care.rs`

### 4. Add regression coverage for the harness boundary itself

Add tests that prove the harness no longer silently grants remote or unperceived knowledge. At minimum:

- a golden/setup regression where an agent does not know about a remote entity unless that knowledge was explicitly seeded or lawfully perceived
- a positive regression where a scenario with explicit initial beliefs still works without global omniscient sync

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify if needed)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify if needed)
- `crates/worldwake-ai/tests/golden_care.rs` (modify if needed)

## Out of Scope

- Production perception changes beyond what already landed in `E14PERBEL-011`
- Rumor/report propagation (`E15`)
- Reintroducing any omniscient planner or belief-view path
- Broad refactors of `PerAgentBeliefView` unrelated to test harness cleanup
- Rewriting every golden scenario if only a subset actually depends on the omniscient helper

## Acceptance Criteria

### Tests That Must Pass

1. The shared golden harness no longer performs automatic full-world belief refresh after setup or after each tick.
2. At least one regression test proves a golden actor does not receive remote/unperceived knowledge by default.
3. Golden scenarios that need prior knowledge use explicit narrow belief fixtures or lawful perception-driving setup instead of a global sync.
4. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
5. Existing suite: `cargo test -p worldwake-ai --test golden_production`
6. Existing suite: `cargo test -p worldwake-ai --test golden_trade`
7. Existing suite: `cargo test -p worldwake-ai --test golden_combat`
8. Existing suite: `cargo test -p worldwake-ai --test golden_care`
9. Existing suite: `cargo test --workspace`
10. Existing lint: `cargo clippy --workspace`

### Invariants

1. Golden tests do not gain free global knowledge that production agents could not lawfully have.
2. Any initial knowledge used by a golden scenario is explicit, scoped, and attributable.
3. The harness does not preserve a compatibility alias for the deleted omniscient refresh behavior.
4. Deterministic replay remains stable for updated golden scenarios.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — add harness-boundary regression coverage proving default setup does not seed remote knowledge.
   Rationale: locks the architectural contract at the shared test entry point.
2. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — update one scenario to use explicit local belief setup or lawful perception instead of hidden global sync.
   Rationale: proves the replacement pattern is viable in a core decision-flow suite.
3. `crates/worldwake-ai/tests/golden_production.rs` — update at least one production-oriented golden that previously depended on broad knowledge seeding.
   Rationale: ensures the cleanup survives a nontrivial end-to-end scenario.
4. Additional golden files only as needed — convert scenarios that break once omniscient refresh is removed.
   Rationale: keep edits minimal and driven by real dependency rather than speculative rewrites.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo test -p worldwake-ai --test golden_trade`
4. `cargo test -p worldwake-ai --test golden_combat`
5. `cargo test -p worldwake-ai --test golden_care`
6. `cargo test --workspace`
7. `cargo clippy --workspace`
