# E16BFORLEGJURCON-007: Surface force-claim actions through handler payload affordances

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` runtime belief surface and `worldwake-systems` office action handlers
**Deps**: E16BFORLEGJURCON-003, E16BFORLEGJURCON-004, E16BFORLEGJURCON-006

## Problem

`press_force_claim` and `yield_force_claim` already exist as action defs and authoritative payload validators in [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs), but unlike `bribe` and `threaten` they do not register `with_affordance_payloads(...)`. Because both actions require payloads and have no bound targets, `get_affordances()` currently produces only payload-less base affordances, which are not executable and do not expose concrete office choices to downstream callers.

## Assumption Reassessment (2026-03-22)

1. Mismatch found: there is no special political-affordance aggregation layer in [crates/worldwake-sim/src/affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs). `get_affordances()` generically enumerates bindings from action defs, then expands handler-provided payload variants via `ActionHandler::with_affordance_payloads(...)`.
2. Verified live architecture: `bribe` and `threaten` already follow that handler-local pattern in [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs). `press_force_claim` and `yield_force_claim` do not. Scope should align to that existing extension point instead of adding a new `affordance_query` special case.
3. Mismatch found: `RuntimeBeliefView` does expose `office_data()`, `believed_office_holder()`, `believed_force_controller()`, and `believed_membership()`, but it does not currently expose the actor's authoritative `contests_office` membership. That relation is available authoritatively on `World::offices_contested_by()` in [crates/worldwake-core/src/world/social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/social.rs), so a small runtime-view accessor is needed if affordance enumeration is to stay on the view surface.
4. Verified live tests: [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) already covers authoritative force-claim validation and commit behavior, but it does not yet cover force-claim affordance surfacing through `get_affordances()`.
5. Additional live-boundary note: `PerAgentBeliefView` only surfaces offices that are in the actor's belief store. Focused affordance tests therefore need explicit office-belief seeding; this ticket should not weaken that knowledge boundary.
6. N/A — not an AI regression, ordering, or heuristic ticket.
7. N/A — no heuristic removal.
8. N/A — not a start-failure ticket.
9. N/A — not a political closure ticket.
10. N/A — no ControlSource manipulation.
11. N/A — no golden scenario.
12. Scope correction: this is not a `worldwake-sim/src/affordance_query.rs` aggregator change. It is a handler-local payload-enumeration change plus a minimal runtime-view accessor needed to keep enumeration on the established belief/runtime abstraction boundary.
13. N/A — no cumulative arithmetic.

## Architecture Check

1. Cleaner than a new affordance-query special case: keep payload expansion where the rest of the action-specific affordance logic already lives, on the action handler in [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs). That preserves the generic nature of [crates/worldwake-sim/src/affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs).
2. Cleaner than reading `World` directly from the handler: add one minimal `RuntimeBeliefView` accessor for self-authoritative contested-office membership so affordance enumeration can stay on the runtime-view boundary. That is more extensible than threading `World` into `affordance_payloads(...)` or adding one-off escape hatches.
3. No backward-compatibility shims or alias paths.

## Verification Layers

1. Handler payload enumeration emits concrete `PressForceClaim` payloads only for locally actionable force offices -> focused `office_actions` affordance tests through `get_affordances()`.
2. Duplicate press is suppressed from the affordance surface when the actor already contests the office -> focused `office_actions` affordance test proving the new runtime-view accessor is wired through.
3. `YieldForceClaim` affordances surface only for currently contested offices at the local jurisdiction -> focused `office_actions` affordance tests through `get_affordances()`.
4. Authoritative start/commit validation remains unchanged and still rejects invalid payloads -> existing `office_actions` validation tests.
5. Planner-op mapping and `ClaimOffice` candidate generation remain separate follow-up work in ticket `-008`; this ticket only fixes executable affordance surfacing.

## What to Change

### 1. Add handler-local `enumerate_press_force_claim_payloads`

Register `with_affordance_payloads(enumerate_press_force_claim_payloads)` on the `press_force_claim` handler in [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs).

The enumerator returns `ActionPayload::PressForceClaim` for each office at the actor's current place where:

- `view.office_data(office)` exists
- `office_data.succession_law == SuccessionLaw::Force`
- the actor is at the office jurisdiction
- the actor is eligible under the office's rules using runtime-view facts (`believed_membership` for faction rules, self-authoritative liveness)
- the actor does not already contest the office
- the actor is not already the believed office holder when that belief is confidently self

This keeps affordance discovery on the view surface and prevents surfacing obviously stale/self-conflicting press options.

### 2. Add handler-local `enumerate_yield_force_claim_payloads`

Register `with_affordance_payloads(enumerate_yield_force_claim_payloads)` on the `yield_force_claim` handler. The enumerator returns `ActionPayload::YieldForceClaim` for each office in the actor's authoritative contested-office set whose jurisdiction matches the actor's current place.

### 3. Extend `RuntimeBeliefView` for self-authoritative contest membership

Add a minimal accessor such as `offices_contested_by(actor) -> Vec<EntityId>` to the runtime-view surface in [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) and implement it in [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) for the acting agent only. This avoids bypassing the runtime abstraction just to answer self-authoritative affordance questions.

## Files to Touch

- `crates/worldwake-systems/src/office_actions.rs` (modify — add and register force-claim payload enumerators; add focused affordance tests)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add runtime-view accessor for self-authoritative contested offices)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement contested-office accessor for acting agent)

## Out of Scope

- Planner op semantics — E16BFORLEGJURCON-008
- Candidate generation changes — E16BFORLEGJURCON-008
- Golden tests — E16BFORLEGJURCON-009
- Force control system — E16BFORLEGJURCON-005
- Belief query methods — E16BFORLEGJURCON-006
- Refactoring `get_affordances()` into political special cases

## Acceptance Criteria

### Tests That Must Pass

1. Eligible actor at a force-office jurisdiction receives a concrete `PressForceClaim { office }` payload through `get_affordances()`.
2. No `PressForceClaim` affordance is surfaced when the actor is at the wrong place or already contests the office; eligibility remains belief-mediated and requires the office plus supporting institutional beliefs to be present in the actor's runtime view.
3. Actor currently contesting a local office receives a concrete `YieldForceClaim { office }` payload through `get_affordances()`.
4. No `YieldForceClaim` affordance is surfaced when the actor contests nothing local.
5. Existing authoritative validation tests for press/yield still pass unchanged.
6. Relevant suites pass with real commands verified against the current test layout.

### Invariants

1. Force-claim affordance discovery stays on `RuntimeBeliefView`; no new direct `World` reads are introduced into the generic affordance pipeline.
2. The actor's own `contests_office` membership is treated as self-authoritative runtime state, not as third-party belief inference.
3. `get_affordances()` surfaces executable payload-bearing affordances for payload-required force-claim actions.
4. No existing tests break.

## Tests

### New/Modified Tests

1. `press_force_claim_affordance_surfaces_payload_for_local_eligible_force_office`
Rationale: proves the real `get_affordances()` pipeline now exposes an executable payload-bearing press action for the intended local office.

2. `press_force_claim_affordance_filters_nonlocal_and_duplicate_cases`
Rationale: locks the main suppression edges that would otherwise leak invalid press options into the runtime/AI surface while preserving the existing knowledge boundary.

3. `yield_force_claim_affordance_surfaces_payload_for_local_contested_office`
Rationale: proves the new self-authoritative contested-office accessor is actually feeding the handler enumerator.

4. `yield_force_claim_affordance_ignores_nonlocal_or_absent_claims`
Rationale: ensures yield affordances remain local and do not appear without a real active claim.

### Commands

1. `cargo test -p worldwake-systems office_actions`
2. `cargo test -p worldwake-ai -- --list | rg "force|claim|office|press|yield"`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-03-22
- Actually changed:
  - added `RuntimeBeliefView::offices_contested_by()` with `PerAgentBeliefView` support for self-authoritative contested-office membership
  - registered and implemented handler-local payload enumerators for `press_force_claim` and `yield_force_claim`
  - added focused `get_affordances()` tests in `office_actions.rs`
- Deviations from original plan:
  - did not modify `worldwake-sim/src/affordance_query.rs`; the generic affordance pipeline was already the right architecture
  - preserved the existing knowledge boundary that offices must be present in the actor's belief store before office affordances surface
- Verification results:
  - `cargo test -p worldwake-systems office_actions`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
