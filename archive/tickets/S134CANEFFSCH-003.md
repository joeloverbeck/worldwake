# S134CANEFFSCH-003: Combat schemas

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds runtime actor/target/action refs to `EffectSchema`, adds typed combat commit effect steps, replaces empty-placeholder schemas with real commit schemas in 7 combat actions, and switches their commit handler bodies to `apply_effects_with_context(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 (Action handler migration) requires every action's imperative handler body to be replaced by a constructed `EffectSchema` and a delegating call to `apply_effects(..., Authoritative)`. This ticket migrates the 7 combat-domain actions in `crates/worldwake-systems/src/combat.rs` (attack, defend, loot, bury, heal, queue_for_corpse_use, queue_for_care_target) — populating their schemas with real preconditions and effect steps, and shrinking their commit handler bodies to a delegation to `apply_effects`. The planner continues to use the old `apply_hypothetical_transition` path; no planner-side change in this ticket. Current combat and survival goldens must remain behaviorally unchanged.

Live reassessment found that the initial S134 substrate could not implement that literally: `ActionDef.effect_schema` is registry-time template data, but `EffectPrecondition` and `EffectStep` stored concrete `EntityId`s. Combat schemas need runtime actor, target, and payload-derived action identities. This ticket therefore owns the prerequisite schema-reference repair required for combat: `EffectEntityRef`, `EffectActionRef`, and a context-aware evaluator entrypoint. Combat-specific steps are typed effect operations interpreted by a combat-owned authoritative sink; they are not wrappers that preserve the old commit body as a second live path.

## Assumption Reassessment (2026-05-04)

1. Combat action registrations live at `crates/worldwake-systems/src/combat.rs` with 7 `register_*_action` functions: `register_attack_action`, `register_defend_action`, `register_loot_action`, `register_bury_action`, `register_heal_action`, `register_queue_for_corpse_use_action`, `register_queue_for_care_target_action`. Each currently has an imperative `commit_*` handler body that mutates ECS through the scheduler.
2. After ticket 001, each `ActionDef` literal in `combat.rs` has `effect_schema: EffectSchema::empty()`. This ticket replaces each empty schema with a real one and switches handler bodies to delegate.
3. Shared abstraction boundary under audit: each combat action's authoritative effect set must produce the same component mutations and event-log entries as today's imperative handler. The schema-driven commit path is the new single authoritative commit surface for these actions.
4. Existing focused/unit coverage exercising combat handlers: tests in `combat.rs` `#[cfg(test)]` block (combat-domain handler tests around `start_attack`, `tick_attack`, `commit_attack`, `abort_attack` etc.) plus broader goldens — `crates/worldwake-ai/tests/golden_combat_*.rs`, `golden_combat_smoke.rs`, `golden_dragon_attack.rs`, `golden_loot_*.rs`, `golden_heal_*.rs`. Enumerate during reassessment via `rg -l "attack\|loot\|heal\|bury\|queue_for_(corpse|care)" crates/worldwake-ai/tests/golden_*.rs`.
5. Behavior-preservation invariant: the schema-driven path must produce the same `EventTag` emissions, the same component mutations (wounds, body parts, contention-grant consumptions, container transfers for loot), and the same canonical state hash post-replay as the pre-ticket imperative path. Current verification exercises this through focused combat tests, live matching goldens, and the full repository verification wrapper.
6. `WoundCause` taxonomy lives at `crates/worldwake-core/src/wounds.rs:44–50`; `WoundSeverity` does NOT exist (per spec D1 type-naming notes). Ticket 002 left `EffectStep::ApplyWound` as a staged variant that both real sinks reject with `Discrepancy::ImproperPlanningState` because the step does not carry enough combat wound payload to construct a real wound. This ticket replaced the combat attack path with `EffectStep::ResolveCombatAttack`.
7. Reassessment correction: registry schemas cannot store only literal runtime `EntityId`s. This ticket adds `EffectEntityRef::{Actor, Target, Entity}` and `EffectActionRef::{CurrentAction, PayloadQueueIntendedAction, Action}` so schemas can be declared on `ActionDef` and resolved at execution time.
8. Reassessment correction: combat attack is not representable by the staged `ApplyWound { target, cause }` shape because the authoritative wound depends on attacker/target combat profiles, needs, existing wounds, target stance, selected weapon payload, tick, and seeded RNG. The landed schema uses `EffectStep::ResolveCombatAttack { attacker, target }` interpreted by the combat-owned authoritative sink.
9. Reassessment correction: the hypothetical sink resolves the new runtime entity refs for existing generic effects, but combat-specific effect methods intentionally remain rejected until ticket 010 implements planner-side parity. This is safe because tickets 003-009 do not switch the planner; ticket 010 remains the owner of hypothetical mode activation and old-path deletion.

## Architecture Check

1. Schema-driven evaluation through `apply_effects_with_context(..., Authoritative)` produces the same authoritative effects as the former imperative commit handler by writing through the same scheduler/event-log write surface. The interpretation layer differs; the world meaning does not (FND-12 — performance compresses computation, never causality).
2. Per-action `EffectSchema` is the single commit truth for the migrated combat actions; the imperative body is removed, not preserved as a fallback (FND-28). No alias path remains.
3. `EffectPrecondition` failures classify into the existing `Discrepancy` taxonomy (S109) rather than introducing combat-specific failure types, keeping the seam with the existing planner fault-handling pipeline (FND-26).

## Verification Layers

1. Behavior-preservation invariant → focused combat tests plus live combat/survival golden selectors exercise event-log and state outcomes after the authoritative migration.
2. Per-action authoritative effects invariant → action trace: `commit_attack`, `commit_loot`, etc. produce the same `ActionTraceSink` events as today (wound application order, contention-grant consumption order).
3. Action-precondition failure invariant → focused runtime/unit test: the schema's `EffectPrecondition`s reject the same cases the imperative handler rejected (verify with adversarial inputs that previously triggered handler-internal validation failures).
4. Canonical state hash invariant → covered by current golden and scenario-coverage verification lanes; a separate pre/post baseline hash-diff run was not captured in this ticket closeout.

## What to Change

### 1. Construct runtime-reference-aware `EffectSchema` literals for the 7 combat actions

In each `register_*_action` function, replace `effect_schema: EffectSchema::empty()` with a real `EffectSchema { preconditions: vec![…], steps: vec![…] }` literal. Schemas use `EffectEntityRef::Actor` and `EffectEntityRef::Target { index }` rather than literal runtime `EntityId`s. Preconditions encode schema-local checks such as co-location; existing action `commit_conditions` and payload validators still own the broader lifecycle admission checks until the category migration proves a stronger fully-schema precondition language.

Per-action landed shape:

- **attack**: `CoLocated { Actor, Target(0) }`; `ResolveCombatAttack { Actor, Target(0) }`.
- **defend**: `ClearCombatStance { Actor }` at commit; start still installs the stance at the correct lifecycle phase.
- **loot**: `CoLocated { Actor, Target(0) }`; `LootPossessionsWithinCapacity { Actor, Target(0) }`; `ClearContentionMembership { Actor, Target(0), CurrentAction }`.
- **bury**: co-location with corpse and grave plot; `BuryCorpse { Target(0), Target(1) }`; `ClearContentionMembership { Actor, Target(0), CurrentAction }`.
- **heal**: `CoLocated { Actor, Target(0) }`; `ClearContentionMembership { Actor, Target(0), CurrentAction }`; `ClearEntityContentionIfNoWounds { Target(0) }`. Healing wound progression remains in `tick_heal`, where medicine consumption and repeated wound reduction lawfully happen over the action duration.
- **queue_for_corpse_use** / **queue_for_care_target**: `CoLocated { Actor, Target(0) }`; `EnqueueContention { Actor, Target(0), PayloadQueueIntendedAction }`.

Combat adds typed commit-effect variants to `EffectStep` and default rejecting methods to `EffectSink`. The authoritative interpretation lives in `combat.rs` because it uses combat-owned helpers and state. The hypothetical sink is updated for runtime entity refs but continues to reject combat-specific steps until ticket 010 switches planner evaluation and proves parity.

### 2. Replace combat commit handler bodies with schema delegation

Each `commit_*` handler in `combat.rs` shrinks to the local combat delegation helper:

```rust
fn commit_attack(...) -> ActionOutcome {
    apply_combat_effect_schema(def, instance, rng, txn)
}
```

The imperative body is deleted, not preserved.

### 3. `EffectStep` and sink-method extensions if needed

If combat surfaces another effect not yet covered by the foundation enum, such as defensive component mutation, add the variant to `EffectStep` in `effect_schema.rs` and implement the authoritative combat sink method. Hypothetical implementation remains explicitly rejected until ticket 010 turns on planner schema evaluation.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — 7 `EffectSchema` literals, 7 commit handler body replacements; existing tests around handlers may need adjustment to construct the new schemas)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs new variants for combat)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export new schema refs/context entrypoint)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (modify if new sink methods are added)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-combat action handlers (per-category tickets 004–009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, `apply_planner_step` (ticket 010).
- Changing `BindingStrictness`, `guard_template`, or `expectation_template` on combat actions (preserved unchanged per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. Live combat and survival golden selectors produce behavior unchanged by the authoritative migration.
2. `cargo test -p worldwake-systems combat` — existing inline tests around combat handlers pass with the schema-driven path.
3. `cargo test -p worldwake-ai golden_survival` — live matching survival goldens pass.
4. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Each combat action has a non-empty `EffectSchema` post-ticket (verified by registry-iteration unit test or by future ticket 010's coverage assertion).
2. No imperative commit handler body remains in `combat.rs` for the 7 migrated actions — each `commit_*` delegates to `apply_combat_effect_schema`, which calls `apply_effects_with_context(..., Authoritative)`.
3. Canonical state hash and generated scenario coverage remain valid under the full verification wrapper after this ticket (FND-12).
4. The planner still uses `apply_hypothetical_transition` after this ticket — no planner-side change yet.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` `#[cfg(test)]` block — modify existing handler tests so they exercise the schema-driven path; add focused tests covering schema-precondition failure cases (e.g., attack with no co-location yields schema-path `Discrepancy::MissingObservation` before commit effects apply).
2. Existing combat goldens — no source change; they verify behavior is unchanged.

### Commands

1. `cargo test -p worldwake-systems combat`
2. `cargo test -p worldwake-ai golden_combat`
3. `cargo test -p worldwake-ai golden_survival`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-04.

The live implementation follows the FOUNDATIONS-aligned Option 1 reassessment:

1. Added runtime-resolvable schema operands (`EffectEntityRef`, `EffectActionRef`) and `EffectEvaluationContext`, with `apply_effects_with_context(...)` as the context-aware evaluator entrypoint.
2. Populated non-empty commit schemas for all 7 combat actions.
3. Replaced all 7 combat commit bodies with `apply_combat_effect_schema(...)`, which invokes `apply_effects_with_context(..., EffectMode::Authoritative)`.
4. Added a combat-owned authoritative effect sink for commit-time combat operations: stance cleanup, contention enqueue/cleanup, corpse loot, burial, attack resolution, and wound-resolution cleanup.
5. Updated the authoritative and hypothetical generic sinks to resolve runtime entity refs for existing generic effects.
6. Left combat-specific hypothetical effect methods explicitly rejected until `S134CANEFFSCH-010`, which remains the owner of planner-side schema activation and old-path deletion.
7. Added focused schema-precondition failure coverage proving a non-colocated combat target is rejected through the schema delegation path before attack commit effects apply.

## Deviations

- The original ticket assumed `ActionDef.effect_schema` could carry concrete runtime `EntityId`s. That was incompatible with registry-time action definitions, so this ticket widened to add runtime schema references before migrating combat.
- `EffectStep::ApplyWound` remains staged. Combat attack uses `ResolveCombatAttack` because wound construction depends on profiles, target stance, payload weapon, existing wounds, tick, and seeded RNG.
- No separate pre/post baseline hash-diff artifact was captured. Behavior preservation was verified through focused combat tests, live golden selectors, and the full repository wrapper.

## Verification Result

Passed:

1. `cargo test -p worldwake-sim effect_schema`
2. `cargo test -p worldwake-systems --lib combat::tests::combat_action_defs_have_non_empty_commit_effect_schemas -- --exact`
3. `cargo test -p worldwake-systems --lib combat::tests::attack_schema_rejects_non_colocated_target_before_commit_effects -- --exact`
4. `cargo test -p worldwake-systems combat`
5. `cargo test -p worldwake-ai golden_combat` (one matching live test)
6. `cargo test -p worldwake-ai golden_survival` (one matching live test)
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `./scripts/verify.sh`
