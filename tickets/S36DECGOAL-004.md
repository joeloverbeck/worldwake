# S36DECGOAL-004: Declaration-backed dynamic invalidation and feasibility strategies

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai`: route dynamic invalidation and feasibility logic through declaration-owned strategy selection keyed by the payload-aware dispatch key
**Deps**: [tickets/S36DECGOAL-002.md](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAL-002.md), [tickets/S36DECGOAL-003.md](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAL-003.md), [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md)

## Problem

After the declaration substrate and static-dispatch migration land, two high-value dynamic AI dispatch surfaces still remain as large `match GoalKind` tables: exhaustion invalidation and cheap feasibility. They should not be flattened into static metadata, but they also should not remain as free-floating, undocumented goal matches once a canonical declaration key exists.

## Assumption Reassessment (2026-03-28)

1. [crates/worldwake-ai/src/exhaustion.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) `derive_invalidation_conditions()` is a large per-`GoalKind` dispatch table. Some branches are payload-sensitive and already proven live by tests, for example `AcquireCommodity { purpose: Restock }` adds a coin invalidation condition while self-consume acquisition does not.
2. [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs) `goal_specific_feasibility()` is another large per-`GoalKind` dispatch table. It mixes true family routing with dynamic local checks against `GoalBeliefView`, blocker memory, evidence places, and target co-location.
3. These two surfaces are not purely static data. `derive_invalidation_conditions()` depends on live threshold bands, recipe inputs, target ids, and baseline snapshots. `feasibility_hint()` depends on live local belief state and blocker memory. Reassessment shows a static declaration struct alone cannot honestly encode their full behavior.
4. The clean architectural target is therefore strategy selection, not static data stuffing. The declaration should own which invalidation strategy and feasibility strategy apply to a dispatch-distinguishing goal shape, while the actual computation remains in strategy helpers that take concrete `GoalKind`, view, agent, and recipe inputs.
5. `agent_tick/frame.rs::progress_op_kinds()` remains domain-owned and should stay out of this ticket. Progress semantics are attached to `IntentionDomain`, not to the goal declaration key under the current live architecture.
6. Coverage gap classification after search:
   - focused invalidation coverage exists, including `exhaustion::tests::derive_invalidation_conditions_covers_every_live_goalkind_variant`
   - focused invalidation payload split coverage exists in `exhaustion::tests::acquire_restock_includes_coin_but_self_consume_does_not`
   - focused feasibility coverage exists across many concrete scenarios in `feasibility.rs`
   - no active ticket currently owns migrating these dynamic dispatch surfaces onto the S36 declaration substrate
7. This is a single-layer `worldwake-ai` ticket. No authoritative layer behavior changes are in scope. The boundary under audit is the AI runtime read-model contract for retry invalidation and cheap feasibility ordering.
8. Mismatch + correction: the current S36 spec implies some of this can be handled as static declaration fields. Reassessment shows the correct design is declaration-owned strategy selection plus family-specific helper functions, not static condition vectors or static feasibility booleans.

## Architecture Check

1. Strategy selection is the robust middle ground. It removes scattered goal matches while preserving P3 and P18 from [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): concrete state still drives the actual invalidation and feasibility results, and the declaration only selects which lawful computation to run.
2. This is cleaner than leaving giant raw `match GoalKind` tables around after the declaration system exists, and cleaner than pretending live recipe/threshold/target-dependent behavior can be frozen into static tables.
3. The ticket must not introduce heuristic aliasing. `feasibility_hint()` may still be heuristic, but its routing should become declarative and explicit, not a hidden second dispatch system competing with S36.

## Verification Layers

1. Invalidation strategy routing resolves through the declaration substrate while preserving existing concrete invalidation behavior -> focused `exhaustion` tests
2. Feasibility strategy routing resolves through the declaration substrate while preserving existing local-likelihood behavior -> focused `feasibility` tests
3. Retry / blocker runtime behavior remains stable after migration -> focused `agent_tick::planning` exhaustion tests
4. Single-layer `worldwake-ai` runtime-read-model ticket; no action trace, event-log, or authoritative world-state mapping applies

## What to Change

### 1. Add declaration-owned strategy selectors

Extend the S36 declaration surface so it can choose:

- an invalidation strategy
- a feasibility strategy

These should be explicit enums or similarly typed strategy selectors, not opaque function pointers hidden from tests and docs unless the codebase proves function pointers are the cleaner option.

### 2. Migrate exhaustion invalidation dispatch

Refactor `derive_invalidation_conditions()` so declaration-owned strategy selection replaces the top-level raw `match GoalKind`. Family-specific helpers may still branch on concrete payload where the strategy genuinely needs payload facts.

Do not weaken the existing baseline and threshold behavior. The cleanup target is routing ownership, not simpler invalidation logic.

### 3. Migrate feasibility dispatch

Refactor `goal_specific_feasibility()` so declaration-owned strategy selection replaces the top-level raw `match GoalKind`. Keep the dynamic local checks belief-view-driven and cheap.

### 4. Preserve domain-owned progress semantics

Document and preserve that `IntentionDomain` progress-op routing remains domain-owned in `agent_tick/frame.rs`. Do not silently fold it into this migration.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify — declaration-owned invalidation strategy routing)
- `crates/worldwake-ai/src/feasibility.rs` (modify — declaration-owned feasibility strategy routing)
- `crates/worldwake-ai/src/goal_model.rs` or declaration module (modify — add strategy selectors to the declaration surface)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify only if strategy-owned invalidation plumbing requires call-site updates)
- `specs/S36-declarative-goal-registration.md` (modify if implementation tightens the dynamic-strategy part of the contract)

## Out of Scope

- Static declaration-key introduction
- Planner-op reverse membership migration
- Decision-trace label migration
- `IntentionDomain` progress-op migration
- Any authoritative-world behavior change

## Acceptance Criteria

### Tests That Must Pass

1. `derive_invalidation_conditions()` no longer depends on one monolithic raw `GoalKind` dispatch table
2. `goal_specific_feasibility()` no longer depends on one monolithic raw `GoalKind` dispatch table
3. Existing payload-sensitive invalidation behavior for restock acquisition remains unchanged
4. Existing feasibility regressions remain unchanged
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. Dynamic invalidation and feasibility behavior remain driven by concrete local state, not by abstract static scores
2. Declaration ownership decides strategy routing, but not the live data inputs those strategies consume
3. No second hidden dispatch system survives beside the declaration-owned strategy selectors
4. `IntentionDomain` progress semantics remain explicitly domain-owned

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` — strengthen focused tests around strategy routing plus payload-sensitive invalidation behavior
2. `crates/worldwake-ai/src/feasibility.rs` — strengthen focused tests around strategy routing while preserving current local-likelihood outcomes
3. `crates/worldwake-ai/src/agent_tick/planning.rs` — keep regression coverage proving exhausted-goal invalidation still behaves correctly in runtime planning

### Commands

1. `cargo test -p worldwake-ai exhaustion::tests::derive_invalidation_conditions_covers_every_live_goalkind_variant`
2. `cargo test -p worldwake-ai feasibility::tests::test_claim_office_evidence_local_likely`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
