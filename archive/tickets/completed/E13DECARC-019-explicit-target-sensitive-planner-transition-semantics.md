# E13DECARC-019: Explicit target-sensitive planner transition semantics

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — AI-layer planner semantics refactor
**Deps**: `archive/tickets/completed/E13DECARC-010-planner-op-kinds-and-semantics-table.md`, `archive/tickets/completed/E13DECARC-011-planning-snapshot-and-planning-state.md`, `archive/tickets/completed/E13DECARC-012-plan-search-and-selection.md`, `archive/tickets/completed/GOLDENE2E-010-three-way-need-competition.md`

## Problem

GOLDENE2E-010 exposed that `GoalModelFallback` in `crates/worldwake-ai/src/planner_ops.rs` is too generic for target-sensitive actions. The immediate bug was fixed by adding a consume-specific guard in `apply_hypothetical_transition()`, but that leaves the planner semantics architecture with an ad hoc branch in the generic fallback path.

That is workable today, but it is not the cleanest long-term seam. As more target-sensitive action families appear, `apply_hypothetical_transition()` will tend to accumulate one-off validations instead of making transition rules explicit in planner semantics.

## Assumption Reassessment (2026-03-13)

1. `PlannerTransitionKind` currently has `GoalModelFallback`, `PickUpGroundLot`, and `PutDownGroundLot` only (confirmed in `crates/worldwake-ai/src/planner_ops.rs`).
2. `PlannerOpKind::Consume` currently covers both `eat` and `drink`, while goal identity still carries the commodity-specific meaning (confirmed).
3. The current consume-target fix is implemented as a pre-check in `apply_hypothetical_transition()`, not as a dedicated transition semantic (confirmed).
4. `GoalKind::apply_planner_step(...)` remains the single planner-side state mutation source for fallback semantics. This ticket should preserve that separation instead of moving hypothetical state logic into search or runtime code.
5. `crates/worldwake-ai/src/planner_ops.rs` already contains a direct negative regression, `consume_transition_rejects_mismatched_target_commodity`, that locks the current pre-check behavior in place (confirmed).
6. `crates/worldwake-ai/src/search.rs` already contains the caller-boundary regression from archived ticket `archive/tickets/completed/E13DECARC-018-search-regression-for-consume-target-matching.md`; this ticket should not claim that the search seam is uncovered or reopen that scope.
7. No active ticket in `tickets/*` currently covers refactoring `PlannerTransitionKind` to express target-sensitive consume semantics explicitly (confirmed).

## Architecture Check

1. Making target-sensitive planner transitions explicit is cleaner than growing the generic fallback branch. It keeps planner behavior declarative at the semantics-table layer, which is the same architectural seam already used for pickup and put-down.
2. This preserves the existing dependency direction: search consumes planner semantics, and goal-model state mutation remains planner-local. No runtime compatibility layer is needed.
3. A dedicated transition kind is more extensible than a growing pile of per-op `if` guards in `apply_hypothetical_transition()`. Future target-sensitive ops can add lawful transition semantics without weakening the generic path.
4. The current architecture is not fundamentally wrong, but the consume pre-check is the wrong long-term seam because it bypasses the `PlannerTransitionKind` dispatch table. Converting that guard into an explicit transition kind is the cleaner, more extensible shape.
5. No backward-compatibility aliasing is needed. The old generic consume-specific guard should be removed once the explicit transition kind replaces it.

## What to Change

### 1. Introduce an explicit planner transition kind for consume-target matching

Refine `PlannerTransitionKind` in `crates/worldwake-ai/src/planner_ops.rs` so consume actions use a dedicated transition semantic instead of generic `GoalModelFallback`.

Acceptable shapes include:

- `ConsumeMatchingTargetCommodity`
- or an equivalently precise name that communicates target-sensitive consume validation

The semantics must:

- require a concrete consume target
- validate that the target commodity matches the consume goal commodity
- then delegate the actual hypothetical need update to the existing goal-model consume semantics

### 2. Update semantics-table classification

Ensure `semantics_for(...)` assigns the new transition kind to `PlannerOpKind::Consume`.

Keep `PickUpGroundLot` and `PutDownGroundLot` behavior unchanged unless the refactor clearly simplifies them without broadening scope.

### 3. Remove the ad hoc consume guard from the generic fallback path

Once the explicit transition kind is in place:

- remove the consume-specific branch from the generic `GoalModelFallback` handling
- keep `apply_hypothetical_transition()` structured as a dispatch on `PlannerTransitionKind`

This ticket should leave the generic fallback path genuinely generic again.

### 4. Strengthen planner-semantics unit coverage

Add or update tests in `crates/worldwake-ai/src/planner_ops.rs` to prove:

- semantics-table classification points `Consume` defs at the new transition kind
- matching consume targets succeed
- mismatched consume targets fail
- non-consume fallback behavior still works

Prefer refining the existing planner-op tests rather than adding duplicate search coverage already owned by `E13DECARC-018`.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)

## Out of Scope

- Reworking `GoalKind::apply_planner_step(...)`
- Search algorithm changes
- Search-layer regression coverage already delivered by `archive/tickets/completed/E13DECARC-018-search-regression-for-consume-target-matching.md`
- Runtime interrupt selection or action-duration behavior
- Non-consume planner transition redesigns beyond what is needed to restore a clean dispatch boundary

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind::Consume` uses an explicit target-sensitive transition kind rather than `GoalModelFallback`
2. Matching consume targets produce a valid hypothetical transition
3. Mismatched consume targets return `None`
4. Generic non-consume fallback behavior remains intact
5. Existing suite: `cargo test -p worldwake-ai planner_ops`
6. Existing suite: `cargo test --workspace`
7. Existing suite: `cargo clippy --workspace`

### Invariants

1. `apply_hypothetical_transition()` remains a planner-semantics dispatcher, not a bag of special-case runtime checks
2. Commodity-specific consume validation stays planner-local and deterministic
3. No backward-compatibility shim preserves the old generic consume-fallback path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — refine transition classification coverage and target-sensitive consume transition tests without duplicating the archived `search.rs` regression

### Commands

1. `cargo test -p worldwake-ai planner_ops`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Corrected the ticket assumptions before implementation: `planner_ops.rs` already had a direct mismatched-consume regression, and `search.rs` already had the caller-boundary regression from `E13DECARC-018`.
  - Added `PlannerTransitionKind::ConsumeMatchingTargetCommodity` in `crates/worldwake-ai/src/planner_ops.rs`.
  - Moved consume target validation out of the generic pre-dispatch guard and into an explicit transition-kind dispatch path.
  - Kept `GoalKind::apply_planner_step(...)` as the single planner-local state mutation seam for fallback semantics, with consume-specific matching acting as a lawful gate before delegation.
  - Strengthened `planner_ops.rs` tests to assert consume transition classification and a positive matching-target consume path.
- **Deviations from original plan**:
  - No search-layer work was needed. The original ticket wording implied a broader coverage gap than the current code actually had.
  - The cleanup stayed inside `planner_ops.rs`; no goal-model, search, or runtime architecture changes were required.
  - The current planner architecture is broadly sound. The only architectural issue was that the consume-specific rule lived outside the semantics-table dispatch seam.
- **Verification results**:
  - `cargo test -p worldwake-ai planner_ops`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
