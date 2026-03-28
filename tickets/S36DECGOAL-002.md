# S36DECGOAL-002: Payload-aware goal declaration key

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai`: replace `GoalKindTag`-only S36 declaration substrate with a payload-aware AI-internal dispatch key; tighten S36 spec wording
**Deps**: [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md), [archive/tickets/completed/S36DECGOAL-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S36DECGOAL-001.md), [archive/specs/S33-opportunity-scoped-goal-identity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S33-opportunity-scoped-goal-identity.md)

## Problem

S36 still assumes `GoalKindTag` can be the canonical declaration key for AI dispatch. Live code already proves that assumption is too coarse. Some dispatch surfaces lawfully depend on payload distinctions inside one `GoalKindTag`, so a `GoalKindTag`-only registration layer would either collapse distinct behavior into one alias path or keep parallel ad hoc refinement matches alive beside the declaration table.

## Assumption Reassessment (2026-03-28)

1. Live code still exposes `GoalKindTag` as the coarse no-payload family key in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) via `GoalKindPlannerExt::goal_kind_tag()`. That mapping is exhaustive over `GoalKind`, but it intentionally erases payload distinctions.
2. The completed provenance cleanup in [archive/tickets/completed/S36DECGOAL-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S36DECGOAL-001.md) had to add a payload-aware escape hatch instead of relying on `GoalKindTag`: [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) now contains `GoalKindPlannerExt::ranked_goal_provenance_family()`, and its focused test `goal_model::tests::ranked_goal_provenance_family_is_payload_aware` proves `GoalKind::AcquireCommodity { purpose: SelfConsume }` and `GoalKind::AcquireCommodity { purpose: Restock }` do not share one lawful provenance family.
3. The payload-sensitive split is not isolated to provenance. [crates/worldwake-ai/src/exhaustion.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) `derive_invalidation_conditions()` adds `CommodityChanged(Coin)` for `GoalKind::AcquireCommodity { purpose: Restock }` but not for self-consume acquisition, and focused test `exhaustion::tests::acquire_restock_includes_coin_but_self_consume_does_not` already proves that distinction is live.
4. Another payload-sensitive static dispatch already exists in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) `GoalKindPlannerExt::relevant_op_kinds()`: `GoalKind::PunishAccused { punishment }` chooses `Fine` versus `Exile` planner-op sets from the punishment payload itself, not from `GoalKindTag::PunishAccused`.
5. `planner_ops.rs` still uses coarse `GoalKindTag` slices in `PlannerOpSemantics.relevant_goal_kinds`, but that is a planner-side compatibility surface, not proof that `GoalKindTag` is the right declaration key. It is one of the remaining consumers that should be migrated after the canonical key exists.
6. `decision_trace.rs` does not currently have a real per-goal declaration label table. It mostly renders `GoalKind` via `Debug` formatting (for example `summary()` uses `format!("{:?}", g.kind)`), so the S36 spec text claiming “trace labels per goal kind” is ahead of the live code and should be corrected before implementation.
7. `agent_tick/frame.rs::progress_op_kinds()` is keyed by `IntentionDomain`, not by `GoalKind` or `GoalKindTag`. Reassessment shows this is an exact abstraction-boundary mismatch in the current S36 spec: intention-progress ownership is domain-level today, so forcing it into goal registration without a separate domain-registration design would be a category error.
8. Coverage gap classification after search:
   - focused coverage exists for payload-aware provenance lookup in `goal_model.rs`
   - focused coverage exists for payload-aware invalidation behavior in `exhaustion.rs`
   - focused coverage exists for planner-op semantics in `planner_ops.rs`
   - no active ticket currently owns the underlying declaration-key correction; the remaining active epistemic tickets [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md) and [S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-011.md) are architecturally orthogonal, while [S34GENEPIACT-009.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-009.md) is already completed traceability work
9. This is a single-layer `worldwake-ai` structural ticket. No authoritative world logic changes are required, and no information path in the simulation is being changed. The contract under audit is the AI-internal declaration substrate.
10. Mismatch + correction: S36 should not continue to describe `GoalKindTag` as the one canonical declaration key. The robust end-state is a payload-aware AI-internal dispatch key derived from concrete `GoalKind`, while `GoalKind` itself remains the source of truth and `GoalKindTag` remains only where a deliberately coarse family contract is still the actual contract.

## Architecture Check

1. The clean long-term architecture is a derived `GoalDispatchKey` (name can vary) that is exhaustive over dispatch-distinguishing goal shapes, not over every payload value. This preserves P3 from [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): the concrete `GoalKind` remains authoritative, while the dispatch key is an explicit derived read-model for AI registration.
2. Putting this key in `worldwake-ai` rather than `worldwake-core` is cleaner. It is an AI dispatch concern, not shared world identity. Moving it into core would leak AI refactor structure into the canonical cross-crate goal type without a gameplay reason.
3. The declaration key must replace the `GoalKindTag`-only registration story, not sit beside it as a second competing declaration identity. `GoalKindTag` may survive only where a coarse family label is still intentionally the contract, not as a shadow registration path.

## Verification Layers

1. Every live payload-sensitive static dispatch distinction maps to a unique canonical declaration key -> focused `goal_model` unit tests
2. Existing payload-sensitive ranking/invalidation behavior remains explainable through the new key -> focused `goal_model` tests plus existing `ranking` and `exhaustion` regression tests
3. The S36 spec no longer claims goal-owned coverage for domain-owned `progress_op_kinds()` or non-existent trace-label tables -> focused spec/ticket reassessment plus implementation-facing tests on the new key
4. Single-layer `worldwake-ai` substrate ticket; no action trace, event-log, or authoritative world-state mapping applies

## What to Change

### 1. Correct the S36 declaration contract

Update [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md) so it no longer treats `GoalKindTag` as the universal declaration key and no longer claims goal-owned progress-op registration where the live architecture is `IntentionDomain`-owned.

The corrected spec should:

- define a payload-aware AI-internal declaration key derived from concrete `GoalKind`
- state that the key only splits where static dispatch actually differs
- keep `GoalKind` as authoritative identity
- state explicitly that `IntentionDomain` progress ownership is out of scope for S36 unless a future ticket introduces domain registration intentionally

### 2. Introduce the canonical payload-aware declaration key

Add an exhaustive derived key in `worldwake-ai` (for example `GoalDispatchKey`) with variants only for dispatch-distinguishing shapes. At minimum, reassessment already proves the key must be able to distinguish:

- `AcquireCommodity` self-consume
- `AcquireCommodity` restock
- `AcquireCommodity` recipe-input
- `PunishAccused` fine
- `PunishAccused` exile

If reassessment during implementation finds more live payload-sensitive static distinctions, include them in-scope rather than leaving a refined ad hoc match behind.

### 3. Expose one canonical lookup surface

Add a canonical derived-key lookup on concrete goals in `goal_model.rs`. This should become the declaration substrate later tickets consume. Do not introduce a separate parallel trait/method per dispatch concern.

### 4. Add structural regression coverage

Add focused tests proving:

- payload-sensitive shapes map to different declaration keys where live dispatch differs
- payload-insensitive shapes still collapse to the same declaration key where live dispatch is intentionally shared
- the lookup stays exhaustive over the current `GoalKind` set

## Files to Touch

- `specs/S36-declarative-goal-registration.md` (modify — correct declaration-key and progress-ownership assumptions)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add canonical payload-aware declaration key and exhaustive lookup)
- `crates/worldwake-ai/src/lib.rs` (modify — export the new declaration key if other AI modules need it)

## Out of Scope

- Migrating every dispatch site to the new key
- Rewriting `planner_ops.rs`, `exhaustion.rs`, or `feasibility.rs` to declarations in this ticket
- Changing authoritative goal identity in `worldwake-core`
- Moving `IntentionDomain` progress ownership into goal registration

## Acceptance Criteria

### Tests That Must Pass

1. The canonical AI declaration key distinguishes every currently live payload-sensitive static dispatch split
2. Adding a new `GoalKind` variant without updating the declaration-key lookup fails compilation
3. Existing focused regression proof still passes for the already-shipped provenance split
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. `GoalKind` remains the authoritative concrete goal identity
2. The new declaration key is a derived AI-internal dispatch surface, not a second world-level identity type
3. No payload-sensitive dispatch distinction remains expressible only through undocumented ad hoc refinement beside the canonical key
4. `IntentionDomain` progress ownership is not silently aliased into goal registration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — add focused tests proving the declaration key is exhaustive and payload-aware for live static splits
2. `specs/S36-declarative-goal-registration.md` — update spec text to match the live architecture and this new substrate

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::ranked_goal_provenance_family_is_payload_aware`
2. `cargo test -p worldwake-ai`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
