# HARPREE14-009: Cache OmniscientBeliefView per agent tick

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- refactor of agent_tick.rs internal flow
**Deps**: None (Wave 2, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-C02

## Problem

The agent-tick flow constructs `OmniscientBeliefView` repeatedly across one tick. The constructors are cheap, but the current pattern makes the authoritative-state boundaries hard to read: it is not obvious which reads share one world snapshot, and which reads intentionally happen after a world mutation. That weakens maintainability and makes future E14 belief work easier to get wrong.

## Assumption Reassessment (2026-03-11)

1. `agent_tick.rs` currently contains 9 `OmniscientBeliefView::new()` call sites.
2. Of those, 8 are direct call sites inside `process_agent()`. The 9th is inside `handle_current_step_failure()`, which is still on the same per-agent tick path but not in `process_agent()` itself.
3. `OmniscientBeliefView::new()` is effectively free in the current implementation: it stores `&World` and an optional runtime, with no allocation or snapshot copy.
4. The authoritative-world mutation points relevant to view freshness are:
   - `scheduler.interrupt_active_action(...)`, which can mutate world and event log through handler execution
   - `handle_current_step_failure(...)`, which persists blocked memory through `WorldTxn`
   - `persist_blocked_memory(...)`, when it commits a changed blocked-memory component
5. `enqueue(...)` mutates scheduler state only; it does not require a refreshed belief view.
6. Existing `agent_tick` unit coverage exercises liveness, AI/human gating, consume flow, and progress-barrier completion. It does not directly cover the blocked-memory persistence helper or document the view-refresh invariant.

## Architecture Check

1. A literal single binding for the entire `process_agent()` body is not the right target. The function mutates authoritative state mid-tick, and Rust borrow boundaries should keep those mutation phases explicit.
2. The better architecture is phase-scoped reuse:
   - one view for the initial liveness check
   - one shared read-phase view after `reconcile_in_flight_state(...)` and before the next world mutation
   - explicit refreshes only after authoritative world mutations before later reads/snapshot updates
3. This is better than the current architecture because it makes freshness boundaries visible without introducing wrappers, aliases, or compatibility shims.
4. Ideal longer-term architecture would push `process_agent()` toward named tick phases with explicit read/mutate transitions. This ticket stays intentionally smaller: clarify the current flow without restructuring the whole file.

## What to Change

### 1. Reuse a shared read-phase `OmniscientBeliefView` inside `process_agent()`

After `reconcile_in_flight_state(...)`, create one `OmniscientBeliefView` and reuse it for the read-only phase:

- blocker cleanup
- observation snapshot dirty check
- candidate generation
- candidate ranking
- planning snapshot creation
- step revalidation

Do not force a single binding across mutation points.

### 2. Identify mutation points

Treat these as view invalidation boundaries:

- `interrupt_active_action(...)`
- `handle_current_step_failure(...)`
- `persist_blocked_memory(...)` when it commits

### 3. Replace redundant constructions without hiding mutation boundaries

Use the shared read-phase view where the world has not changed. After a world mutation, refresh only if a later read needs the post-mutation world.

### 4. Add concise lifecycle comments

Add short comments around the major read/mutate phase boundaries. Avoid brittle comments that mention exact line numbers.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `tickets/HARPREE14-009-cache-belief-view-per-agent-tick.md` (update assumptions/scope before implementation)

## Out of Scope

- Changing `OmniscientBeliefView` API or internals
- Modifying `process_agent()` logic or behavior
- Changes to `decision_runtime.rs` or other files
- Large-scale decomposition of `process_agent()` into multiple new functions
- Performance optimization (this remains primarily a clarity and correctness-of-flow improvement)

## Acceptance Criteria

### Tests That Must Pass

1. The `agent_tick` suite passes, including the new blocked-memory persistence coverage
2. Golden e2e hashes identical
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. `process_agent()` behavior unchanged
2. The same logical read phases still observe the same authoritative state as before
3. Golden e2e state hashes identical
4. Any post-mutation world read uses a freshly constructed view

## Test Plan

### New/Modified Tests

1. Add a targeted `agent_tick` unit test for `persist_blocked_memory(...)` so the touched mutation path has direct coverage.
2. Existing `agent_tick` and golden e2e tests still carry the behavior-preservation burden for the refactor.

### Commands

1. `cargo test -p worldwake-ai agent_tick::tests`
2. `cargo test -p worldwake-ai --test golden_e2e`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-12
- Completed: 2026-03-11
- Actually changed:
  - Reworked `process_agent()` into explicit read phases that reuse one `OmniscientBeliefView` per read-only section instead of recreating views for each nearby read.
  - Kept explicit fresh-view reconstruction after authoritative world mutations before snapshot updates.
  - Added direct unit coverage for `persist_blocked_memory(...)` no-op and commit behavior.
  - On 2026-03-12, further decomposed `process_agent()` into named phase helpers for active-action lookup, read-phase refresh, active-action handling, planning/validation, and finalization.
- Deviations from original plan:
  - Did not force a single top-level view binding across the whole function; phase-scoped reuse was cleaner and more correct with the existing mutation boundaries and borrow semantics.
  - Left the helper-local `OmniscientBeliefView::new()` inside `handle_current_step_failure()` in place because it belongs to a separate mutation path and keeping it local preserves clarity.
  - The follow-up refactor did restructure the orchestration function itself after archival because the helper boundaries proved to be a net architectural improvement with low blast radius.
- Verification:
  - `cargo test -p worldwake-ai --lib agent_tick::tests`
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
