# S108PERACTBIN-003: Strictness gate on plan-revalidation best-effort fallbacks

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `revalidate_next_step`'s two best-effort-like fallback paths in `plan_revalidation.rs` consult `check_binding_strictness` and return `false` for `ExactIdentity` actions when the primary affordance enumeration did not match.
**Deps**: archive/tickets/S108PERACTBIN-001.md (needs `BindingStrictness`, `check_binding_strictness`).

## Problem

`crates/worldwake-ai/src/plan_revalidation.rs::revalidate_next_step` (lines 14–49) has three revalidation paths:

1. Primary: enumerate affordances via `get_affordances_for_defs`, check each with `requested_affordance_matches`.
2. Fallback 1 (`revalidate_best_effort_payload_override_step`, lines 51–82): accept a planner-synthesized payload if `actor_constraints`, `preconditions`, and `payload_override_is_valid` all pass for the raw requested targets.
3. Fallback 2 (`revalidate_exact_target_step`, lines 84–118): if the `ActionDef.targets` are ALL `TargetSpec::SpecificEntity(_)`, synthesize an `Affordance` from the step's targets and rerun `requested_affordance_matches`.

Both fallbacks permit the revalidation to succeed against targets that the planner synthesized without going through affordance enumeration. That's the same substitution permissiveness T-002 now narrows at dispatch. For `ExactIdentity` actions, these fallbacks must refuse, or `revalidate_next_step` would green-light a step that then survives to dispatch/start-time validation against a stale concrete target — producing avoidable per-tick churn and weakening the failure surface.

Fallback 2 has partial pre-existing coverage: its all-`TargetSpec::SpecificEntity` precondition is a narrower subset of `ExactIdentity`. For actions like `loot` (which uses `TargetSpec::EntityAtActorPlace { kind: Agent }`) the fallback does NOT currently fire, so `ExactIdentity` gating via `check_binding_strictness` is the first authoritative gate there too.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `revalidate_next_step` is at `crates/worldwake-ai/src/plan_revalidation.rs:14`; `revalidate_best_effort_payload_override_step` at line 51; `revalidate_exact_target_step` at line 84. All three return `bool`. Existing focused tests in the same file's `#[cfg(test)]` block include `build_transport_registry` (line 707), `build_specific_entity_payload_registry` (line 758), `build_explicit_payload_registry` (line 813), and their exercising tests — these will be re-verified to confirm the strictness gate preserves the intended existing paths (FungibleEquivalentCommodity, non-ExactIdentity flows).
2. The reassessed spec (`specs/S108-per-action-binding-strictness.md`, D5) explicitly names this ticket's scope and calls out the partial overlap with `revalidate_exact_target_step`: "After D4/D5 land, `ExactIdentityRequired` is the authoritative gate; the all-`SpecificEntity` check becomes redundant for the strictness decision but may remain as a fast-path optimization." This ticket implements the gate and does NOT remove `revalidate_exact_target_step` (deferred per spec's "Open Migration Work").
3. Shared abstraction boundary: `plan_revalidation.rs`'s three-path revalidation surface. This ticket tightens Fallbacks 1 and 2 under the authoritative `BindingStrictness` classifier from T-001 without widening the primary path.
4. Not applicable — no failing golden motivates this ticket.
5. Not applicable — not a planner- or golden-driven ticket. Revalidation is belief-first by construction (FND-14); it reads the agent's belief view.
6. AI regression intended layer: runtime `agent_tick` revalidation boundary; local unit tests against `revalidate_next_step` are sufficient here. Goldens live in T-005.
7. Not applicable — no ordering claim.
8. Not applicable — no heuristic removal.
9. First failure boundary for this path: plan revalidation (runs before request resolution on each tick). If revalidation returns `false`, the AI drops the step and replans — the request never reaches `resolve_affordance`. If revalidation returns `true` incorrectly (without the gate), the stale step survives to sim dispatch and authoritative start-time validation a tick later. Covering both surfaces with the same predicate avoids that asymmetry.
10. Not applicable.
11. Not applicable.
12. Not applicable.
13. Adjacent observation: `revalidate_exact_target_step` already acts like `ExactIdentity` gating but only for a narrower set of actions (all-`SpecificEntity` TargetSpecs). The spec classifies its removal as follow-up Open Migration Work; this ticket explicitly does not remove it. The gate is additive: Fallback 2 first applies the strictness check, then the existing all-`SpecificEntity` logic.
14. No mismatch discovered during reassessment.
15. Not applicable.

## Architecture Check

1. Single source of substitution policy. Both sim dispatch (T-002) and AI revalidation (this ticket) consult the same `check_binding_strictness` predicate over the same `ActionDef::binding_strictness` authoritative metadata (FND-26). Shared classifier behavior on both surfaces avoids the one-tick-late catch-up where revalidation greenlights a stale step and the sim rejects it later at request resolution or authoritative start-time validation.
2. No backward-compatibility shim (FND-28). The existing `revalidate_exact_target_step`'s all-`SpecificEntity` logic is retained as a fast path under the reassessed spec's Open Migration Work deferment; it is NOT promoted to authority. The authoritative gate is the strictness classifier.

## Verification Layers

1. Revalidation refusal for `ExactIdentity` + no primary match -> focused unit test in `plan_revalidation.rs` asserting `revalidate_next_step` returns `false`.
2. Revalidation continues to succeed for non-`ExactIdentity` classes (preserves existing fallback behavior for `FungibleEquivalentCommodity`, `EquivalentWorkstationTagAtSamePlace`, etc.) -> focused unit tests reusing the existing `build_transport_registry`-style harnesses with explicit classification.
3. Symmetry with T-002 dispatch gate -> verified end-to-end in T-005 goldens; not in this ticket's contract.
4. Single-layer ticket: belief-view + pure predicate. No action trace, event-log, or authoritative-world surface is touched.

## What to Change

### 1. Gate `revalidate_best_effort_payload_override_step`

`crates/worldwake-ai/src/plan_revalidation.rs` at line 51. Add an early return at the top of the function body:

```rust
fn revalidate_best_effort_payload_override_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    targets: &[EntityId],
    def: &worldwake_sim::ActionDef,
    handler: &worldwake_sim::ActionHandler,
) -> bool {
    if matches!(
        worldwake_sim::check_binding_strictness(def, worldwake_sim::ActionRequestMode::BestEffort),
        worldwake_sim::StrictnessGate::ExactIdentityRequired,
    ) {
        return false;
    }
    // ...existing body unchanged...
}
```

### 2. Gate `revalidate_exact_target_step`

`crates/worldwake-ai/src/plan_revalidation.rs` at line 84. Add the same early return at the top of the function body:

```rust
fn revalidate_exact_target_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    targets: &[EntityId],
    def: &worldwake_sim::ActionDef,
    handler: &worldwake_sim::ActionHandler,
) -> bool {
    if matches!(
        worldwake_sim::check_binding_strictness(def, worldwake_sim::ActionRequestMode::BestEffort),
        worldwake_sim::StrictnessGate::ExactIdentityRequired,
    ) {
        return false;
    }
    // ...existing all-SpecificEntity logic unchanged...
}
```

Semantic note: for actions already classified `ExactIdentity` whose `ActionDef.targets` are all `TargetSpec::SpecificEntity(_)`, the early return short-circuits the existing all-`SpecificEntity` path with the same answer (`false` when no match). For actions classified `ExactIdentity` but with broader `TargetSpec`s (e.g., `loot`), the early return newly enforces strict identity. For actions classified non-`ExactIdentity`, the early return does not fire, preserving existing behavior.

### 3. Unit tests

Add `#[cfg(test)]` tests in `plan_revalidation.rs` covering:
- `revalidate_next_step` returns `false` for an `ExactIdentity` action whose planned targets don't match any enumerated affordance (reuse an accuse-like fixture with `binding_strictness = ExactIdentity`).
- `revalidate_next_step` returns `true` for a `FungibleEquivalentCommodity` action via `revalidate_best_effort_payload_override_step` when the payload validator accepts (reuse the transport fixture with `binding_strictness = FungibleEquivalentCommodity`).
- `revalidate_next_step` returns `false` for an `ExactIdentity` action whose `ActionDef.targets` are all `SpecificEntity` but the gate fires before reaching the all-`SpecificEntity` check (confirms the gate precedes the legacy path).

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — two early returns, new unit tests, test-fixture `binding_strictness` fields if not already set by T-001)

## Out of Scope

- Removing `revalidate_exact_target_step` — spec's Open Migration Work.
- Sim-side dispatch gate — T-002.
- Decision trace field — T-004.
- Golden/integration tests — T-005.

## Acceptance Criteria

### Tests That Must Pass

1. New unit tests in `plan_revalidation.rs` covering the three cases above.
2. Existing `plan_revalidation` tests continue to pass (test fixtures' `binding_strictness` values preserved from T-001).
3. Existing suite: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. For any `ExactIdentity` action, both `revalidate_best_effort_payload_override_step` and `revalidate_exact_target_step` return `false` under `ActionRequestMode::BestEffort` semantics — no fallback substitution is accepted.
2. For non-`ExactIdentity` actions, fallback behavior is preserved byte-for-byte (only the early-return branch changes).
3. The gate at both fallbacks uses the same `check_binding_strictness(def, BestEffort)` predicate as sim dispatch (T-002), producing symmetric behavior across revalidation and dispatch surfaces.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` — three new `#[cfg(test)]` cases on `revalidate_next_step` covering the gate behavior per class.

### Commands

1. `cargo test -p worldwake-ai plan_revalidation`
2. `cargo test -p worldwake-ai`
3. `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
