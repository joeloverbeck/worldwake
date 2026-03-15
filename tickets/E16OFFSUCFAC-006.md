# E16OFFSUCFAC-006: Implement Bribe, Threaten, DeclareSupport Action Handlers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new action module in worldwake-systems, action registry updates
**Deps**: E16OFFSUCFAC-002, E16OFFSUCFAC-003, E16OFFSUCFAC-004, E16OFFSUCFAC-005

## Problem

E16 defines three new social actions (Bribe, Threaten, DeclareSupport) that operate through the existing action framework. Each needs a full `ActionDef` with preconditions, duration, visibility, and a handler with start/tick/commit/abort lifecycle. This ticket implements the complete action surface for all three.

## Assumption Reassessment (2026-03-15)

1. `ActionDomain::Social` already exists — confirmed, used by Tell action.
2. `EventTag::Social` and `EventTag::Political` already exist — confirmed.
3. `VisibilitySpec::SamePlace` exists — confirmed, used for social visibility.
4. The action handler pattern (start, tick, commit, abort callbacks) is established by Tell, Trade, Combat — confirmed.
5. `loyal_to` relation API exists with weighted `Permille` — confirmed, Bribe/Threaten modify loyalty.
6. `hostile_to` relation API exists — confirmed, Threaten resist sets hostility.
7. `support_declarations` API from E16OFFSUCFAC-003 will be available — dependency.
8. `courage: Permille` on `UtilityProfile` from E16OFFSUCFAC-004 will be available — dependency.
9. `CombatProfile` (wound_capacity, attack_skill_base) exists — confirmed, used for Threaten advantage calculation.
10. Possession transfer APIs exist (used by Trade) — confirmed, Bribe uses same mechanism for goods transfer.
11. `SocialObservationKind` may need new variants for `WitnessedObligation` and `WitnessedConflict` — must verify and add if missing.

## Architecture Check

1. All three actions follow the established `ActionDef` + handler pattern from Tell/Trade/Combat.
2. Bribe reuses possession transfer (like Trade) to enforce conservation invariant.
3. Threaten's yield/resist logic compares combat advantage against courage — simple derived computation, not stored.
4. DeclareSupport writes to the new `support_declarations` relation.
5. All events use `SamePlace` visibility — witnesses can observe and record social observations.
6. No backward-compatibility shims needed.

## What to Change

### 1. Create `crates/worldwake-systems/src/office_actions.rs`

New module containing all three action definitions and handlers.

#### Bribe Action
- **Domain**: `ActionDomain::Social`
- **Preconditions**: Actor alive, target exists, target alive, target is Agent, actor and target co-located, actor possesses offered commodity in offered quantity
- **Duration**: 2 ticks
- **Body cost**: Low (social exertion)
- **Interruptibility**: `FreelyInterruptible`
- **Commit conditions**: Same as preconditions (goods still possessed, still co-located)
- **Commit effects**:
  - Transfer goods from actor to target (possession change — conservation invariant)
  - Increase target's `loyal_to` actor by amount proportional to goods value
  - Emit Bribe event with `SamePlace` visibility and `EventTag::Social`
- **Abort**: No side effects (goods not transferred on abort)
- **Social observation**: Bystanders record `WitnessedObligation`

#### Threaten Action
- **Domain**: `ActionDomain::Social`
- **Preconditions**: Actor alive, target exists, target alive, target is Agent, actor and target co-located, actor has `CombatProfile`
- **Duration**: 1 tick
- **Body cost**: Low
- **Interruptibility**: Not interruptible
- **Commit effects**:
  - Compute actor's combat advantage: `wound_capacity + attack_skill_base` (from `CombatProfile`)
  - Read target's `courage` from `UtilityProfile`
  - If advantage > courage threshold: target yields → increase target's `loyal_to` actor, emit `ThreatenEvent(Yielded)`
  - If advantage <= courage threshold: target resists → set `hostile_to` between actor and target, emit `ThreatenEvent(Resisted)`
  - Visibility: `SamePlace`, tags: `EventTag::Social`
- **Social observation**: Bystanders record `WitnessedConflict`

#### DeclareSupport Action
- **Domain**: `ActionDomain::Social`
- **Preconditions**: Actor alive, actor at office's jurisdiction place, office entity exists, office is vacant (has `vacancy_since` set), agent believes office is vacant (belief-mediated)
- **Duration**: 1 tick
- **Body cost**: None
- **Interruptibility**: Not interruptible
- **Commit effects**:
  - Call `declare_support(actor, office, candidate)` (overwrites any previous declaration)
  - Emit `DeclareSupportEvent` with `SamePlace` visibility and `EventTag::Political`
- **Social observation**: Bystanders record `WitnessedCooperation` between declarer and candidate

### 2. Add `SocialObservationKind` variants if missing

Check if `WitnessedObligation`, `WitnessedConflict`, `WitnessedCooperation` exist. If not, add them to the enum in core.

### 3. Register actions in action catalog

Update `crates/worldwake-systems/src/action_registry.rs` to register `bribe`, `threaten`, and `declare_support` action definitions.

### 4. Wire module into `crates/worldwake-systems/src/lib.rs`

Add `pub mod office_actions;` and update exports.

### 5. Update catalog test

The full-registry test should assert `bribe`, `threaten`, and `declare_support` are present.

## Files to Touch

- `crates/worldwake-systems/src/office_actions.rs` (new — action defs + handlers)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register 3 new actions)
- `crates/worldwake-systems/src/lib.rs` (modify — add module, update exports)
- `crates/worldwake-core/src/` — social observation kind variants if missing (modify)

## Out of Scope

- Succession system that uses DeclareSupport outcomes (E16OFFSUCFAC-007)
- AI planner ops that plan these actions (E16OFFSUCFAC-009)
- AI affordance generation for these actions (E16OFFSUCFAC-009)
- Public order function (E16OFFSUCFAC-008)
- Modifying existing action handlers (Tell, Trade, Combat)

## Acceptance Criteria

### Tests That Must Pass

1. **Bribe**: goods transfer from actor to target on commit, conservation invariant holds.
2. **Bribe**: target's `loyal_to` actor increases after bribe.
3. **Bribe**: bribe rejected if actor doesn't possess offered goods.
4. **Bribe**: bystanders witness `WitnessedObligation`.
5. **Bribe**: abort does NOT transfer goods.
6. **Threaten**: target yields when actor combat advantage exceeds target courage.
7. **Threaten**: target resists when target courage exceeds actor combat advantage.
8. **Threaten**: yield increases target `loyal_to` actor.
9. **Threaten**: resist sets `hostile_to` between actor and target.
10. **Threaten**: bystanders witness `WitnessedConflict`.
11. **DeclareSupport**: sets `support_declarations[(agent, office)] = candidate`.
12. **DeclareSupport**: overwrites previous declaration for same (agent, office).
13. **DeclareSupport**: requires actor at office jurisdiction place.
14. **DeclareSupport**: requires office to be vacant.
15. **DeclareSupport**: bystanders witness `WitnessedCooperation`.
16. All three actions registered in full action catalog with `ActionDomain::Social`.
17. `verify_completeness()` still passes.
18. `cargo clippy --workspace --all-targets -- -D warnings`
19. `cargo test --workspace`

### Invariants

1. Conservation invariant: goods used in bribery are transferred, not created or destroyed.
2. All events use `SamePlace` visibility — no information teleportation.
3. No existing action registrations change behavior.
4. Determinism: no floats, all comparisons use `Permille` integer arithmetic.
5. Threaten yield/resist is a derived computation, not stored.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/office_actions.rs` — focused handler tests for each action's commit/abort/precondition behavior.
2. `crates/worldwake-systems/src/action_registry.rs` — update catalog test to include new actions.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core` (if social observation kinds added)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
