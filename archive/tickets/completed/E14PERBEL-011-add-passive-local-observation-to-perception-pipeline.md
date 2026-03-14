# E14PERBEL-011: Add Passive Local Observation To Perception Pipeline

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` perception behavior and perception-focused test coverage
**Deps**: E14PERBEL-005, E14PERBEL-006, `specs/E14-perception-beliefs.md`

## Problem

The current perception pipeline updates `AgentBeliefStore` only from emitted events. That is sufficient for explicit world changes, but it leaves a gap for static or already-present local state:

- a co-located corpse, workstation, or grave plot may exist without any new event that refreshes local awareness
- an agent can stand next to a visible entity and still fail to plan around it because no fresh event passed through perception

This creates a brittle boundary where subjective planning correctness depends on test setup order or unrelated world mutations instead of on stable local observation rules.

## Assumption Reassessment (2026-03-14)

1. `specs/E14-perception-beliefs.md` requires direct perception based on visibility/locality. It does not require all useful knowledge to arrive only via mutation events.
2. The current `perception_system()` only processes `event_log.events_at_tick(tick)` and updates beliefs from entities referenced by those events. There is no passive same-place observation pass for already-present local entities.
3. The existing perception tests live in `crates/worldwake-systems/src/perception.rs`. The ticket's original reference to `crates/worldwake-systems/tests/perception_integration.rs` does not match the repo.
4. `crates/worldwake-ai/tests/golden_harness/mod.rs` currently performs broad omniscient `refresh_test_beliefs()` sync after setup and after each tick. That helper is compensating for more than just this ticket's gap:
   - it seeds remote entities
   - it seeds entities before any production perception tick has run
   - it covers broader test ergonomics outside same-place static observation
5. Because of that, "reduce/remove compensating test-only belief seeding" is not a guaranteed outcome of this ticket. Removing the omniscient harness sync wholesale would be a larger architectural change than passive local observation alone.

## Architecture Check

1. Adding passive local observation aligns with Principles 7, 12, and 13: knowledge still arrives locally and physically, but it no longer depends on unrelated event emissions.
2. This must be implemented as a lawful perception mechanism, not as planner-side world peeking.
3. The resulting belief updates must remain deterministic, attributable, and capacity-bounded.
4. This is a net architectural improvement over the current event-only behavior:
   - it removes a brittle dependency on setup order and incidental mutation events
   - it keeps belief acquisition in the perception system instead of leaking world truth into planner/tests
   - it makes same-place awareness a durable rule rather than a harness convention
5. A separate future cleanup may still be warranted for the AI golden harness, but that should be driven by a broader review of belief-view ergonomics, not bundled into this narrower ticket.

## What to Change

### 1. Extend perception with passive same-place observation

Introduce a direct-observation pass that lets co-located agents refresh beliefs about visible local entities even when no new mutation event occurred that tick.

### 2. Keep capacity and fidelity rules intact

Passive observation must still respect:

- `PerceptionProfile.observation_fidelity`
- memory retention and capacity
- locality constraints

### 3. Add integration coverage for static-world local awareness

Add tests that prove co-located visible entities become known without requiring a separate mutation event to happen after setup.

### 4. Do not force unrelated belief-harness rewrites

Only touch the AI golden harness if the new production behavior clearly makes a specific helper unnecessary. Do not turn this ticket into a broad de-omniscience refactor of test infrastructure.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify tests in-place)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a narrow, justified cleanup becomes possible after production behavior changes)

## Out of Scope

- Rumor/report propagation (`E15`)
- Trait-boundary cleanup (`E14PERBEL-009`)
- Any omniscient planner shortcut
- Broad removal of `refresh_test_beliefs()` from the golden harness without a wider belief-test architecture pass

## Acceptance Criteria

### Tests That Must Pass

1. New passive-observation coverage in `crates/worldwake-systems/src/perception.rs`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai --test golden_ai_decisions`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

### Invariants

1. Passive observation stays local and visibility-bounded.
2. Agents do not gain remote knowledge for free.
3. Belief updates remain traceable to direct observation, not planner omniscience.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — static co-located entities become known through passive observation even when no new event references them.
2. `crates/worldwake-systems/src/perception.rs` — passive observation respects observation fidelity and memory-capacity boundaries.
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — regression check only if a specific scenario meaningfully benefits from the production change.
4. `crates/worldwake-ai/tests/golden_harness/mod.rs` — modify only if a narrow helper cleanup is justified by the final implementation.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-ai --test golden_ai_decisions`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Added a passive same-place observation pass to `perception_system()` so agents can refresh beliefs about already-present co-located entities even when no current-tick event references them.
  - Added focused perception tests covering static same-place observation without event references and the zero-fidelity boundary for passive observation.
  - Updated the existing `participants_only` test so it still isolates event visibility semantics under the new passive observation architecture.
  - Corrected this ticket's assumptions and scope to match the actual repo structure and the broader reality of the omniscient golden-harness belief sync.
- Deviations from the corrected plan:
  - No `worldwake-ai` production or test-harness cleanup was performed. Passive same-place observation improves the architecture, but it does not by itself justify removing the broad omniscient `refresh_test_beliefs()` helper from golden tests.
  - Coverage stayed in `crates/worldwake-systems/src/perception.rs` instead of creating a new perception integration test file, because that is where the current perception-focused tests already live.
- Verification results:
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai --test golden_ai_decisions`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
