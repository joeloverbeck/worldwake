# E12COMHEA-013: Heal action definition + handler

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — primarily `worldwake-systems` combat action registration/handler, plus minimal `worldwake-core` / `worldwake-sim` extensions for treatment semantics if required
**Deps**: `archive/tickets/completed/E12COMHEA-001.md`, `archive/tickets/completed/E12COMHEA-002.md`, `archive/tickets/completed/E12COMHEA-004.md`, `archive/tickets/completed/E12COMHEA-005.md`, `archive/tickets/completed/E12COMHEA-006.md`, `archive/tickets/completed/E12COMHEA-009.md`, `specs/E12-combat-health.md`, `tickets/E12COMHEA-000-index.md`

## Problem

The repo now has the shared wound model, wound progression, death handling, Attack, Defend, and Loot. What is still missing is Heal: a same-place care action that lets one living, capable agent consume Medicine to reduce a wounded agent's bleeding and wound severity without introducing a parallel health abstraction.

## Assumption Reassessment (2026-03-11)

1. This is not a greenfield combat ticket anymore. [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) already implements:
   - `combat_system()`
   - wound progression
   - death detection / `DeadAt`
   - `register_attack_action()`
   - `register_defend_action()`
   - `register_loot_action()`
2. The original dependency references were stale. The foundational E12 combat tickets it depends on are already completed and archived.
3. Concrete action registration in this codebase belongs to the system module that owns the action. Heal should be registered from [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs), not by editing generic sim registries directly.
4. `CommodityKind::Medicine` exists, but its [`CommodityKindSpec`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) currently has no `consumable_profile`. The old ticket's claim that medicine duration could be derived from the existing consumable pattern was incorrect.
5. The current action framework already binds a chosen target through `ActionInstance.targets`. Because Heal only needs the treated target, introducing a redundant `HealActionPayload { target }` would duplicate existing authoritative state rather than improve it.
6. [`ActionDomain::Care`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_domain.rs) already exists and is currently unused. Heal is the first natural fit for that domain.
7. There is currently no precondition or belief-view capability for "target has wounds". If Heal should be filtered out of affordances for unwounded agents, that is a real framework gap, not just a handler concern.
8. There is currently no treatment-specific commodity semantics. If duration and per-tick treatment strength are meant to derive from medicine data, the clean implementation point is a treatment profile on the medicine commodity spec, not aliasing food/drink consumable semantics.
9. [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs) currently exports attack/defend/loot combat registration helpers. Heal will need to join that export surface if tests or integration harnesses register combat actions together.

## Architecture Decision

Implement Heal as a care-domain action in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs), using the existing wound model directly and adding treatment-specific item semantics rather than overloading unrelated food/drink consumable machinery.

This is better than the original ticket framing because:

1. It keeps wounds as the single authoritative bodily-harm carrier. Heal mutates `WoundList` directly instead of introducing `Health`, healing scores, or shadow state.
2. It uses the existing action target binding rather than inventing a payload alias for the same target identity.
3. It keeps treatment local and state-mediated: same-place actor, same-place target, concrete Medicine commodity consumption, and direct wound mutation.
4. It adds reusable treatment semantics at the commodity-spec layer, which is more extensible than special-casing Heal around `consumable_profile` or hardcoded durations.

### Payload Handling

1. Heal should use the bound action target as the authoritative treated target.
2. Do not add `ActionPayload::Heal` unless implementation discovers genuinely new action state that is not already represented by bound targets or local action state.
3. Avoid aliasing Loot or Combat payloads for Heal. If a payload ever becomes necessary, it should be a real Heal payload, not reuse of an unrelated variant.

### Treatment Semantics

1. Do not reuse `CommodityConsumableProfile` for medicine. Eating/drinking semantics are not care semantics.
2. Add explicit medicine-treatment semantics if profile-derived heal timing and strength are required.
3. The treatment profile should drive duration and treatment effect; the target's current wound state should determine how much work remains.

### Affordance Filtering

1. Heal should not rely purely on commit-time rejection for unwounded targets if the framework can cleanly express "has wounds" as a precondition.
2. If adding that precondition requires a small BeliefView expansion, that is a justified general framework improvement because it exposes concrete authoritative state already central to combat and care.

## Revised Scope

This ticket now covers:

1. Heal action definition/registration in the combat module
2. treatment-specific commodity semantics needed for profile-derived healing
3. any minimal framework extension needed to express "target has wounds"
4. Heal handler implementation and tests

Concretely:

1. Add a treatment profile for Medicine in `worldwake-core` if profile-driven treatment timing/effect requires it.
2. Add a Heal action definition in the combat module under `ActionDomain::Care`.
3. Add a reusable precondition for wounded targets if needed for affordance/start-gate correctness.
4. Implement deterministic medicine consumption and wound treatment in the Heal handler.
5. Export Heal registration from `worldwake-systems` if integration harnesses need it.

## What to Change

### 1. Add treatment-specific commodity semantics

Extend [`crates/worldwake-core/src/items.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) with a treatment profile concept for commodities that can be used by Heal.

The profile should support:

- profile-driven treatment timing
- profile-driven bleeding reduction
- profile-driven wound-severity reduction

Medicine should carry that profile. Do not piggyback on food/drink consumable semantics.

### 2. Add any minimal sim semantics needed for Heal

If implementation requires it, extend the action framework with:

- a treatment-oriented duration expression derived from commodity treatment profile plus target wound state
- `Precondition::TargetHasWounds(u8)` and the matching affordance / authoritative validation plumbing

These are justified only if they keep Heal aligned with the existing generic action architecture. Do not widen sim abstractions further than necessary.

### 3. Define Heal ActionDef in combat

- Domain: `ActionDomain::Care`
- Constraints:
  - `ActorAlive`
  - `ActorNotDead`
  - `ActorNotIncapacitated`
  - `ActorNotInTransit`
  - `ActorHasControl`
  - `ActorHasCommodity { kind: Medicine, min_qty: 1 }`
- Targets:
  - one `EntityAtActorPlace { kind: EntityKind::Agent }`
- Preconditions:
  - `ActorAlive`
  - `TargetExists(0)`
  - `TargetAtActorPlace(0)`
  - `TargetAlive(0)`
  - `TargetIsAgent(0)`
  - `TargetHasWounds(0)` if that precondition is introduced
- Duration:
  - derived from medicine treatment profile and target wound state, not a hardcoded constant
- Interruptibility:
  - `InterruptibleWithPenalty`
- Payload:
  - `ActionPayload::None` unless a real non-redundant Heal payload becomes necessary
- Visibility:
  - `VisibilitySpec::SamePlace`

### 4. Register Heal alongside existing combat actions

- Add `register_heal_action(defs, handlers) -> ActionDefId` in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs)
- Export it from [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)

### 5. Implement Heal handler in combat

The handler should:

1. validate that the treated target remains the bound same-place living agent
2. deterministically consume one unit of Medicine from the healer's controlled inventory
3. reduce bleeding first, then reduce severity, using treatment-profile values
4. mutate `WoundList` directly and remove fully healed wounds when appropriate
5. emit the normal public same-place action event through the action pipeline

Implementation should avoid:

- any stored health abstraction
- direct scheduler coupling
- special-case aliases to unrelated payload or consumable systems
- backward-compatibility wrappers

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — treatment profile data and tests)
- `crates/worldwake-core/src/lib.rs` (modify — export new treatment profile type if added)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — only if Heal needs a new duration expression and/or `TargetHasWounds`)
- `crates/worldwake-sim/src/belief_view.rs` (modify — only if `TargetHasWounds` is added)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — only if `TargetHasWounds` is added)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — only if `TargetHasWounds` is added)
- `crates/worldwake-sim/src/action_validation.rs` (modify — only if `TargetHasWounds` is added)
- `crates/worldwake-systems/src/inventory.rs` (optional — only if medicine-consumption helpers are extracted/shared)
- `crates/worldwake-systems/src/needs_actions.rs` (optional — only if medicine-consumption helpers are shared)
- `crates/worldwake-systems/src/combat.rs` (modify — Heal action def/handler/tests)
- `crates/worldwake-systems/src/lib.rs` (modify — export Heal registration)

## Out of Scope

- AI deciding when to heal (E13)
- Medical skill trees, doctor profession logic, or training modifiers
- New medicine commodities beyond the existing `CommodityKind::Medicine`
- Ranged/non-local healing
- Bury/inspect corpse interactions
- Any backward-compatibility alias between treatment semantics and existing food/drink consumable semantics

## Acceptance Criteria

### Tests That Must Pass

1. Heal action is registered from the combat module with the expected care-domain constraints, preconditions, visibility, and non-hardcoded duration semantics
2. Heal is offered only for same-place wounded living agent targets if `TargetHasWounds` is added
3. Heal reduces `bleed_rate_per_tick` on the treated target's wounds
4. Heal reduces wound severity without introducing any stored health component
5. Heal consumes exactly one unit of Medicine from the healer's controlled inventory
6. Heal cannot start without Medicine
7. Heal cannot target dead agents
8. Heal requires co-location
9. Heal duration derives from treatment profile plus target wound state, not a hardcoded constant
10. Fully healed wounds are removed when severity reaches zero
11. The resulting action event is visible at the place through the normal action pipeline
12. Relevant focused suites pass, followed by full workspace tests/lint

### Invariants

1. Conservation holds: Medicine is consumed, not conjured or duplicated
2. No stored `Health` component or healing alias is introduced
3. Treatment logic is profile-driven, not hardcoded magic numbers
4. Heal remains state-mediated and local; it does not query non-local state on behalf of agents
5. No backward-compatibility shims or alias payload paths are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs`
   - treatment profile round-trip / spec coverage for Medicine if a new profile type is added
2. `crates/worldwake-sim/src/action_semantics.rs`
   - duration resolution and/or `TargetHasWounds` coverage if new semantics are added
3. `crates/worldwake-sim/src/affordance_query.rs`
   - Heal precondition filtering coverage if `TargetHasWounds` is added
4. `crates/worldwake-sim/src/action_validation.rs`
   - authoritative `TargetHasWounds` coverage if that precondition is added
5. `crates/worldwake-systems/src/combat.rs`
   - Heal action registration metadata
   - successful same-place treatment that reduces bleeding and severity
   - medicine consumption and conservation behavior
   - dead-target / no-medicine / co-location / no-wounds gate coverage
   - full-heal wound removal behavior

### Commands

1. `cargo test -p worldwake-systems combat::tests`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-03-11

What actually changed:

1. Added explicit treatment semantics to [`crates/worldwake-core/src/items.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) via `CommodityTreatmentProfile`, and attached that profile to `CommodityKind::Medicine`.
2. Added generic Heal-supporting action semantics in `worldwake-sim`: `Precondition::TargetHasWounds`, `DurationExpr::TargetTreatment`, and the corresponding belief-view / affordance / authoritative-validation plumbing.
3. Added `register_heal_action()` plus Heal action definition and handler implementation in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs).
4. Heal now consumes one concrete unit of Medicine on action start, applies deterministic per-tick treatment that reduces bleeding before severity, and removes fully healed wounds.
5. Extracted shared lot-consumption helpers into [`crates/worldwake-systems/src/inventory.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/inventory.rs) and reused them from [`crates/worldwake-systems/src/needs_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs_actions.rs).
6. Exported Heal registration from [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs).

Deviations from the original plan:

1. No `HealActionPayload` was added. Heal uses the bound target as authoritative state, which is cleaner than duplicating target identity in payload.
2. No edits were needed in `crates/worldwake-sim/src/action_def_registry.rs` or `crates/worldwake-sim/src/action_handler_registry.rs`; concrete action registration still belongs in the owning systems module.
3. Medicine duration/effect semantics were not derived from `consumable_profile`, because that would have aliased care semantics onto eating/drinking semantics. A dedicated treatment profile was added instead.
4. Self-healing remains out of scope and is rejected authoritatively in the Heal handler rather than widened into additional generic target-identity semantics in this ticket.

Verification results:

1. `cargo test -p worldwake-systems combat::tests`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
