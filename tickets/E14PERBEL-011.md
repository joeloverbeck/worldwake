# E14PERBEL-011: Add Passive Local Observation To Perception Pipeline

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` perception behavior and related integration coverage
**Deps**: E14PERBEL-005, E14PERBEL-006, `specs/E14-perception-beliefs.md`

## Problem

The current perception pipeline updates `AgentBeliefStore` only from emitted events. That is sufficient for explicit world changes, but it leaves a gap for static or already-present local state:

- a co-located corpse, workstation, or grave plot may exist without any new event that refreshes local awareness
- an agent can stand next to a visible entity and still fail to plan around it because no fresh event passed through perception

This creates a brittle boundary where subjective planning correctness depends on test setup order or unrelated world mutations instead of on stable local observation rules.

## Assumption Reassessment (2026-03-14)

1. `specs/E14-perception-beliefs.md` requires direct perception based on visibility/locality. It does not require all useful knowledge to arrive only via mutation events.
2. The current `perception_system()` primarily consumes `event_log.events_at_tick(tick)`. That means passive same-place observation of already-present entities is not yet modeled as a first-class perception path.
3. Golden and integration harnesses currently need explicit belief seeding to compensate for this gap. That is acceptable in tests short-term, but it is not the clean long-term architecture.

## Architecture Check

1. Adding passive local observation aligns with Principles 7, 12, and 13: knowledge still arrives locally and physically, but it no longer depends on unrelated event emissions.
2. This must be implemented as a lawful perception mechanism, not as planner-side world peeking.
3. The resulting belief updates must remain deterministic, attributable, and capacity-bounded.

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

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/tests/perception_integration.rs` (modify or create)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (reduce/remove compensating test-only belief seeding if production behavior makes it unnecessary)

## Out of Scope

- Rumor/report propagation (`E15`)
- Trait-boundary cleanup (`E14PERBEL-009`)
- Any omniscient planner shortcut

## Acceptance Criteria

### Tests That Must Pass

1. New passive-observation integration tests
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai --test golden_ai_decisions`
4. `cargo test --workspace`

### Invariants

1. Passive observation stays local and visibility-bounded.
2. Agents do not gain remote knowledge for free.
3. Belief updates remain traceable to direct observation, not planner omniscience.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/perception_integration.rs` — static co-located entities become known through passive observation.
2. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — scenarios relying on nearby visible entities work without broad test-only omniscient belief sync.
3. `crates/worldwake-ai/tests/golden_harness/mod.rs` — remove or narrow compensating sync once production perception covers the cases.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-ai --test golden_ai_decisions`
3. `cargo test --workspace`
