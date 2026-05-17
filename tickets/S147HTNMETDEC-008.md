# S147HTNMETDEC-008: Planner integration in build_stages

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modifies `build_stages` in `crates/worldwake-ai/src/search/strategic.rs:324` to consult the method selector before flat-GOAP fallback. Updates 7 existing inline tests for new parameters.
**Deps**: `archive/tickets/S147HTNMETDEC-006.md` (MethodRegistry + explicit method binding templates), `archive/tickets/S147HTNMETDEC-007.md` (`select_method` with actor-relative belief evaluation)

## Problem

S147 D4 wires `select_method` into the strategic search so methods can substitute their subgoals into the planner's stage list. Without this integration, the method registry and selector are unreachable from the planner and have no behavioral effect. The integration must (a) preserve the flat-GOAP fallback path exactly (no behavioral regression when no method applies, per spec Non-Goal #4), (b) modify the existing `build_stages` function rather than introducing a parallel `build_stages_with_method` (per reassessment finding M4), and (c) update the 7 existing inline tests in `strategic.rs` for the new parameters.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `build_stages` exists at `crates/worldwake-ai/src/search/strategic.rs:324`. Called from `strategic.rs:119` (single caller). Returns `Vec<StrategicStage>`. Current parameters include the goal and existing planning context; new parameters (`actor: EntityId`, `registry: &MethodRegistry`, `profile: &AgentSchemaContextProfile`, `belief_view: &dyn RuntimeBeliefView`, `motives: &[MotiveSourceRef]`) propagate from the caller. Verify caller signature at line 119 during implementation — it may already carry some of these as it sits inside the agent-tick planning path.
2. Existing inline tests on `build_stages` in `crates/worldwake-ai/src/search/strategic.rs` (test module starts at line 542):
   - `test_single_location_goal_no_travel` (line 1045)
   - `test_multi_location_prerequisite_then_goal` (line 1072)
   - `test_belief_only_excludes_unknown_locations` (line 1139)
   - `test_empty_beliefs_exploration_fallback_uses_barrier_required_variant_for_supported_goal` (line 1177)
   - `test_empty_beliefs_exploration_fallback_uses_generic_variant_for_unsupported_goal` (line 1215)
   - `test_social_query_when_colocated_agents` (line 1249)
   - `test_no_fallback_returns_none` (line 1289)
   All 7 must be updated to pass the new arguments (empty `MethodRegistry`, `AgentSchemaContextProfile::default()`, an empty belief-view stub or real test belief view, and `&[]` for motives).
3. `StrategicStage` and `StrategicStageKind` live at `crates/worldwake-ai/src/search/strategic.rs:46-55`. `SubgoalTemplate` (from `archive/tickets/S147HTNMETDEC-004.md`) expands into `StrategicStage` values via a new helper `template_to_stages` — this helper is part of this ticket's scope (a method's subgoals become a sequence of `StrategicStage`s; the existing strategic-search loop then iterates them).
4. Shared boundary: the planner-to-htn boundary is `select_method()` (ticket 007's contract). This ticket reaches into that contract to dispatch into a method's subgoals; the existing strategic search machinery (stage iteration, tactical descent) is unchanged.
5. The 7 existing tests exercise the flat-GOAP path. They prove that path remains the actual current path (per S147 spec FND-28 alignment), so they must continue to pass after the method-selection branch is added — they should hit the "no method matches" → fall-through to `build_stages_default` (or the existing in-place flat-GOAP logic) path. This is the regression guard for "no behavioral regression on goals without methods" (Non-Goal #4).

## Architecture Check

1. Modifying `build_stages` in place (rather than introducing a parallel `build_stages_with_method`) is cleaner because it preserves single-entry-point semantics — every caller routes through one function. The method-selection branch is a prefix; if no method matches, the function falls through to the existing flat-GOAP logic, which is the current behavior. This satisfies FND-28 (no parallel authority paths).
2. Per the Authoritative-to-AI Impact Rule (AGENTS.md): method selection does NOT modify action preconditions, `validate_*` functions, affordance generation, candidate emission, or goal satisfaction. It modifies which `StrategicStage`s the planner iterates. The 7-point Auth-to-AI checklist does not trigger for this ticket — confirmed during reassessment.
3. No backwards-compatibility shims. The method-selection branch is purely additive within `build_stages`.
4. Per FND-20: method-driven decomposition is "reusable affordance composition" (acceptable). It is not plot progression. Verified during reassessment.

## Verification Layers

1. Method selection branch fires when a method matches → focused unit test that constructs a registry with one method matching `ProduceCommodity{recipe=Bake Bread}`, calls `build_stages`, and asserts the returned stages match the method's subgoals (not the flat-GOAP default).
2. Flat-GOAP fallback fires when no method matches → all 7 existing tests continue to pass with the same returned stages as before (the new parameters are passed in but the method selection returns `None`, falling through to the existing logic).
3. Single-entry-point invariant → `cargo build -p worldwake-ai` succeeds and grep confirms there's still only one `fn build_stages` in `strategic.rs` (no parallel function introduced).
4. End-to-end planner integration → ticket 011 goldens prove the runtime behavior end-to-end.

## What to Change

### 1. Modify `build_stages` signature

Modify `crates/worldwake-ai/src/search/strategic.rs:324`:

```rust
fn build_stages(
    goal: &GoalOffer,
    actor: EntityId,                            // NEW
    registry: &MethodRegistry,                  // NEW
    profile: &AgentSchemaContextProfile,        // NEW
    belief_view: &dyn RuntimeBeliefView,        // NEW
    motives: &[MotiveSourceRef],                // NEW
    /* existing parameters unchanged */
) -> Vec<StrategicStage> {
    if let Some(method) = crate::htn::select_method(actor, goal, registry, profile, belief_view, motives) {
        return method.subgoals.iter()
            .flat_map(|template| template_to_stages(template, goal, belief_view))
            .collect();
    }
    // Existing flat-GOAP decomposition logic (unchanged below this line)
    /* … */
}
```

### 2. Add `template_to_stages` helper

In `strategic.rs` (private, near `build_stages`):

```rust
fn template_to_stages(
    template: &SubgoalTemplate,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
) -> Vec<StrategicStage> {
    match template {
        SubgoalTemplate::AcquireCommodity { commodity, .. } => {
            resolve_commodity(commodity, goal, belief_view)
                .map(|commodity| vec![StrategicStage {
                    kind: StrategicStageKind::Acquire(commodity),
                    places: vec![],
                }])
                .unwrap_or_default()
        }
        SubgoalTemplate::TravelTo(loc_tmpl) => {
            let place = resolve_location(loc_tmpl, goal, belief_view);
            vec![StrategicStage { kind: StrategicStageKind::Goal, places: place.into_iter().collect() }]
        }
        // ... etc per spec D1's SubgoalTemplate variants
    }
}

fn resolve_location(
    tmpl: &LocationTemplate,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
) -> Option<EntityId> {
    // Resolve LocationTemplate variants against the belief view + goal context.
    // First-ship: only the variants used by ticket 006's methods need full implementations.
    /* … */
}

fn resolve_commodity(
    tmpl: &CommodityTemplate,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
) -> Option<CommodityKind> {
    // Resolve GoalCommodity and RecipeInput templates against the goal and belief view.
    /* ... */
}
```

### 3. Update caller at `strategic.rs:119`

Modify the single existing call site to thread the new arguments through from the caller's scope. Verify caller has access to the registry, profile, belief view, and motives; if not, propagate the signature change one level up.

### 4. Update all 7 existing inline tests

Each existing test in `strategic.rs` lines 1045-1289 needs the new arguments. Use a shared test helper:

```rust
#[cfg(test)]
fn empty_registry() -> MethodRegistry { MethodRegistry::default() }

#[cfg(test)]
fn default_profile() -> AgentSchemaContextProfile { AgentSchemaContextProfile::default() }
```

Pass these (plus an empty belief-view stub and `&[]` motives) to each `build_stages` call in the tests. The tests should produce identical `Vec<StrategicStage>` results as before — verifying the "no method matches → fall through" path.

### 5. New test covering method-selection branch

Inline test in `strategic.rs` test module:
- `test_method_selection_substitutes_method_subgoals_into_stage_list` — constructs a registry with `produce_with_gather()` method matching the goal, calls `build_stages`, and asserts returned stages match the method's subgoals (not the default flat-GOAP decomposition).

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (modify — `build_stages` signature + method-selection branch + `template_to_stages` helper + 7 test updates + new method-selection test + caller at line 119)

## Out of Scope

- `MethodPlanAttemptTrace` recording during plan attempts (ticket 009).
- `Discrepancy::MethodFailure` emission from the planner when a method fails (ticket 009 handles trace; emission paths may need separate wiring in agent_tick — defer to ticket 009 if it surfaces).
- Observer rendering of method choice (ticket 010).
- Authoritative-to-AI Impact Rule 7-point coverage — not triggered for this ticket (verified during reassessment; method selection does not modify action preconditions, validation, affordance generation, candidate emission, or goal satisfaction).

## Acceptance Criteria

### Tests That Must Pass

1. All 7 existing inline tests in `strategic.rs` pass with new arguments (regression guard).
2. New `test_method_selection_substitutes_method_subgoals_into_stage_list` passes — method-selection branch fires correctly.
3. Existing suite: `cargo test -p worldwake-ai` passes (planning, agenda, plan repair, all golden tests).
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` clean.

### Invariants

1. `build_stages` has exactly one definition in `strategic.rs` (no parallel function introduced).
2. When `select_method` returns `None`, `build_stages` produces the same `Vec<StrategicStage>` as before this ticket — flat-GOAP fallback is the actual current path.
3. `template_to_stages` is deterministic — same template + goal + belief view → same stages across runs.
4. No floats introduced into the planner integration path (AGENTS.md determinism invariant).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` — modify 7 existing inline tests (new args), add 1 new test (method-selection branch).

### Commands

1. `cargo test -p worldwake-ai --lib search::strategic`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
