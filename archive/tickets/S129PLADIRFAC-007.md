# S129PLADIRFAC-007: wash basin-state refactor with partial success

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `wash` action's precondition list, target arity, and commit handler all change. Authoritative-to-AI precondition surface is modified.
**Deps**: archive/tickets/S129PLADIRFAC-001.md, archive/tickets/S129PLADIRFAC-002.md, archive/tickets/S129PLADIRFAC-003.md

## Problem

Today's `wash` action consumes water directly from the well's `ResourceSource` at commit time (`needs_actions.rs:702–721`) and zeros the actor's dirtiness in one binary step. The well-vs-basin asymmetry means: (a) basins carry no per-facility state, so multiple basins at one place are indistinguishable to the agent; (b) wash either succeeds fully or fails the start, with no partial outcome (PR-11's gap); (c) well depletion happens per individual wash event rather than per supply-chain demand. S129's D7 redesigns the action: the basin becomes the consumed water buffer, the well's only role is per-tick refill (ticket 008), and partial success is a first-class outcome when basin water is below `units_per_full_wash`.

This ticket performs the action-level refactor and exercises the Authoritative-to-AI Impact Rule's checklist point 4 (`BestEffort` action start) at the wash start boundary, plus the generic point-5 handoff that existing `handle_plan_failure` already performs for `PreconditionFailed` start failures. Checklist points 1 (`get_affordances`) and 2 (`generate_candidates`) are covered by tickets 003 and 009; checklist point 3 (`search_plan`) emerges from those; point 6 (payload revalidation) is N/A for wash; point 7 (golden tests) is ticket 012.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `wash` action's precondition function `wash_preconditions()` at `crates/worldwake-systems/src/needs_actions.rs:225–250` (called from line 94 — single call site, contained scope per spot-check (g)). Existing preconditions: `TargetExists(0)`, `TargetHasWorkstationTag { target_index: 0, tag: WashBasin }`, `TargetExists(1)`, `TargetHasResourceSource { target_index: 1, commodity: Water, min_available: 1 }`. The last must be replaced with `TargetHasWashBasinClean { target_index: 0, min: 1 }` (ticket 003) — and the now-unused `TargetExists(1)` may also be removed since the second target (water source) is no longer needed by the action.
2. `wash` commit handler at `needs_actions.rs:694–734`. Existing behavior: reads `instance.targets.get(1)` (the water source); calls `txn.get_component_resource_source(source)`; decrements `available_quantity` by 1 via `checked_sub` (failing with `ActionError::PreconditionFailed` if insufficient); writes the updated `ResourceSource` back; zeros agent dirtiness. After this ticket: reads `instance.targets.get(0)` (the basin); reads `WashBasinState` (ticket 001); branches on `clean_water_units` for full / partial / unreachable-zero outcomes.
3. The shared abstraction boundary under audit is the wash action's full Authoritative-to-AI surface: precondition list (`needs_actions.rs:225`), target spec (currently produces two targets — basin index 0, water source index 1; this ticket reduces target arity to one — basin only), commit handler, and the affordance-discovery + candidate-generation paths in tickets 003 and 009. Reducing target arity from 2 to 1 is observable to scenario authoring (no scenario today supplies the wash targets directly — affordance discovery resolves them from `WashBasin` workstations + co-located `Water` `ResourceSource` — but downstream tickets like 009 must produce candidates with the new arity).
4. Existing focused/unit coverage: `wash_consumes_local_water_source_and_clears_dirtiness` (line 1402), `wash_rejects_water_source_without_wash_basin` (line 1551), `wash_accepts_local_basin_and_water_source` (line 1580). All three need rewriting because their assertions hard-code the two-target shape and the well-water-consumption semantics. The `wash_def_id` test helper at line 1462 may also need updating if it builds `wash` targets explicitly.
5. The `forensic_wash_vs_water_competition.rs` golden at `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` — verify during implementation whether its narrative depends on the two-target shape or on basin/water-source contention; the per-basin candidate split (ticket 009) plus this ticket's basin-as-water-buffer change may require golden revision. Classify the divergence per precision-rules §13: if reassessment reveals the golden's actual contract is well-water contention (not basin contention), the golden may need to be rewritten in ticket 012 alongside the new place-dirtiness coverage, with this ticket's scope updated accordingly.
6. Authoritative-to-AI Impact Rule per CLAUDE.md: this ticket is the action-level half of the wash refactor. Checklist points 4 (`BestEffort` start) and 5 (`handle_plan_failure`) are addressed in this ticket. Specifically: when `clean_water_units` drops between affordance discovery and action start (e.g., another agent washed first), `BestEffort` start fails with `ActionError::PreconditionFailed` (driving replan via existing `handle_plan_failure` machinery — no new code there). The fail-then-replan path preserves the candidate-emission gate (ticket 003) as authoritative. Heuristic Removal Discipline (precision-rules §12): this ticket does **not** weaken any existing heuristic — it replaces a precondition with a different precondition that is strictly more accurate (basin-state-aware rather than well-state-aware).
7. Mismatch + correction: the spec's D7 reads `target[0]` for the basin (per spec text); current code reads `target[1]` for the water source. The change is a clean swap of which target is read, plus removal of the second target. Confirm during implementation that no other call site assumes the two-target wash shape.
8. Mismatch + correction (2026-05-01): the original verification layer and acceptance criteria asked this ticket to prove replan onto a different basin after start failure. Live sibling ticket `S129PLADIRFAC-009` still owns per-basin candidate emission, so basin-specific replan selection is not a lawful 007 proof. This ticket keeps point 4 as a direct stale-affordance start-failure proof and relies on the existing generic `handle_plan_failure` mapping for point 5; basin-specific replan/golden proof remains with 009/012.

## Architecture Check

1. Reducing wash to a single basin target (instead of basin + water-source) decouples the action from the well — the well's role is now exclusively to refill the basin per-tick (ticket 008). This is a cleaner supply-chain model: water is conserved through three concrete buffers (well source → basin clean_water → consumed by agent), each with its own state, rather than the agent reaching past the basin to drain the well directly. Per FND-3 (concrete state over abstract scores) and FND-4 (explicit transfer), every step is observable.
2. Partial-success in commit (rather than scaling the action's duration or splitting into "wash" + "partial-wash" actions) keeps the planner surface simple — one action, one precondition gate, two outcome shapes. The proportional reduction `(available / units_per_full_wash) * agent_dirtiness` is concrete arithmetic on the basin's state, not an abstract success score (FND-3, FND-10).
3. No backward-compat shim. The `TargetHasResourceSource` precondition stays in the enum (other actions use it); only `wash`'s use is removed. The previous well-water-consumption code path is **deleted**, not aliased — per FND-28, no shimmed dual-source.

## Verification Layers

1. Precondition swap is observable to affordance discovery → action-trace assertion in a focused test that constructs a wash affordance and asserts the precondition list does not contain `TargetHasResourceSource` and does contain `TargetHasWashBasinClean { target_index: 0, min: 1 }`.
2. Full-success commit consumes `units_per_full_wash` from basin and zeros agent dirtiness → focused unit test seeding `WashBasinState { clean_water_units: 5, units_per_full_wash: 2, dirtiness_per_use: pm(50) }`, agent dirtiness `pm(800)`, run commit, assert `clean_water_units == 3`, agent dirtiness `pm(0)`, basin `dirtiness_level` incremented by `pm(50)`, `WashFacilityUsed { partial: false }` event emitted.
3. Partial-success commit reduces dirtiness proportionally → focused unit test with `clean_water_units: 1, units_per_full_wash: 2`, agent dirtiness `pm(800)`. Assert post-commit: `clean_water_units == 0`, agent dirtiness `pm(400)` (proportional half-reduction), basin `dirtiness_level` incremented by half of `dirtiness_per_use` (`pm(25)`), `WashFacilityUsed { partial: true, water_consumed: 1 }`.
4. Race-condition path (basin emptied between affordance and start) → focused integration test seeding two basins; run wash on basin A; immediately drain basin A's water externally; attempt second wash; assert `BestEffort` start fails with `ActionError::PreconditionFailed`. (This exercises Auth-to-AI checklist point 4.)
5. Authoritative-to-AI checklist point 5 — existing `handle_plan_failure` already maps `ActionStartFailureReason::PreconditionFailed` into blocker/discrepancy classification. Basin-specific replanning onto another wash basin is deferred to ticket 009's per-basin candidate emission and ticket 012's golden proof.

## What to Change

### 1. `wash_preconditions()` in `needs_actions.rs:225–250`

Replace the precondition list. Remove `TargetExists(1)` and `TargetHasResourceSource { ... }`. Add `Precondition::TargetHasWashBasinClean { target_index: 0, min: 1 }`. The final list:

```rust
vec![
    Precondition::ActorAlive,
    Precondition::TargetExists(0),
    Precondition::TargetHasWorkstationTag { target_index: 0, tag: WorkstationTag::WashBasin },
    Precondition::TargetHasWashBasinClean { target_index: 0, min: 1 },
]
```

(Confirm `ActorAlive` and other existing wash preconditions during implementation — list above is illustrative.)

### 2. Reduce wash action target arity

Wherever wash's `TargetSpec` is declared (likely the wash action definition near `needs_actions.rs:94` or in a `wash_def()` helper), reduce the target list from two entries to one. Verify there are no scenario authoring sites that supply wash targets manually — affordance discovery (ticket 009) is the canonical producer.

### 3. Refactor `wash` commit handler at `needs_actions.rs:694–734`

Replace the body. Pseudocode:

```rust
fn commit_wash(...) -> Result<(), ActionError> {
    let basin = instance.targets.first().ok_or(...)?;  // target[0] is the basin facility
    let mut basin_state = txn.get_component_wash_basin_state(*basin).copied().ok_or_else(|| {
        ActionError::InternalError(format!("wash target {} lacks WashBasinState", basin))
    })?;

    if basin_state.clean_water_units == 0 {
        // Race: precondition gate (ticket 003) ruled this out at affordance time, but state may have changed.
        return Err(ActionError::PreconditionFailed(format!("basin {} has no clean water", basin)));
    }

    let actor_needs = txn.get_component_homeostatic_needs(instance.actor).copied().ok_or(...)?;
    let prev_dirtiness = actor_needs.dirtiness;

    let (water_consumed, partial) = if basin_state.clean_water_units >= basin_state.units_per_full_wash {
        // Full success
        (basin_state.units_per_full_wash, false)
    } else {
        // Partial success
        (basin_state.clean_water_units, true)
    };

    // Proportional dirtiness reduction
    let dirtiness_reduction_permille = (u32::from(prev_dirtiness.value())
        * u32::from(water_consumed))
        / u32::from(basin_state.units_per_full_wash);
    let new_dirtiness = prev_dirtiness.saturating_sub(Permille::new_unchecked(dirtiness_reduction_permille as u16));
    let agent_dirtiness_delta = Permille::new_unchecked(prev_dirtiness.value() - new_dirtiness.value());

    // Proportional basin dirtiness increase
    let basin_dirtiness_inc_permille = (u32::from(basin_state.dirtiness_per_use.value())
        * u32::from(water_consumed))
        / u32::from(basin_state.units_per_full_wash);
    let basin_dirtiness_delta = Permille::new_unchecked(basin_dirtiness_inc_permille as u16);

    basin_state.clean_water_units -= water_consumed;
    basin_state.dirtiness_level = basin_state.dirtiness_level.saturating_add(basin_dirtiness_delta);
    txn.set_component_wash_basin_state(*basin, basin_state)?;

    set_actor_needs(txn, instance.actor, HomeostaticNeeds { dirtiness: new_dirtiness, ..actor_needs })?;

    txn.add_tag(EventTag::WashFacilityUsed)
        .set_decision_payload(DecisionEventPayload::WashFacilityUsed(WashFacilityUsedPayload {
        user: instance.actor,
        basin: *basin,
        water_consumed,
        agent_dirtiness_delta,
        basin_dirtiness_delta,
        partial,
    }));

    Ok(())
}
```

Verify the helper APIs (`set_actor_needs`, etc.) match the codebase conventions during implementation.

### 4. Rewrite existing tests in `needs_actions.rs`

- `wash_consumes_local_water_source_and_clears_dirtiness` (line 1402): rename to `wash_full_success_consumes_basin_water_and_clears_dirtiness`. Seed the basin with `WashBasinState`; assert basin water decremented (not well's `ResourceSource`).
- `wash_rejects_water_source_without_wash_basin` (line 1551): the precondition shape changes from "well water minimum" to "basin water minimum". Rename to `wash_rejects_basin_with_zero_clean_water` and update the seeded state accordingly.
- `wash_accepts_local_basin_and_water_source` (line 1580): rename to `wash_accepts_basin_with_sufficient_clean_water`; remove the water-source target setup (it no longer matters to wash).
- `wash_def_id` (line 1462): if this helper supplies the two-target wash shape, update to one target.

### 5. New focused tests

- `wash_partial_success_when_basin_water_below_full_wash` — partial-outcome assertions per spec D7.
- `wash_emits_wash_facility_used_event_with_partial_flag` — payload-flag assertion.
- `wash_race_condition_basin_emptied_between_affordance_and_start_returns_precondition_failed` — stale-affordance start revalidation for Auth-to-AI checklist point 4.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — precondition list at 225, target arity at 94 and/or wash action def, commit handler at 694, three existing tests rewritten, three new tests added)
- Likely: `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` (modify — if the golden's contract depends on the two-target shape; confirm during implementation per Assumption Reassessment §5; if substantial rewrite is needed, defer to ticket 012's scope)

## Out of Scope

- Per-basin candidate emission (deferred to ticket 009).
- Basin natural refill from co-located `ResourceSource` (deferred to ticket 008).
- AI ranking that prefers basins with higher `clean_water_units` and lower `dirtiness_level` (deferred to ticket 010).
- `forensic_wash_vs_water_competition.rs` rewrite if reassessment shows it requires substantial revision — escalate to ticket 012 if so.
- Removal of `TargetHasResourceSource` from the cross-crate `Precondition` enum — that variant stays, other actions use it.

## Acceptance Criteria

### Tests That Must Pass

1. Rewritten `wash_full_success_consumes_basin_water_and_clears_dirtiness` — basin water decremented by `units_per_full_wash`, agent dirtiness zeroed, `WashFacilityUsed { partial: false }` event emitted.
2. Rewritten `wash_rejects_basin_with_zero_clean_water` — affordance/precondition rejects the wash candidate (verify via affordance test, since commit shouldn't be reachable with empty basin).
3. New focused test `wash_partial_success_when_basin_water_below_full_wash` — proportional reduction.
4. New focused test `wash_emits_wash_facility_used_event_with_partial_flag` — payload contents.
5. New focused test `wash_race_condition_basin_emptied_between_affordance_and_start_returns_precondition_failed` — stale-affordance start revalidation path.
6. `forensic_wash_vs_water_competition.rs` either continues to pass or is updated in-scope per Assumption Reassessment §5; if rewrite is substantial, the rewrite is deferred to ticket 012 with explicit scope handoff.
7. Existing suite: `cargo test -p worldwake-systems` and `cargo test -p worldwake-ai`.

### Invariants

1. Wash action has exactly one target (the basin); the second target (water source) is removed from the action's `TargetSpec`.
2. Wash precondition list contains `TargetHasWashBasinClean { target_index: 0, min: 1 }` and does NOT contain any `TargetHasResourceSource` referring to wash's targets.
3. Full-success commit always consumes exactly `units_per_full_wash` from `basin_state.clean_water_units`; partial success consumes exactly `min(clean_water_units, units_per_full_wash)`. No other consumption amounts are produced.
4. Agent dirtiness reduction is proportional to water consumed: `delta = (prev_dirtiness * water_consumed) / units_per_full_wash`. Basin dirtiness increase is proportional in the same way.
5. Every wash commit emits exactly one `WashFacilityUsed` event with the correct `partial` flag — no double emissions.
6. The well's `ResourceSource.available_quantity` is **never** mutated by the wash commit handler — that's now exclusively ticket 008's territory.
7. Per FND-12: well depletion now happens per-tick proportional to basin demand (via ticket 008's refill), not per-action — this retiming is the declared world law.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` — three test rewrites + three new tests covering full success, partial success, and race-condition paths.
2. Possibly: `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` — minimal update if its contract still holds; rewrite deferred to ticket 012 if needed.

### Commands

1. `cargo test -p worldwake-systems wash`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai forensic_wash`
4. `cargo build --workspace`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-01.

Implemented the wash action refactor against the live S129 substrate.

- `crates/worldwake-systems/src/needs_actions.rs`: `wash` now has one basin target, gates on `TargetHasWashBasinClean`, consumes `WashBasinState.clean_water_units`, supports proportional partial success, mutates basin dirtiness, preserves co-located `ResourceSource` water, and emits `WashFacilityUsed`.
- `crates/worldwake-ai/src/planning_snapshot.rs` and `crates/worldwake-ai/src/planning_state.rs`: planning snapshots now carry facility-side `WashBasinState`, with the same default-basin convention used by authoritative wash-basin fixtures.
- `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/tests/golden_ai_decisions.rs`, and `crates/worldwake-ai/tests/planner_conformance.rs`: AI/search/golden/conformance surfaces now use the basin-buffer contract and one-target wash shape.

`crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` did not need an in-scope rewrite; the focused `forensic_wash` lane still passes.

## Deviations / Scope Corrections

- The original ticket expected basin-specific replan proof after a stale-affordance failure. Live sibling ticket `S129PLADIRFAC-009` still owns per-basin candidate emission, so this ticket proves stale-affordance start revalidation and leaves basin-specific replan/golden proof with 009/012.
- The full `worldwake-ai` lane exposed real follow-on surfaces beyond `needs_actions.rs`: planning snapshots/state, relevant-place search, golden wash assertions, and planner conformance all needed basin-state awareness to keep the authoritative-to-AI boundary truthful.

## Verification Result

Focused checks passed:

1. `cargo test -p worldwake-systems --lib wash`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai forensic_wash`
4. `cargo test -p worldwake-ai --lib search::tests::search_local_wash_candidates_require_clean_basin`
5. `cargo test -p worldwake-ai --lib search::tests::search_wash_finds_travel_then_wash_plan_at_believed_access_place`
6. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action`
7. `cargo test -p worldwake-ai --test planner_conformance conformance_wash`
8. `cargo test -p worldwake-ai`

Broad workspace checks passed:

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
