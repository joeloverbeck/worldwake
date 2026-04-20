# S112PORPLAN-003: Feasibility probe

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new `worldwake-ai` module `feasibility_probe.rs`
**Deps**: S112PORPLAN-002 (consumes `FeasibilityVerdict`)

## Problem

Before committing tactical search budget to a portfolio slot, S112 D3 mandates a cheap probe: three belief-scoped checks (discrepancy/blocker memory; known-target existence; affordance existence) that together decide whether a slot is `Plausible` or `RejectedBeforeSearch`. The probe never runs tactical search — it is O(candidates × belief-lookup), not O(search budget).

This ticket introduces the probe function standalone so ticket 002's `assemble_portfolio` can accept it as a closure and ticket 005 can plug in the real probe at integration time.

## Assumption Reassessment (2026-04-20)

1. `DiscrepancyMemory` (`crates/worldwake-core/src/discrepancy.rs:52-77`) provides `is_suppressed(&BlockerKey, Tick) -> bool`. `BlockerMemory` (`crates/worldwake-core/src/blocker_memory.rs:24-40`) provides `is_blocked(&GoalKey, Option<EntityId>, Option<EntityId>, Option<ActionDefId>, Tick) -> bool`. Both are live and agent-owned, accessed through the agent's belief store.
2. `Discrepancy` variants relevant to the probe exist at `crates/worldwake-core/src/discrepancy.rs:7-25`: `MissingObservation` (for unknown-target rejections), `RouteUnknown` (for known-but-unreachable-place rejections). `PartialExecutionDrift` is the fallback when a `BlockerMemory` hit doesn't carry its own discrepancy. No new `Discrepancy` variant is introduced by S112.
3. Shared boundary: `GoalBeliefView` (`crates/worldwake-sim/src/belief_view.rs`) is the agent-scoped read surface for belief-backed target existence and route knowledge. The probe reads *only* from belief-scoped state — never from authoritative world state (FND-14).
4. Mismatch + correction: the original spec D3 referenced `Discrepancy::StructurallyImpossible`, which does not exist. Reassessment corrected to reuse `MissingObservation` + `RouteUnknown`. No new variant needed.

## Architecture Check

1. Belief-only probe (FND-14): reads `DiscrepancyMemory`, `BlockerMemory`, and the agent's `GoalBeliefView` — never touches authoritative world state. A correct perception pipeline would deliver any missing fact through belief; the probe respects that contract.
2. Bounded probe cost (FND-20): O(candidates × belief-lookup) is bounded by `max_candidates_to_plan × number_of_slot_categories` per tick. Unlike tactical search, the probe cannot consume search budget.
3. Dampener (FND-11 / FND-20): S109's typed TTLs (`CognitiveProfile::*_backoff_ticks`) naturally dampen probe-reject loops — a rejected slot stays rejected until the `DiscrepancyEntry::expires_tick` elapses.

## Verification Layers

1. `RejectedBeforeSearch { reason }` is produced for a specific `BlockerKey` hit → focused unit test constructing a `DiscrepancyMemory` with a single non-expired `DiscrepancyEntry`; probe returns the recorded `Discrepancy`.
2. `Plausible` is produced when no suppressive memory entry exists and target/affordance are belief-known → focused unit test with an empty memory and a mock `GoalBeliefView`.
3. Single-layer ticket — probe is pure function over belief + memory, no action or event-log surface.

## What to Change

### 1. Create `crates/worldwake-ai/src/feasibility_probe.rs`

Declare:

```rust
use worldwake_core::{Discrepancy, Tick};
use crate::agent_tick::portfolio::FeasibilityVerdict;
use crate::goal_model::RankedGoal;

pub(crate) struct ProbeContext<'a> {
    pub belief_view: &'a dyn GoalBeliefView,
    pub discrepancy_memory: &'a DiscrepancyMemory,
    pub blocker_memory: &'a BlockerMemory,
    pub current_tick: Tick,
    pub agent_place: Option<EntityId>,
}

pub(crate) fn probe(
    ranked: &RankedGoal,
    context: &ProbeContext<'_>,
) -> FeasibilityVerdict;
```

### 2. Implement the three probe checks

Order enforced (fail-fast on cheapest first):

1. **Discrepancy/blocker memory check**: build the goal's `BlockerKey { goal_key, place, target, action_def }` from `ranked.grounded` anchor. If `context.discrepancy_memory.is_suppressed(&key, current_tick)`, return `RejectedBeforeSearch { reason: entry.discrepancy }` using the recorded discrepancy. If `context.blocker_memory.is_blocked(&key.goal_key, key.place, key.target, key.action_def, current_tick)` (without a corresponding `DiscrepancyMemory` entry), return `RejectedBeforeSearch { reason: Discrepancy::PartialExecutionDrift }`.
2. **Known-target check**: for goals with an anchor referencing a target entity, confirm the agent believes that target exists (via `GoalBeliefView` accessor). Return `RejectedBeforeSearch { reason: Discrepancy::MissingObservation }` when the target is unknown. For goals with a target place known but no believed route, return `RejectedBeforeSearch { reason: Discrepancy::RouteUnknown }`.
3. **Affordance existence check**: at least one affordance of the goal's action-kind must be believed-reachable from `context.agent_place`. If none exists in belief, return `RejectedBeforeSearch { reason: Discrepancy::MissingObservation }`.

If all three pass, return `FeasibilityVerdict::Plausible`.

### 3. Register module

Add `pub(crate) mod feasibility_probe;` to `crates/worldwake-ai/src/lib.rs`.

### 4. Unit tests

In the same file under `#[cfg(test)]`:

1. `probe_rejects_on_discrepancy_memory_hit` — pre-seed `DiscrepancyMemory` with a non-expired `DiscrepancyEntry` for the goal's `BlockerKey`; probe returns `RejectedBeforeSearch { reason: entry.discrepancy }`.
2. `probe_rejects_on_blocker_memory_hit` — pre-seed `BlockerMemory` without a `DiscrepancyMemory` counterpart; probe returns `RejectedBeforeSearch { reason: PartialExecutionDrift }`.
3. `probe_rejects_on_missing_target` — belief view reports target does not exist; probe returns `MissingObservation`.
4. `probe_passes_when_belief_satisfied` — empty memories, target known, affordance reachable; probe returns `Plausible`.

## Files to Touch

- `crates/worldwake-ai/src/feasibility_probe.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `mod feasibility_probe;`)

## Out of Scope

- Runtime wiring into `assemble_portfolio` and the planning loop — ticket 005.
- Modifying any `validate_*` or `can_exercise_control` function; probe reads only, never writes.
- New `Discrepancy` variants — corrected during reassessment to reuse existing variants.
- Extending `GoalBeliefView` with new accessors; the probe uses only the accessors present today.

## Acceptance Criteria

### Tests That Must Pass

1. The 4 new unit tests in `feasibility_probe.rs` pass.
2. Existing suite: `cargo test -p worldwake-ai`, `cargo test --workspace`.
3. `cargo clippy --workspace --all-targets -- -D warnings` remains clean.

### Invariants

1. Probe reads only from belief-scoped state (agent's `GoalBeliefView`, `DiscrepancyMemory`, `BlockerMemory`). No authoritative world-state reads.
2. Probe does not invoke tactical search (no calls into `search_plan` or `build_planning_snapshot_*`).
3. Check order is fail-fast — memory hits short-circuit the belief-view accessors.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/feasibility_probe.rs` — inline `#[cfg(test)]` module with 4 unit tests per the What to Change section.

### Commands

1. `cargo test -p worldwake-ai feasibility_probe`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
