# S135PLAPERBUD-002: GoalBeliefView accessor for ObservationOmissionLog

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief-view trait surface
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`

## Problem

Tickets 004 and 005 (Discrepancy::Omission revalidation, RootCandidateTrace annotation) need to read each agent's `ObservationOmissionLog` from the AI crate. Direct world reads from AI would violate FND-26 (systems interact through state, not direct calls). The canonical surface is `GoalBeliefView` in `worldwake-sim`. This ticket adds a single accessor method backed by the existing `agent_belief_store` read surface and the live blanket `GoalBeliefView` implementation so consumers in tickets 004/005 can read the log without reaching into world state.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalBeliefView` trait lives at `crates/worldwake-sim/src/belief_view.rs` with 20+ existing accessors (`agent_belief_store`, `claim_confidence_threshold`, `discrepancy_memory`, `blocker_memory`, `repair_memory`, `learned_opportunity_memory`, `survey_memory`, `known_institutional_beliefs`, etc.). The live implementation uses a blanket `impl<T> GoalBeliefView for T` that forwards from narrower belief-view traits. Existing focused tests live in the `crates/worldwake-sim/src/belief_view.rs` cfg-test block.
2. No existing accessor surfaces `ObservationOmissionLog` (validated during S135 reassessment). Ticket 001 stores the log inside `AgentBeliefStore`, so the accessor signature mirrors existing per-agent belief-store reads while returning the nested omission log.
3. Shared abstraction boundary under audit: the `GoalBeliefView` trait surface (the canonical AI-readable view of agent state) plus its delta-pinned accessor contract. The new method must follow the same pattern as adjacent existing accessors and respect the trait's `Sync + Send` requirements (verify against current trait bounds during implementation).

## Architecture Check

1. The accessor returns `Option<&ObservationOmissionLog>` to match the existing nullable belief-store pattern (an agent may not have `AgentBeliefStore` yet during partial-state replay or test harnesses). After ticket 001, every fully bootstrapped agent has `Some(&AgentBeliefStore.observation_omission_log)`.
2. No backward compatibility — the method is added to the live trait surface. Because the landed method has a default implementation based on `agent_belief_store`, non-runtime test mocks inherit `None` unless they already expose an `AgentBeliefStore`, while runtime/per-agent views read the nested log.
3. FND-26 alignment: AI consumers reach `ObservationOmissionLog` through this trait, not via direct world or component-table reads. The trait is the canonical AI-side accessor surface.

## Verification Layers

1. Trait method exists and compiles → focused unit test in `crates/worldwake-sim/src/belief_view.rs` cfg-test block (or workspace build is the test for the trait-method-existence claim).
2. `PerAgentBeliefView` returns the seeded `ObservationOmissionLog::default()` for a freshly-created agent → focused unit test using `PerAgentBeliefView::from_world(...)` against a minimal world.
3. The blanket `GoalBeliefView` implementation keeps downstream trait consumers compiling → confirmed via workspace build.

## What to Change

### 1. Trait method on `GoalBeliefView`

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait near existing per-agent component accessors:

```rust
fn observation_omission_log(&self, agent: EntityId) -> Option<&ObservationOmissionLog>;
```

Place it adjacent to existing per-agent belief-store accessors (e.g., near `agent_belief_store`). Update the trait's import line to bring `ObservationOmissionLog` into scope (`use worldwake_core::ObservationOmissionLog;` or similar).

### 2. Runtime/per-agent backing

In the same file, add `GoalBeliefView::observation_omission_log` as a default method that reads via the existing `agent_belief_store` accessor:

```rust
fn observation_omission_log(&self, agent: EntityId) -> Option<&ObservationOmissionLog> {
    self.agent_belief_store(agent)
        .map(|store| &store.observation_omission_log)
}
```

For `PerAgentBeliefView`, `agent_belief_store` is already backed by the stored per-agent `AgentBeliefStore` reference built from `World::get_component_agent_belief_store`.

### 3. Blanket-impl forwarding

The live branch has no `impl_goal_belief_view!` macro. The existing blanket implementation of `GoalBeliefView` remains the forwarding surface; no separate macro expansion is required.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `archive/specs/S135-planner-perception-budget.md` (truth-sync live blanket implementation wording)

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

1. `crates/worldwake-sim/src/belief_view.rs` cfg-test block — new test: construct a minimal world, create an agent via `WorldTxn::create_agent()`, assert `GoalBeliefView::observation_omission_log(agent)` on `PerAgentBeliefView::from_world(...)` returns `Some(&ObservationOmissionLog::default())` through the nested `AgentBeliefStore` field.

### Commands

1. `cargo test -p worldwake-sim --lib belief_view`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-05.

- Added `GoalBeliefView::observation_omission_log(agent) -> Option<&ObservationOmissionLog>` in `crates/worldwake-sim/src/belief_view.rs`.
- The accessor reads through the existing `GoalBeliefView::agent_belief_store` surface and returns the nested `AgentBeliefStore.observation_omission_log`, preserving the single canonical belief-store path added by ticket 001.
- Added focused coverage proving a world-created agent's `PerAgentBeliefView` exposes `Some(&ObservationOmissionLog::default())` through the `GoalBeliefView` accessor.
- Truth-synced this ticket and `archive/specs/S135-planner-perception-budget.md` from the drafted macro wording to the live blanket implementation boundary.

## Deviations

- The live code does not have an `impl_goal_belief_view!` macro. `GoalBeliefView` is blanket-implemented from narrower belief-view traits, so the landed accessor is a trait default backed by the already-forwarded `agent_belief_store` method.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib observation_omission_log -- --list` (confirmed exact test id).
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::goal_belief_view_observation_omission_log_reads_agent_belief_store -- --exact`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-sim --lib belief_view`.
- Passed `cargo build --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`, whose live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `git diff --check` after final ticket/spec Markdown truth-sync.
