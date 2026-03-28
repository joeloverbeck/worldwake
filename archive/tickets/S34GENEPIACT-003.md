# S34GENEPIACT-003: verify_belief action handler and runtime registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` new `epistemic_actions.rs` verify handler; minimal `worldwake-sim` runtime duration/view support; registry wiring
**Deps**: S34GENEPIACT-001 (core epistemic types), S34GENEPIACT-002 (action payload types), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

Agents have a `GoalKind::VerifyBelief` vocabulary in core and partial AI scaffolding, but the action framework still has no `verify_belief` action definition or handler. That leaves the epistemic architecture half-built: beliefs can be modeled as things worth verifying, yet no lawful action exists to spend time, observe locally, and update belief/violation state.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is: `VerificationSubject` and `VerifyBeliefPayload` in core/sim, `ActionDef.duration` resolution in sim, `RuntimeBeliefView` affordance/runtime access to `VerificationDispositionProfile`, and authoritative belief/violation mutation in the systems handler.
2. `VerificationSubject` and `VerificationDispositionProfile` already exist in [epistemic.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/epistemic.rs). This ticket must not recreate or rename those types.
3. `ActionPayload::VerifyBelief`, `VerifyBeliefPayload`, and `ActionDomain::Epistemic` already exist in [action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) and [action_domain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_domain.rs). The original ticket overstated the missing scope; the actual gap is handler/runtime integration, not payload/domain creation.
4. `register_all_actions()` and `build_full_action_registries_returns_complete_action_catalog()` in [action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) are still the canonical registry surfaces to extend.
5. `investigate_actions.rs` is still the closest handler precedent for start/tick/commit/abort structure and authoritative payload validation, but `verify_belief` is not architecturally the same action. Investigation consumes an existing `ViolationId`; verification consumes a `VerificationSubject` and may create a new violation. The ticket should borrow handler patterns, not duplicate investigate-specific state shape.
6. The current runtime cannot host profile-driven verify duration yet. `DurationExpr` has `ActorInvestigationDisposition` but no verification-specific variant in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs). `estimate_duration_from_beliefs()` mirrors that limitation in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs). A clean implementation requires a dedicated verification duration path instead of reusing investigation duration.
7. `RuntimeBeliefView` and `PerAgentBeliefView` expose `violation_disposition_profile()` but not `verification_disposition_profile()` in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) and [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs). Because affordance enumeration is runtime-belief-view based, this missing hook is part of the live blocker.
8. `ActionState` currently has only `Empty`, `Heal`, `Investigate`, and `Travel` in [action_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_state.rs). `verify_belief` does not need a new action-local state variant if the canonical binding remains the bound place target plus immutable payload subject. Adding a redundant alias state would violate the repo’s “no aliasing” direction without buying correctness.
9. The belief store already has the right mutation primitives. `AgentBeliefStore::update_entity()` and `build_believed_entity_state()` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) provide the correct direct-observation update surface. `ViolationKind::{EntityMissing,SupplyDepleted}` and `ViolationMemory::record()` already exist in [violation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs).
10. `cargo test -p worldwake-systems -- --list` confirms there is currently no `verify_belief` coverage in `worldwake-systems`; the nearest focused coverage is the `investigate_actions::*` suite. This is a missing focused/unit coverage gap, not just a missing golden scenario.
11. `cargo test -p worldwake-ai -- --list` confirms planner/golden surfaces already know about `GoalKind::VerifyBelief`, and planner conformance currently has no `conformance_verify_belief` test. That means this ticket should stay at the action/runtime layer and leave planner-op integration to ticket 005, but it must keep planner duration inventory green because `worldwake-ai` validates live non-fixed durations.
12. Mismatch + correction: the original ticket said this was a pure `worldwake-systems` handler ticket. The minimal correct scope is cross-crate: add the handler in `worldwake-systems`, add a verification duration/view surface in `worldwake-sim`, and update any compile-time duration inventory tests in `worldwake-ai` that must know about the new non-fixed action duration.

## Architecture Check

1. The clean architecture is a dedicated epistemic handler in a new [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) module plus a dedicated `DurationExpr::ActorVerificationDisposition` runtime path. Reusing investigation duration/profile plumbing would couple two distinct motive families and make future `ask_witness` duration handling harder to reason about.
2. `verify_belief` should bind through the existing action identity surfaces: bound place target plus immutable `VerifyBeliefPayload`. Do not add a parallel `ActionState::VerifyBelief` copy of the same fact unless a later commit needs transient state that the payload/target pair cannot represent.
3. No backward-compatibility shims or alias paths. If a new epistemic duration category exists, it gets its own enum variant and trait method; it does not borrow the investigation path under a misleading name.

## Verification Layers

1. verify at current place updates belief with fresh direct observation -> focused handler test asserting authoritative `AgentBeliefStore`
2. verify detects missing entity and records `ViolationKind::EntityMissing` -> focused handler test asserting authoritative `ViolationMemory`
3. verify detects depleted source and records `ViolationKind::SupplyDepleted` -> focused handler test asserting authoritative `ViolationMemory`
4. verify aborts cleanly when commit conditions fail after start (for example actor no longer at target place) -> focused action lifecycle test asserting stale belief retained and no new violation
5. verify action definition is fully registered and catalog-complete -> `action_registry` test plus `verify_completeness()` via `build_full_action_registries`
6. new non-fixed duration stays planner-readable -> `worldwake-ai` planner duration inventory test

## What to Change

### 1. Add `verify_belief` handler module and registry wiring

Create [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) with:

- `register_verify_belief_action(defs, handlers) -> ActionDefId`
- `verify_belief_action_def(...)`
- `enumerate_verify_belief_payloads(...)`
- `validate_verify_belief_payload_authoritatively(...)`
- `start_verify_belief(...)`
- `tick_verify_belief(...)`
- `commit_verify_belief(...)`
- `abort_verify_belief(...)`

Action contract:

- name: `"verify_belief"`
- domain: `ActionDomain::Epistemic`
- target surface: actor-place bound place target, with payload subject place required to match that target
- duration: dedicated verification disposition duration
- interruptibility: `FreelyInterruptible`
- visibility: `SamePlace`
- no action-local alias state unless implementation proves a real transient need

Commit behavior:

- `EntityLocation { entity, place }`
  - if the entity is still effectively at `place`, refresh the actor’s belief for `entity` using `build_believed_entity_state(..., DirectObservation)`
  - if absent or destroyed, record `ViolationKind::EntityMissing { entity, expected_place: place }`
- `SupplyAvailability { commodity, source, place }`
  - if `source` still exists at `place` and its resource source for `commodity` has `available_quantity > 0`, refresh the belief for `source` using direct observation
  - if the source is absent, destroyed, mismatched, or has zero available quantity, record `ViolationKind::SupplyDepleted { commodity, source, place }`

### 2. Add the minimal runtime support the handler needs

In `worldwake-sim`:

- add `DurationExpr::ActorVerificationDisposition`
- resolve it from `VerificationDispositionProfile::verify_belief_duration_ticks` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
- expose `verification_disposition_profile()` on `RuntimeBeliefView` and implement it in `PerAgentBeliefView`
- teach `estimate_duration_from_beliefs()` about the new duration kind in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)

In `worldwake-ai`, update duration inventory helpers if required by compiler/test coverage so the new live non-fixed duration remains mapped.

### 3. Wire the new module into the systems crate

- export the module from [lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)
- call `register_verify_belief_action()` from [action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)
- extend the full-catalog test to require `"verify_belief"`

## Files to Touch

- `crates/worldwake-systems/src/epistemic_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planner_duration_contract.rs` (modify if required)
- `crates/worldwake-ai/src/planning_state.rs` (modify if required by duration dependency exhaustiveness)

## Out of Scope

- `ask_witness` handler and memory transfer logic — ticket 004
- planner op kinds and `GoalKindPlannerExt` terminal wiring — ticket 005
- candidate generation for `GoalKind::VerifyBelief` — ticket 006
- ranking/motive policy for verification — ticket 007
- golden E2E coverage — ticket 008
- action trace detail expansion for `VerifyBelief` — ticket 009

## Acceptance Criteria

### Tests That Must Pass

1. `verify_belief` refreshes belief for present entity-location subject with `PerceptionSource::DirectObservation`
2. `verify_belief` records `ViolationKind::EntityMissing` when the expected entity is absent at commit
3. `verify_belief` refreshes belief for productive supply source
4. `verify_belief` records `ViolationKind::SupplyDepleted` when the source is empty or absent at commit
5. `verify_belief` rejects authoritative payloads whose subject place does not match the bound target place
6. `verify_belief` is not exposed as an affordance when the actor lacks `VerificationDispositionProfile`
7. `verify_belief` aborts without mutating belief or violation memory when commit conditions fail after start
8. Existing suite: `cargo test -p worldwake-systems`
9. Existing suite: `cargo test -p worldwake-ai planner_duration_inventory_matches_live_non_fixed_planner_surface`

### Invariants

1. `verify_belief` mutates only subjective belief/violation state, never unrelated authoritative world state
2. Verification duration is sourced from `VerificationDispositionProfile`, not from violation/investigation profiles
3. The canonical verification binding remains target place + payload subject; no redundant alias state is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/epistemic_actions.rs` — add focused handler tests for present/missing entity, productive/depleted supply, affordance gating, payload validation, and abort-without-mutation behavior
2. `crates/worldwake-systems/src/action_registry.rs` — require `"verify_belief"` in the full action catalog
3. `crates/worldwake-sim/src/action_semantics.rs` or `crates/worldwake-sim/src/belief_view.rs` — add focused duration-resolution coverage for `ActorVerificationDisposition`
4. `crates/worldwake-ai/src/planner_duration_contract.rs` and/or `crates/worldwake-ai/src/planning_state.rs` — update inventory/exhaustiveness coverage if the new duration category requires it

### Commands

1. `cargo test -p worldwake-systems verify_belief`
2. `cargo test -p worldwake-sim ActorVerificationDisposition`
3. `cargo test -p worldwake-ai planner_duration_inventory_matches_live_non_fixed_planner_surface`
4. `cargo test -p worldwake-systems`
5. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
6. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-28
- Actual changes:
  - added `verify_belief` in `crates/worldwake-systems/src/epistemic_actions.rs`
  - registered the new action in `crates/worldwake-systems/src/action_registry.rs` and exported the module from `crates/worldwake-systems/src/lib.rs`
  - added `DurationExpr::ActorVerificationDisposition` plus runtime belief-view support in `worldwake-sim`
  - added focused tests for entity-location verification, supply verification, affordance gating, payload validation, and same-action abort behavior
- Deviations from original plan:
  - no `ActionState::VerifyBelief` alias state was added; payload + bound place target remained canonical
  - no planner-op or planner-duration-surface change was needed in `worldwake-ai`; the planner duration inventory remains unchanged until ticket 005 exposes `verify_belief` through planner semantics
  - depleted-supply verification refreshes the observed source belief to zero stock before recording `ViolationKind::SupplyDepleted`, which is cleaner than leaving stale positive stock in subjective state
- Verification results:
  - `cargo test -p worldwake-systems verify_belief` passed
  - `cargo test -p worldwake-sim ActorVerificationDisposition` passed
  - `cargo test -p worldwake-ai planner_duration_inventory_matches_live_non_fixed_planner_surface` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy -p worldwake-systems --all-targets -- -D warnings` passed
  - `cargo build --workspace` passed
