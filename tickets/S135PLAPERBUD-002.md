# S135PLAPERBUD-002: GoalBeliefView accessor for ObservationOmissionLog

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief-view trait surface
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`

## Problem

Tickets 004 and 005 (Discrepancy::Omission revalidation, RootCandidateTrace annotation) need to read each agent's `ObservationOmissionLog` from the AI crate. Direct world reads from AI would violate FND-26 (systems interact through state, not direct calls). The canonical surface is `GoalBeliefView` in `worldwake-sim`. This ticket adds a single accessor method, its `RuntimeBeliefView` backing, and the `impl_goal_belief_view!` macro forwarding so consumers in tickets 004/005 can read the log without reaching into world state.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalBeliefView` trait lives at `crates/worldwake-sim/src/belief_view.rs:268` with 20+ existing accessors (`agent_belief_store`, `claim_confidence_threshold`, `discrepancy_memory`, `blocker_memory`, `repair_memory`, `learned_opportunity_memory`, `survey_memory`, `known_institutional_beliefs`, etc.). `RuntimeBeliefView` impl backs each accessor by reading from world state. `impl_goal_belief_view!` macro (or blanket impl) forwards methods to non-runtime impls. Existing focused tests: `crates/worldwake-sim/src/belief_view.rs` cfg-test block (line numbers to confirm during reassessment).
2. No existing accessor surfaces `ObservationOmissionLog` (validated during S135 reassessment). Ticket 001 stores the log inside `AgentBeliefStore`, so the accessor signature mirrors existing per-agent belief-store reads while returning the nested omission log.
3. Shared abstraction boundary under audit: the `GoalBeliefView` trait surface (the canonical AI-readable view of agent state) plus its delta-pinned accessor contract. The new method must follow the same pattern as adjacent existing accessors and respect the trait's `Sync + Send` requirements (verify against current trait bounds during implementation).

## Architecture Check

1. The accessor returns `Option<&ObservationOmissionLog>` to match the existing nullable belief-store pattern (an agent may not have `AgentBeliefStore` yet during partial-state replay or test harnesses). After ticket 001, every fully bootstrapped agent has `Some(&AgentBeliefStore.observation_omission_log)`.
2. No backward compatibility — adding a trait method requires every implementation to be updated. The `impl_goal_belief_view!` macro forwards uniformly, so non-runtime impls (e.g., test mocks) get the new method automatically through the macro.
3. FND-26 alignment: AI consumers reach `ObservationOmissionLog` through this trait, not via direct world or component-table reads. The trait is the canonical AI-side accessor surface.

## Verification Layers

1. Trait method exists and compiles → focused unit test in `crates/worldwake-sim/src/belief_view.rs` cfg-test block (or workspace build is the test for the trait-method-existence claim).
2. `RuntimeBeliefView` returns the seeded `ObservationOmissionLog::default()` for a freshly-created agent → focused unit test using `RuntimeBeliefView::new(...)` against a minimal world.
3. The `impl_goal_belief_view!` macro forwards the new method to any non-runtime impl → confirmed via workspace compile (any impl that doesn't have the method via macro is a compile error).

## What to Change

### 1. Trait method on `GoalBeliefView`

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait near existing per-agent component accessors:

```rust
fn observation_omission_log(&self, agent: EntityId) -> Option<&ObservationOmissionLog>;
```

Place it adjacent to existing per-agent belief-store accessors (e.g., near `agent_belief_store`). Update the trait's import line to bring `ObservationOmissionLog` into scope (`use worldwake_core::ObservationOmissionLog;` or similar).

### 2. `RuntimeBeliefView` impl

In the same file, add the `RuntimeBeliefView::observation_omission_log` impl that reads via the world:

```rust
fn observation_omission_log(&self, agent: EntityId) -> Option<&ObservationOmissionLog> {
    self.world
        .get_component_agent_belief_store(agent)
        .map(|store| &store.observation_omission_log)
}
```

(The exact form depends on whether `RuntimeBeliefView` carries `&World` directly or via a wrapper — match the existing accessor implementations.)

### 3. Macro / blanket-impl forwarding

If `impl_goal_belief_view!` macro forwards methods, add the new method to the macro's expansion. If the trait uses a blanket impl, add the forwarding line. Match the existing pattern used by sibling accessors (likely visible by reading the existing `agent_belief_store` implementation).

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)

## Out of Scope

- Constructing or mutating `ObservationOmissionLog` — that lives in ticket 003 (perception write integration).
- Reading the log from the AI crate — those wirings live in tickets 004 (Discrepancy::Omission) and 005 (RootCandidateTrace).
- New goldens — ticket 007.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim --lib belief_view` passes (new accessor returns the seeded `ObservationOmissionLog::default()` for a freshly-created agent).
2. `cargo build --workspace` succeeds — every `GoalBeliefView` impl carries the new method (manual or via macro).
3. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. `GoalBeliefView::observation_omission_log` returns `Some(&ObservationOmissionLog)` for any agent whose `AgentBeliefStore` has been seeded by `create_agent()`.
2. The accessor never reads from world state via paths other than the registered `get_component_agent_belief_store` accessor (preserves FND-26).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` cfg-test block — new test: construct a minimal world, create an agent via `World::create_agent()`, assert `RuntimeBeliefView::observation_omission_log(agent)` returns `Some(&ObservationOmissionLog::default())` through the nested `AgentBeliefStore` field.

### Commands

1. `cargo test -p worldwake-sim --lib belief_view`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
