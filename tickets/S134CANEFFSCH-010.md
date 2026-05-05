# S134CANEFFSCH-010: Planner switch + old-path deletion + conformance rewrite

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — switches the planner's hypothetical evaluation to `apply_effects(..., Hypothetical)` / `apply_effects_with_context(..., Hypothetical)` as needed by payload-bearing schemas, deletes `apply_hypothetical_transition`, `PlannerTransitionKind`, `GoalKindPlannerExt::apply_planner_step` and per-`GoalKind` impls, and replaces the dual-implementation conformance harness with three coverage tests
**Deps**: archive/tickets/S134CANEFFSCH-003.md, archive/tickets/S134CANEFFSCH-004.md, archive/tickets/S134CANEFFSCH-005.md, archive/tickets/S134CANEFFSCH-006.md, archive/tickets/S134CANEFFSCH-007.md, archive/tickets/S134CANEFFSCH-008.md, tickets/S134CANEFFSCH-009.md

## Problem

S134 deliverables D3, D4, and D6 land here as the unification trigger. After tickets 003–009, every `ActionDef` carries a real `EffectSchema` and every action handler delegates its commit to `apply_effects(..., Authoritative)` or `apply_effects_with_context(..., Authoritative)` when payload/current-action context is required. This ticket flips the planner side: the single runtime call to `apply_hypothetical_transition` at `crates/worldwake-ai/src/search/transition.rs:154` is replaced with `apply_effects(..., Hypothetical)` / `apply_effects_with_context(..., Hypothetical)` as needed, the now-dead 9-arm `PlannerTransitionKind` dispatch and the 14+ per-`GoalKind` `apply_planner_step` impls are deleted, and the 21 dual-implementation conformance tests in `planner_conformance.rs` are replaced with three taxonomic coverage tests. Combining the switch with the deletion and test rewrite avoids a transient dead-code state between landings (FND-28 — no backward-compatibility paths in live authority).

## Assumption Reassessment (2026-05-04)

1. After tickets 003–009, every action's `EffectSchema` is non-empty and every commit handler delegates. The planner, however, still calls `apply_hypothetical_transition` from `crates/worldwake-ai/src/search/transition.rs:154`. The 9 `PlannerTransitionKind` arms (8 explicit + `GoalModelFallback`) and the 14+ per-`GoalKind` `apply_planner_step` impls (`crates/worldwake-ai/src/goal_model.rs:54` trait method, `:1051` impl block onward) all remain. They are unused-after-this-ticket, but they exist at the start.
2. Single runtime call site to delete: `apply_hypothetical_transition` is called from `crates/worldwake-ai/src/search/transition.rs:154` (production) and 8 test-only sites in `crates/worldwake-ai/src/planner_ops.rs:2268, 2305, 2343, 2374, 2418, 2480, 2551, 2593`. The lib.rs re-export at `crates/worldwake-ai/src/lib.rs:116` also goes.
3. `apply_planner_step` runtime usage: trait method at `goal_model.rs:54`, impl block starting `:1051`, ~14 test usages in the same file's `#[cfg(test)]` block (lines 6517, 6524, 6528, 6803–9402). The trait method is reached only via `PlannerTransitionKind::GoalModelFallback` arm at `planner_ops.rs:319` (`apply_goal_model_fallback_transition`); once `apply_hypothetical_transition` is deleted, `apply_planner_step` becomes unreachable from runtime code and is deleted alongside.
4. Conformance tests: 21 tests in `crates/worldwake-ai/tests/planner_conformance.rs` (lines 232, 321, 389, 450, 511, 616, 681, 744, 834, 932, 993, 1129, 1218, 1327, 1459, 1569, 1655, 1726, 1832, 1908, 1976) each compare the planner's `apply_hypothetical_transition` output against the imperative authoritative handler's output. After this ticket, both paths consume the same schema — there is no dual implementation to compare. The 21 tests are replaced by 3 taxonomic coverage tests per spec D6.
5. Hypothetical sink: ticket 002's `EffectSink` impl over `PlanningState` in `crates/worldwake-ai/src/effect_sink_hypothetical.rs` is the consumer of the planner-side schema evaluation call. Confirm during reassessment that the sink covers every `EffectStep` variant introduced by tickets 003–009.
6. Shared abstraction boundary under audit: the planner's hypothetical evaluation seam at `search/transition.rs:154`. Before this ticket: planner uses `apply_hypothetical_transition` over `PlannerTransitionKind` (delegating to per-`GoalKind` `apply_planner_step` for fallback cases). After: planner evaluates `ActionDef.effect_schema` through the shared schema evaluator. The planner's call signature changes to construct the hypothetical sink and pass `EffectMode::Hypothetical`.
7. Existing focused/unit coverage to extend or rewrite:
   - 21 conformance tests in `planner_conformance.rs` — replaced.
   - 14+ `apply_planner_step` tests in `goal_model.rs` `#[cfg(test)]` block — deleted (the trait method is gone).
   - All goldens in `crates/worldwake-ai/tests/golden_*.rs` (~36 files) must continue to pass with bitwise-identical event logs — this is the unification's load-bearing regression check.
8. Rename/removal blast radius (per Step 2 sub-check g): 6 files within `worldwake-ai/` — `goal_model.rs`, `lib.rs`, `planner_ops.rs`, `search/tests.rs`, `search/transition.rs`, `tests/planner_conformance.rs`. No external crate references the deleted symbols.

## Architecture Check

1. Switching the planner's hypothetical evaluation to consume the same `EffectSchema` the authoritative path consumes is the spec's load-bearing unification (Design Goal 1 — "Single forward model per action"). Drift between authoritative and hypothetical paths becomes architecturally impossible because there is one declarative truth.
2. Deleting both `apply_hypothetical_transition` and `apply_planner_step` together (rather than in sequential tickets) eliminates a transient dead-code state where the old paths exist but are unreachable. FND-28 is satisfied — no backward-compatibility shims survive into the live authority path.
3. Replacing the 21-case dual-implementation conformance harness with three taxonomic coverage tests reflects the spec's Design Goal 6: conformance via coverage, not duplication. The existing tests asserted "two implementations agree"; after unification there's only one implementation, so coverage assertions (every action has a schema, every Discrepancy variant is reachable, partial outcomes emit typed facts) are the right contract.

## Verification Layers

1. Planner-side switch invariant → action trace + event-log delta: every golden in `crates/worldwake-ai/tests/golden_*.rs` produces bitwise-identical event logs to the pre-ticket baseline (the planner's hypothetical evaluation now goes through the shared schema evaluator but should produce identical hypothetical projections, which means identical plan selection, which means identical authoritative outcomes).
2. Old-path deletion invariant → grep verification: post-ticket, `rg -n "apply_hypothetical_transition|PlannerTransitionKind|apply_planner_step" crates/` returns zero matches. Compile-time enforcement complements this — any missed deletion site fails to compile.
3. Coverage-test invariants:
   - `every_actiondef_has_effect_schema()` → focused unit test asserting registry coverage.
   - `every_discrepancy_variant_reachable_from_some_schema_precondition()` → focused unit test asserting taxonomic completeness over `Discrepancy`'s 11 variants.
   - `partial_outcome_steps_emit_typed_facts()` → focused unit test asserting no ad-hoc `partial: bool` flag survives in handler-internal state.
4. Canonical state hash invariant → soak: 1440-tick replay of `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` produce identical `blake3` hashes pre- and post-ticket. This is the unification's load-bearing acceptance criterion.

## What to Change

### 1. Switch planner hypothetical evaluation to schema evaluation

In `crates/worldwake-ai/src/search/transition.rs:154`, replace the call:

```rust
// BEFORE
let transition = apply_hypothetical_transition(&goal, semantics, state, &targets, payload);
```

with:

```rust
// AFTER
let action_def = registry.get(action_def_id).expect("action def in registry");
let mut sink = HypotheticalSink::new(state); // from worldwake-ai::effect_sink_hypothetical
let outcome = apply_effects_with_context(
    &action_def.effect_schema,
    EffectEvaluationContext {
        actor,
        targets: &targets,
        payload,
        action_def_id,
    },
    &mut sink,
    EffectMode::Hypothetical,
);
```

The exact local-variable names and the way the `HypotheticalSink` reads back the post-evaluation `PlanningState` overlay depend on ticket 002's sink shape — confirm during reassessment. Update `transition.rs`'s surrounding code to interpret the new `Result<EffectOutcome, Discrepancy>` shape correctly.

Remove the `apply_hypothetical_transition` import from `transition.rs:9`.

### 2. Delete `apply_hypothetical_transition` and `PlannerTransitionKind`

In `crates/worldwake-ai/src/planner_ops.rs`:
- Delete `pub enum PlannerTransitionKind` at lines 69–79.
- Delete `pub fn apply_hypothetical_transition` at line 311 and the helper functions it dispatches into (`apply_goal_model_fallback_transition`, the per-arm helpers).
- Delete the `transition_kind: PlannerTransitionKind` field on `PlannerOpSemantics` at line 65 (and update `PlannerOpSemantics`'s usages — `semantics_for`, `base_semantics`, `social_or_combat_semantics`).
- Delete the test module's `apply_hypothetical_transition` usages at lines 2268, 2305, 2343, 2374, 2418, 2480, 2551, 2593. The corresponding test functions are deleted entirely (their reason for existing was to exercise the dispatch).

In `crates/worldwake-ai/src/lib.rs`:
- Remove `apply_hypothetical_transition` from the re-export at line 116.
- Remove `PlannerTransitionKind` if it was re-exported (confirm during reassessment).

### 3. Delete `GoalKindPlannerExt::apply_planner_step` and per-`GoalKind` impls

In `crates/worldwake-ai/src/goal_model.rs`:
- Delete the `apply_planner_step` method from the `GoalKindPlannerExt` trait at line 54.
- Delete the impl-block method body at line 1051 onward (per-`GoalKind` match arms).
- Delete the ~14 test functions in the `#[cfg(test)]` block that exercise `apply_planner_step` (lines 6517, 6524, 6528, 6803, 6811, 6830, 6856, 6897, 6918, 6972, 7010, 7026, 7267, 7353, 9295, 9331, 9362, 9402 — approximate line list; enumerate during reassessment).

### 4. Replace 21 dual-implementation conformance tests

Delete the 21 conformance tests in `crates/worldwake-ai/tests/planner_conformance.rs`. Replace the file contents with the three coverage tests per spec D6:

- `every_actiondef_has_effect_schema()` — iterate the `ActionDefRegistry` and assert every `ActionDef.effect_schema` has at least one step (or one precondition for read-only actions). Empty schemas surviving past tickets 003–009 are a regression.
- `every_discrepancy_variant_reachable_from_some_schema_precondition()` — for each `Discrepancy` variant in `crates/worldwake-core/src/discrepancy.rs:8` (11 variants — `BeliefStale`, `BeliefContradicted`, `SourceInvalidated`, `ImproperPlanningState`, `MissingObservation`, `NoLegalBinding`, `NoWillingCounterparty`, `RouteUnknown`, `SearchBudgetExhausted`, `PartialExecutionDrift`, `NeedHorizonExceeded`), assert at least one schema's precondition list can produce that variant. Test fixtures for each. The test enforces taxonomic completeness — a new `Discrepancy` variant added without a producing precondition is a coverage gap.
- `partial_outcome_steps_emit_typed_facts()` — assert that every action whose handler used to surface `partial_quantity` (S127) now produces a typed `EffectFact::PartialQuantity` through its schema path rather than a handler-internal `Option<Quantity>` flag. S134CANEFFSCH-005 emits this fact through the `HarvestResource` category step, not through generic `PartialOnFailure`.

### 5. Clean up dead references

After the deletions:
- Remove `transition_kind`-related dead code in `planner_ops.rs`.
- Update any sibling test or trace surface that referenced the deleted symbols (likely `crates/worldwake-ai/src/search/tests.rs` per Step 2's blast-radius check).
- Confirm `cargo clippy --workspace --all-targets -- -D warnings` passes — no dead-code warnings remain.

## Files to Touch

- `crates/worldwake-ai/src/search/transition.rs` (modify — line 154 call site, line 9 import, surrounding result-handling)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — delete enum at 69–79, function at 311, helpers, ~8 test sites; touch `PlannerOpSemantics` struct)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export at line 116)
- `crates/worldwake-ai/src/goal_model.rs` (modify — delete trait method at 54, impl at 1051, ~14 test functions)
- `crates/worldwake-ai/src/search/tests.rs` (modify — clean up any referenced deleted symbols; confirm during reassessment)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify — delete 21 dual-impl tests, add 3 coverage tests)

## Out of Scope

- Adding new behaviors beyond the unification — schemas land in tickets 003–009; this ticket only switches the planner's evaluation surface and deletes dead code.
- Changing `EffectSchema` shape or adding new step variants — those land in the per-category tickets that surface them.
- Modifying any non-planner runtime code — action handler bodies were already migrated in tickets 003–009.
- Adjusting `BindingStrictness`, `guard_template`, or `expectation_template` (preserved per spec Non-Goals throughout S134).
- Changes to `Discrepancy` (`worldwake-core/src/discrepancy.rs`) — taxonomy is reused as-is.
- Save-format changes — `SAVE_FORMAT_VERSION` does not change (registry-only data structure per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. All ~36 goldens in `crates/worldwake-ai/tests/golden_*.rs` produce bitwise-identical event logs to pre-ticket baseline (the unification's load-bearing regression check).
2. New coverage tests in `crates/worldwake-ai/tests/planner_conformance.rs`:
   - `every_actiondef_has_effect_schema()` passes (registry coverage).
   - `every_discrepancy_variant_reachable_from_some_schema_precondition()` passes (taxonomic completeness over 11 `Discrepancy` variants).
   - `partial_outcome_steps_emit_typed_facts()` passes (no ad-hoc `partial: bool` flags).
3. `cargo test --workspace` — all existing tests pass after the dual-impl conformance tests are deleted and the trait-method tests in `goal_model.rs` are deleted.
4. `cargo clippy --workspace --all-targets -- -D warnings` — no dead-code or unused-import warnings from the deletions.
5. `./scripts/verify.sh` — full lint + typecheck + test gate.

### Invariants

1. `rg -n "apply_hypothetical_transition\|PlannerTransitionKind\|apply_planner_step" crates/` returns zero matches post-ticket. Compile-time enforcement complements this.
2. Bitwise-identical canonical state hash on `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` over 1440 ticks pre- and post-ticket — the load-bearing unification regression check.
3. Every `ActionDef` in the registry has a non-empty `EffectSchema` (asserted by `every_actiondef_has_effect_schema()`).
4. Every `Discrepancy` variant is reachable from at least one schema's precondition (asserted by `every_discrepancy_variant_reachable_from_some_schema_precondition()`).
5. The planner's hypothetical evaluation seam is `apply_effects(..., Hypothetical)` / `apply_effects_with_context(..., Hypothetical)` only — `crates/worldwake-ai/src/search/transition.rs` has no reference to the deleted symbols.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/planner_conformance.rs` — delete 21 dual-impl tests, add 3 coverage tests:
   - `every_actiondef_has_effect_schema` — iterate registry, assert all schemas non-empty.
   - `every_discrepancy_variant_reachable_from_some_schema_precondition` — for each of the 11 `Discrepancy` variants, prove a producing precondition exists.
   - `partial_outcome_steps_emit_typed_facts` — for each action whose pre-S134 handler surfaced `partial_quantity`, assert the schema path produces `EffectFact::PartialQuantity`.
2. `crates/worldwake-ai/src/goal_model.rs` `#[cfg(test)]` block — delete ~14 `apply_planner_step` tests (the function is gone).
3. `crates/worldwake-ai/src/planner_ops.rs` `#[cfg(test)]` block — delete ~8 `apply_hypothetical_transition` tests (the function is gone).
4. Existing goldens — no source change; their pass-without-modification is the load-bearing regression check.

### Commands

1. `cargo test -p worldwake-ai --test planner_conformance` (verify the 3 new coverage tests)
2. `cargo test -p worldwake-ai goal_model` (verify the trait-method test deletions don't break sibling tests)
3. `cargo test -p worldwake-ai golden_survival` (soak smoke)
4. `cargo test -p worldwake-ai` (full ai-crate test suite, including all 36 goldens)
5. `cargo test --workspace` (full workspace regression)
6. `cargo clippy --workspace --all-targets -- -D warnings` (dead-code check)
7. `./scripts/verify.sh` (PR gate)
