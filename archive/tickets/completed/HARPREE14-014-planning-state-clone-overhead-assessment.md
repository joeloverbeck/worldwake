# HARPREE14-014: Reassess PlanningState clone optimization scope

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None
**Deps**: None (Wave 5, optional)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-C03

## Problem

`PlanningState` is cloned during hypothetical successor construction, and its overlay collections are value-owned. That can become relevant if planner branching grows substantially, but the current codebase does not show this as an established bottleneck. This ticket is therefore a reassessment and scope-correction task first, not an automatic data-structure migration.

## Assumption Reassessment (2026-03-12)

1. `PlanningState` does not contain only "9 BTreeMap override maps". It currently owns 8 `BTreeMap` override maps, 1 `BTreeSet` for removals, plus a `BTreeMap` hypothetical registry and a monotonic hypothetical-id counter.
2. Cloning is not tied to frontier popping. `search.rs` already uses a `BinaryHeap` frontier. The relevant clone happens in `build_successor()` when `apply_hypothetical_transition(...)` receives `node.state.clone()` for each viable candidate branch.
3. HARDEN-C01 and HARDEN-D02 have already landed. The search module now has explicit heap-ordering and beam-pruning coverage, so this ticket must not assume the old Vec-sort frontier or missing beam tests.
4. `planning_state.rs` already has targeted tests covering clone-relevant invariants such as snapshot sharing (`overlay_clones_share_snapshot_owned_heavy_vectors`) and branch-local hypothetical ID evolution (`spawn_hypothetical_lot_allocates_monotonic_ids_and_clones_preserve_branch_counters`).
5. There is still no evidence in the current repo that `PlanningState` clone cost is a production bottleneck. This remains speculative future-proofing, not a demonstrated regression.

## Architecture Check

1. Replacing the current stdlib collections with `im::OrdMap` is not clearly more beneficial than the current architecture today. It would add an external dependency and widen the cognitive surface area of a central planner state type without a demonstrated need.
2. The current `PlanningState` design is already structurally conservative where it matters most: the large authoritative snapshot is borrowed, and existing tests prove clones share snapshot-owned heavy vectors instead of duplicating them.
3. The remaining cloned data is exactly the mutable hypothetical overlay. That is a reasonable tradeoff for clarity, determinism, and simple ownership until profiling proves otherwise.
4. If clone overhead ever becomes a measured problem, the cleaner long-term direction is a profiling-guided branch-overlay design for planner deltas, not a speculative blanket swap of every overlay collection to persistent maps.
5. No backward-compatibility layers, aliases, or dual-path data structures are warranted for this ticket.

## What to Change

### 1. Correct the ticket scope

Update the ticket so it matches the current code and tests:

- `search.rs` already uses a heap frontier
- beam-width coverage already exists
- the remaining question is whether clone overhead is proven enough to justify architectural churn

### 2. Do not implement speculative collection migration

Do not add `im` or a custom COW layer unless profiling first shows that `PlanningState` overlay cloning is a real bottleneck under representative workloads.

### 3. Preserve the current architecture for now

Close this ticket as complete with no engine-code changes. The beneficial work here is the reassessment: retaining the simpler current architecture until evidence justifies a more invasive planner-state redesign.

## Files to Touch

- This ticket document only

## Out of Scope

- Adding `im` or any other external dependency
- Converting `PlanningState` overlay fields to persistent maps
- Adding benchmark-only infrastructure without an identified performance problem
- Changing `PlanningState` API or planner semantics
- Reworking search or planner-operator architecture

## Acceptance Criteria

### Tests That Must Pass

1. Existing planner/search tests pass unchanged
2. Golden e2e hashes remain unchanged
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` passes with no new warnings

### Invariants

1. `PlanningState` behavior remains unchanged
2. Determinism remains unchanged
3. No new dependencies are introduced
4. No speculative compatibility path or dual architecture is added

## Test Plan

### New/Modified Tests

1. No new tests required for this ticket after reassessment; existing search and planning-state coverage already exercises the relevant invariants
2. This ticket should not add benchmark-style tests without a concrete performance target and acceptance threshold grounded in real workload data

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo test -p worldwake-ai --test golden_e2e`
4. `cargo test --workspace`
5. `cargo clippy --workspace -- -D warnings`

## Outcome

Completion date: 2026-03-12

What actually changed:
- Reassessed the ticket against the current codebase and corrected its assumptions.
- Closed the ticket without production-code changes because the proposed `PlanningState` collection migration is not justified by current architecture or evidence.

Deviations from original plan:
- Did not add `im`, custom COW wrappers, or benchmark harnesses.
- Narrowed the scope from "optimize PlanningState cloning" to "verify whether such optimization is architecturally justified now."
- Recognized that related search hardening work assumed by this ticket is already complete elsewhere.

Verification results:
- `cargo test -p worldwake-ai search`
- `cargo test -p worldwake-ai planning_state`
- `cargo test -p worldwake-ai --test golden_e2e`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
